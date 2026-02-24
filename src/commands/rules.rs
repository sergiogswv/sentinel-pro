use crate::config::SentinelConfig;
use colored::Colorize;

pub fn handle_rules_command(project_root: &std::path::Path) {
    let config = SentinelConfig::load(project_root);
    let rule_cfg = config
        .as_ref()
        .map(|c| c.rule_config.clone())
        .unwrap_or_default();

    println!("\n{}", "Reglas activas:".bold());

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
}
