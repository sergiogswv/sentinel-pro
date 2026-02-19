use crate::agents::base::{AgentContext, Task, TaskResult, TaskType};
use crate::agents::orchestrator::AgentOrchestrator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub task_template: TaskTemplate, // Plantilla para crear la tarea
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub description: String,
    pub task_type: TaskType,
}

pub struct WorkflowContext {
    pub _shared_memory: HashMap<String, String>,
    pub step_results: Vec<TaskResult>,
    pub current_file: Option<String>,
}

impl WorkflowContext {
    pub fn new(initial_file: Option<String>) -> Self {
        Self {
            _shared_memory: HashMap::new(),
            step_results: Vec::new(),
            current_file: initial_file,
        }
    }
}

pub struct WorkflowEngine {
    orchestrator: AgentOrchestrator,
}

impl WorkflowEngine {
    pub fn new(orchestrator: AgentOrchestrator) -> Self {
        Self { orchestrator }
    }

    pub async fn execute_workflow(
        &self,
        workflow: &Workflow,
        agent_context: &AgentContext,
        initial_file: Option<String>,
    ) -> anyhow::Result<WorkflowContext> {
        println!("🚀 Iniciando Workflow: {}...", workflow.name);
        
        let mut wf_context = WorkflowContext::new(initial_file);

        for (i, step) in workflow.steps.iter().enumerate() {
            println!("\n   ➡️  Paso {}: {} ({})", i + 1, step.name, step.agent);

            // Construir la tarea real basada en la plantilla y el contexto actual
            let mut description = step.task_template.description.clone();
            
            // Reemplazar variables simples en la descripción
            if let Some(file) = &wf_context.current_file {
                description = description.replace("{file}", file);
            }
            
            // Si hay un resultado previo, inyectarlo en el contexto de la nueva tarea
            let previous_output = if let Some(last_result) = wf_context.step_results.last() {
                Some(format!("Resultado del paso anterior:\n{}", last_result.output))
            } else {
                None
            };
            
            // Leer archivo actual si existe para contexto
            let file_content = if let Some(file) = &wf_context.current_file {
                 std::fs::read_to_string(file).ok()
            } else {
                None
            };
            
            let combined_context = match (file_content, previous_output) {
                (Some(fc), Some(po)) => Some(format!("Archivo:\n{}\n\nContexto previo:\n{}", fc, po)),
                (Some(fc), None) => Some(fc),
                (None, Some(po)) => Some(po),
                (None, None) => None,
            };

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description,
                task_type: step.task_template.task_type.clone(),
                file_path: wf_context.current_file.as_ref().map(std::path::PathBuf::from),
                context: combined_context,
            };

            // Mostrar progreso
            let pb = crate::ui::crear_progreso(&format!("Ejecutando paso: {}...", step.name));

            // Ejecutar el paso
            let result = self.orchestrator.execute_task(&step.agent, &task, agent_context).await;
            
            pb.finish_and_clear();

            match result {
                Ok(result) => {
                    // Si el agente generó artefactos (código), podríamos querer guardarlos AUTOMÁTICAMENTE
                    // o actualizar el 'file_content' virtual para el siguiente paso.
                    // Por ahora, solo guardamos el resultado en el historial.
                    
                    // Estrategia simple: Si es 'Refactor' o 'Fix' y hay artifacts, aplicarlos al archivo real
                    // para que el siguiente paso (ej: Verification) lea el archivo actualizado.
                    if !result.artifacts.is_empty() && wf_context.current_file.is_some() {
                        let path = wf_context.current_file.as_ref().unwrap();
                         if let Err(e) = std::fs::write(path, &result.artifacts[0]) {
                            println!("      ⚠️ Error al escribir archivo intermedio: {}", e);
                        } else {
                            println!("      💾 Archivo actualizado por el agente.");
                        }
                    }

                    wf_context.step_results.push(result);
                    println!("      ✅ Paso completado.");
                }
                Err(e) => {
                    println!("      ❌ Paso fallido: {}", e);
                    return Err(e);
                }
            }
        }

        println!("\n🏁 Workflow '{}' finalizado exitosamente.", workflow.name);
        Ok(wf_context)
    }
}
