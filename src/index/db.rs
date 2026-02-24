use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

pub struct IndexDb {
    conn: Mutex<Connection>,
}

impl IndexDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(14), // SQLITE_CANTOPEN
                        Some(format!("No se pudo crear el directorio padre: {}", e)),
                    )
                })?;
            }
        }
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_tables()?;
        Ok(db)
    }

    fn initialize_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 1. ÍNDICE DE SÍMBOLOS
        conn.execute(
            "CREATE TABLE IF NOT EXISTS symbols (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL,
                kind        TEXT NOT NULL,
                file_path   TEXT NOT NULL,
                line_start  INTEGER,
                line_end    INTEGER,
                language    TEXT,
                framework   TEXT
            )",
            [],
        )?;

        // 2. GRAFO DE LLAMADAS
        conn.execute(
            "CREATE TABLE IF NOT EXISTS call_graph (
                id              INTEGER PRIMARY KEY,
                caller_file     TEXT NOT NULL,
                caller_symbol   TEXT NOT NULL,
                callee_symbol   TEXT NOT NULL,
                line_number     INTEGER
            )",
            [],
        )?;

        // 3. USO DE IMPORTS
        conn.execute(
            "CREATE TABLE IF NOT EXISTS import_usage (
                id          INTEGER PRIMARY KEY,
                file_path   TEXT NOT NULL,
                import_name TEXT NOT NULL,
                import_src  TEXT NOT NULL,
                is_used     BOOLEAN DEFAULT FALSE
            )",
            [],
        )?;

        // 4. HISTORIAL DE CALIDAD
        conn.execute(
            "CREATE TABLE IF NOT EXISTS quality_history (
                id                  INTEGER PRIMARY KEY,
                timestamp           DATETIME DEFAULT CURRENT_TIMESTAMP,
                file_path           TEXT NOT NULL,
                dead_functions      INTEGER DEFAULT 0,
                unused_imports      INTEGER DEFAULT 0,
                complexity_score    REAL DEFAULT 0.0,
                violations_count    INTEGER DEFAULT 0,
                tests_passing       BOOLEAN
            )",
            [],
        )?;

        // 5. ÍNDICE DE ARCHIVOS
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_index (
                file_path       TEXT PRIMARY KEY,
                content_hash    TEXT NOT NULL,
                last_indexed    DATETIME,
                language        TEXT,
                framework       TEXT
            )",
            [],
        )?;

        // Índices para velocidad
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_call_callee ON call_graph(callee_symbol)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_imports_file ON import_usage(file_path)",
            [],
        )?;

        Ok(())
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .expect("Failed to lock database connection")
    }

    /// Returns true if the file_index table has been populated (i.e., indexing has run at least once).
    pub fn is_populated(&self) -> bool {
        let conn = self.lock();
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM file_index LIMIT 1)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|v| v == 1)
        .unwrap_or(false)
    }

    /// Top N symbols: (name, kind, file_path, line_start)
    pub fn get_symbols(&self, limit: usize) -> Vec<(String, String, String, i32)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT name, kind, file_path, line_start FROM symbols ORDER BY file_path, kind LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3).unwrap_or(0),
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Top N call relationships: (caller_file, caller_symbol, callee_symbol)
    pub fn get_call_graph(&self, limit: usize) -> Vec<(String, String, String)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT caller_file, caller_symbol, callee_symbol FROM call_graph LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Top N active imports (is_used = 1 only): (file_path, import_name, import_src)
    pub fn get_import_usage(&self, limit: usize) -> Vec<(String, String, String)> {
        let conn = self.lock();
        let mut stmt = match conn.prepare(
            "SELECT file_path, import_name, import_src FROM import_usage WHERE is_used = 1 LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Clears index tables (for --rebuild): symbols, call_graph, import_usage, file_index.
    /// quality_history is intentionally preserved (audit history survives rebuilds).
    /// Does NOT drop the tables.
    pub fn clear_all(&self) -> rusqlite::Result<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM symbols", [])?;
        conn.execute("DELETE FROM call_graph", [])?;
        conn.execute("DELETE FROM import_usage", [])?;
        conn.execute("DELETE FROM file_index", [])?;
        Ok(())
    }

    /// Number of files currently in the index.
    pub fn indexed_file_count(&self) -> usize {
        let conn = self.lock();
        conn.query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get::<_, i64>(0))
            .map(|v| v as usize)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_db() -> (NamedTempFile, IndexDb) {
        let f = NamedTempFile::new().unwrap();
        let db = IndexDb::open(f.path()).unwrap();
        (f, db)
    }

    #[test]
    fn test_is_populated_false_when_empty() {
        let (_f, db) = make_db();
        assert!(!db.is_populated());
    }

    #[test]
    fn test_is_populated_true_after_file_index_insert() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/main.rs", "abc123"],
            )
            .unwrap();
        }
        assert!(db.is_populated());
    }

    #[test]
    fn test_clear_all_empties_index() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/a.ts", "hash1"],
            )
            .unwrap();
        }
        assert!(db.is_populated());
        db.clear_all().unwrap();
        assert!(!db.is_populated());
    }

    #[test]
    fn test_indexed_file_count() {
        let (_f, db) = make_db();
        assert_eq!(db.indexed_file_count(), 0);
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/a.ts", "h1"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO file_index (file_path, content_hash) VALUES (?, ?)",
                rusqlite::params!["src/b.ts", "h2"],
            )
            .unwrap();
        }
        assert_eq!(db.indexed_file_count(), 2);
    }

    #[test]
    fn test_get_symbols_returns_inserted_row() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO symbols (name, kind, file_path, line_start) VALUES (?, ?, ?, ?)",
                rusqlite::params!["myFn", "function", "src/a.ts", 10i32],
            )
            .unwrap();
        }
        let symbols = db.get_symbols(10);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].0, "myFn");
        assert_eq!(symbols[0].1, "function");
        assert_eq!(symbols[0].2, "src/a.ts");
        assert_eq!(symbols[0].3, 10i32);
    }

    #[test]
    fn test_get_call_graph_returns_inserted_row() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO call_graph (caller_file, caller_symbol, callee_symbol) VALUES (?, ?, ?)",
                rusqlite::params!["src/a.ts", "fnA", "fnB"],
            )
            .unwrap();
        }
        let calls = db.get_call_graph(10);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], ("src/a.ts".to_string(), "fnA".to_string(), "fnB".to_string()));
    }

    #[test]
    fn test_get_import_usage_only_active() {
        let (_f, db) = make_db();
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO import_usage (file_path, import_name, import_src, is_used) VALUES (?, ?, ?, ?)",
                rusqlite::params!["src/a.ts", "UsedSvc", "@app/used", true],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO import_usage (file_path, import_name, import_src, is_used) VALUES (?, ?, ?, ?)",
                rusqlite::params!["src/a.ts", "UnusedSvc", "@app/unused", false],
            )
            .unwrap();
        }
        let imports = db.get_import_usage(10);
        assert_eq!(imports.len(), 1, "only active imports returned");
        assert_eq!(imports[0].1, "UsedSvc");
    }
}
