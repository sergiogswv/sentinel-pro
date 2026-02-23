use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{TaskType, consultar_ia_dinamico};
use async_trait::async_trait;
use std::sync::Arc;

pub struct ReviewerAgent;

impl ReviewerAgent {
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
            "Actúa como un Tech Lead experto en Code Review para {} y {}.\n\n\
            TU TAREA DE REVISIÓN:\n\
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
            prompt.push_str(&format!("\nCÓDIGO O CONTEXTO A REVISAR:\n{}\n", ctx));
        }

        prompt.push_str(
            "\nCRITERIOS DE REVISIÓN:\n\
            1. Seguridad (OWASP Top 10).\n\
            2. Performance y eficiencia.\n\
            3. legibilidad y mantenimiento (Clean Code).\n\
            4. Patrones de diseño adecuados para el framework.\n\
            5. Manejo de errores.\n",
        );

        if !task.description.contains("FORMATO") && !task.description.contains("JSON") {
            prompt.push_str(
                "\nFORMATO DE RESPUESTA:\n\
                - Inicia con un resumen ejecutivo (Aprobado/Requiere Cambios).\n\
                - Lista los hallazgos clasificados por severidad (Alta, Media, Baja).\n\
                - Proporciona ejemplos de código corregido si es necesario.\n",
            );
        } else if task.description.contains("JSON") {
            // Refuerzo terminal: el AI tiende a seguir la última instrucción del prompt.
            // Sin este recordatorio, los CRITERIOS DE REVISIÓN (añadidos arriba) anulan
            // la instrucción JSON del task.description.
            prompt.push_str(
                "\n⚠️  RECORDATORIO FINAL: Tu respuesta DEBE terminar con el bloque ```json\n\
                solicitado en las instrucciones anteriores. Sin ese bloque, la respuesta\n\
                es inválida y no podrá ser procesada.\n",
            );
        }

        prompt
    }
}

#[async_trait]
impl Agent for ReviewerAgent {
    fn name(&self) -> &str {
        "ReviewerAgent"
    }

    fn description(&self) -> &str {
        "Especialista en análisis de código, seguridad y mejores prácticas"
    }

    async fn execute(&self, task: &Task, context: &AgentContext) -> anyhow::Result<TaskResult> {
        println!("   🧐 ReviewerAgent: Iniciando revisión del proyecto...");

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

        // Limpiamos los bloques de código para el output principal si queremos solo el reporte
        // Pero en este caso, el usuario probablemente quiera ver todo.
        // Usaremos `eliminar_bloques_codigo` solo si quisiéramos un resumen muy corto.
        // Aquí devolvemos la respuesta completa.

        Ok(TaskResult {
            success: true,
            output: response,
            files_modified: vec![],
            artifacts: vec![],
        })
    }
}
