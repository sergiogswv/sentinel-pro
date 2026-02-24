use crate::commands::ignore::load_ignore_entries;
use crate::rules::RuleLevel;
use colored::*;
use serde::Serialize;
use super::render::SarifIssue;

#[derive(Serialize)]
struct JsonIssue {
    file: String,
    rule: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

struct FileViolation {
    file_path: String,
    rule_name: String,
    symbol: Option<String>,
    message: String,
    level: crate::rules::RuleLevel,
    line: Option<usize>,
    value: Option<usize>,
}

pub fn handle_check(
    target: String,
    format: String,
    _quiet: bool,
    _verbose: bool,
    agent_context: &crate::agents::base::AgentContext,
    output_mode: crate::commands::OutputMode,
    index_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
) {
    let (json_mode, sarif_mode) = super::format_to_mode(&format);

    let path = agent_context.project_root.join(&target);

    if !path.exists() {
        if json_mode {
            println!("{{\"error\":\"El destino '{}' no existe\"}}",  target);
        } else if sarif_mode {
            let empty = super::render_sarif(&[]);
            println!("{}", empty);
        } else {
            println!("{} El destino '{}' no existe en el proyecto.", "❌".red(), target);
        }
        if let Some(h) = index_handle { let _ = h.join(); }
        std::process::exit(2);
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
                    if agent_context.config.file_extensions.contains(&ext.to_string()) {
                        files_to_check.push(p.to_path_buf());
                    }
                }
            }
        }
    }

    if files_to_check.is_empty() {
        if json_mode {
            let index_populated = agent_context
                .index_db
                .as_ref()
                .map(|db| db.is_populated())
                .unwrap_or(false);
            println!(
                "{{\"checked\":0,\"errors\":0,\"warnings\":0,\"infos\":0,\"index_populated\":{},\"issues\":[]}}",
                index_populated
            );
        } else if sarif_mode {
            println!("{}", super::render_sarif(&[]));
        } else {
            println!("{} No se encontraron archivos para revisar en '{}'.", "⚠️".yellow(), target);
        }
        return;
    }

    if !json_mode && !sarif_mode && output_mode != crate::commands::OutputMode::Quiet {
        // TS-first note: shown when no TS/JS files in target
        let has_ts_js = files_to_check.iter().any(|f| {
            matches!(
                f.extension().and_then(|e| e.to_str()),
                Some("ts" | "js" | "tsx" | "jsx")
            )
        });
        if !has_ts_js {
            println!(
                "ℹ️  Análisis estático optimizado para TypeScript/JavaScript."
            );
            println!(
                "   Soporte para Go, Python, Rust, Java y otros lenguajes: próxima versión.\n"
            );
        }
        println!("\n{} Capa 1 — Análisis Estático en {} archivo(s)...",
            "⚡".cyan(), files_to_check.len());
    }

    if output_mode == crate::commands::OutputMode::Verbose && !json_mode && !sarif_mode {
        println!("\n📂 Archivos procesados:");
        for file_path in &files_to_check {
            let rel = file_path
                .strip_prefix(&agent_context.project_root)
                .unwrap_or(file_path);
            println!("   {}", rel.display());
        }
    }

    let mut rule_engine = crate::rules::engine::RuleEngine::new();
    if let Some(ref db) = agent_context.index_db {
        rule_engine = rule_engine.with_index_db(std::sync::Arc::clone(db));
    }
    let rules_path = agent_context.project_root.join(".sentinel/rules.yaml");
    if rules_path.exists() {
        let _ = rule_engine.load_from_yaml(&rules_path);
    }

    let mut violations: Vec<FileViolation> = Vec::new();

    for file_path in &files_to_check {
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let file_violations = rule_engine.validate_file(file_path, &content);

        let rel = file_path
            .strip_prefix(&agent_context.project_root)
            .unwrap_or(file_path);
        let rel_str = rel.display().to_string();

        for v in file_violations {
            violations.push(FileViolation {
                file_path: rel_str.clone(),
                rule_name: v.rule_name,
                symbol: v.symbol,
                message: v.message,
                level: v.level,
                line: v.line,
                value: v.value,
            });
        }
    }

    // Apply ignore list: remove suppressed findings
    let ignore_entries = load_ignore_entries(&agent_context.project_root);
    if !ignore_entries.is_empty() {
        violations.retain(|v| {
            !ignore_entries.iter().any(|e| {
                e.rule == v.rule_name
                    && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                    && e.symbol
                        .as_ref()
                        .map(|s| {
                            let norm_entry = crate::commands::ignore::normalize_symbol(s);
                            let norm_violation = v.symbol.as_deref()
                                .map(|vs| crate::commands::ignore::normalize_symbol(vs))
                                .unwrap_or_default();
                            norm_entry == norm_violation
                        })
                        .unwrap_or(true)
                })
            });
    }

    // Filter by rule config thresholds — mirrors filter semantics: only keep violations
    // that exceed configured thresholds or belong to enabled rule categories.
    let rule_cfg = &agent_context.config.rule_config;
    violations.retain(|v| match v.rule_name.as_str() {
        "HIGH_COMPLEXITY" => v.value.map(|n| n > rule_cfg.complexity_threshold).unwrap_or(true),
        "FUNCTION_TOO_LONG" => v.value.map(|n| n > rule_cfg.function_length_threshold).unwrap_or(true),
        "DEAD_CODE" | "DEAD_CODE_GLOBAL" => rule_cfg.dead_code_enabled,
        "UNUSED_IMPORT" => rule_cfg.unused_imports_enabled,
        _ => true,
    });

    let mut json_issues: Vec<JsonIssue> = Vec::new();
    let mut sarif_issues: Vec<SarifIssue> = Vec::new();
    let mut n_errors = 0usize;
    let mut n_warnings = 0usize;
    let mut n_infos = 0usize;

    // Group by file for display
    let mut current_file = String::new();
    for v in &violations {
        if !json_mode && !sarif_mode && v.file_path != current_file {
            current_file = v.file_path.clone();
            println!("\n📄 {}", current_file.bold().cyan());
        }

        let (sev_str, icon) = match v.level {
            RuleLevel::Error   => { n_errors   += 1; ("error",   "❌ ERROR") }
            RuleLevel::Warning => { n_warnings += 1; ("warning", "⚠️  WARN ") }
            RuleLevel::Info    => { n_infos    += 1; ("info",    "ℹ️  INFO ") }
        };

        if json_mode {
            json_issues.push(JsonIssue {
                file: v.file_path.clone(),
                rule: v.rule_name.clone(),
                severity: sev_str.to_string(),
                message: v.message.clone(),
                line: v.line,
            });
        }
        if sarif_mode {
            let sev = match v.level {
                RuleLevel::Error   => "error",
                RuleLevel::Warning => "warning",
                RuleLevel::Info    => "note",
            };
            sarif_issues.push(SarifIssue {
                file: v.file_path.clone(),
                rule: v.rule_name.clone(),
                severity: sev.to_string(),
                message: v.message.clone(),
                line: v.line,
            });
        }
        if !json_mode && !sarif_mode {
            let line_info = v.line.map(|l| format!(":{}", l)).unwrap_or_default();
            println!("   {} [{}{}]: {}", icon.color(match v.level {
                RuleLevel::Error   => "red",
                RuleLevel::Warning => "yellow",
                RuleLevel::Info    => "blue",
            }), v.rule_name.yellow(), line_info, v.message);
            // Per-violation copy-ready ignore hint
            let rel_file = v.file_path
                .strip_prefix(agent_context.project_root.to_string_lossy().as_ref())
                .unwrap_or(&v.file_path)
                .trim_start_matches('/')
                .to_string();
            let hint_file = if rel_file.is_empty() { v.file_path.as_str() } else { rel_file.as_str() };
            if let Some(ref sym) = v.symbol {
                println!(
                    "      {} sentinel ignore {} {} {}",
                    "👉".dimmed(),
                    v.rule_name.dimmed(),
                    hint_file.dimmed(),
                    sym.dimmed()
                );
            } else {
                println!(
                    "      {} sentinel ignore {} {}",
                    "👉".dimmed(),
                    v.rule_name.dimmed(),
                    hint_file.dimmed()
                );
            }
        }
    }

    if sarif_mode {
        println!("{}", super::render_sarif(&sarif_issues));
    } else if json_mode {
        #[derive(serde::Serialize)]
        struct JsonOutput {
            checked: usize,
            errors: usize,
            warnings: usize,
            infos: usize,
            index_populated: bool,
            issues: Vec<JsonIssue>,
        }
        let index_populated = agent_context
            .index_db
            .as_ref()
            .map(|db| db.is_populated())
            .unwrap_or(false);
        let out = JsonOutput {
            checked: files_to_check.len(),
            errors: n_errors,
            warnings: n_warnings,
            infos: n_infos,
            index_populated,
            issues: json_issues,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else if output_mode != crate::commands::OutputMode::Quiet {
        if n_errors == 0 && n_warnings == 0 && n_infos == 0 {
            println!("\n✅ Sin problemas detectados en {} archivo(s).", files_to_check.len());
        } else {
            println!("\n🚩 {} error(s)  ⚠️  {} warning(s)  ℹ️  {} info(s)",
                n_errors.to_string().red().bold(),
                n_warnings.to_string().yellow(),
                n_infos.to_string().blue());
        }
    }

    // Exit 1 si hay errores → CI falla el build
    if n_errors > 0 {
        if let Some(h) = index_handle { let _ = h.join(); }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::ignore::IgnoreEntry;

    #[test]
    fn test_ignore_filter_removes_matching_entry() {
        // Simulate the filter logic used in the check handler
        struct FakeViolation {
            rule_name: String,
            file_path: String,
            symbol: Option<String>,
        }

        let mut violations = vec![
            FakeViolation {
                rule_name: "DEAD_CODE".into(),
                file_path: "src/user.ts".into(),
                symbol: Some("userId".into()),
            },
            FakeViolation {
                rule_name: "DEAD_CODE".into(),
                file_path: "src/user.ts".into(),
                symbol: Some("getUser".into()),
            },
            FakeViolation {
                rule_name: "UNUSED_IMPORT".into(),
                file_path: "src/auth.ts".into(),
                symbol: None,
            },
        ];

        let entries = vec![IgnoreEntry {
            rule: "DEAD_CODE".into(),
            file: "src/user.ts".into(),
            symbol: Some("userId".into()),
            added: "2026-02-23".into(),
        }];

        violations.retain(|v| {
            !entries.iter().any(|e| {
                e.rule == v.rule_name
                    && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                    && e.symbol
                        .as_ref()
                        .map(|s| v.symbol.as_deref() == Some(s.as_str()))
                        .unwrap_or(true)
            })
        });

        // userId filtered out; getUser and UNUSED_IMPORT kept
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].symbol.as_deref(), Some("getUser"));
        assert_eq!(violations[1].rule_name, "UNUSED_IMPORT");
    }
}
