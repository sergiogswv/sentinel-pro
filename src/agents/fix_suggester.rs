use crate::agents::base::{Agent, AgentContext, Task, TaskResult};
use crate::ai::client::{consultar_ia_dinamico, TaskType};
use async_trait::async_trait;
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
            4. Si la mejora implica múltiples archivos, genera UN bloque ```lang separado por cada archivo.\n\
            5. La PRIMERA LÍNEA de cada bloque de código DEBE ser un comentario con la ruta relativa del archivo:\n\
               Ejemplo TypeScript: // src/domain/user/user.entity.ts\n\
               Ejemplo Python:     # app/domain/user.py\n\
            6. CRÍTICO: Debes envolver el código en bloques markdown (```) indicando el lenguaje.\n\
            7. Debes devolver el archivo COMPLETO con las correcciones aplicadas. \n\
               ESTÁ PROHIBIDO devolver solo resúmenes, snippets parciales o comentarios tipo \"// ... resto del código\".\n\
            8. Mantén la lógica original intacta, enfocándote solo en la mejora solicitada.\n"
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
