use anyhow::{anyhow, bail, ensure, Context, Result};
use fn_utils::PullResult;
use macros::try_as;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::{collections::HashMap, stringify};
use thiserror::Error;
use toml::Value;

mod dependency_graph;

use crate::conf_dir;
use dependency_graph::DependencyGraph;

pub type StringMap = HashMap<String, Vec<String>>;
pub type BpMap = HashMap<String, FieldBlueprint>;

#[derive(Debug)]
pub struct NpcBuilder {
    constructed_npc: StringMap,
    blueprint: NpcBlueprint,
}

#[derive(Debug, Clone)]
pub struct NpcBlueprint {
    blueprints: BpMap,
    dependency_graph: DependencyGraph,
}

#[derive(Debug, Clone)]
pub struct FieldBlueprint {
    n_selections: usize,
    pub sources: Vec<ChoiceSource>,
}

#[derive(Debug, Clone)]
pub struct ChoiceSource {
    options: Vec<String>,
    pub filter: ChoiceFilter,
}

#[derive(Debug, Clone)]
pub enum ChoiceFilter {
    FieldValue {
        target_field: String,
        target_value: String,
    },
    None,
}

macro_rules! try_field_as {
    ($obj: ident, $field: literal, $type: ident) => {{
        let field = $obj
            .get($field)
            .ok_or_else(|| anyhow!("No field named {:?}", $field))?;
        try_as!(field, $type)
    }};
}
pub(crate) use try_field_as;

#[derive(Error, Debug)]
pub enum SetFieldError {
    #[error("got {0} values, expected {1}")]
    WrongN(usize, usize),

    #[error("{0} is not a valid value. Valid values are:\n{1:?}")]
    InvalidValue(String, Vec<String>),

    #[error("The NPC is already completed")]
    NPCCompleteError,
}

impl NpcBlueprint {
    pub fn parse(toml_val: Value) -> Result<NpcBlueprint> {
        let tab = try_as!(toml_val, table)?;
        let blueprints = HashMap::from_iter(
            tab.into_iter()
                .map(|(k, v)| (k.clone(), FieldBlueprint::parse(v.clone()))),
        )
        .pull_result()?;

        let dependency_graph = DependencyGraph::from_blueprints(&blueprints)?;
        Ok(NpcBlueprint {
            blueprints,
            dependency_graph,
        })
    }
}

impl NpcBuilder {
    pub fn new(blueprint: NpcBlueprint) -> NpcBuilder {
        NpcBuilder {
            constructed_npc: HashMap::new(),
            blueprint,
        }
    }
    /// returns the name of the current field, the values that are allowed, and the number of
    /// values that should be set for this field.
    /// Returns None, if the NPC is complete.
    pub fn current_field_infos(&self) -> Option<(String, Vec<String>, usize)> {
        let fields = self
            .blueprint
            .dependency_graph
            .get_available_unset_fields(&self.constructed_npc);
        if fields.len() > 0 {
            let field = &fields[0];
            let bp = &self.blueprint.blueprints[field];
            let opts = bp
                .sources
                .iter()
                .filter_map(|src| match &src.filter {
                    ChoiceFilter::FieldValue {
                        target_field,
                        target_value,
                    } if self.constructed_npc[target_field].contains(target_value) => {
                        Some(src.options.clone())
                    }
                    ChoiceFilter::None => Some(src.options.clone()),
                    _ => None,
                })
                .flatten()
                .collect();
            Some((field.to_owned(), opts, bp.n_selections))
        } else {
            None
        }
    }

    /// proceeds to build an NPC. Accepts a value, which will be set for the current field.
    /// Checks if the value is a valid value, if so returns an option, which will contain the
    /// NPC if building is done, and None otherwise
    pub fn set_current_field_val(
        &mut self,
        values: Vec<String>,
    ) -> StdResult<Option<StringMap>, SetFieldError> {
        match self.current_field_infos() {
            Some((field, opts, n)) => {
                if values.len() != n {
                    Err(SetFieldError::WrongN(values.len(), n))
                } else if values.iter().all(|v| opts.contains(v)) {
                    self.constructed_npc.insert(field, values);
                    if self.npc_completed() {
                        Ok(Some(self.constructed_npc.clone()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(SetFieldError::InvalidValue(
                        values
                            .iter()
                            .filter(|v| !opts.contains(v))
                            .next()
                            .unwrap()
                            .into(),
                        opts,
                    ))
                }
            }
            None => Err(SetFieldError::NPCCompleteError),
        }
    }

    pub fn npc_completed(&self) -> bool {
        self.blueprint
            .blueprints
            .keys()
            .all(|k| self.constructed_npc.contains_key(k))
    }
}

impl FieldBlueprint {
    fn simple(cs: ChoiceSource) -> Self {
        FieldBlueprint {
            n_selections: 1,
            sources: vec![cs],
        }
    }

    fn parse(toml_val: Value) -> Result<FieldBlueprint> {
        match toml_val {
            Value::String(s) => Ok(FieldBlueprint::simple(choice_source_from_file(s)?)),
            Value::Table(tab) => {
                let n_selections = if let Some(n_val) = tab.get("n") {
                    try_as!(n_val, integer)?
                } else {
                    1
                };

                let sources = parse_choice_sources(tab)?;
                Ok(FieldBlueprint {
                    n_selections: n_selections.try_into()?,
                    sources,
                })
            }
            Value::Array(array) => Ok(FieldBlueprint::simple(ChoiceSource::from_array(array)?)),
            otherwise => Err(anyhow!("Unexpected toml node: {:#?}", otherwise)),
        }
    }
}

fn parse_choice_sources(tab: toml::value::Table) -> Result<Vec<ChoiceSource>> {
    // either the table has a file key, or it has a choices key. Or it is invalid
    // a file key means we load a choice frm file without filter, a choices key is an array of
    // tables, which each represent a choice source
    let has_file = tab.contains_key("file");
    let has_choices = tab.contains_key("choices");

    if has_file && !has_choices {
        Ok(vec![choice_source_from_file(try_field_as!(
            tab, "file", str
        )?)?])
    } else if !has_file && has_choices {
        let choice_array = try_field_as!(tab, "choices", array)?;
        Ok(choice_array
            .into_iter()
            .map(|ca| {
                try_as!(ca, table)
                    .cloned()
                    .and_then(ChoiceSource::from_table)
            })
            .collect::<Vec<Result<ChoiceSource>>>()
            .pull_result()?)
    } else {
        bail!(
            "A field must have either a file key or a choices key, but not both. Problem:\n{:#?}",
            tab
        )
    }
}

impl ChoiceSource {
    fn from_path(p: impl AsRef<Path>) -> Result<Self> {
        let p: &Path = p.as_ref();
        let contents = std::fs::read_to_string(p).context(p.display().to_string())?;
        let values = contents
            .lines()
            .filter_map(|l| {
                let clean = l.split('#').next().unwrap().trim();
                if clean.len() > 0 {
                    Some(clean.into())
                } else {
                    None
                }
            })
            .collect();
        Ok(ChoiceSource::from_strings(values))
    }

    fn from_array(a: Vec<Value>) -> Result<Self> {
        let values = a
            .into_iter()
            .map(|v| try_as!(v, str).map(|x| x.into()))
            .collect::<Vec<Result<String>>>()
            .pull_result()?;
        Ok(ChoiceSource::from_strings(values))
    }

    fn from_strings(vals: Vec<String>) -> ChoiceSource {
        ChoiceSource {
            options: vals,
            filter: ChoiceFilter::None,
        }
    }

    fn from_table(tab: toml::value::Table) -> Result<Self> {
        let has_file = tab.contains_key("file");
        let has_values = tab.contains_key("values");

        let mut result = if has_file && !has_values {
            let path = try_field_as!(tab, "file", str)?;
            choice_source_from_file(path)?
        } else if !has_file && has_values {
            let vals = try_field_as!(tab, "values", array)?;
            ChoiceSource::from_array(vals.clone())?
        } else {
            bail!("a choice source must have a file or a values entry, but not both");
        };

        if let Some(filter_val) = tab.get("filter") {
            result.filter = ChoiceFilter::from_str(try_as!(filter_val, str)?)?;
        }

        Ok(result)
    }
}

impl ChoiceFilter {
    fn from_str(src: &str) -> Result<Self> {
        let splits: Vec<&str> = src.split(':').map(|x| x.trim()).collect();
        ensure!(
            splits.len() == 2,
            "a filter definition must contain exactly one colon, yet I found:\n{}",
            src
        );
        Ok(ChoiceFilter::FieldValue {
            target_field: splits[0].into(),
            target_value: splits[1].into(),
        })
    }
}

fn relative_to_conf_file(p: impl AsRef<Path>) -> Result<PathBuf> {
    let p: &Path = p.as_ref();
    ensure!(p.is_relative(), "{} is not a relative path", p.display());
    Ok(conf_dir().join(p))
}

fn choice_source_from_file(p: impl AsRef<Path>) -> Result<ChoiceSource> {
    ChoiceSource::from_path(relative_to_conf_file(p)?)
}

pub fn load_blueprints_from_table(
    tab: toml::value::Table,
) -> Result<HashMap<String, NpcBlueprint>> {
    let entries = tab.into_iter().map(|(k, v)| (k, NpcBlueprint::parse(v)));
    HashMap::from_iter(entries).pull_result()
}
