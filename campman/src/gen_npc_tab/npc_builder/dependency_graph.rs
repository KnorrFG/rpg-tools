use std::rc::Rc;

use super::*;
use anyhow::{ensure, Result};
use many_to_many::ManyToMany;

/// represents a directed graph, with multiple root nodes
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// nodes that don't depend on other nodes
    roots: Vec<String>,
    /// left depends on right
    dependencies: Rc<ManyToMany<String, String>>,
}

impl DependencyGraph {
    pub fn from_blueprints(bps: &BpMap) -> Result<Self> {
        let mut roots = vec![];
        let mut dependencies = ManyToMany::new();

        for (current_field, blueprint) in bps {
            let field_deps = blueprint
                .sources
                .iter()
                .filter_map(|cs| match &cs.filter {
                    ChoiceFilter::None => None,
                    ChoiceFilter::FieldValue {
                        target_field,
                        target_value: _,
                    } => Some(target_field.clone()),
                })
                .collect::<Vec<String>>();
            if field_deps.len() == 0 {
                roots.push(current_field.clone());
            } else {
                for dep in field_deps {
                    dependencies.insert(current_field.into(), dep.into());
                }
            }
        }
        ensure!(
            roots.len() > 0,
            "There are no fields that don't depend on other fields. This won't work"
        );
        Ok(DependencyGraph {
            roots,
            dependencies: Rc::new(dependencies),
        })
    }

    pub fn get_depending_fields(&self, dependant: &String) -> Vec<String> {
        self.dependencies.get_right(dependant).unwrap_or(vec![])
    }

    pub fn get_available_unset_fields(&self, npc: &StringMap) -> Vec<String> {
        self.roots
            .iter()
            .cloned()
            .chain(self.get_determined_fields(npc))
            .filter(|f| !npc.contains_key(f))
            .collect()
    }

    pub fn get_determined_fields(&self, npc: &StringMap) -> Vec<String> {
        let fields_with_deps = self.dependencies.get_left_keys();
        let mut res = vec![];
        for field in fields_with_deps {
            let deps = self.dependencies.get_left(field).unwrap_or(vec![]);
            if deps.iter().all(|f| npc.contains_key(f)) {
                res.push(field.clone())
            }
        }
        res
    }
}
