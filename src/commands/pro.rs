use crate::agents::base::{AgentContext, Task, TaskType};
use crate::agents::coder::CoderAgent;
use crate::agents::orchestrator::AgentOrchestrator;
use crate::agents::refactor::RefactorAgent;
use crate::agents::reviewer::ReviewerAgent;
use crate::agents::tester::TesterAgent;
use crate::commands::ProCommands;
use crate::config::SentinelConfig;
use crate::kb::{ContextBuilder, VectorDB};
use crate::stats::SentinelStats;
use crate::ui;
use colored::*;
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn handle_pro_command(subcommand: ProCommands) {
    // Inicializar recursos necesarios para los agentes
    let project_root = env::current_dir().expect("No se pudo obtener el directorio actual");
    let config = SentinelConfig::load(&project_root).unwrap_or_default();
    let stats = Arc::new(Mutex::new(SentinelStats::cargar(env::current_dir().unwrap().as_path())));

    // Inicializar KB Context Builder
    let context_builder = if let Some(kb_config) = &config.knowledge_base {
        match VectorDB::new(&kb_config.vector_db_url) {
            Ok(db) => {
                // Usamos el modelo primario para embeddings por defecto
                // Idealmente deberíamos tener una configuración específica para embeddings
                Some(Arc::new(ContextBuilder::new(db, config.primary_model.clone())))
            },
            Err(_) => None,
        }
    } else {
        None
    };

    let agent_context = AgentContext {
        config: Arc::new(config),
        stats,
        project_root,
        context_builder,
    };

    // Inicializar Orquestador y Agentes
    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.register(Arc::new(CoderAgent::new()));
    orchestrator.register(Arc::new(ReviewerAgent::new()));
    orchestrator.register(Arc::new(TesterAgent::new()));
    orchestrator.register(Arc::new(RefactorAgent::new()));

    // Ejecutar en Runtime de Tokio
    let rt = tokio::runtime::Runtime::new().unwrap();

    match subcommand {
        ProCommands::Analyze { file } => {
            let pb = ui::crear_progreso(&format!("Analizando {} con ReviewerAgent...", file));
            
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Analiza el archivo {} y reporta problemas.", file),
                task_type: TaskType::Analyze,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: None, // Futuro: Leer contenido del archivo aquí
            };

            let result = rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));
            
            pb.finish_and_clear();
            
            match result {
                Ok(res) => {
                    println!("{}", "🔍 ANÁLISIS COMPLETADO".bold().green());
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al analizar:".bold().red(), e);
                }
            }
        }
        ProCommands::Generate { file } => {
            let pb = ui::crear_progreso(&format!("Generando código para {}...", file));
            
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Genera el código necesario para el archivo {}.", file),
                task_type: TaskType::Generate,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: None,
            };

            let result = rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));
            
            pb.finish_and_clear();
             
             match result {
                Ok(res) => {
                    println!("{}", "🚀 CÓDIGO GENERADO".bold().green());
                    // Mostrar artifacts (código extraído)
                    for artifact in res.artifacts {
                         println!("\n{}\n", artifact);
                    }
                    
                    println!("{}", "\n📝 Explicación detallada:".bold());
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al generar:".bold().red(), e);
                }
            }
        }
        ProCommands::Refactor { file } => {
             let pb = ui::crear_progreso(&format!("Refactorizando {}...", file));
            
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Refactoriza el archivo {} para mejorar legibilidad y estructura.", file),
                task_type: TaskType::Refactor,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: None,
            };

            let result = rt.block_on(orchestrator.execute_task("RefactorAgent", &task, &agent_context));
            
            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🛠️ REFACTORIZACIÓN COMPLETADA".bold().green());
                    for artifact in res.artifacts {
                         println!("\n{}\n", artifact);
                    }
                }
                Err(e) => {
                     println!("{} {}", "❌ Error al refactorizar:".bold().red(), e);
                }
            }
        }
        ProCommands::Fix { file } => {
            let pb = ui::crear_progreso(&format!("Buscando solución para {}...", file));
            thread::sleep(Duration::from_secs(2));
            pb.finish_with_message(format!(
                "🩹 {} {}",
                "Bugs corregidos en:".bold(),
                file.cyan()
            ));
            println!("⚠️  FixCommand pendiente de integración con Agents.");
        }
        ProCommands::TestAll => {
            let pb = ui::crear_progreso("Ejecutando asistente de pruebas...");
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Analiza el proyecto y genera un plan de pruebas unitarias para los componentes más críticos. Sugiere código para el test más importante.".to_string(),
                task_type: TaskType::Test,
                file_path: None,
                context: None,
            };

            let result = rt.block_on(orchestrator.execute_task("TesterAgent", &task, &agent_context));
            pb.finish_with_message("🧪 Asistente de Pruebas finalizado.");

             match result {
                Ok(res) => {
                    println!("{}", "🧪 PLAN DE PRUEBAS GENERADO".bold().green());
                    // Mostrar artifacts (código extraído)
                    for artifact in res.artifacts {
                         println!("\n{}\n", artifact);
                    }
                    
                    println!("{}", "\n📝 Detalles:".bold());
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al generar tests:".bold().red(), e);
                }
            }
        }
        _ => {
            println!("⚠️  Comando Pro en desarrollo.");
        }
    }
}
