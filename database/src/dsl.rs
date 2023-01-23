use std::fmt::Display;

#[derive(Clone, Copy)]
pub enum NodeFieldName {
    Name,
    Type,
    Meta,
    Data,
}

#[derive(Clone, Copy)]
pub enum FilterOp {
    Equals,
    Nequals,
    Like,
    In,
}

pub trait ToSql {
    fn to_sql(&self) -> String;
}

pub struct FieldFilter<T: ToSql> {
    op: FilterOp,
    field: T,
    val: String,
}

macro_rules! descriptor_primitive {
    ($field_type:tt, $fn_name:tt, $op_name:tt) => {
        pub fn $fn_name(self, term: &str) -> FieldFilter<$field_type> {
            FieldFilter {
                op: FilterOp::$op_name,
                field: self,
                val: term.into(),
            }
        }
    };
}

impl NodeFieldName {
    descriptor_primitive! {NodeFieldName, eq, Equals}
    descriptor_primitive! {NodeFieldName, ne, Nequals}
    descriptor_primitive! {NodeFieldName, like, Like}
    descriptor_primitive! {NodeFieldName, r#in, In}
}

impl ToSql for NodeFieldName {
    fn to_sql(&self) -> String {
        use NodeFieldName::*;
        match self {
            Name => "name",
            Type => "type",
            Meta => "meta",
            Data => "data",
        }
        .into()
    }
}

impl<T: ToSql> ToSql for FieldFilter<T> {
    fn to_sql(&self) -> String {
        use FilterOp::*;
        let opstr = match self.op {
            Equals => "=",
            Nequals => "!=",
            Like => "LIKE",
            In => "IN",
        };
        format!("({} {} {})", self.field.to_sql(), opstr, self.val)
    }
}
