use crate::index::db::IndexDb;
use rusqlite::params;

pub struct SymbolTable<'a> {
    db: &'a IndexDb,
}

impl<'a> SymbolTable<'a> {
    pub fn new(db: &'a IndexDb) -> Self {
        Self { db }
    }

    pub fn find_symbol(&self, name: &str) -> anyhow::Result<Vec<SymbolInfo>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT name, kind, file_path, line_start FROM symbols WHERE name = ?")?;
        let rows = stmt.query_map(params![name], |row| {
            Ok(SymbolInfo {
                name: row.get(0)?,
                kind: row.get(1)?,
                file_path: row.get(2)?,
                line_start: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_file_symbols(&self, file_path: &str) -> anyhow::Result<Vec<SymbolInfo>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT name, kind, file_path, line_start FROM symbols WHERE file_path = ?")?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok(SymbolInfo {
                name: row.get(0)?,
                kind: row.get(1)?,
                file_path: row.get(2)?,
                line_start: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line_start: i32,
}
