use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{consultar_ia_dinamico, TaskType};
use crate::ai::utils::extraer_codigo;
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;

pub struct TesterAgent;

impl TesterAgent {
    pub fn new() -> Self {
        Self
    }

    fn build_prompt(&self, task: &Task, context: &AgentContext, rag_context: Option<&str>) -> String {
        let framework = &context.config.framework;
        let language = &context.config.code_language;
        let testing_framework = context.config.testing_framework.as_deref().unwrap_or("Jest/Vitest");

        let mut prompt = format!(
            "Actúa como un QA Lead experto en Tests Automatizados para {} usando {}.\n\n\
            TU TAREA:\n\
            {}\n\n\
            CONTEXTO DEL PROYECTO:\n\
            - Framework: {}\n\
            - Lenguaje: {}\n\
            - Testing Framework: {}\n",
            framework,
            testing_framework,
            task.description,
            framework,
            language,
            testing_framework
        );

        if let Some(ctx) = rag_context {
            prompt.push_str(&format!("\nCONTEXTO DE KNOWLEDGE BASE (RAG):\n{}\n", ctx));
        }

        if let Some(ctx) = &task.context {
            prompt.push_str(&format!("\nCÓDIGO O CONTEXTO A TESTEAR:\n{}\n", ctx));
        }

        prompt.push_str(
            "\nREQUISITOS:\n\
            1. Genera tests unitarios completos y robustos.\n\
            2. Cubre casos de éxito (happy path) y casos de error (edge cases).\n\
            3. Usa Mocks/Spies para dependencias externas.\n\
            4. Sigue las convenciones de nombrado del framework (ej: .spec.ts o .test.js).\n\
            5. Devuelve SOLO el código del test dentro de un bloque markdown (```).\n"
        );

        prompt
    }
}

#[async_trait]
impl Agent for TesterAgent {
    fn name(&self) -> &str {
        "TesterAgent"
    }

    fn description(&self) -> &str {
        "Especialista en generación y ejecución de tests automatizados"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   🧪 TesterAgent: Procesando tarea '{}'...", task.description);

        // Intentar obtener contexto relevante de la Knowledge Base
        let mut rag_context = String::new();
        if let Some(kb) = &context.context_builder {
            print!("{}", "   🧠 Consultando Knowledge Base...".dimmed());
            match kb.build_context(&task.description, 3, false).await {
                Ok(ctx) => {
                    rag_context = ctx;
                    println!("{}", " OK".green());
                },
                Err(e) => {
                    println!("{}", " Error".red());
                    eprintln!("      Error consultando KB: {}", e);
                }
            }
        }

        let prompt_context = if rag_context.is_empty() { None } else { Some(rag_context.as_str()) };
        let prompt = self.build_prompt(task, context, prompt_context);

        let config_clone = context.config.clone();
        let stats_clone = Arc::clone(&context.stats);
        let project_root_clone = context.project_root.clone();

        let response = tokio::task::spawn_blocking(move || {
            consultar_ia_dinamico(
                prompt,
                TaskType::Deep,
                &config_clone,
                stats_clone,
                &project_root_clone,
            )
        })
        .await??;

        let code = extraer_codigo(&response);
        let success = !code.trim().is_empty();

        Ok(TaskResult {
            success,
            output: response,
            files_modified: vec![],
            artifacts: vec![code],
        })
    }
}
