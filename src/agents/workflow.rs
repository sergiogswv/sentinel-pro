use serde::{Deserialize, Serialize};
use crate::agents::base::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub agent: String, // Nombre del agente a usar (e.g., "CoderAgent")
    pub task: Task,
}
