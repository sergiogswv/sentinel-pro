use crate::agents::base::{AgentContext, Task, TaskType};
use crate::agents::fix_suggester::FixSuggesterAgent;
use crate::agents::orchestrator::AgentOrchestrator;
use crate::agents::refactor::RefactorAgent;
use crate::agents::reviewer::ReviewerAgent;
use crate::agents::tester::TesterAgent;
use crate::commands::ProCommands;
use crate::config::SentinelConfig;
use crate::index::IndexDb;
use crate::rules::RuleLevel;
use crate::stats::SentinelStats;
use crate::ui;
use colored::*;
use dialoguer::{MultiSelect, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct AuditIssue {
    title: String,
    description: String,
    severity: String,
    suggested_fix: String,
    #[serde(default)]
    file_path: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct ReviewSuggestion {
    title: String,
    description: String,
    impact: String,
    action_item: String,
    #[serde(default)]
    files_involved: Vec<String>,
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

    let config = SentinelConfig::load(&project_root).unwrap_or_else(|| {
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

    let db_path = project_root.join(".sentinel/index.db");
    let index_db = match IndexDb::open(&db_path) {
        Ok(db) => Some(Arc::new(db)),
        Err(_) => {
            // Si falla abrirlo, intentamos crear el directorio si no existe
            let _ = std::fs::create_dir_all(project_root.join(".sentinel"));
            IndexDb::open(&db_path).ok().map(Arc::new)
        }
    };

    let stats = Arc::new(Mutex::new(SentinelStats::cargar(&project_root)));

    let agent_context = AgentContext {
        config: Arc::new(config),
        stats,
        project_root,
        index_db,
    };

    // Inicializar Orquestador y Agentes
    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.register(Arc::new(FixSuggesterAgent::new()));
    orchestrator.register(Arc::new(ReviewerAgent::new()));
    orchestrator.register(Arc::new(TesterAgent::new()));
    orchestrator.register(Arc::new(RefactorAgent::new()));

    // Ejecutar en Runtime de Tokio
    let rt = tokio::runtime::Runtime::new().unwrap();

    match subcommand {
        ProCommands::Check { target } => {
            let path = agent_context.project_root.join(&target);
            if !path.exists() {
                println!("{} El destino '{}' no existe en el proyecto.", "❌".red(), target);
                return;
            }

            let mut files_to_check = Vec::new();
            if path.is_file() {
                files_to_check.push(path.clone());
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
                                files_to_check.push(p.to_path_buf());
                            }
                        }
                    }
                }
            }

            if files_to_check.is_empty() {
                println!("{} No se encontraron archivos para revisar en '{}'.", "⚠️".yellow(), target);
                return;
            }

            println!("\n{} Ejecutando Capa 1 (Análisis Estático) en {} archivos...", "⚡".cyan(), files_to_check.len());

            let mut rule_engine = crate::rules::engine::RuleEngine::new();
            if let Some(ref db) = agent_context.index_db {
                rule_engine = rule_engine.with_index_db(Arc::clone(db));
            }
            let rules_path = agent_context.project_root.join(".sentinel/rules.yaml");
            if rules_path.exists() {
                let _ = rule_engine.load_from_yaml(&rules_path);
            }

            let mut total_violations = 0;
            for file_path in &files_to_check {
                let content = std::fs::read_to_string(file_path).unwrap_or_default();
                let violations = rule_engine.validate_file(file_path, &content);

                if !violations.is_empty() {
                    let rel_path = file_path
                        .strip_prefix(&agent_context.project_root)
                        .unwrap_or(file_path);
                    
                    println!("\n📄 {}", rel_path.display().to_string().bold().cyan());
                    for v in &violations {
                        let level_icon = match v.level {
                            RuleLevel::Error => "❌ ERROR".red(),
                            RuleLevel::Warning => "⚠️  WARN ".yellow(),
                            RuleLevel::Info => "ℹ️  INFO ".blue(),
                        };
                        println!("   {} [{}]: {}", level_icon, v.rule_name.yellow(), v.message);
                    }
                    total_violations += violations.len();
                }
            }

            if total_violations == 0 {
                println!("\n✅ {} No se detectaron problemas estáticos.", "Perfecto:".green());
            } else {
                println!("\n🚩 Se detectaron {} problemas potenciales.", total_violations.to_string().red().bold());
            }
        }
        ProCommands::Analyze { file } => {
            let path = agent_context.project_root.join(&file);
            println!("\n🔍 Analizando: {}", file.cyan().bold());

            // Leer contenido del archivo
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            // --- CAPA 1: Análisis Estático (Tree-sitter) ---
            let mut rule_engine = crate::rules::engine::RuleEngine::new();
            if let Some(ref db) = agent_context.index_db {
                rule_engine = rule_engine.with_index_db(Arc::clone(db));
            }
            let rules_path = agent_context.project_root.join(".sentinel/rules.yaml");
            if rules_path.exists() {
                let _ = rule_engine.load_from_yaml(&rules_path);
            }

            let pb_static = ui::crear_progreso("   ⚡ Ejecutando análisis estático (L1)...");
            let static_violations = rule_engine.validate_file(&path, &content);
            pb_static.finish_and_clear();

            if !static_violations.is_empty() {
                println!("{}", "🚩 VIOLACIONES ESTÁTICAS DETECTADAS:".red().bold());
                for v in &static_violations {
                    let level_icon = match v.level {
                        RuleLevel::Error => "❌ ERROR".red(),
                        RuleLevel::Warning => "⚠️  WARN ".yellow(),
                        RuleLevel::Info => "ℹ️  INFO ".blue(),
                    };
                    println!("   {} [{}]: {}", level_icon, v.rule_name.yellow(), v.message);
                }
                println!();
            } else {
                println!("   ✅ Capa 1: No se detectaron violaciones estáticas.\n");
            }

            // --- CAPA 2: Análisis Semántico con IA ---
            let pb_ana = ui::crear_progreso(&format!("   🤖 Consultando Guardián de IA (L2) para {}...", file));

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!(
                    "Actúa como el Guardián de Calidad para el archivo '{}'.\n\
                    TU OBJETIVO: Identificar problemas profundos de arquitectura, lógica de negocio, seguridad y cuellos de botella de RENDIMIENTO que el análisis estático no puede detectar.\n\n\
                    INSTRUCCIONES DE RESPUESTA:\n\
                    1. Inicia con un análisis técnico detallado (incluyendo sugerencias de optimización).\n\
                    2. FINALIZA TU RESPUESTA OBLIGATORIAMENTE con un bloque JSON (```json) que contenga un array de acciones recomendadas (objeto AuditIssue).\n\n\
                    ESTRUCTURA DEL JSON:\n\
                    ```json\n\
                    [\n\
                      {{\n\
                        \"title\": \"Nombre de la mejora/optimización\",\n\
                        \"description\": \"Por qué es necesaria\",\n\
                        \"severity\": \"High/Medium/Low\",\n\
                        \"suggested_fix\": \"Instrucción técnica para el FixSuggesterAgent\"\n\
                      }}\n\
                    ]\n\
                    ```", 
                    file
                ),
                task_type: TaskType::Analyze,
                file_path: Some(path.clone()),
                context: Some(content.clone()),
            };

            let result =
                rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));

            pb_ana.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🔍 ANÁLISIS COMPLETADO".bold().green());
                    
                    // Mostrar reporte humano (sin el código JSON)
                    let report_only = crate::ai::utils::eliminar_bloques_codigo(&res.output);
                    println!("{}", report_only);

                    // 3. Extraer y procesar sugerencias JSON
                    let json_str = crate::ai::utils::extraer_json(&res.output);
                    if let Ok(issues) = serde_json::from_str::<Vec<AuditIssue>>(&json_str) {
                         if !issues.is_empty() {
                            println!("\n💡 Se detectaron {} acciones recomendadas.", issues.len().to_string().cyan());
                            
                            let options: Vec<String> = issues.iter()
                                .map(|i| format!("[{}] {} - {}", i.severity.to_uppercase(), i.title.bold(), i.description))
                                .collect();

                            let selected = MultiSelect::with_theme(&ColorfulTheme::default())
                                .with_prompt("Selecciona las acciones que deseas ejecutar:")
                                .items(&options)
                                .interact()
                                .unwrap_or_default();

                            if !selected.is_empty() {
                                println!("\n🚀 Aplicando {} mejoras seleccionadas...", selected.len());
                                
                                for &idx in &selected {
                                    let issue = &issues[idx];
                                    println!("\n🛠️  Ejecutando: {}", issue.title.cyan().bold());

                                    // Para cada fix, leemos el contenido ACTUAL (que pudo cambiar en el paso anterior)
                                    let current_content = std::fs::read_to_string(&path).unwrap_or_else(|_| content.clone());
                                    
                                    let pb_fix = ui::crear_progreso("   🤖 Generando cambios...");
                                    
                                    let fix_task = Task {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        description: format!(
                                            "Aplica la siguiente mejora específica al archivo {}.\n\
                                            TÍTULO: {}\n\
                                            DESCRIPCIÓN: {}\n\
                                            ACCIÓN REQUERIDA: {}\n\n\
                                            Devuelve el código COMPLETO actualizado para este archivo.",
                                            file, issue.title, issue.description, issue.suggested_fix
                                        ),
                                        task_type: TaskType::Fix,
                                        file_path: Some(path.clone()),
                                        context: Some(current_content),
                                    };

                                    let fix_result = rt.block_on(orchestrator.execute_task("FixSuggesterAgent", &fix_task, &agent_context));
                                    pb_fix.finish_and_clear();

                                    if let Ok(f_res) = fix_result {
                                        if let Some(code) = f_res.artifacts.first() {
                                            if !code.trim().is_empty() {
                                                if let Err(e) = std::fs::write(&path, code) {
                                                    println!("   ❌ Error al guardar en {}: {}", file, e);
                                                } else {
                                                    println!("   ✅ Mejora '{}' aplicada.", issue.title.green());
                                                    
                                                    let mut s = agent_context.stats.lock().unwrap();
                                                    s.total_analisis += 1;
                                                    s.sugerencias_aplicadas += 1;
                                                    s.tiempo_estimado_ahorrado_mins += 15;
                                                    s.guardar(&agent_context.project_root);
                                                }
                                            }
                                        }
                                    }
                                }
                                println!("\n✨ Todas las mejoras seleccionadas han sido procesadas.");
                            }
                         }
                    } else {
                        // Si no hay JSON o falla el parse, simplemente no mostramos el menú interactivo
                        // pero el reporte humano ya se mostró arriba.
                    }
                }
                Err(e) => {
                    println!("{} {}", "❌ Error al analizar:".bold().red(), e);
                }
            }
        }
        ProCommands::Report { format } => {
            println!("\n📊 Generando Reporte de Calidad del Proyecto...");
            
            let mut rule_engine = crate::rules::engine::RuleEngine::new();
            if let Some(ref db) = agent_context.index_db {
                rule_engine = rule_engine.with_index_db(Arc::clone(db));
            }
            let rules_path = agent_context.project_root.join(".sentinel/rules.yaml");
            if rules_path.exists() {
                let _ = rule_engine.load_from_yaml(&rules_path);
            }

            let walker = ignore::WalkBuilder::new(&agent_context.project_root)
                .hidden(false)
                .git_ignore(true)
                .build();

            let mut files_count = 0;
            let mut total_violations = 0;
            let mut errors = 0;
            let mut warnings = 0;
            let mut info = 0;
            let mut violations_list = Vec::new();

            for result in walker {
                if let Ok(entry) = result {
                    let p = entry.path();
                    if p.is_file() {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if agent_context.config.file_extensions.contains(&ext.to_string()) {
                            files_count += 1;
                            let content = std::fs::read_to_string(p).unwrap_or_default();
                            let file_violations = rule_engine.validate_file(p, &content);
                            
                            // Guardar métricas en el historial (SQLite)
                            if let Some(ref db) = agent_context.index_db {
                                let history = crate::index::quality_history::QualityHistory::new(db);
                                let mut dead_func = 0;
                                let mut unused_imp = 0;
                                for v in &file_violations {
                                    if v.rule_name.contains("DEAD_CODE") { dead_func += 1; }
                                    if v.rule_name.contains("UNUSED_IMPORT") { unused_imp += 1; }
                                }
                                let _ = history.record_metrics(&crate::index::quality_history::FileMetrics {
                                    file_path: p.strip_prefix(&agent_context.project_root).unwrap_or(p).to_string_lossy().to_string(),
                                    dead_functions: dead_func,
                                    unused_imports: unused_imp,
                                    complexity_score: 0.0, // TODO: Extraer complejidad real
                                    violations_count: file_violations.len() as i32,
                                    tests_passing: true,
                                });
                            }
                            for v in &file_violations {
                                total_violations += 1;
                                match v.level {
                                    crate::rules::RuleLevel::Error => errors += 1,
                                    crate::rules::RuleLevel::Warning => warnings += 1,
                                    crate::rules::RuleLevel::Info => info += 1,
                                }
                                
                                violations_list.push(serde_json::json!({
                                    "file": p.strip_prefix(&agent_context.project_root).unwrap_or(p).to_string_lossy(),
                                    "rule": v.rule_name,
                                    "message": v.message,
                                    "level": format!("{:?}", v.level)
                                }));
                            }
                        }
                    }
                }
            }

            let report_data = serde_json::json!({
                "project": agent_context.config.project_name,
                "framework": agent_context.config.framework,
                "timestamp": chrono::Local::now().to_rfc3339(),
                "summary": {
                    "total_files": files_count,
                    "total_violations": total_violations,
                    "errors": errors,
                    "warnings": warnings,
                    "info": info
                },
                "violations": violations_list
            });

            if format == "json" {
                let json_output = serde_json::to_string_pretty(&report_data).unwrap();
                let output_path = agent_context.project_root.join("sentinel-report.json");
                std::fs::write(&output_path, json_output).expect("Error al escribir reporte JSON");
                println!("✅ Reporte JSON generado en: {}", output_path.display().to_string().cyan());
            } else if format == "html" {
                 let html_template = format!(
                     "<!DOCTYPE html><html><head><meta charset='UTF-8'><title>Sentinel Report - {project}</title>\
                     <style>body {{ font-family: 'Segoe UI', Roboto, sans-serif; padding: 40px; background: #f8f9fa; color: #333; }}\
                     .card {{ background: white; padding: 25px; border-radius: 12px; box-shadow: 0 4px 15px rgba(0,0,0,0.05); margin-bottom: 25px; }}\
                     h1 {{ color: #1a202c; border-bottom: 3px solid #4a90e2; padding-bottom: 12px; display: flex; align-items: center; gap: 10px; }}\
                     .summary {{ display: flex; gap: 20px; flex-wrap: wrap; justify-content: space-between; }}\
                     .stat {{ flex: 1; min-width: 140px; text-align: center; padding: 20px; border-radius: 10px; color: white; transition: transform 0.2s; }}\
                     .stat:hover {{ transform: translateY(-3px); }}\
                     .bg-blue {{ background: #4a90e2; }} .bg-red {{ background: #e53e3e; }} .bg-orange {{ background: #ed8936; }} .bg-green {{ background: #48bb78; }}\
                     table {{ width: 100%; border-collapse: separate; border-spacing: 0; margin-top: 20px; overflow: hidden; border-radius: 8px; border: 1px solid #eee; }}\
                     th, td {{ padding: 14px; text-align: left; border-bottom: 1px solid #eee; }}\
                     th {{ background-color: #f1f5f9; color: #4a5568; font-weight: 600; text-transform: uppercase; font-size: 12px; letter-spacing: 0.05em; }}\
                     tr:hover {{ background-color: #fdfdfd; }}\
                     .level-error {{ color: #e53e3e; font-weight: bold; padding: 4px 8px; background: #fff5f5; border-radius: 4px; }}\
                     .level-warning {{ color: #dd6b20; font-weight: bold; padding: 4px 8px; background: #fffaf0; border-radius: 4px; }}\
                     .level-info {{ color: #3182ce; font-weight: bold; padding: 4px 8px; background: #ebf8ff; border-radius: 4px; }}\
                     </style></head><body>\
                     <h1>🛡️ Sentinel Quality Report: {project}</h1>\
                     <div class='card summary'>\
                        <div class='stat bg-blue'><h3>Archivos</h3><p style='font-size: 24px; font-weight: bold;'>{files}</p></div>\
                        <div class='stat bg-red'><h3>Errores</h3><p style='font-size: 24px; font-weight: bold;'>{errors}</p></div>\
                        <div class='stat bg-orange'><h3>Avisos</h3><p style='font-size: 24px; font-weight: bold;'>{warnings}</p></div>\
                        <div class='stat bg-green'><h3>Info</h3><p style='font-size: 24px; font-weight: bold;'>{info}</p></div>\
                     </div>\
                     <div class='card'>\
                        <h2>Hallazgos de Capa 1 ({total})</h2>\
                        <table><thead><tr><th>Archivo</th><th>Nivel</th><th>Regla</th><th>Mensaje</th></tr></thead><tbody>",
                     project = agent_context.config.project_name,
                     files = files_count,
                     errors = errors,
                     warnings = warnings,
                     info = info,
                     total = total_violations
                 );
                 let mut rows = String::new();
                 for v in report_data["violations"].as_array().unwrap() {
                     let level_label = v["level"].as_str().unwrap();
                     let level_class = match level_label {
                         "Error" => "level-error",
                         "Warning" => "level-warning",
                         "Info" => "level-info",
                         _ => "",
                     };
                     rows.push_str(&format!(
                         "<tr><td><code style='color: #4a5568;'>{file}</code></td><td><span class='{class}'>{level}</span></td><td><strong style='color: #2d3748;'>{rule}</strong></td><td>{msg}</td></tr>",
                         file = v["file"].as_str().unwrap(),
                         class = level_class,
                         level = level_label.to_uppercase(),
                         rule = v["rule"].as_str().unwrap(),
                         msg = v["message"].as_str().unwrap()
                     ));
                 }
                 let final_html = format!("{}{}{}</tbody></table></div><p style='text-align: center; color: #a0aec0; font-size: 13px;'>Generado por Sentinel Pro • {date}</p></body></html>", 
                     html_template, rows, "", date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
                 let output_path = agent_context.project_root.join("sentinel-report.html");
                 std::fs::write(&output_path, final_html).expect("Error al escribir reporte HTML");
                 println!("✅ Reporte HTML generado en: {}", output_path.display().to_string().cyan());
            } else {
                println!("⚠️ Formato '{}' no soportado. Usa json o html.", format);
            }
        }
        ProCommands::Refactor { file } => {
            let path = agent_context.project_root.join(&file);
            // Leer contenido original
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let pb = ui::crear_progreso(&format!("Refactorizando {}...", file));

            // Crear Backup
            let backup_path = path.with_extension(format!("{}.bak", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
            if let Err(e) = std::fs::copy(&path, &backup_path) {
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
                file_path: Some(path.clone()),
                context: Some(content),
            };

            let result =
                rt.block_on(orchestrator.execute_with_guard("RefactorAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🛠️ REFACTORIZACIÓN COMPLETADA".bold().green());
                    println!("   🔙 Backup creado en: {}", backup_path.display().to_string().dimmed());

                    if let Some(code) = res.artifacts.first() {
                        match std::fs::write(&path, code) {
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
            let path = agent_context.project_root.join(&file);
            // Leer contenido original
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    println!("{} {}", "❌ Error al leer archivo:".bold().red(), e);
                    return;
                }
            };

            let pb = ui::crear_progreso(&format!("Corrigiendo bugs en {}...", file));

            // Crear Backup
            let backup_path = path.with_extension(format!("{}.bak", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
            let _ = std::fs::copy(&path, &backup_path);

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Identifica y corrige bugs en el archivo {}.", file),
                task_type: TaskType::Fix,
                file_path: Some(path.clone()),
                context: Some(content),
            };

            // Usamos CoderAgent para fixes por ahora
            let result =
                rt.block_on(orchestrator.execute_with_guard("FixSuggesterAgent", &task, &agent_context));

            pb.finish_and_clear();

            match result {
                Ok(res) => {
                    println!("{}", "🩹 BUGS CORREGIDOS".bold().green());
                    if let Some(code) = res.artifacts.first() {
                        match std::fs::write(&path, code) {
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
                description: "Analiza el proyecto y genera pruebas unitarias para los archivos que no tienen cobertura. \
                              IMPORTANTE 1: Todos los tests generados deben apuntar obligatoriamente a la carpeta raíz `test/` (o su equivalente), NO dentro de root/src. \
                              IMPORTANTE 2: Para cada archivo generado, envuelve el código en un bloque markdown y la PRIMERA LÍNEA del código DEBE ser un comentario con la ruta de destino. \
                              Ejemplo: // test/components/auth.spec.ts".to_string(),
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
                    for artifact in &res.artifacts {
                        println!("\n{}\n", artifact);
                    }

                    println!("{}", "\n📝 Detalles:".bold());
                    println!("{}", res.output);

                    let ok = dialoguer::Confirm::new()
                        .with_prompt("¿Deseas guardar automáticamente los tests generados en tu proyecto?")
                        .default(false)
                        .interact()
                        .unwrap_or(false);

                    if ok {
                        let mut saved = 0;
                        let mut current_block = String::new();
                        let mut in_block = false;

                        for line in res.output.lines() {
                            if line.starts_with("```") {
                                if in_block {
                                    // procesar bloque
                                    let mut lines = current_block.lines();
                                    if let Some(first_line) = lines.next() {
                                        let trimmed = first_line.trim();
                                        if trimmed.starts_with("//") || trimmed.starts_with("#") {
                                            let path_str = trimmed
                                                .trim_start_matches(|c| c == '/' || c == '#' || c == ' ')
                                                .to_string();
                                                
                                            if path_str.contains('.') {
                                                let target = agent_context.project_root.join(&path_str);
                                                if let Some(parent) = target.parent() {
                                                    let _ = std::fs::create_dir_all(parent);
                                                }
                                                if let Ok(_) = std::fs::write(&target, &current_block) {
                                                    println!("   💾 Test guardado: {}", path_str.cyan());
                                                    saved += 1;
                                                }
                                            }
                                        }
                                    }
                                    current_block.clear();
                                    in_block = false;
                                } else {
                                    in_block = true;
                                }
                            } else if in_block {
                                current_block.push_str(line);
                                current_block.push('\n');
                            }
                        }

                        if saved > 0 {
                            println!("{}", format!("✅ {} archivos de prueba guardados.", saved).green());

                            // NUEVO: Preguntar para ejecutar tests y auto-fix
                            let run_tests = dialoguer::Confirm::new()
                                .with_prompt("¿Deseas ejecutar los tests ahora y solucionar problemas si fallan?")
                                .default(false)
                                .interact()
                                .unwrap_or(false);

                            if run_tests {
                                let ref test_cmd = agent_context.config.test_command;
                                println!("🚀 Ejecutando tests: {}", test_cmd.cyan());

                                let mut cmd_parts = test_cmd.split_whitespace();
                                if let Some(program) = cmd_parts.next() {
                                    let args: Vec<&str> = cmd_parts.collect();
                                    
                                    let output_result = std::process::Command::new(program)
                                        .args(&args)
                                        .current_dir(&agent_context.project_root)
                                        .output();

                                    match output_result {
                                        Ok(out) => {
                                            if out.status.success() {
                                                println!("{}", "✅ Todos los tests pasaron exitosamente.".green());
                                            } else {
                                                let error_output = String::from_utf8_lossy(&out.stderr).to_string() 
                                                    + &String::from_utf8_lossy(&out.stdout).to_string();
                                                    
                                                println!("{}", "❌ Algunos tests fallaron. Intentando auto-arreglar...".red());
                                                
                                                let pb_fix = ui::crear_progreso("Diagnosticando el fallo con AI...");
                                                let fix_task = Task {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    description: format!(
                                                        "Acabamos de generar y ejecutar tests, pero fallaron con esta salida:\n\n{}\n\nRevisa el error, encuentra el fallo lógico y proporciona SOLO el código corregido.", 
                                                        error_output
                                                    ),
                                                    task_type: TaskType::Fix,
                                                    file_path: None,
                                                    context: Some(error_output),
                                                };

                                                let fix_result = rt.block_on(orchestrator.execute_task("FixSuggesterAgent", &fix_task, &agent_context));
                                                pb_fix.finish_and_clear();

                                                match fix_result {
                                                    Ok(f_res) => {
                                                        println!("{}", "🩹 SUGERENCIAS DE CORRECCIÓN".bold().green());
                                                        println!("{}", f_res.output);
                                                    },
                                                    Err(e) => println!("{} {}", "❌ Error al intentar aplicar fix:".red(), e),
                                                }
                                            }
                                        },
                                        Err(e) => println!("{} {}", "❌ Error del sistema al intentar ejecutar tests:".red(), e),
                                    }
                                }
                            }
                        } else {
                            println!("{}", "⚠️ No se pudo extraer automáticamente las rutas. Recuerda que la IA debe poner el nombre del archivo como comentario en la primera línea.".yellow());
                        }
                    }
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
        ProCommands::CleanCache { target } => {
            let path_str = target.unwrap_or_else(|| ".".to_string());
            let target_path = agent_context.project_root.join(&path_str);

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
                             agent: "FixSuggesterAgent".to_string(),
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
                             agent: "FixSuggesterAgent".to_string(),
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
                              2. Analiza las dependencias.\n\
                              3. Identifica cuellos de botella o deuda técnica.\n\n\
                              INSTRUCCIONES DE SALIDA:\n\
                              - Primero escribe tu análisis detallado en lenguaje humano.\n\
                              - AL FINAL DE TODO, añade un bloque de código JSON con sugerencias que el usuario pueda seleccionar para desarrollar.\n\
                              - NO incluyas explicaciones dentro del bloque JSON.\n\n\
                              ESTRUCTURA DEL JSON (Obligatorio):\n\
                              ```json\n\
                              [\n\
                                {\n\
                                  \"title\": \"Título breve\",\n\
                                  \"description\": \"Descripción de la mejora\",\n\
                                  \"impact\": \"High/Medium/Low\",\n\
                                  \"action_item\": \"Instrucción técnica para el CoderAgent\",\n\
                                  \"files_involved\": [\"ruta/al/archivo\"]\n\
                                }\n\
                              ]\n\
                              ```".to_string(),
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
                    
                    // Mostrar solo el texto humano, ocultar el JSON del output principal
                    let report_only = crate::ai::utils::eliminar_bloques_codigo(&res.output);
                    println!("{}", report_only);

                    // 3. Extraer y procesar sugerencias JSON
                    let json_str = crate::ai::utils::extraer_json(&res.output);
                    if let Ok(mut suggestions) = serde_json::from_str::<Vec<ReviewSuggestion>>(&json_str) {
                         while !suggestions.is_empty() {
                            println!("\n💡 {} sugerencias de mejora detectadas.", suggestions.len().to_string().cyan());
                            
                            let mut options: Vec<String> = suggestions.iter()
                                .map(|s| format!("[{}] {} - {}", s.impact.to_uppercase(), s.title.bold(), s.description))
                                .collect();
                            
                            options.push("🚪 Salir".to_string());

                            let selection = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Selecciona una sugerencia para desarrollar:")
                                .items(&options)
                                .default(0)
                                .interact_opt()
                                .unwrap_or(None);

                            match selection {
                                Some(idx) if idx < suggestions.len() => {
                                    let suggestion = &suggestions[idx];
                                    println!("\n🚀 Desarrollando: {}", suggestion.title.cyan().bold());
                                    
                                    // Ejecutar implementación
                                    let pb_dev = ui::crear_progreso(&format!("Aplicando mejora: {}...", suggestion.title));
                                    
                                    let dev_task = Task {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        description: format!(
                                            "IMPLEMENTACIÓN DE MEJORA ARQUITECTÓNICA\n\n\
                                            TÍTULO: {}\n\
                                            DESCRIPCIÓN: {}\n\
                                            ACCIÓN REQUERIDA: {}\n\n\
                                            OBJETIVO: Realiza los cambios necesarios en el proyecto para implementar esta mejora. \
                                            Si se mencionan archivos específicos ({:?}), priorízalos. \
                                            Devuelve el código COMPLETO corregido o las nuevas implementaciones.",
                                            suggestion.title, suggestion.description, suggestion.action_item, suggestion.files_involved
                                        ),
                                        task_type: TaskType::Fix,
                                        file_path: suggestion.files_involved.first().map(|f| std::path::PathBuf::from(f)),
                                        context: None,
                                    };

                                    let dev_result = rt.block_on(orchestrator.execute_task("FixSuggesterAgent", &dev_task, &agent_context));
                                    pb_dev.finish_and_clear();

                                    match dev_result {
                                        Ok(d_res) => {
                                            println!("{}", "\n✨ MEJORA GENERADA".bold().green());
                                            
                                            if let Some(code) = d_res.artifacts.first() {
                                                if !code.trim().is_empty() {
                                                    println!("\n{}", code);
                                                    
                                                    let apply = dialoguer::Confirm::new()
                                                        .with_prompt("¿Deseas aplicar estos cambios automáticamente?")
                                                        .default(true)
                                                        .interact()
                                                        .unwrap_or(false);
                                                        
                                                    if apply {
                                                        if let Some(file_path_str) = suggestion.files_involved.first() {
                                                            let file_path = agent_context.project_root.join(file_path_str);
                                                            if let Some(parent) = file_path.parent() {
                                                                let _ = std::fs::create_dir_all(parent);
                                                            }
                                                            
                                                            if let Err(e) = std::fs::write(&file_path, code) {
                                                                println!("   ❌ Error al guardar en {}: {}", file_path.display(), e);
                                                            } else {
                                                                println!("   ✅ Cambios aplicados con éxito en {}.", file_path_str.green());
                                                                
                                                                let mut s = agent_context.stats.lock().unwrap();
                                                                s.sugerencias_aplicadas += 1;
                                                                s.tiempo_estimado_ahorrado_mins += 30;
                                                                s.guardar(&agent_context.project_root);
                                                                
                                                                // ELIMINAR LA SUGERENCIA YA APLICADA
                                                                suggestions.remove(idx);
                                                            }
                                                        } else {
                                                            println!("   ⚠️ No se especificó un archivo de destino en la sugerencia. Por favor, aplica los cambios manualmente.");
                                                            // Aun así la quitamos para no repetir el prompt si el usuario no puede hacer nada
                                                            suggestions.remove(idx);
                                                        }
                                                    }
                                                } else {
                                                    println!("{}", d_res.output);
                                                }
                                            } else {
                                                println!("{}", d_res.output);
                                            }
                                        },
                                        Err(e) => println!("{} {}", "\n❌ Error al desarrollar la sugerencia:".red(), e),
                                    }
                                },
                                _ => break, // Salir del loop (Selección de "Salir" o Esc)
                            }
                         }
                         if suggestions.is_empty() {
                             println!("\n✨ {} Todas las sugerencias han sido procesadas o aplicadas.", "Review completado:".green());
                         }
                    } else {
                        println!("\n{} No se pudo procesar el bloque de sugerencias inteligentes.", "⚠️".yellow());
                    }
                }
                Err(e) => {
                    println!("{} {}", "❌ Error en Review:", e);
                }
            }
        }
        ProCommands::Audit { target } => {
            let path = agent_context.project_root.join(&target);
            if !path.exists() {
                println!("{} El destino '{}' no existe en el proyecto.", "❌".red(), target);
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
                    rt.block_on(orchestrator.execute_task("FixSuggesterAgent", &fix_task, &agent_context));
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
