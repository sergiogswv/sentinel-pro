use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AgentOrchestrator {
    agents: HashMap<String, Arc<dyn Agent>>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register(&mut self, agent: Arc<dyn Agent>) {
        let name = agent.name().to_string();
        self.agents.insert(name.clone(), agent);
        println!("   🤖 Agente registrado: {}", name);
    }

    pub fn get_agent(&self, name: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(name).cloned()
    }

    pub async fn execute_task(
        &self,
        agent_name: &str,
        task: &Task,
        context: &AgentContext,
    ) -> anyhow::Result<TaskResult> {
        if let Some(agent) = self.get_agent(agent_name) {
            println!(
                "   🚀 Ejecutando tarea '{}' con agente: {}",
                task.description, agent_name
            );
            agent.execute(task, context).await
        } else {
            Err(anyhow!("Agente '{}' no encontrado", agent_name))
        }
    }

    #[allow(dead_code)]
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}
