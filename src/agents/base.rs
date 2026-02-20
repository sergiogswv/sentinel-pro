use crate::config::SentinelConfig;
use crate::index::IndexDb;
use crate::stats::SentinelStats;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub file_path: Option<PathBuf>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Analyze,
    Generate,
    Refactor,
    Fix,
    Test,
    Review,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: String,
    pub files_modified: Vec<PathBuf>,
    pub artifacts: Vec<String>,
}

pub struct AgentContext {
    pub config: Arc<SentinelConfig>,
    pub stats: Arc<Mutex<SentinelStats>>,
    pub project_root: PathBuf,
    pub index_db: Option<Arc<IndexDb>>,
}

impl AgentContext {
    pub fn build_rag_context(&self, file_path: &std::path::Path) -> String {
        let mut ctx = String::new();
        if let Some(ref db) = self.index_db {
            let rel_path = file_path.strip_prefix(&self.project_root).unwrap_or(file_path).to_string_lossy().to_string();
            let symbol_table = crate::index::symbol_table::SymbolTable::new(db);
            let call_graph = crate::index::call_graph::CallGraph::new(db);
            
            if let Ok(symbols) = symbol_table.get_file_symbols(&rel_path) {
                if !symbols.is_empty() {
                    ctx.push_str("SÍMBOLOS DEFINIDOS EN ESTE ARCHIVO:\n");
                    for sym in symbols {
                        ctx.push_str(&format!(" - [{}] {} (línea {})\n", sym.kind, sym.name, sym.line_start + 1));
                    }
                    ctx.push('\n');
                }
            }
            
            if let Ok(dead_code) = call_graph.get_dead_code(Some(&rel_path)) {
                if !dead_code.is_empty() {
                    ctx.push_str("POSIBLE CÓDIGO MUERTO (Funciones declaradas que no parecen ser llamadas desde este u otros archivos indexados):\n");
                    for f in dead_code {
                        ctx.push_str(&format!(" - {}\n", f));
                    }
                    ctx.push('\n');
                }
            }
        }
        ctx
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn description(&self) -> &str;

    /// Ejecuta una tarea asignada al agente
    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult>;
}
