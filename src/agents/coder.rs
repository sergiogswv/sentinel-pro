use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{consultar_ia_dinamico, TaskType};
use crate::ai::utils::extraer_codigo;
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;

pub struct CoderAgent;

impl CoderAgent {
    pub fn new() -> Self {
        Self
    }

    fn build_prompt(&self, task: &Task, context: &AgentContext, rag_context: Option<&str>) -> String {
        let framework = &context.config.framework;
        let language = &context.config.code_language;
        let mut prompt = format!(
            "Actúa como un Desarrollador Senior experto en {} y {}.\n\n\
            TU TAREA:\n\
            {}\n\n\
            CONTEXTO DEL PROYECTO:\n\
            - Framework: {}\n\
            - Lenguaje: {}\n",
            framework,
            language,
            task.description,
            framework,
            language
        );

        if let Some(ctx) = rag_context {
            prompt.push_str(&format!("\nCONTEXTO DE KNOWLEDGE BASE (RAG):\n{}\n", ctx));
        }

        if let Some(ctx) = &task.context {
            prompt.push_str(&format!("\nINFORMACIÓN ADICIONAL:\n{}\n", ctx));
        }

        prompt.push_str(
            "\nREQUISITOS:\n\
            1. Genera código limpio, moderno y siguiendo las mejores prácticas.\n\
            2. Usa tipado fuerte si el lenguaje lo permite.\n\
            3. Si es una modificación, mantén el estilo del código existente.\n\
            4. Devuelve SOLO el código necesario dentro de un bloque markdown (```).\n\
            5. Si necesitas explicar algo, hazlo brevemente DESPUÉS del bloque de código.\n"
        );

        prompt
    }
}

#[async_trait]
impl Agent for CoderAgent {
    fn name(&self) -> &str {
        "CoderAgent"
    }

    fn description(&self) -> &str {
        "Especialista en generación y refactorización de código con IA"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   🤖 CoderAgent: Analizando tarea '{}'...", task.description);

        // Intentar obtener contexto relevante de la Knowledge Base
        let mut rag_context = String::new();
        if let Some(kb) = &context.context_builder {
            print!("{}", "   🧠 Consultando Knowledge Base...".dimmed());
            match kb.build_context(&task.description, 3, true).await {
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
        
        // Ejecutar consulta a la IA (esto es bloqueante, idealmente deberíamos usar spawn_blocking si fuera muy pesado,
        // pero consultar_ia_dinamico ya maneja http request que lleva tiempo)
        // Nota: consultar_ia_dinamico es síncrono (reqwest blocking), así que envolvemos en spawn_blocking
        // para no bloquear el runtime async de Tokio.
        
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
            files_modified: vec![], // Por ahora no escribimos archivos directamente, dejamos que el usuario decida
            artifacts: vec![code],
        })
    }
}
