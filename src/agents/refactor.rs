use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{TaskType, consultar_ia_dinamico};
use crate::ai::utils::extraer_codigo;
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;

pub struct RefactorAgent;

impl RefactorAgent {
    pub fn new() -> Self {
        Self
    }

    fn build_prompt(
        &self,
        task: &Task,
        context: &AgentContext,
        rag_context: Option<&str>,
    ) -> String {
        let framework = &context.config.framework;
        let language = &context.config.code_language;
        let mut prompt = format!(
            "Actúa como un Arquitecto de Software experto en Refactorización y Patrones de Diseño para {} y {}.\n\n\
            TU OBJETIVO:\n\
            Mejorar la estructura, legibilidad y mantenibilidad del código SIN alterar su comportamiento externo (Refactoring).\n\n\
            TAREA ESPECÍFICA:\n\
            {}\n\n\
            CONTEXTO DEL PROYECTO:\n\
            - Framework: {}\n\
            - Lenguaje: {}\n",
            framework, language, task.description, framework, language
        );

        if let Some(ctx) = rag_context {
            prompt.push_str(&format!("\nCONTEXTO DE KNOWLEDGE BASE (RAG):\n{}\n", ctx));
        }

        if let Some(ctx) = &task.context {
            prompt.push_str(&format!("\nCÓDIGO A REFACTORIZAR:\n{}\n", ctx));
        }

        prompt.push_str(
            "\nESTRATEGIA DE REFACTORIZACIÓN:\n\
            1. Identifica Code Smells (duplicidad, complejidad ciclomática, funciones largas, etc.).\n\
            2. Aplica principios SOLID y Clean Code.\n\
            3. Si es necesario, divide el código en funciones o clases más pequeñas.\n\
            4. Mejora el nombrado de variables y funciones para que sea autodocumentado.\n\
            5. Mantén la consistencia con el estilo del framework.\n\n\
            FORMATO DE RESPUESTA:\n\
            1. Breve análisis de los problemas encontrados.\n\
            2. Explicación de las mejoras aplicadas.\n\
            3. BLOQUE DE CÓDIGO ÚNICO (```) con la versión refactorizada completa.\n"
        );

        prompt
    }
}

#[async_trait]
impl Agent for RefactorAgent {
    fn name(&self) -> &str {
        "RefactorAgent"
    }

    fn description(&self) -> &str {
        "Especialista en refactorización, limpieza de código y patrones de diseño"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   🛠️  RefactorAgent: Analizando código para refactorización...");

        // Intentar obtener contexto relevante de la Knowledge Base
        let mut rag_context = String::new();
        if let Some(kb) = &context.context_builder {
            print!("{}", "   🧠 Consultando Knowledge Base...".dimmed());
            match kb.build_context(&task.description, 3, true).await {
                Ok(ctx) => {
                    rag_context = ctx;
                    println!("{}", " OK".green());
                }
                Err(e) => {
                    println!("{}", " Error".red());
                    eprintln!("      Error consultando KB: {}", e);
                }
            }
        }

        let prompt_context = if rag_context.is_empty() {
            None
        } else {
            Some(rag_context.as_str())
        };
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
