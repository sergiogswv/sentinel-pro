use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{TaskType, consultar_ia_dinamico};
use async_trait::async_trait;
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
            3. BLOQUE DE CÓDIGO ÚNICO usando triple comilla (```) con la versión refactorizada COMPLETA Y FUNCIONAL.\n\
               CRÍTICO: NO devuelvas resúmenes ni diffs parciales. El bloque debe contener el archivo enterito.\n"
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

        let rag_context = if let Some(path) = &task.file_path {
            context.build_rag_context(path)
        } else {
            String::new()
        };

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

        let bloques = crate::ai::utils::extraer_todos_bloques(&response);
        let success = !bloques.is_empty();
        let artifacts = bloques.into_iter().map(|(_, code)| code).collect::<Vec<_>>();

        if success {
            println!("   ✅ {} bloque(s) de código extraídos.", artifacts.len());
        }

        Ok(TaskResult {
            success,
            output: response,
            files_modified: vec![],
            artifacts,
        })
    }
}
