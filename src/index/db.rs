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
}
