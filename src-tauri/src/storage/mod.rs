use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;
use std::sync::Mutex;

pub mod models;
pub mod repo;

const SCHEMA: &str = include_str!("schema.sql");

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> SqlResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA)?;
        // 迁移：为旧 wechat_account 表添加 persona_id 列
        let _ = conn.execute_batch(
            "ALTER TABLE wechat_account ADD COLUMN persona_id TEXT;"
        );
        // 迁移：为旧 message 表添加 segments 列
        let _ = conn.execute_batch(
            "ALTER TABLE message ADD COLUMN segments TEXT;"
        );
        // 迁移：将旧 workflow 表重命名为 workflows
        let _ = conn.execute_batch(
            "ALTER TABLE workflow RENAME TO workflows;"
        );
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn with_conn<F, T>(&self, f: F) -> SqlResult<T>
    where
        F: FnOnce(&Connection) -> SqlResult<T>,
    {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::InvalidParameterName("database lock poisoned".into())
        })?;
        f(&conn)
    }
}
