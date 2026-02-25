use crate::config::SentinelConfig;
use crate::rules::load_custom_rules;
use colored::Colorize;
use std::path::Path;

pub fn handle_rules_command(project_root: &Path) {
    let config = SentinelConfig::load(project_root);
    let rule_cfg = config
        .as_ref()
        .map(|c| c.rule_config.clone())
        .unwrap_or_default();

    println!("\n{}", "Reglas Framework:".bold());

    struct Rule {
        name: &'static str,
        level: &'static str,
        desc: &'static str,
        enabled: bool,
        threshold: Option<String>,
    }

    let rules = vec![
        Rule { name: "DEAD_CODE",            level: "ERROR",   desc: "Funciones/variables no referenciadas",              enabled: rule_cfg.dead_code_enabled,       threshold: None },
        Rule { name: "UNUSED_IMPORT",        level: "WARNING", desc: "Imports sin uso en el archivo",                     enabled: rule_cfg.unused_imports_enabled,   threshold: None },
        Rule { name: "HIGH_COMPLEXITY",      level: "ERROR",   desc: "Complejidad ciclomatica excede umbral",              enabled: true,                             threshold: Some(format!("threshold: {}", rule_cfg.complexity_threshold)) },
        Rule { name: "FUNCTION_TOO_LONG",    level: "WARNING", desc: "Funciones que exceden el limite de lineas",          enabled: true,                             threshold: Some(format!("threshold: {} lineas", rule_cfg.function_length_threshold)) },
        Rule { name: "UNCHECKED_ERROR",      level: "WARNING", desc: "Error de Go sin verificar (blank identifier)",       enabled: true,                             threshold: None },
        Rule { name: "NAMING_CONVENTION_GO", level: "INFO",    desc: "Constante Go en formato ALL_CAPS",                  enabled: true,                             threshold: None },
        Rule { name: "DEFER_IN_LOOP",        level: "WARNING", desc: "defer dentro de bucle for",                         enabled: true,                             threshold: None },
    ];

    for r in &rules {
        let status = if r.enabled { "[ON] " } else { "[OFF]" };
        let threshold_info = r.threshold.as_deref().unwrap_or("");
        println!(
            "  {} {:<28} {:<12} {}  {}",
            status.green(),
            r.name.yellow(),
            format!("[{}]", r.level),
            r.desc,
            threshold_info.dimmed()
        );
    }

    println!();
    if config.is_none() {
        println!("   Info: No se encontro .sentinelrc.toml. Usando valores por defecto.");
    } else {
        println!("   Info: Para cambiar umbrales, edita la seccion [rule_config] en .sentinelrc.toml:");
    }
    println!("   [rule_config]");
    println!("   complexity_threshold = {}", rule_cfg.complexity_threshold);
    println!("   function_length_threshold = {}", rule_cfg.function_length_threshold);
    println!("   dead_code_enabled = {}", rule_cfg.dead_code_enabled);
    println!("   unused_imports_enabled = {}", rule_cfg.unused_imports_enabled);

    // Display custom rules if they exist
    match load_custom_rules(project_root) {
        Ok(custom_rules) => {
            if !custom_rules.is_empty() {
                println!("\n{}", "Reglas Personalizadas:".bold());
                for rule in custom_rules {
                    match rule {
                        crate::rules::CustomRule::Pattern(p) => {
                            let status = if p.enabled { "[ON] " } else { "[OFF]" };
                            let level_str = match p.severity {
                                crate::rules::RuleSeverity::Info => "[info]",
                                crate::rules::RuleSeverity::Warning => "[warning]",
                                crate::rules::RuleSeverity::Error => "[error]",
                            };
                            println!(
                                "  {} {:<28} {:<12} {}",
                                status.green(),
                                p.name.yellow(),
                                level_str,
                                p.message
                            );
                        }
                        crate::rules::CustomRule::Ast(a) => {
                            let status = if a.enabled { "[ON] " } else { "[OFF]" };
                            let level_str = match a.severity {
                                crate::rules::RuleSeverity::Info => "[info]",
                                crate::rules::RuleSeverity::Warning => "[warning]",
                                crate::rules::RuleSeverity::Error => "[error]",
                            };
                            println!(
                                "  {} {:<28} {:<12} {} ({})",
                                status.green(),
                                a.name.yellow(),
                                level_str,
                                a.message,
                                a.language.cyan()
                            );
                        }
                    }
                }
                println!("\n   Info: Las reglas personalizadas se cargan desde .sentinel/custom-rules/");
            }
        }
        Err(e) => {
            eprintln!("   {} Error al cargar reglas personalizadas: {}", "⚠".yellow(), e);
        }
    }
}
