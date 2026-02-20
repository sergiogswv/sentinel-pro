use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{consultar_ia_dinamico, TaskType};
use crate::ai::utils::extraer_codigo;
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;

pub struct FixSuggesterAgent;

impl FixSuggesterAgent {
    pub fn new() -> Self {
        Self
    }

    fn build_prompt(&self, task: &Task, context: &AgentContext, rag_context: Option<&str>) -> String {
        let framework = &context.config.framework;
        let language = &context.config.code_language;
        let mut prompt = format!(
            "Actúa como el AI Code Quality Guardian (FixSuggesterAgent), un Desarrollador Senior experto en {} y {}.\n\n\
            TU MISIÓN:\n\
            Eres el guardián de la calidad del código. Tu trabajo es proponer correcciones precisas para los problemas detectados por los analizadores estáticos o revisiones de seguridad.\n\n\
            TAREA ESPECÍFICA:\n\
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
            prompt.push_str(&format!("\nCÓDIGO/INFORMACIÓN A CORREGIR:\n{}\n", ctx));
        }

        // Obtener dependencias
        let deps = crate::files::leer_dependencias(&context.project_root);
        let deps_list = if deps.is_empty() {
            "No se detectaron dependencias explícitas.".to_string()
        } else {
            deps.iter().take(50).cloned().collect::<Vec<_>>().join(", ")
        };

        prompt.push_str(&format!(
            "\nDEPENDENCIAS DISPONIBLES:\n{}\n",
            deps_list
        ));

        prompt.push_str(
            "\nREQUISITOS DE CALIDAD:\n\
            1. NO generes lógica de negocio nueva si no es necesaria para corregir el problema.\n\
            2. Asegúrate de que el código propuesto sea production-ready y respete los estándares del framework.\n\
            3. Elimina código muerto o importaciones innecesarias si las detectas en el contexto.\n\
            4. Devuelve el código corregido dentro de un bloque markdown (```).\n\
            5. Mantén la lógica original intacta, enfocándote solo en resolver la vulnerabilidad o el fallo detectado.\n"
        );

        prompt
    }
}

#[async_trait]
impl Agent for FixSuggesterAgent {
    fn name(&self) -> &str {
        "FixSuggesterAgent"
    }

    fn description(&self) -> &str {
        "AI Code Quality Guardian: Propone correcciones precisas para mejorar la calidad y seguridad del código"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   🤖 FixSuggesterAgent: Analizando y preparando correcciones...");

        let rag_context = if let Some(path) = &task.file_path {
            context.build_rag_context(path)
        } else {
            String::new()
        };

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
