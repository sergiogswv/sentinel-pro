use crate::agents::base::{Agent, AgentContext, Task, TaskType};
use crate::agents::orchestrator::AgentOrchestrator;
use crate::ui;
use colored::*;
use dialoguer::{Select, theme::ColorfulTheme, Confirm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewRecord {
    pub timestamp: String,
    pub project_root: String,
    pub files_reviewed: usize,
    pub suggestions: Vec<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ReviewSuggestion {
    pub title: String,
    pub description: String,
    pub impact: String,
    pub action_item: String,
    #[serde(default)]
    pub files_involved: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum ReviewMode { Small, Medium, Large }

pub fn review_size_mode(file_count: usize) -> ReviewMode {
    if file_count < 20 { ReviewMode::Small }
    else if file_count <= 80 { ReviewMode::Medium }
    else { ReviewMode::Large }
}

pub fn save_review_record(project_root: &std::path::Path, record: &ReviewRecord) -> anyhow::Result<()> {
    let dir = project_root.join(".sentinel").join("reviews");
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{}.json", record.timestamp.replace(':', "-").replace('T', "-"));
    let path = dir.join(&filename);
    let json = serde_json::to_string_pretty(record)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_review_records(project_root: &std::path::Path) -> Vec<ReviewRecord> {
    let dir = project_root.join(".sentinel").join("reviews");
    if !dir.exists() { return vec![]; }
    let mut records: Vec<ReviewRecord> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                .filter_map(|e| {
                    std::fs::read_to_string(e.path()).ok()
                        .and_then(|s| serde_json::from_str::<ReviewRecord>(&s).ok())
                })
                .collect()
        })
        .unwrap_or_default();
    records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    records
}

pub fn diff_reviews(
    old: &[serde_json::Value],
    new: &[serde_json::Value],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let old_titles: std::collections::HashSet<String> = old.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .map(|t| t.to_lowercase())
        .collect();
    let new_titles: std::collections::HashSet<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .map(|t| t.to_lowercase())
        .collect();

    let resolved: Vec<String> = old.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| !new_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();
    let added: Vec<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| !old_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();
    let persistent: Vec<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| old_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();

    (resolved, added, persistent)
}

pub fn handle_review(
    history: bool,
    diff: bool,
    _quiet: bool,
    _verbose: bool,
    agent_context: &AgentContext,
    output_mode: crate::commands::OutputMode,
    rt: &tokio::runtime::Runtime,
) {
    if output_mode == crate::commands::OutputMode::Verbose {
        eprintln!("[DEBUG] Generating review report");
    }

    if history {
        let records = load_review_records(&agent_context.project_root);
        if records.is_empty() {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!("📋 No hay reviews guardados aún. Ejecuta `sentinel pro review` para generar el primero.");
            }
        } else {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!("📋 Historial de reviews ({}):", records.len());
                for r in records.iter().rev().take(5) {
                    let first_title = r.suggestions.first()
                        .and_then(|s| s.get("title"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("(sin sugerencias)");
                    println!(
                        "  {}  ·  {} sugerencia(s)  ·  \"{}\"",
                        r.timestamp, r.suggestions.len(), first_title
                    );
                }
            }
        }
        return;
    }

    if diff {
        let records = load_review_records(&agent_context.project_root);
        if records.len() < 2 {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!("⚠️  Se necesitan al menos 2 reviews para comparar. Ejecuta `sentinel pro review` dos veces.");
            }
        } else {
            let prev = &records[records.len() - 2];
            let last = &records[records.len() - 1];
            let (resolved, added, persistent) = diff_reviews(&prev.suggestions, &last.suggestions);
            if output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "🔍 Comparando reviews ({} vs {}):",
                    prev.timestamp, last.timestamp
                );
                if !resolved.is_empty() {
                    println!("  ✅ Resueltas ({}):", resolved.len());
                    for t in &resolved { println!("     \"{}\"", t); }
                }
                if !added.is_empty() {
                    println!("  🆕 Nuevas ({}):", added.len());
                    for t in &added { println!("     \"{}\"", t); }
                }
                if !persistent.is_empty() {
                    println!("  ⏳ Persistentes ({}):", persistent.len());
                    for t in persistent.iter().take(5) { println!("     \"{}\"", t); }
                    if persistent.len() > 5 {
                        println!("     ... y {} más", persistent.len() - 5);
                    }
                }
            }
        }
        return;
    }

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

    // Cap del árbol de directorios a 100 líneas
    let project_tree = {
        let lines: Vec<&str> = project_tree.lines().collect();
        if lines.len() > 100 {
            format!(
                "{}\n... (proyecto grande, se muestran primeras 100 líneas del árbol)",
                lines[..100].join("\n")
            )
        } else {
            project_tree
        }
    };

    // 3. Muestra de archivos fuente reales (máx 8 archivos, 100 líneas c/u)
    // Prioriza src/ y tipos de archivo NestJS/arquitectura relevante.
    let dirs_ignorados = [
        "node_modules", "dist", "build", ".next", ".nuxt",
        "vendor", "target", ".git", "__pycache__", "coverage",
    ];
    // Recolectar todos los candidatos primero para poder priorizarlos
    let walk_root = {
        let src = agent_context.project_root.join("src");
        if src.exists() { src } else { agent_context.project_root.clone() }
    };
    let walker_src = ignore::WalkBuilder::new(&walk_root)
        .hidden(false)
        .git_ignore(true)
        .build();
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    for entry_result in walker_src {
        if let Ok(entry) = entry_result {
            let p = entry.path();
            if dirs_ignorados.iter().any(|d| p.components().any(|c| c.as_os_str() == *d)) {
                continue;
            }
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if agent_context.config.file_extensions.contains(&ext.to_string()) {
                candidates.push(p.to_path_buf());
            }
        }
    }

    // Build set of changed files (those matching configured extensions)
    let changed_files = super::render::get_changed_files(&agent_context.project_root);
    let changed_set: std::collections::HashSet<std::path::PathBuf> = changed_files
        .into_iter()
        .filter(|cf| {
            let ext = cf.extension().and_then(|e| e.to_str()).unwrap_or("");
            agent_context.config.file_extensions.contains(&ext.to_string())
        })
        .collect();
    let changed_count = changed_set.len();

    // Unified 3-tier sort: changed files (0) > architecture patterns (1) > rest (2)
    let priority_patterns = [
        ".service.ts", ".module.ts", ".controller.ts",
        ".gateway.ts", ".repository.ts", ".entity.ts",
    ];
    candidates.sort_by_key(|p| {
        if changed_set.contains(p) {
            0usize
        } else {
            let name = p.to_string_lossy();
            if priority_patterns.iter().any(|pat| name.ends_with(pat)) { 1usize } else { 2usize }
        }
    });

    let mut codigo_muestra = String::new();
    let mut muestras = 0usize;
    let mut total_lines_loaded = 0usize;

    let review_mode = review_size_mode(candidates.len());

    match review_mode {
        ReviewMode::Small => {
            // < 20 files: top 8 × 100 lines
            for p in candidates.iter().take(8) {
                if let Ok(contenido) = std::fs::read_to_string(p) {
                    let lines: Vec<&str> = contenido.lines().collect();
                    let preview_lines = lines.len().min(100);
                    codigo_muestra.push_str(&format!(
                        "\n\n=== {} ===\n{}",
                        p.strip_prefix(&agent_context.project_root)
                            .map(|r| r.display().to_string())
                            .unwrap_or_else(|_| p.display().to_string()),
                        lines[..preview_lines].join("\n")
                    ));
                    muestras += 1;
                    total_lines_loaded += preview_lines;
                }
            }
        }
        ReviewMode::Medium => {
            // 20-80 files: centrality-based selection, top 20 × 150 lines
            let central_files: Vec<std::path::PathBuf> = if let Some(ref db) = agent_context.index_db {
                let conn = db.lock();
                let mut stmt = conn.prepare(
                    "SELECT s.file_path, COUNT(*) as hits \
                     FROM call_graph c \
                     JOIN symbols s ON c.callee_symbol = s.name \
                     GROUP BY s.file_path \
                     ORDER BY hits DESC \
                     LIMIT 20"
                ).ok();
                if let Some(ref mut stmt) = stmt {
                    stmt.query_map([], |row| row.get::<_, String>(0))
                        .map(|rows| rows.flatten().map(std::path::PathBuf::from).collect())
                        .unwrap_or_default()
                } else { vec![] }
            } else { vec![] };

            let selected = if central_files.is_empty() {
                candidates.iter().take(20).cloned().collect::<Vec<_>>()
            } else {
                central_files
            };

            for p in selected.iter().take(20) {
                if let Ok(contenido) = std::fs::read_to_string(p) {
                    let lines: Vec<&str> = contenido.lines().collect();
                    let preview_lines = lines.len().min(150);
                    codigo_muestra.push_str(&format!(
                        "\n\n=== {} ===\n{}",
                        p.strip_prefix(&agent_context.project_root)
                            .map(|r| r.display().to_string())
                            .unwrap_or_else(|_| p.display().to_string()),
                        lines[..preview_lines].join("\n")
                    ));
                    muestras += 1;
                    total_lines_loaded += preview_lines;
                }
            }
        }
        ReviewMode::Large => {
            // 80+ files: group by top-level subdir, up to 6 groups × 10 files × 80 lines
            use std::collections::HashMap;
            let mut groups: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();
            for p in &candidates {
                let rel = p.strip_prefix(&agent_context.project_root)
                    .unwrap_or(p.as_path());
                let top_dir = rel.components().next()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .unwrap_or_else(|| "root".to_string());
                groups.entry(top_dir).or_default().push(p.clone());
            }
            let mut group_keys: Vec<String> = groups.keys().cloned().collect();
            group_keys.sort();
            for key in group_keys.iter().take(6) {
                let group_files = &groups[key];
                for p in group_files.iter().take(10) {
                    if let Ok(contenido) = std::fs::read_to_string(p) {
                        let lines: Vec<&str> = contenido.lines().collect();
                        let preview_lines = lines.len().min(80);
                        codigo_muestra.push_str(&format!(
                            "\n\n=== {} ===\n{}",
                            p.strip_prefix(&agent_context.project_root)
                                .map(|r| r.display().to_string())
                                .unwrap_or_else(|_| p.display().to_string()),
                            lines[..preview_lines].join("\n")
                        ));
                        muestras += 1;
                        total_lines_loaded += preview_lines;
                    }
                }
            }
        }
    }

    pb.finish_with_message("Estructura analizada.");

    let mode_label = match review_size_mode(candidates.len()) {
        ReviewMode::Small  => "proyecto pequeño",
        ReviewMode::Medium => "modo centralidad",
        ReviewMode::Large  => "modo multi-grupo",
    };
    let diff_note = if changed_count > 0 {
        format!(" · {} del diff reciente", changed_count)
    } else {
        String::new()
    };
    println!(
        "   📎 Contexto: {} archivo(s) · {} líneas · {}{} ({} en total)",
        muestras, total_lines_loaded, mode_label, diff_note, candidates.len()
    );

    // Aviso si el modelo configurado es local
    let model = &agent_context.config.primary_model;
    let is_local = matches!(model.provider.as_str(), "ollama" | "local" | "lm-studio")
        || model.url.contains("localhost")
        || model.url.contains("127.0.0.1");
    if is_local {
        println!(
            "\n{} Modelo local detectado ({}).",
            "⚠️ ".yellow(),
            model.name.yellow()
        );
        println!(
            "   {} Para análisis profundo (pro review, pro analyze) se recomiendan",
            "ℹ️ ".cyan()
        );
        println!("   modelos de 70B+ o APIs en la nube (Claude / Gemini).");
        println!("   Los modelos pequeños pueden producir sugerencias genéricas.\n");
    }

    let pb_agent = ui::crear_progreso("Ejecutando Auditoría de Arquitectura (ReviewerAgent)...");

    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.register(std::sync::Arc::new(crate::agents::fix_suggester::FixSuggesterAgent::new()));
    orchestrator.register(std::sync::Arc::new(crate::agents::reviewer::ReviewerAgent::new()));

    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Realiza una auditoría técnica de alto nivel del proyecto.".to_string(),
        task_type: TaskType::Analyze,
        file_path: None,
        context: Some({
            let arch_ctx = agent_context.build_architectural_context();
            format!(
                "ESTADÍSTICAS:\nArchivos escaneados: {}\n\nESTRUCTURA DE DIRECTORIOS:\n{}\n\nSTACK TECNOLÓGICO (Dependencias):\n{}{}\n\nMUESTRA DE CÓDIGO FUENTE (para análisis concreto):\n{}",
                file_count, project_tree, deps_list, arch_ctx, codigo_muestra
            )
        }),
    };

    let result = rt.block_on(orchestrator.execute_task("ReviewerAgent", &task, &agent_context));

    pb_agent.finish_and_clear();

    match result {
        Ok(res) => {
            println!("{}", "🏗️  AUDITORÍA DE ARQUITECTURA COMPLETADA".bold().green());
            let report_only = crate::ai::utils::eliminar_bloques_codigo(&res.output);
            let report_display = report_only
                .trim_start_matches("[... Código guardado en .suggested ...]")
                .trim();
            println!("{}", report_display);

            // Save review record for history/diff
            let suggestions_json: Vec<serde_json::Value> = {
                let json_start = res.output.find("```json")
                    .map(|i| i + 7)
                    .or_else(|| res.output.find('[').map(|i| i));
                let json_end = json_start.and_then(|start| {
                    res.output[start..].find("```").map(|i| i + start)
                }).unwrap_or(res.output.len());
                if let Some(start) = json_start {
                    serde_json::from_str::<Vec<serde_json::Value>>(res.output[start..json_end].trim())
                        .unwrap_or_default()
                } else {
                    vec![]
                }
            };
            let record = ReviewRecord {
                timestamp: chrono::Local::now().format("%Y-%m-%dT%H-%M-%S").to_string(),
                project_root: agent_context.project_root.display().to_string(),
                files_reviewed: muestras,
                suggestions: suggestions_json,
            };
            if let Err(e) = save_review_record(&agent_context.project_root, &record) {
                eprintln!("⚠️  No se pudo guardar el review: {}", e);
            }

            let raw_json = crate::ai::utils::extraer_json_sugerencias(&res.output);
            let json_str = if raw_json.trim_start().starts_with('{') {
                format!("[{}]", raw_json)
            } else {
                raw_json
            };
            match serde_json::from_str::<Vec<ReviewSuggestion>>(&json_str) {
                Ok(mut suggestions) if !suggestions.is_empty() => {
                    while !suggestions.is_empty() {
                        println!("\n💡 {} sugerencias de mejora detectadas.", suggestions.len().to_string().cyan());

                        let mut options: Vec<String> = suggestions.iter()
                            .map(|s| {
                                let line = format!("[{}] {} — {}", s.impact.to_uppercase(), s.title, s.description);
                                if line.chars().count() > 90 {
                                    format!("{}…", line.chars().take(89).collect::<String>())
                                } else {
                                    line
                                }
                            })
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

                                let pb_dev = ui::crear_progreso(&format!("Aplicando mejora: {}...", suggestion.title));

                                let file_context = suggestion.files_involved.first().and_then(|f| {
                                    let path = agent_context.project_root.join(f);
                                    std::fs::read_to_string(&path)
                                        .ok()
                                        .map(|content| format!("CONTENIDO ACTUAL DE {}:\n```\n{}\n```", f, content))
                                });

                                let dev_task = Task {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    description: format!(
                                        "IMPLEMENTACIÓN DE MEJORA ARQUITECTÓNICA\n\n\
                                        TÍTULO: {}\n\
                                        DESCRIPCIÓN: {}\n\
                                        ACCIÓN REQUERIDA: {}\n\n\
                                        OBJETIVO: Aplica la mejora al código real adjunto.",
                                        suggestion.title, suggestion.description, suggestion.action_item
                                    ),
                                    task_type: TaskType::Fix,
                                    file_path: suggestion.files_involved.first().map(|f| std::path::PathBuf::from(f)),
                                    context: file_context,
                                };

                                let dev_result = rt.block_on(orchestrator.execute_task("FixSuggesterAgent", &dev_task, &agent_context));
                                pb_dev.finish_and_clear();

                                match dev_result {
                                    Ok(d_res) => {
                                        println!("{}", "\n✨ MEJORA GENERADA".bold().green());

                                        let bloques = crate::ai::utils::extraer_todos_bloques(&d_res.output);

                                        if bloques.is_empty() {
                                            println!("{}", d_res.output);
                                        } else {
                                            println!("\n📂 {} archivo(s) a generar/modificar:", bloques.len().to_string().cyan());
                                            for (path_opt, _) in &bloques {
                                                match path_opt {
                                                    Some(p) => println!("   • {}", p.cyan()),
                                                    None => println!("   • (sin ruta — se mostrará en consola)"),
                                                }
                                            }

                                            let apply = Confirm::new()
                                                .with_prompt("¿Deseas aplicar estos cambios automáticamente?")
                                                .default(true)
                                                .interact()
                                                .unwrap_or(false);

                                            if apply {
                                                let mut saved = 0;
                                                for (path_opt, code) in &bloques {
                                                    match path_opt {
                                                        Some(rel_path) => {
                                                            let target = agent_context.project_root.join(rel_path);

                                                            if target.is_dir() {
                                                                println!("   ⚠️  '{}' es un directorio, omitido.", rel_path.yellow());
                                                                continue;
                                                            }

                                                            if let Some(parent) = target.parent() {
                                                                let _ = std::fs::create_dir_all(parent);
                                                            }

                                                            if target.exists() {
                                                                let original_len = std::fs::metadata(&target)
                                                                    .map(|m| m.len() as usize)
                                                                    .unwrap_or(0);

                                                                if original_len > 0 && code.len() < original_len / 3 {
                                                                    println!(
                                                                        "   ⚠️  '{}': respuesta truncada ({} chars vs {} original), saltando.",
                                                                        rel_path, code.len(), original_len
                                                                    );
                                                                    continue;
                                                                }

                                                                let bak = {
                                                                    let mut p = target.clone();
                                                                    let mut fname = target.file_name().unwrap_or_default().to_os_string();
                                                                    fname.push(".bak");
                                                                    p.set_file_name(fname);
                                                                    p
                                                                };
                                                                if let Err(e) = std::fs::copy(&target, &bak) {
                                                                    println!("   ⚠️  No se pudo crear backup de '{}': {}", rel_path, e);
                                                                    continue;
                                                                }
                                                            }

                                                            match std::fs::write(&target, code) {
                                                                Ok(_) => {
                                                                    println!("   ✅ {}", rel_path.green());
                                                                    saved += 1;
                                                                }
                                                                Err(e) => println!("   ❌ '{}': {}", rel_path, e),
                                                            }
                                                        }
                                                        None => {
                                                            println!("\n{}", "[Código sin ruta — cópialo manualmente:]".yellow());
                                                            println!("{}", code);
                                                        }
                                                    }
                                                }

                                                if saved > 0 {
                                                    let mut s = agent_context.stats.lock().unwrap();
                                                    s.sugerencias_aplicadas += 1;
                                                    s.tiempo_estimado_ahorrado_mins += 30;
                                                    s.guardar(&agent_context.project_root);
                                                    suggestions.remove(idx);
                                                    println!("\n✅ {} archivo(s) guardados.", saved.to_string().green());
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => println!("{} {}", "\n❌ Error al desarrollar la sugerencia:".red(), e),
                                }
                            },
                            _ => break,
                        }
                    }
                    if suggestions.is_empty() {
                        println!("\n✨ {} Todas las sugerencias han sido procesadas o aplicadas.", "Review completado:".green());
                    }
                }
                Ok(_) => {
                    println!("\n{} El análisis no generó sugerencias de mejora concretas.", "ℹ️".cyan());
                }
                Err(_) => {
                    println!("\n{} Las sugerencias interactivas no están disponibles (respuesta demasiado extensa).", "ℹ️".cyan());
                }
            }
        }
        Err(e) => {
            println!("{} {}", "❌ Error en Review:", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_size_thresholds() {
        assert_eq!(review_size_mode(5),   ReviewMode::Small);
        assert_eq!(review_size_mode(19),  ReviewMode::Small);
        assert_eq!(review_size_mode(20),  ReviewMode::Medium);
        assert_eq!(review_size_mode(80),  ReviewMode::Medium);
        assert_eq!(review_size_mode(81),  ReviewMode::Large);
        assert_eq!(review_size_mode(200), ReviewMode::Large);
    }

    #[test]
    fn test_review_record_save_and_load() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let record = ReviewRecord {
            timestamp: "2026-02-23T14-32-00".to_string(),
            project_root: root.display().to_string(),
            files_reviewed: 5,
            suggestions: vec![
                serde_json::json!({"title": "Test suggestion", "impact": "High"}),
            ],
        };

        save_review_record(root, &record).unwrap();

        let loaded = load_review_records(root);
        assert_eq!(loaded.len(), 1, "should load 1 saved record");
        assert_eq!(loaded[0].files_reviewed, 5);
        assert_eq!(loaded[0].suggestions.len(), 1);
    }

    #[test]
    fn test_review_diff_categorizes_correctly() {
        let old = vec![
            serde_json::json!({"title": "Old and resolved"}),
            serde_json::json!({"title": "Persistent issue"}),
        ];
        let new_v = vec![
            serde_json::json!({"title": "Persistent issue"}),
            serde_json::json!({"title": "Brand new issue"}),
        ];

        let (resolved, added, persistent) = diff_reviews(&old, &new_v);
        assert_eq!(resolved.len(), 1, "Old and resolved should be resolved");
        assert_eq!(added.len(), 1, "Brand new issue should be new");
        assert_eq!(persistent.len(), 1, "Persistent issue should be persistent");
    }
}
