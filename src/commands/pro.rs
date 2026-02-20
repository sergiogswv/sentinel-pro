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
use dialoguer::{MultiSelect, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct AuditIssue {
    title: String,
    description: String,
    severity: String,
    suggested_fix: String,
    #[serde(default)]
    file_path: String,
}

pub fn handle_pro_command(subcommand: ProCommands) {
    // Buscar la raíz del proyecto inteligentemente
    let project_root = SentinelConfig::find_project_root()
        .unwrap_or_else(|| env::current_dir().expect("No se pudo obtener el directorio actual"));

    if project_root != env::current_dir().unwrap_or_default() {
        println!(
            "{} {}",
            "📂 Proyecto Activo:".cyan().bold(),
            project_root.display().to_string().bright_blue()
        );
    }

    let mut config = SentinelConfig::load(&project_root).unwrap_or_else(|| {
        if !project_root.join(".sentinelrc.toml").exists() {
            println!(
                "{} {}",
                "⚠️".yellow(),
                "No se encontró configuración (.sentinelrc.toml) en este directorio ni en padres."
                    .yellow()
            );
            println!("   Ejecuta 'sentinel' primero para configurar un proyecto.");
        }
        SentinelConfig::default()
    });

    // Auto-fix inteligente para la URL de KB si es necesario
    if let Some(ref mut kb) = config.knowledge_base {
        let mut current_valid = false;
        let target = kb
            .vector_db_url
            .replace("http://", "")
            .replace("https://", "");
        if let Some((host, port_str)) = target.split_once(':') {
            let port = port_str.parse::<u16>().unwrap_or(6334);
            let actual_host = if host == "localhost" {
                "127.0.0.1"
            } else {
                host
            };
            if let Ok(addr) = format!("{}:{}", actual_host, port).parse::<std::net::SocketAddr>() {
                current_valid =
                    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok();
            }
        }

        if !current_valid
            && (kb.vector_db_url.contains("localhost") || kb.vector_db_url.contains("6333"))
        {
            let healed_addr: std::net::SocketAddr = "127.0.0.1:6334".parse().unwrap();
            if std::net::TcpStream::connect_timeout(&healed_addr, Duration::from_millis(200))
                .is_ok()
            {
                println!(
                    "   🔧 {} Conexión fallida con {}. Usando {}...",
                    "Smart-Heal:".cyan(),
                    kb.vector_db_url.yellow(),
                    "127.0.0.1:6334".green()
                );
                kb.vector_db_url = "http://127.0.0.1:6334".to_string();
                let _ = config.save(&project_root);
            }
        }
    }

    let stats = Arc::new(Mutex::new(SentinelStats::cargar(&project_root)));

    // Inicializar KB Context Builder
    let context_builder = if let Some(kb_config) = &config.knowledge_base {
        match VectorDB::new(
            &kb_config.vector_db_url,
            config.primary_model.embedding_dimension(),
        ) {
            Ok(db) => {
                // Usamos el modelo primario para embeddings por defecto
                // Idealmente deberíamos tener una configuración específica para embeddings
                Some(Arc::new(ContextBuilder::new(
                    db,
                    config.primary_model.clone(),
                )))
            }
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

            // Leer contenido del archivo
            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    pb.finish_and_clear();
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Analiza el archivo {} y reporta problemas.", file),
                task_type: TaskType::Analyze,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: Some(content),
            };

            let result =
                rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));

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

            // Intentar leer contenido si existe (para contexto)
            let content = std::fs::read_to_string(&file).ok();

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Genera el código necesario para el archivo {}.", file),
                task_type: TaskType::Generate,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: content,
            };

            let result =
                rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🚀 CÓDIGO GENERADO".bold().green());

                    // Si hay artifacts, guardarlos en el archivo
                    if let Some(code) = res.artifacts.first() {
                        match std::fs::write(&file, code) {
                            Ok(_) => {
                                println!("   💾 Archivo guardado: {}", file.cyan());
                                // Update Stats
                                let mut s = agent_context.stats.lock().unwrap();
                                s.total_analisis += 1;
                                s.sugerencias_aplicadas += 1;
                                s.tiempo_estimado_ahorrado_mins += 10;
                                s.guardar(&agent_context.project_root);
                            }
                            Err(e) => println!("   ⚠️  No se pudo guardar el archivo: {}", e),
                        }
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
            // Leer contenido original
            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let pb = ui::crear_progreso(&format!("Refactorizando {}...", file));

            // Crear Backup
            let backup_path = format!("{}.bak", file);
            if let Err(e) = std::fs::copy(&file, &backup_path) {
                pb.finish_and_clear();
                println!("{} {}", "❌ Error al crear backup:".bold().red(), e);
                return;
            }

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Refactoriza el archivo {} para mejorar legibilidad y estructura.",
                    file
                ),
                task_type: TaskType::Refactor,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: Some(content),
            };

            let result =
                rt.block_on(orchestrator.execute_task("RefactorAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🛠️ REFACTORIZACIÓN COMPLETADA".bold().green());
                    println!("   🔙 Backup creado en: {}", backup_path.dimmed());

                    if let Some(code) = res.artifacts.first() {
                        match std::fs::write(&file, code) {
                            Ok(_) => {
                                println!("   💾 Cambios aplicados a: {}", file.cyan());
                                // Update Stats
                                let mut s = agent_context.stats.lock().unwrap();
                                s.total_analisis += 1;
                                s.sugerencias_aplicadas += 1;
                                s.tiempo_estimado_ahorrado_mins += 15;
                                s.guardar(&agent_context.project_root);
                            }
                            Err(e) => println!("   ⚠️  No se pudo escribir el archivo: {}", e),
                        }
                    } else {
                        println!("   ⚠️  El agente no retornó código válido para reemplazar.");
                    }

                    println!("\n{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al refactorizar:".bold().red(), e);
                }
            }
        }
        ProCommands::Fix { file } => {
            // Leer contenido original
            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let pb = ui::crear_progreso(&format!("Corrigiendo bugs en {}...", file));

            // Crear Backup
            let backup_path = format!("{}.bak", file);
            let _ = std::fs::copy(&file, &backup_path);

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Identifica y corrige bugs en el archivo {}.", file),
                task_type: TaskType::Fix,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: Some(content),
            };

            // Usamos CoderAgent para fixes por ahora
            let result =
                rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🩹 BUGS CORREGIDOS".bold().green());
                    if let Some(code) = res.artifacts.first() {
                        match std::fs::write(&file, code) {
                            Ok(_) => {
                                println!("   💾 Correcciones aplicadas a: {}", file.cyan());
                                // Update Stats
                                let mut s = agent_context.stats.lock().unwrap();
                                s.total_analisis += 1;
                                s.sugerencias_aplicadas += 1;
                                s.bugs_criticos_evitados += 1;
                                s.tiempo_estimado_ahorrado_mins += 20;
                                s.guardar(&agent_context.project_root);
                            }
                            Err(e) => println!("   ⚠️  No se pudo escribir el archivo: {}", e),
                        }
                    }
                    println!("\n{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al corregir:".bold().red(), e);
                }
            }
        }
        ProCommands::Chat => {
            println!("{}", "💬 Sentinel Pro Chat".bold().blue());
            println!("{}", "Escribe 'exit' o 'quit' para salir.\n".dimmed());

            use std::io::{self, Write};

            // Historial simple en memoria para la sesión
            let mut conversation_history = String::new();

            loop {
                print!("{}", "You > ".bold().green());
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();

                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    break;
                }

                if input.is_empty() {
                    continue;
                }

                // Mostrar indicador de pensamiento
                print!("{}", "   Thinking...".dimmed());
                io::stdout().flush().unwrap();

                // Construir prompt con historial
                let prompt = if conversation_history.is_empty() {
                    format!(
                        "Eres un asistente experto en programación y en este proyecto. Responde a: {}",
                        input
                    )
                } else {
                    format!(
                        "{}\nUser: {}\nAssistant: Responde corto y conciso.",
                        conversation_history, input
                    )
                };

                let config_clone = agent_context.config.clone();
                let stats_clone = Arc::clone(&agent_context.stats);
                let project_root = agent_context.project_root.clone();

                let response_result = crate::ai::client::consultar_ia_dinamico(
                    prompt,
                    crate::ai::client::TaskType::Deep, // Usar Deep para mejor razonamiento en chat
                    &config_clone,
                    stats_clone,
                    &project_root,
                );

                // Envolver en Ok(Ok(...)) para coincidir con el match de abajo que espera Result<Result<...>>
                // o simplificar el match
                let response_result: anyhow::Result<anyhow::Result<String>> = Ok(response_result);

                // Borrar indicador "Thinking..." (retorno de carro + espacios)
                print!("\r               \r");

                match response_result {
                    Ok(Ok(response)) => {
                        print!("{}", "Sentinel > ".bold().blue());
                        println!("{}\n", response);
                        // Limitar historial para no exceder tokens infinitamente
                        if conversation_history.len() > 4000 {
                            conversation_history =
                                conversation_history.split_off(conversation_history.len() / 2);
                        }
                        conversation_history
                            .push_str(&format!("\nUser: {}\nAssistant: {}", input, response));
                    }
                    Ok(Err(e)) => println!("{}", format!("Error: {}", e).red()),
                    Err(e) => println!("{}", format!("System Error: {}", e).red()),
                }
            }
        }
        ProCommands::Docs { dir } => {
            let pb = ui::crear_progreso(&format!("Generando documentación para {}...", dir));

            // Simplificado: Listar archivos y pedir un README general
            let path = std::path::PathBuf::from(&dir);
            if !path.exists() {
                pb.finish_and_clear();
                println!("{}", "❌ El directorio no existe.".red());
                return;
            }

            // Recolectar nombres de archivos para dar contexto de estructura
            let mut structure = String::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        structure.push_str(&format!("- {}\n", name));
                    }
                }
            }

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Genera una documentación técnica (README.md) detallada para el directorio '{}' que contiene los siguientes archivos:\n{}",
                    dir, structure
                ),
                task_type: TaskType::Generate, // Reusamos Generate
                file_path: Some(path.clone()),
                context: Some(format!("Estructura de archivos:\n{}", structure)),
            };

            let result =
                rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "📚 DOCUMENTACIÓN GENERADA".bold().green());
                    if let Some(doc_content) = res.artifacts.first() {
                        let doc_path = path.join("PROJECT_DOCS.md");
                        match std::fs::write(&doc_path, doc_content) {
                            Ok(_) => println!(
                                "   💾 Documentación guardada en: {}",
                                doc_path.display().to_string().cyan()
                            ),
                            Err(e) => println!("   ⚠️  No se pudo guardar el archivo: {}", e),
                        }
                    }
                    println!("\n{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al documentar:".bold().red(), e);
                }
            }
        }
        ProCommands::TestAll => {
            let pb = ui::crear_progreso("Ejecutando asistente de pruebas...");

            // 1. Escaneo Inteligente de Archivos sin Test
            let mut archivos_sin_test = Vec::new();
            let src_path = agent_context.project_root.join("src"); // Asumimos convention src/

            if src_path.exists() {
                // Buscar recursivamente
                let walker = ignore::WalkBuilder::new(&src_path)
                    .hidden(false)
                    .git_ignore(true)
                    .build();

                for result in walker {
                    if let Ok(entry) = result {
                        // ignore::DirEntry
                        let entry: ignore::DirEntry = entry;
                        let path = entry.path();

                        // Verificar si es archivo
                        if !path.is_file() {
                            continue;
                        }

                        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                        // Filtrar por extensiones configuradas
                        let ext_opt = path.extension().and_then(|e| e.to_str());
                        let ext = ext_opt.unwrap_or("").to_string();
                        if !agent_context.config.file_extensions.contains(&ext) {
                            continue;
                        }

                        // Ignorar archivos de test existentes
                        if file_name.ends_with(".spec.ts")
                            || file_name.ends_with(".test.ts")
                            || file_name.ends_with("_test.go")
                            || file_name.ends_with(".test.js")
                        {
                            continue;
                        }

                        // Verificar si tiene test
                        let base_name = file_name
                            .split('.')
                            .next()
                            .unwrap_or(&file_name)
                            .to_string();

                        let test_exists = crate::files::buscar_archivo_test(
                            &base_name,
                            &agent_context.project_root,
                            &agent_context.config.test_patterns,
                        )
                        .is_some();

                        if !test_exists {
                            if let Ok(rel) = path.strip_prefix(&agent_context.project_root) {
                                archivos_sin_test.push(rel.display().to_string());
                            } else {
                                archivos_sin_test.push(path.display().to_string());
                            }
                        }
                    }
                }
            }

            // Limitar la lista para no exceder tokens
            let total_missing = archivos_sin_test.len();
            archivos_sin_test.truncate(20);

            let context_msg = if archivos_sin_test.is_empty() {
                "No se detectaron archivos fuente obvios sin tests en src/ (o el proyecto tiene una estructura diferente).".to_string()
            } else {
                format!(
                    "Se detectaron {} archivos que NO parecen tener tests asociados.\nLista de prioridad (Top 20):\n- {}",
                    total_missing,
                    archivos_sin_test.join("\n- ")
                )
            };

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Analiza el proyecto y genera un plan de pruebas unitarias priorizado. Enfócate en los archivos listados que no tienen cobertura.".to_string(),
                task_type: TaskType::Test,
                file_path: None,
                context: Some(context_msg),
            };

            let result =
                rt.block_on(orchestrator.execute_task("TesterAgent", &task, &agent_context));
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
        ProCommands::Ml { subcommand } => match subcommand {
            crate::commands::MlCommands::Download => {
                let start = std::time::Instant::now();
                match crate::ml::embeddings::EmbeddingModel::new() {
                    Ok(_) => {
                        let duration = start.elapsed();
                        println!(
                            "{} ({}s)",
                            "✅ Modelo descargado y verificado correctamente.".green(),
                            duration.as_secs()
                        );
                    }
                    Err(e) => println!("{} {}", "❌ Error al descargar modelo:".red(), e),
                }
            }
            crate::commands::MlCommands::Test { text } => {
                println!("{}", "🧠 Generando embeddings de prueba...".cyan());
                match crate::ml::embeddings::EmbeddingModel::new() {
                    Ok(model) => match model.embed_one(&text) {
                        Ok(emb) => {
                            println!("{}", "✅ Operación exitosa.".green());
                            println!("   📝 Texto: \"{}\"", text);
                            println!("   📊 Dimensión: {}", emb.len());
                            println!("   🔢 Vector [0..5]: {:?}", &emb[0..5]);
                        }
                        Err(e) => println!("{} {}", "❌ Error al generar embedding:".red(), e),
                    },
                    Err(e) => println!("{} {}", "❌ Error al cargar modelo:".red(), e),
                }
            }
        },
        ProCommands::Kb { subcommand } => match subcommand {
            crate::commands::KbCommands::Check => {
                println!("\n🧠 {}", "CONEXIÓN KNOWLEDGE BASE".bold().cyan());

                if let Some(kb) = &agent_context.config.knowledge_base {
                    println!("   📍 URL Configurada: {}", kb.vector_db_url.dimmed());

                    // Prueba de conexión real
                    let url = kb.vector_db_url.clone();
                    let dimension = agent_context.config.primary_model.embedding_dimension();

                    match VectorDB::new(&url, dimension) {
                        Ok(db) => {
                            let pb = ui::crear_progreso("Verificando latencia y colecciones...");
                            match rt.block_on(db.initialize_collection()) {
                                Ok(_) => {
                                    pb.finish_and_clear();
                                    println!(
                                        "   ✅ Estado: {}",
                                        "CONECTADO Y LISTO".green().bold()
                                    );
                                }
                                Err(e) => {
                                    pb.finish_and_clear();
                                    println!("   ❌ Estado: {}", "ERROR DE CONEXIÓN".red().bold());
                                    println!("   ⚠️  Detalle: {}", e);

                                    if url.contains("6333") {
                                        println!(
                                            "   💡 Sugerencia: El cliente Rust prefiere gRPC (puerto 6334). Prueba cambiando 6333 por 6334."
                                        );
                                    }
                                    if url.contains("localhost") {
                                        println!(
                                            "   💡 Sugerencia: En Windows, intenta cambiar 'localhost' por '127.0.0.1'."
                                        );
                                    }
                                    if e.to_string().contains("h2")
                                        || e.to_string().contains("http2")
                                    {
                                        println!(
                                            "   ✨ Tip: El error HTTP2 suele ser un conflicto de puertos. Asegúrate de usar el puerto gRPC (6334)."
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => println!("   ❌ Error al crear cliente: {}", e),
                    }
                } else {
                    println!("   ❌ Estado: {}", "NO CONFIGURADO".red().bold());
                }
            }
            crate::commands::KbCommands::Retry => {
                println!("\n🔄 {}", "RE-INTENTANDO CONEXIÓN KB".bold().yellow());
                let project_root = &agent_context.project_root;
                let config = SentinelConfig::load(project_root).unwrap_or_default();

                if let Some(kb_config) = &config.knowledge_base {
                    let mut url = kb_config.vector_db_url.clone();

                    // Auto-fix para problemas comunes de gRPC/Windows
                    let mut modified = false;
                    if url.contains("localhost") {
                        url = url.replace("localhost", "127.0.0.1");
                        modified = true;
                    }
                    if url.contains(":6333") {
                        url = url.replace(":6333", ":6334");
                        modified = true;
                    }

                    if modified {
                        println!("   🔧 Aplicando auto-fix: Usando {}...", url.cyan());
                    }

                    match VectorDB::new(&url, config.primary_model.embedding_dimension()) {
                        Ok(db) => {
                            let pb = ui::crear_progreso("Iniciando conexión...");
                            match rt.block_on(db.initialize_collection()) {
                                Ok(_) => {
                                    pb.finish_and_clear();
                                    println!(
                                        "   ✅ Conexión establecida con {} con éxito.",
                                        url.cyan()
                                    );

                                    // Si la URL fue modificada y funcionó, persistir el cambio en el config
                                    if modified {
                                        let mut new_config = config.clone();
                                        if let Some(ref mut kb) = new_config.knowledge_base {
                                            kb.vector_db_url = url.clone();
                                        }
                                        if let Ok(_) = new_config.save(project_root) {
                                            println!(
                                                "   💾 Configuración actualizada permanentemente con la nueva URL."
                                            );
                                        }
                                    }

                                    println!("   🚀 KB ahora está disponible.");
                                }
                                Err(e) => {
                                    pb.finish_and_clear();
                                    println!("   ❌ Error al inicializar: {}", e);
                                    if !modified && url.contains("6333") {
                                        println!(
                                            "   💡 Tip: Qdrant usa 6334 para gRPC. Intenta cambiar el puerto en tu .sentinelrc.toml"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => println!("   ❌ Error al conectar: {}", e),
                    }
                } else {
                    println!("   ⚠️  Knowledge Base no está configurado.");
                }
            }
        },
        ProCommands::CleanCache { target } => {
            let path_str = target.unwrap_or_else(|| ".".to_string());
            let target_path = std::path::PathBuf::from(&path_str);

            println!(
                "🧹 {} en: {}...",
                "Limpiando caché de Sentinel AI".cyan(),
                path_str.bold()
            );
            match crate::ai::limpiar_cache(&target_path) {
                Ok(_) => {
                    println!("   ✅ Caché limpiada correctamente.");
                }
                Err(e) => {
                    println!("   ❌ Error al limpiar caché: {}", e);
                }
            }
        }
        ProCommands::Workflow { name, file } => {
            use crate::agents::workflow::{TaskTemplate, Workflow, WorkflowEngine, WorkflowStep};

            let pb = ui::crear_progreso(&format!("Preparando workflow '{}'...", name));

            // --- WORKFLOWS DEFINIDOS (Hardcoded por ahora, luego .yaml) ---
            let workflow = match name.as_str() {
                 "fix-and-verify" => Some(Workflow {
                     name: "Fix & Verify".to_string(),
                     description: "Intenta arreglar un bug y luego verifica con tests.".to_string(),
                     steps: vec![
                         WorkflowStep {
                             name: "Identificar y Corregir Bugs".to_string(),
                             agent: "CoderAgent".to_string(),
                             task_template: TaskTemplate {
                                 description: "Analiza el archivo {file} en busca de bugs lógicos o de sintaxis. Si encuentras errores, corrígelos y devuelve el código completo corregido.".to_string(),
                                 task_type: TaskType::Fix,
                             },
                         },
                         WorkflowStep {
                             name: "Refactorizar para Calidad".to_string(),
                             agent: "RefactorAgent".to_string(),
                             task_template: TaskTemplate {
                                 description: "Toma el código del paso anterior (si hubo cambios) o del archivo {file}. Mejora su legibilidad y estructura aplicando Clean Code, sin romper la lógica corregida.".to_string(),
                                 task_type: TaskType::Refactor,
                             },
                         },
                         WorkflowStep {
                             name: "Verificar con Plan de Pruebas".to_string(),
                             agent: "TesterAgent".to_string(),
                             task_template: TaskTemplate {
                                 description: "Genera un plan de pruebas unitarias para el código resultante del paso anterior (fichero {file}). Asegúrate de cubrir los casos de borde de los bugs corregidos.".to_string(),
                                 task_type: TaskType::Test,
                             },
                         },
                     ],
                 }),
                 "review-security" => Some(Workflow {
                     name: "Security Auditing".to_string(),
                     description: "Análisis de seguridad profundo.".to_string(),
                     steps: vec![
                         WorkflowStep {
                             name: "Análisis de Seguridad Estático".to_string(),
                             agent: "ReviewerAgent".to_string(),
                             task_template: TaskTemplate {
                                 description: "Realiza una auditoría de seguridad OWASP Top 10 sobre el archivo {file}. Enfócate solo en vulnerabilidades críticas.".to_string(),
                                 task_type: TaskType::Analyze,
                             },
                         },
                         WorkflowStep {
                             name: "Sugerencia de Mitigación".to_string(),
                             agent: "CoderAgent".to_string(),
                             task_template: TaskTemplate {
                                 description: "Basado en el análisis de seguridad anterior, sugiere código seguro para mitigar las vulnerabilidades encontradas en {file}.".to_string(),
                                 task_type: TaskType::Generate,
                             },
                         },
                     ]
                 }),
                 _ => None,
             };

            if let Some(wf) = workflow {
                pb.finish_with_message("Workflow cargado.");
                let engine = WorkflowEngine::new(orchestrator); // Movemos orchestrator aquí

                let result = rt.block_on(engine.execute_workflow(&wf, &agent_context, file));

                match result {
                    Ok(ctx) => {
                        println!("{}", "\n✨ WORKFLOW COMPLETADO".bold().green());
                        println!("   📄 Archivo final: {:?}", ctx.current_file);
                        println!("   🔄 Pasos ejecutados: {}", ctx.step_results.len());
                    }
                    Err(e) => {
                        println!("{} {}", "❌ Error en workflow:".bold().red(), e);
                    }
                }
            } else {
                pb.finish_and_clear();
                println!("{} Workflow '{}' no encontrado.", "❌".red(), name);
                println!("   Workflows disponibles: fix-and-verify, review-security");
            }
        }
        ProCommands::Migrate { src, dst } => {
            let pb = ui::crear_progreso(&format!("Migrando {} a {}...", src, dst));

            // 1. Leer archivo origen
            let content = match std::fs::read_to_string(&src) {
                Ok(c) => c,
                Err(e) => {
                    pb.finish_and_clear();
                    println!("{} {}", "❌ Error al leer archivo origen:".bold().red(), e);
                    return;
                }
            };

            // 2. Construir tarea de migración
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "TU TAREA ES MIGRAR CÓDIGO.\n\
                    ORIGEN: Archivo '{}'\n\
                    DESTINO: Framework '{}'\n\n\
                    OBJETIVO: Reescribe el código fuente completamente para que funcione en el framework destino.\n\
                    REGLAS:\n\
                    1. ADAPTA la estructura (ej: de funciones Express a Clases NestJS con Decoradores).\n\
                    2. MANTÉN la lógica de negocio intacta.\n\
                    3. Si el destino es 'nestjs', usa Inyección de Dependencias, DTOs y Decoradores (@Controller, @Get).\n\
                    4. Si el destino es 'react', migra a Functional Components con Hooks.\n\
                    5. Genera todo el código necesario (imports, clase, export).",
                    src, dst
                ),
                task_type: TaskType::Generate, // Generate es más apropiado que Refactor para cambios drásticos
                file_path: Some(std::path::PathBuf::from(&src)),
                context: Some(format!("CÓDIGO A MIGRAR:\n{}", content)),
            };

            // 3. Ejecutar agente
            let result =
                rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));

            pb.finish_and_clear();

            // 4. Procesar resultado
            match result {
                Ok(res) => {
                    println!("{}", "🔄 MIGRACIÓN GENERADA".bold().green());

                    // Sugerir nombre de archivo destino
                    let nueva_ext = match dst.to_lowercase().as_str() {
                        "nestjs" | "angular" | "ts" => "ts",
                        "react" | "nextjs" => "tsx",
                        "rust" => "rs",
                        "python" => "py",
                        _ => "ts", // Default
                    };

                    let path_origen = std::path::Path::new(&src);
                    let nombre_base = path_origen.file_stem().unwrap().to_str().unwrap();
                    let nuevo_nombre = format!("{}.migrated.{}", nombre_base, nueva_ext);

                    if let Some(code) = res.artifacts.first() {
                        println!("\n{}", code);

                        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
                        use std::io::{self, Write};
                        print!("💾 ¿Guardar como '{}'? (s/n): ", nuevo_nombre.cyan());
                        io::stdout().flush().unwrap();

                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();

                        if input.trim().to_lowercase() == "s" {
                            if let Err(e) = std::fs::write(&nuevo_nombre, code) {
                                println!("❌ Error al guardar: {}", e);
                            } else {
                                println!("✅ Archivo guardado: {}", nuevo_nombre.green());
                                // Update Stats
                                let mut s = agent_context.stats.lock().unwrap();
                                s.total_analisis += 1;
                                s.sugerencias_aplicadas += 1;
                                s.tiempo_estimado_ahorrado_mins += 60;
                                s.guardar(&agent_context.project_root);
                            }
                        }
                    } else {
                        println!("⚠️  El agente no generó código válido.");
                        println!("{}", res.output);
                    }
                }
                Err(e) => {
                    println!("{} {}", "❌ Error en migración:".bold().red(), e);
                }
            }
        }
        ProCommands::Review => {
            let pb = ui::crear_progreso("Analizando estructura del proyecto...");

            // 1. Generar mapa del proyecto (Tree)
            let mut project_tree = String::new();
            let mut file_count = 0;

            let walker = ignore::WalkBuilder::new(&agent_context.project_root)
                .hidden(false)
                .git_ignore(true)
                .build();

            for result in walker {
                if let Ok(entry) = result {
                    let path = entry.path();
                    if let Ok(rel) = path.strip_prefix(&agent_context.project_root) {
                        let depth = rel.components().count();
                        if depth > 4 {
                            continue;
                        } // Limitar profundidad para no saturar

                        let indent = "  ".repeat(depth);
                        let name = path.file_name().unwrap_or_default().to_string_lossy();

                        project_tree.push_str(&format!("{}{}\n", indent, name));
                        file_count += 1;
                    }
                }
            }

            // 2. Leer dependencias
            let deps = crate::files::leer_dependencias(&agent_context.project_root);
            let deps_list = deps.join(", ");

            pb.finish_with_message("Estructura analizada.");

            let pb_agent =
                ui::crear_progreso("Ejecutando Auditoría de Arquitectura (ReviewerAgent)...");

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Realiza una auditoría técnica de alto nivel del proyecto. \n\
                              TU OBJETIVO: Evaluar la arquitectura, organización y stack tecnológico.\n\
                              1. Analiza la estructura de directorios: ¿Sigue buenas prácticas (DDD, Clean Arch, MVC)?\n\
                              2. Analiza las dependencias: ¿Hay librerías obsoletas o redundantes? ¿El stack es coherente?\n\
                              3. Identifica posibles cuellos de botella o deuda técnica basada en la organización.\n\
                              4. Sugiere mejoras arquitectónicas.".to_string(),
                task_type: TaskType::Analyze,
                file_path: None,
                context: Some(format!(
                    "ESTADÍSTICAS:\nArchivos escaneados: {}\n\nESTRUCTURA DE DIRECTORIOS:\n{}\n\nSTACK TECNOLÓGICO (Dependencias):\n{}", 
                    file_count, project_tree, deps_list
                )),
            };

            let result =
                rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));

            pb_agent.finish_and_clear();

            match result {
                Ok(res) => {
                    println!(
                        "{}",
                        "🏗️  AUDITORÍA DE ARQUITECTURA COMPLETADA".bold().green()
                    );
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error en Review:", e);
                }
            }
        }
        ProCommands::Explain { file } => {
            let pb = ui::crear_progreso(&format!("Analizando {} para explicación...", file));

            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    pb.finish_and_clear();
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Explica detalladamente qué hace este código, cómo funciona y sus puntos clave. Sé didáctico."
                ),
                task_type: TaskType::Analyze, // Analyze fits well for explanation
                file_path: Some(std::path::PathBuf::from(&file)),
                context: Some(content),
            };

            // Usamos CoderAgent porque suele ser mejor explicando lógica de código
            let result =
                rt.block_on(orchestrator.execute_task("CoderAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "📘 EXPLICACIÓN DE CÓDIGO".bold().cyan());
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al explicar:", e);
                }
            }
        }
        ProCommands::Optimize { file } => {
            let pb = ui::crear_progreso(&format!("Buscando optimizaciones en {}...", file));

            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    pb.finish_and_clear();
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Analiza el código en busca de cuellos de botella de rendimiento, uso ineficiente de memoria o complejidad algorítmica innecesaria. Sugiere optimizaciones concretas."
                ),
                task_type: TaskType::Analyze,
                file_path: Some(std::path::PathBuf::from(&file)),
                context: Some(content),
            };

            // ReviewerAgent es bueno encontrando problemas
            let result =
                rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "⚡ REPORTE DE OPTIMIZACIÓN".bold().yellow());
                    println!("{}", res.output);
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al optimizar:", e);
                }
            }
        }
        ProCommands::Audit { target } => {
            let path = std::path::PathBuf::from(&target);
            if !path.exists() {
                println!("{} El destino '{}' no existe.", "❌".red(), target);
                return;
            }

            let mut files_to_audit = Vec::new();
            if path.is_file() {
                files_to_audit.push(path.clone());
            } else {
                let walker = ignore::WalkBuilder::new(&path)
                    .hidden(false)
                    .git_ignore(true)
                    .build();
                for result in walker {
                    if let Ok(entry) = result {
                        let p = entry.path();
                        if p.is_file() {
                            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if agent_context
                                .config
                                .file_extensions
                                .contains(&ext.to_string())
                            {
                                files_to_audit.push(p.to_path_buf());
                            }
                        }
                    }
                }
            }

            if files_to_audit.is_empty() {
                println!(
                    "{} No se encontraron archivos cargables para auditar en '{}'.",
                    "⚠️".yellow(),
                    target
                );
                return;
            }

            println!(
                "🔍 Inciando Auditoría en {} archivos...",
                files_to_audit.len().to_string().cyan()
            );
            let mut all_issues = Vec::new();

            for (i, file_path) in files_to_audit.iter().enumerate() {
                let rel_path = file_path
                    .strip_prefix(&agent_context.project_root)
                    .unwrap_or(file_path);
                let pb = ui::crear_progreso(&format!(
                    "[{}/{}] Auditando {}...",
                    i + 1,
                    files_to_audit.len(),
                    rel_path.display()
                ));

                let content = std::fs::read_to_string(file_path).unwrap_or_default();
                let audit_prompt = format!(
                    "Realiza una auditoría técnica del archivo '{}'.\n\
                    OBJETIVO: Identificar problemas de calidad, seguridad o bugs que sean CORREGIBLES.\n\
                    REGLAS:\n\
                    1. Genera un array JSON con los problemas.\n\
                    2. Cada objeto DEBE tener: title, description, severity (High/Medium/Low), suggested_fix.\n\
                    3. Responde SOLO con el JSON.\n\n\
                    CONTENIDO:\n{}",
                    rel_path.display(),
                    content
                );

                let task = Task {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: format!("Auditando {}", rel_path.display()),
                    task_type: TaskType::Analyze,
                    file_path: Some(file_path.clone()),
                    context: Some(audit_prompt),
                };

                if let Ok(res) =
                    rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context))
                {
                    let json_str = crate::ai::utils::extraer_json(&res.output);
                    if let Ok(mut issues) = serde_json::from_str::<Vec<AuditIssue>>(&json_str) {
                        for issue in &mut issues {
                            issue.file_path = file_path.to_string_lossy().to_string();
                            all_issues.push(issue.clone());
                        }
                    }
                }
                pb.finish_and_clear();
            }

            if all_issues.is_empty() {
                println!("{} No se detectaron problemas corregibles.", "✅".green());
                return;
            }

            println!(
                "\n📑 Resumen de Auditoría ({} issues detectados):",
                all_issues.len().to_string().bold().yellow()
            );

            let options: Vec<String> = all_issues
                .iter()
                .map(|i| {
                    let rel_file = std::path::Path::new(&i.file_path)
                        .strip_prefix(&agent_context.project_root)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| i.file_path.clone());

                    let raw_str = format!(
                        "[{}] {} - {} ({})",
                        i.severity.to_uppercase(),
                        i.title,
                        i.description,
                        rel_file
                    );

                    // Truncar la línea completa agresivamente para evitar line-wraps que rompen dialoguer
                    let max_len = 90;
                    if raw_str.chars().count() > max_len {
                        format!(
                            "{}...",
                            raw_str.chars().take(max_len - 3).collect::<String>()
                        )
                    } else {
                        raw_str
                    }
                })
                .collect();

            let selected = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Selecciona los fixes que deseas aplicar (espacio=seleccionar, enter=confirmar):")
                .max_length(10)
                .items(&options)
                .interact()
                .unwrap_or_default();

            if selected.is_empty() {
                println!("   ⏭️  Operación cancelada.");
                return;
            }

            println!("\n🚀 Aplicando {} correcciones...", selected.len());

            for &idx in &selected {
                let issue = &all_issues[idx];
                let file_path = std::path::Path::new(&issue.file_path);
                let rel_file = file_path
                    .strip_prefix(&agent_context.project_root)
                    .unwrap_or(file_path);

                println!(
                    "\n🛠️  Fixing '{}' in {}...",
                    issue.title.bold(),
                    rel_file.display().to_string().cyan()
                );

                // Backup
                let backup_path = format!("{}.audit_bak", issue.file_path);
                let _ = std::fs::copy(file_path, &backup_path);

                let content = std::fs::read_to_string(file_path).unwrap_or_default();
                let fix_task = Task {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: format!(
                        "Aplica este fix específico: {}.\nPROBLEMA: {}\nSOLUCIÓN SUGERIDA: {}\nDevuelve el código COMPLETO actualizado.",
                        issue.title, issue.description, issue.suggested_fix
                    ),
                    task_type: TaskType::Fix,
                    file_path: Some(file_path.to_path_buf()),
                    context: Some(content),
                };

                let pb = ui::crear_progreso("   🤖 Generando parche...");
                let result =
                    rt.block_on(orchestrator.execute_task("CoderAgent", &fix_task, &agent_context));
                pb.finish_and_clear();

                if let Ok(res) = result {
                    if let Some(code) = res.artifacts.first() {
                        if !code.trim().is_empty() {
                            if let Err(e) = std::fs::write(file_path, code) {
                                println!("   ❌ Error escribiendo: {}", e);
                            } else {
                                println!("   ✅ Corregido.");
                                // Update Stats
                                let mut s = agent_context.stats.lock().unwrap();
                                s.total_analisis += 1;
                                s.sugerencias_aplicadas += 1;
                                s.tiempo_estimado_ahorrado_mins += 20;
                                s.guardar(&agent_context.project_root);
                            }
                        }
                    }
                }
            }

            println!("\n✨ Proceso de auditoría y corrección finalizado.");
        }
    }
}
