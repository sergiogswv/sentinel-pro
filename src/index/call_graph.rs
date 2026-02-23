use crate::index::db::IndexDb;
use rusqlite::params;

pub struct CallGraph<'a> {
    db: &'a IndexDb,
}

impl<'a> CallGraph<'a> {
    pub fn new(db: &'a IndexDb) -> Self {
        Self { db }
    }

    pub fn get_dead_code(&self, file_path: Option<&str>) -> anyhow::Result<Vec<String>> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        if let Some(path) = file_path {
            let mut stmt = conn.prepare(
                "SELECT name FROM symbols \
                 WHERE kind IN ('function', 'method') \
                 AND file_path = ? \
                 AND name NOT IN (SELECT callee_symbol FROM call_graph)",
            )?;
            let rows = stmt.query_map(params![path], |row| row.get(0))?;
            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT name FROM symbols \
                 WHERE kind IN ('function', 'method') \
                 AND name NOT IN (SELECT callee_symbol FROM call_graph)",
            )?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }

    /// Returns true if `symbol` is called from any file OTHER than `file_path`.
    /// Used to suppress DEAD_CODE false positives for cross-file symbols.
    pub fn is_called_from_other_file(&self, symbol: &str, file_path: &str) -> bool {
        let conn = self.db.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM call_graph \
             WHERE callee_symbol = ? AND caller_file != ?",
            rusqlite::params![symbol, file_path],
            |row| row.get(0),
        ).unwrap_or(0);
        count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::IndexDb;
    use tempfile::NamedTempFile;

    fn make_db() -> (NamedTempFile, std::sync::Arc<IndexDb>) {
        let f = NamedTempFile::new().unwrap();
        let db = std::sync::Arc::new(IndexDb::open(f.path()).unwrap());
        (f, db)
    }

    #[test]
    fn test_get_dead_code_with_special_chars_does_not_panic() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        let result = cg.get_dead_code(Some("src/user's-service.ts"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_dead_code_returns_empty_when_no_symbols() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        let result = cg.get_dead_code(None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_is_called_from_other_file_false_when_no_callers() {
        let (_f, db) = make_db();
        let cg = CallGraph::new(&db);
        // Empty call_graph table → must return false
        assert!(!cg.is_called_from_other_file("myFunction", "src/app.service.ts"));
    }
}
