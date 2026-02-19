use crate::config::SentinelConfig;
use crate::kb::ContextBuilder;
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
    pub context_builder: Option<Arc<ContextBuilder>>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn description(&self) -> &str;

    /// Ejecuta una tarea asignada al agente
    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult>;
}
