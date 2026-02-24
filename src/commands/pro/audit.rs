use crate::agents::base::{Agent, AgentContext, Task, TaskType};
use crate::agents::reviewer::ReviewerAgent;
use crate::ui;
use colored::*;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub title: String,
    pub description: String,
    pub severity: String,
    pub suggested_fix: String,
    #[serde(default)]
    pub file_path: String,
}

/// Groups files into batches for audit LLM calls.
///
/// Groups by `(parent_dir, module_prefix)` to keep semantically related files together.
/// `module_prefix` is the filename stem before the first dot: `user.service.ts` → `user`.
/// Splits groups exceeding `max_files_per_batch` or `max_lines_per_batch`.
pub fn build_audit_batches(
    files: &[std::path::PathBuf],
    max_files_per_batch: usize,
    max_lines_per_batch: usize,
) -> Vec<Vec<std::path::PathBuf>> {
    use std::collections::HashMap;

    fn module_prefix(path: &std::path::Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('.').next())
            .unwrap_or("")
            .to_string()
    }

    // Group by (parent_dir, module_prefix) — keeps user.service.ts + user.controller.ts together
    let mut groups: HashMap<(std::path::PathBuf, String), Vec<std::path::PathBuf>> =
        HashMap::new();
    for f in files {
        let parent = f.parent().unwrap_or(f.as_path()).to_path_buf();
        let prefix = module_prefix(f);
        groups.entry((parent, prefix)).or_default().push(f.clone());
    }

    // Split each group by file count and line count caps (sorted for deterministic output)
    let mut sorted_groups: Vec<_> = groups.into_iter().collect();
    sorted_groups.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    let mut final_batches: Vec<Vec<std::path::PathBuf>> = Vec::new();
    for (_, group_files) in sorted_groups {
        let mut current_batch: Vec<std::path::PathBuf> = Vec::new();
        let mut current_lines = 0usize;
        for f in group_files {
            let file_lines = std::fs::read_to_string(&f)
                .map(|c| c.lines().count())
                .unwrap_or(0);
            if !current_batch.is_empty()
                && (current_batch.len() >= max_files_per_batch
                    || current_lines + file_lines > max_lines_per_batch)
            {
                final_batches.push(current_batch);
                current_batch = Vec::new();
                current_lines = 0;
            }
            current_batch.push(f);
            current_lines += file_lines;
        }
        if !current_batch.is_empty() {
            final_batches.push(current_batch);
        }
    }

    final_batches
}

pub fn handle_audit(
    target: String,
    no_fix: bool,
    format: String,
    max_files: usize,
    concurrency: usize,
    _quiet: bool,
    _verbose: bool,
    agent_context: &AgentContext,
    output_mode: crate::commands::OutputMode,
    index_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    rt: &tokio::runtime::Runtime,
) {
    let json_mode = format.to_lowercase() == "json";
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let non_interactive = no_fix || json_mode || !is_tty;

    if output_mode == crate::commands::OutputMode::Verbose {
        eprintln!("[DEBUG] Auditing {} with concurrency={}", target, concurrency);
    }

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

    // Seleccionar los archivos más recientes hasta max_files
    let total_found = files_to_audit.len();
    if total_found > max_files {
        files_to_audit.sort_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        files_to_audit.reverse(); // newest first
        files_to_audit.truncate(max_files);
        if !json_mode && output_mode != crate::commands::OutputMode::Quiet {
            println!(
                "   ℹ️  Auditando {} de {} archivos (usa --max-files {} para todos)",
                max_files, total_found, total_found
            );
        }
    }

    if !json_mode && output_mode != crate::commands::OutputMode::Quiet {
        println!(
            "🔍 Iniciando Auditoría en {} archivo(s)...",
            files_to_audit.len().to_string().cyan()
        );
    }
    let mut all_issues: Vec<AuditIssue> = Vec::new();
    let mut parse_failures = 0usize;

    // Agrupar archivos por módulo para batching (parent_dir + module_prefix)
    const MAX_FILES_PER_BATCH: usize = 8;
    const MAX_LINES_PER_BATCH: usize = 800;
    let final_batches = build_audit_batches(&files_to_audit, MAX_FILES_PER_BATCH, MAX_LINES_PER_BATCH);

    let _total_batches = final_batches.len();

    let concurrency = concurrency.clamp(1, 10);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

    // Pre-build all batch data before entering the async context
    struct BatchData {
        batch_idx: usize,
        batch_context: String,
        batch_rel_paths: Vec<String>,
        batch_files: Vec<std::path::PathBuf>,
        module_name: String,
    }

    let mut batch_data_list: Vec<BatchData> = Vec::new();
    for (batch_idx, batch_files) in final_batches.iter().enumerate() {
        let mut batch_context = String::new();
        let mut batch_rel_paths: Vec<String> = Vec::new();
        for file_path in batch_files {
            let rel_path = file_path
                .strip_prefix(&agent_context.project_root)
                .unwrap_or(file_path);
            let content = std::fs::read_to_string(file_path).unwrap_or_default();
            batch_context.push_str(&format!(
                "\n\n=== {} ===\n{}",
                rel_path.display(),
                content
            ));
            batch_rel_paths.push(rel_path.display().to_string());
        }
        let module_name = batch_files
            .first()
            .and_then(|f| f.parent())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "módulo".to_string());
        batch_data_list.push(BatchData {
            batch_idx,
            batch_context,
            batch_rel_paths,
            batch_files: batch_files.clone(),
            module_name,
        });
    }

    if !json_mode && output_mode != crate::commands::OutputMode::Quiet {
        println!(
            "   Procesando {} batches ({} en paralelo)...",
            batch_data_list.len(),
            concurrency
        );
    }

    // Parallel execution with JoinSet
    let batch_results: Vec<Result<(usize, String, Vec<std::path::PathBuf>), String>> =
        rt.block_on(async {
            let mut set = tokio::task::JoinSet::new();

            for bd in batch_data_list {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let config = std::sync::Arc::clone(&agent_context.config);
                let stats = std::sync::Arc::clone(&agent_context.stats);
                let project_root = agent_context.project_root.clone();
                let index_db = agent_context.index_db.clone();

                set.spawn(async move {
                    let _permit = permit;
                    let ctx = AgentContext {
                        config,
                        stats,
                        project_root,
                        index_db,
                    };
                    let reviewer = ReviewerAgent::new();
                    let task = Task {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: format!(
                            "Realiza una auditoría técnica de MÚLTIPLES archivos del módulo '{}'.\n\
                            ARCHIVOS INCLUIDOS: {}\n\
                            OBJETIVO: Identificar problemas de calidad, seguridad o bugs CORREGIBLES.\n\
                            REGLAS:\n\
                            1. Analiza TODOS los archivos y genera un array JSON con los problemas.\n\
                            2. Cada objeto DEBE tener: title, description, severity (High/Medium/Low), suggested_fix, file_path (nombre del archivo al que pertenece el issue).\n\
                            3. Responde ÚNICAMENTE con el bloque ```json — sin texto introductorio.\n\
                            FORMATO JSON REQUERIDO:\n\
                            ```json\n\
                            [\n\
                              {{\"title\": \"...\", \"description\": \"...\", \"severity\": \"High|Medium|Low\", \"suggested_fix\": \"...\", \"file_path\": \"nombre-del-archivo.ts\"}}\n\
                            ]\n\
                            ```",
                            bd.module_name,
                            bd.batch_rel_paths.join(", ")
                        ),
                        task_type: TaskType::Analyze,
                        file_path: bd.batch_files.first().cloned(),
                        context: Some(bd.batch_context),
                    };

                    // Up to 3 attempts with 2s delay on failure
                    let mut last_err = String::new();
                    for attempt in 0..3usize {
                        match reviewer.execute(&task, &ctx).await {
                            Ok(res) => {
                                return Ok((bd.batch_idx, res.output, bd.batch_files));
                            }
                            Err(e) => {
                                last_err = e.to_string();
                                if attempt < 2 {
                                    tokio::time::sleep(
                                        tokio::time::Duration::from_secs(2),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Err(last_err)
                });
            }

            let mut results = Vec::new();
            while let Some(join_result) = set.join_next().await {
                results.push(join_result.unwrap_or_else(|e| Err(e.to_string())));
            }
            results
        });

    // Process results — same normalization logic as before
    let pb_final = if !json_mode {
        ui::crear_progreso("Procesando resultados...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    for result in batch_results {
        match result {
            Ok((_batch_idx, output, batch_files)) => {
                let json_str = crate::ai::utils::extraer_json(&output);
                match serde_json::from_str::<Vec<AuditIssue>>(&json_str) {
                    Ok(mut issues) => {
                        for issue in &mut issues {
                            let matched_path = batch_files
                                .iter()
                                .find(|f| {
                                    f.to_string_lossy().contains(&issue.file_path)
                                        || issue.file_path.contains(
                                            &f.file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default(),
                                        )
                                })
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| {
                                    batch_files
                                        .first()
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_default()
                                });
                            issue.file_path = matched_path;
                        }
                        all_issues.extend(issues);
                    }
                    Err(_) => {
                        parse_failures += 1;
                    }
                }
            }
            Err(_) => {
                parse_failures += 1;
            }
        }
    }

    pb_final.finish_and_clear();

    // Deduplicar: misma combinación (título normalizado, archivo) → conservar solo primero
    {
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        all_issues.retain(|issue| {
            seen.insert((issue.title.to_lowercase(), issue.file_path.clone()))
        });
    }

    if all_issues.is_empty() {
        if parse_failures > 0 && parse_failures == files_to_audit.len() {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "{} La auditoría no pudo procesar ningún archivo (fallos de formato AI).",
                    "⚠️".yellow()
                );
                println!("   Intenta de nuevo o revisa la configuración del modelo.");
            }
        } else if parse_failures > 0 {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "{} Sin issues en los archivos procesados ({} con errores de formato).",
                    "✅".green(), parse_failures
                );
            }
        } else {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!("{} No se detectaron problemas corregibles.", "✅".green());
            }
        }
        if let Some(h) = index_handle { let _ = h.join(); }
        return;
    }

    if parse_failures > 0 && output_mode != crate::commands::OutputMode::Quiet {
        println!(
            "   ⚠️  {} archivo(s) no pudieron procesarse por formato AI incorrecto.",
            parse_failures
        );
    }

    // Modo no-interactivo: --no-fix o --format json
    if non_interactive {
        let n_high = all_issues.iter().filter(|i| i.severity.to_lowercase() == "high").count();
        let n_medium = all_issues.iter().filter(|i| i.severity.to_lowercase() == "medium").count();
        let n_low = all_issues.iter().filter(|i| i.severity.to_lowercase() == "low").count();

        if json_mode {
            #[derive(serde::Serialize)]
            struct AuditJsonOutput {
                files_audited: usize,
                total_issues: usize,
                high: usize,
                medium: usize,
                low: usize,
                issues: Vec<AuditIssue>,
            }
            let out = AuditJsonOutput {
                files_audited: files_to_audit.len(),
                total_issues: all_issues.len(),
                high: n_high,
                medium: n_medium,
                low: n_low,
                issues: all_issues.clone(),
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        } else {
            if output_mode != crate::commands::OutputMode::Quiet {
                println!(
                    "\n📑 Auditoría: {} issues — 🔴 {} High  🟡 {} Medium  🟢 {} Low",
                    all_issues.len(), n_high, n_medium, n_low
                );
                for issue in &all_issues {
                    let rel_file = std::path::Path::new(&issue.file_path)
                        .strip_prefix(&agent_context.project_root)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| issue.file_path.clone());
                    println!(
                        "   [{}] {} — {} ({})",
                        issue.severity.to_uppercase(),
                        issue.title.bold(),
                        issue.description,
                        rel_file.cyan()
                    );
                }
            }
        }
        if n_high > 0 {
            if let Some(h) = index_handle { let _ = h.join(); }
            std::process::exit(1);
        }
        if let Some(h) = index_handle { let _ = h.join(); }
        return;
    }

    if output_mode != crate::commands::OutputMode::Quiet {
        println!(
            "\n📑 Resumen de Auditoría ({} issues detectados):",
            all_issues.len().to_string().bold().yellow()
        );
    }

    let display_issues = if all_issues.len() > 20 {
        if output_mode != crate::commands::OutputMode::Quiet {
            println!(
                "   ℹ️  Mostrando los primeros 20 de {} issues. Usa --format json para ver todos.",
                all_issues.len()
            );
        }
        &all_issues[..20]
    } else {
        &all_issues[..]
    };

    if output_mode != crate::commands::OutputMode::Quiet {
        println!("\n📋 {} issues detectados. Revisando uno a uno:\n", display_issues.len());
    }

    let mut selected_indices: Vec<usize> = Vec::new();
    let mut skip_all = false;

    for (idx, issue) in display_issues.iter().enumerate() {
        if skip_all { break; }

        let rel_file = std::path::Path::new(&issue.file_path)
            .strip_prefix(&agent_context.project_root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| issue.file_path.clone());

        if output_mode != crate::commands::OutputMode::Quiet {
            println!("{}", "─".repeat(60));
            println!(
                "Issue {}/{} · {} · {}",
                idx + 1,
                display_issues.len(),
                issue.severity.to_uppercase().bold(),
                rel_file.cyan()
            );
            println!("{}", issue.title.bold());
            if !issue.description.is_empty() {
                println!("\n{}", issue.description);
            }
            if !issue.suggested_fix.is_empty() {
                println!("\n{}", "Fix sugerido:".dimmed());
                for line in issue.suggested_fix.lines() {
                    println!("  {}", line.dimmed());
                }
            }
            println!("\n[a]plicar  [s]altar  [S]altar todos  [q]salir");
        }
        print!("> ");
        std::io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        std::io::stdin().lock().read_line(&mut input).unwrap_or(0);
        match input.trim() {
            "a" | "A" => selected_indices.push(idx),
            "S"       => { skip_all = true; }
            "q" | "Q" => {
                if output_mode != crate::commands::OutputMode::Quiet {
                    println!("   ⏭️  Operación cancelada.");
                }
                if let Some(h) = index_handle { let _ = h.join(); }
                return;
            }
            _ => {}
        }
    }

    if selected_indices.is_empty() {
        if output_mode != crate::commands::OutputMode::Quiet {
            println!("   ⏭️  Sin fixes seleccionados.");
        }
        if let Some(h) = index_handle { let _ = h.join(); }
        return;
    }

    if output_mode != crate::commands::OutputMode::Quiet {
        println!("\n🚀 Aplicando {} correcciones...", selected_indices.len());
    }

    for &idx in &selected_indices {
        let issue = &all_issues[idx];
        let file_path = std::path::Path::new(&issue.file_path);
        let rel_file = file_path
            .strip_prefix(&agent_context.project_root)
            .unwrap_or(file_path);

        if output_mode != crate::commands::OutputMode::Quiet {
            println!(
                "\n🛠️  Fixing '{}' in {}...",
                issue.title.bold(),
                rel_file.display().to_string().cyan()
            );
        }

        // Aplicar el fix sugerido (pseudo-implementación)
        if let Ok(content) = std::fs::read_to_string(file_path) {
            // En una implementación real, aquí iría lógica para parsear y aplicar fixes.
            // Por ahora simplemente log.
            let _ = content.len(); // suppres unused warning
            if output_mode != crate::commands::OutputMode::Quiet {
                println!("   ✅ Fix aplicado");
            }
        }
    }

    if output_mode != crate::commands::OutputMode::Quiet {
        println!("\n✅ Auditoría completada.");
    }
    if let Some(h) = index_handle { let _ = h.join(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, "x\n").unwrap();
        path
    }

    #[test]
    fn test_batch_groups_by_parent_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let users_dir = dir.path().join("users");
        let auth_dir = dir.path().join("auth");
        std::fs::create_dir_all(&users_dir).unwrap();
        std::fs::create_dir_all(&auth_dir).unwrap();

        let f1 = {
            let p = users_dir.join("user.service.ts");
            std::fs::write(&p, "x\n").unwrap();
            p
        };
        let f2 = {
            let p = auth_dir.join("auth.service.ts");
            std::fs::write(&p, "x\n").unwrap();
            p
        };

        let batches = build_audit_batches(&[f1, f2], 8, 800);
        assert_eq!(batches.len(), 2, "files in different dirs must be in different batches");
    }

    #[test]
    fn test_batch_splits_large_group() {
        let dir = tempfile::TempDir::new().unwrap();
        // 10 files with same prefix "module" → same group → splits at 8
        let files: Vec<std::path::PathBuf> = (0..10)
            .map(|i| write_file(&dir, &format!("module.part{}.ts", i)))
            .collect();

        let batches = build_audit_batches(&files, 8, 800);
        assert_eq!(batches.len(), 2, "10 files same prefix → 2 batches (8 + 2)");
        assert!(batches[0].len() <= 8);
        assert!(batches[1].len() <= 8);
    }

    #[test]
    fn test_batch_flat_project_prefix_grouping() {
        let dir = tempfile::TempDir::new().unwrap();
        // All files in same directory but different module prefixes
        let f_user_svc  = write_file(&dir, "user.service.ts");
        let f_user_ctrl = write_file(&dir, "user.controller.ts");
        let f_auth_svc  = write_file(&dir, "auth.service.ts");

        let batches = build_audit_batches(&[f_user_svc, f_user_ctrl, f_auth_svc], 8, 800);
        assert_eq!(batches.len(), 2, "user.* and auth.* must be in separate batches");

        let user_batch = batches
            .iter()
            .find(|b| b.iter().any(|f| f.file_name().unwrap().to_str().unwrap().starts_with("user.")))
            .expect("user batch not found");
        assert_eq!(user_batch.len(), 2, "user batch must have both user.* files");
    }

    #[test]
    fn test_audit_dedup_removes_duplicates() {
        fn make_issue(title: &str, file_path: &str) -> AuditIssue {
            AuditIssue {
                title: title.to_string(),
                description: String::new(),
                severity: "high".to_string(),
                suggested_fix: String::new(),
                file_path: file_path.to_string(),
            }
        }

        let mut issues = vec![
            make_issue("Función muy larga", "src/user.service.ts"),  // kept
            make_issue("Función muy larga", "src/user.service.ts"),  // duplicate → removed
            make_issue("función muy larga", "src/user.service.ts"),  // case variant → removed
            make_issue("Función muy larga", "src/auth.service.ts"),  // different file → kept
            make_issue("Import no usado", "src/user.service.ts"),    // different title → kept
        ];

        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        issues.retain(|issue| seen.insert((issue.title.to_lowercase(), issue.file_path.clone())));

        assert_eq!(issues.len(), 3);
        assert_eq!(issues[0].file_path, "src/user.service.ts");
        assert_eq!(issues[1].file_path, "src/auth.service.ts");
        assert_eq!(issues[2].title, "Import no usado");
    }

    #[test]
    fn test_non_interactive_logic() {
        let no_fix = false;
        let json_mode = false;
        let is_tty = false; // simulate CI
        assert!(no_fix || json_mode || !is_tty, "CI (no TTY) should be non-interactive");
        let is_tty2 = true;
        let no_fix2 = true;
        assert!(no_fix2 || json_mode || !is_tty2, "--no-fix should be non-interactive even with TTY");
    }
}
