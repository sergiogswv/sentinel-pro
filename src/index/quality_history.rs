use crate::index::db::IndexDb;
use rusqlite::params;

pub struct QualityHistory<'a> {
    db: &'a IndexDb,
}

impl<'a> QualityHistory<'a> {
    pub fn new(db: &'a IndexDb) -> Self {
        Self { db }
    }

    pub fn record_metrics(&self, metrics: &FileMetrics) -> anyhow::Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO quality_history (file_path, dead_functions, unused_imports, complexity_score, violations_count, tests_passing) \
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                metrics.file_path,
                metrics.dead_functions,
                metrics.unused_imports,
                metrics.complexity_score,
                metrics.violations_count,
                metrics.tests_passing
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, file_path: &str) -> anyhow::Result<Vec<QualitySnapshot>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT timestamp, dead_functions, unused_imports, complexity_score FROM quality_history WHERE file_path = ? ORDER BY timestamp DESC")?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok(QualitySnapshot {
                timestamp: row.get(0)?,
                dead_functions: row.get(1)?,
                unused_imports: row.get(2)?,
                complexity_score: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

pub struct FileMetrics {
    pub file_path: String,
    pub dead_functions: i32,
    pub unused_imports: i32,
    pub complexity_score: f64,
    pub violations_count: i32,
    pub tests_passing: bool,
}

pub struct QualitySnapshot {
    pub timestamp: String,
    pub dead_functions: i32,
    pub unused_imports: i32,
    pub complexity_score: f64,
}
