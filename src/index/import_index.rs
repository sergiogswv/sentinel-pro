use crate::index::db::IndexDb;
use rusqlite::params;

pub struct ImportIndex<'a> {
    db: &'a IndexDb,
}

impl<'a> ImportIndex<'a> {
    pub fn new(db: &'a IndexDb) -> Self {
        Self { db }
    }

    pub fn mark_as_used(&self, file_path: &str, import_name: &str) -> anyhow::Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "UPDATE import_usage SET is_used = 1 WHERE file_path = ? AND import_name = ?",
            params![file_path, import_name],
        )?;
        Ok(())
    }

    pub fn get_unused_imports(&self, file_path: &str) -> anyhow::Result<Vec<String>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT import_name FROM import_usage WHERE file_path = ? AND is_used = 0")?;
        let rows = stmt.query_map(params![file_path], |row| row.get(0))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}
