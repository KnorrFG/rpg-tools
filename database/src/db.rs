use std::ops::{Deref, DerefMut};
use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, Row};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};

use crate::dsl::ToSql;
use crate::schema::*;

use fn_utils::{PullResult, WrapIter};

macro_rules! migrations {
    () => {
        Migrations::new(vec![M::up(CREATE_STMT)])
    };
}

pub struct DB {
    conn: Connection,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub meta: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Link {
    pub id: i64,
    pub left: i64,
    pub right: i64,
    pub r#type: String,
    pub data: Option<Vec<u8>>,
}

impl DB {
    pub fn new(path: &Path) -> Result<DB> {
        let mut conn = Connection::open(path)?;
        migrations!().to_latest(&mut conn)?;
        Ok(DB { conn })
    }

    pub fn insert_node(
        &mut self,
        name: &str,
        r#type: &str,
        meta: Option<String>,
        data: &[u8],
    ) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("insert into nodes (name, type, meta, data) values (?, ?, ?, ?)")?;
        stmt.execute((name, r#type, meta, data))?;
        Ok(())
    }

    pub fn select_nodes<T: ToSql>(&mut self, filter: &T) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare(&format!(
            "select (rowid as id, name, type, meta, data) from nodes where {}",
            filter.to_sql()
        ))?;

        let res = Ok(stmt
            .query_map((), |row: &Row<'_>| -> rusqlite::Result<Node> {
                let f = || {
                    Ok(Node {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        r#type: row.get(2)?,
                        meta: row.get(3)?,
                        data: row.get(4)?,
                    })
                };
                f()
            })?
            .wrap_iter()
            .pull_result()?);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_stuff() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        migrations!().to_latest(&mut conn)?;
        let mut db = DB { conn };
        db.insert_node("Node1", "test", Some("meta info".into()), &vec![])?;
        db.insert_node("Node2", "test", None, &vec![1, 2, 10])?;
        Ok(())
    }
}
