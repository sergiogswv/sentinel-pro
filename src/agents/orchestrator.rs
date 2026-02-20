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
            let summary = task.description.lines().next().unwrap_or("");
            println!(
                "   🚀 Ejecutando: {} (Agente: {})",
                summary, agent_name
            );
            agent.execute(task, context).await
        } else {
            Err(anyhow!("Agente '{}' no encontrado", agent_name))
        }
    }

    pub async fn execute_with_guard(
        &self,
        agent_name: &str,
        task: &Task,
        context: &AgentContext,
    ) -> anyhow::Result<TaskResult> {
        let result = self.execute_task(agent_name, task, context).await?;

        if !result.success || result.artifacts.is_empty() {
            return Ok(result);
        }

        if let Some(original_code) = &task.context {
            if let Some(new_code) = result.artifacts.first() {
                if let Some(reviewer) = self.get_agent("ReviewerAgent") {
                    use colored::*;
                    println!("   🛡️  BusinessLogicGuard: Verificando que no se haya roto la lógica de negocio...");
                    
                    let guard_task = Task {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: "Compara el CÓDIGO ORIGINAL y el CÓDIGO NUEVO. TU ÚNICA SALIDA DEBE SER 'BUSINESS_LOGIC_CHANGED: YES' si la lógica, reglas de negocio o validaciones críticas cambiaron, se rompieron o desaparecieron. Si solo se refactorizó, aplicaron buenas prácticas o se arregló un bug sin romper nada, responde 'BUSINESS_LOGIC_CHANGED: NO'. Sé estricto pero justo.".to_string(),
                        task_type: crate::agents::base::TaskType::Review,
                        file_path: task.file_path.clone(),
                        context: Some(format!("CÓDIGO ORIGINAL:\n{}\n\nCÓDIGO NUEVO:\n{}", original_code, new_code)),
                    };

                    let guard_result = reviewer.execute(&guard_task, context).await?;
                    if guard_result.output.contains("BUSINESS_LOGIC_CHANGED: YES") {
                        println!("   ❌ {} El código modificado parece alterar la lógica de negocio. Para prevenir regresiones, la operación fue cancelada.", "ALERTA BUSINESS LOGIC:".red().bold());
                        return Err(anyhow::anyhow!("BusinessLogicGuard detectó cambios riesgosos en la lógica de negocio."));
                    } else {
                        println!("   ✅ {} Aprobado. Las reglas de negocio permanecen intactas.", "BusinessLogicGuard:".green().bold());
                    }
                }
            }
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}
