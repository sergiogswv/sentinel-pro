use std::path::Path;
use std::collections::HashSet;
use colored::*;

/// Scans `root` recursively (up to depth 3) and returns unique file extensions
/// that Sentinel supports. Ignores node_modules, .git, target, vendor, dist, .sentinel.
pub fn detect_project_extensions(root: &Path) -> Vec<String> {
    const SUPPORTED: &[&str] = &["ts", "tsx", "js", "jsx", "go", "py"];
    const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "vendor", "dist", ".sentinel"];

    let mut found: HashSet<String> = HashSet::new();
    walk_extensions(root, 0, 3, SUPPORTED, SKIP_DIRS, &mut found);
    let mut result: Vec<String> = found.into_iter().collect();
    result.sort();
    result
}

fn walk_extensions(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    supported: &[&str],
    skip_dirs: &[&str],
    found: &mut HashSet<String>,
) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !skip_dirs.contains(&name) {
                walk_extensions(&path, depth + 1, max_depth, supported, skip_dirs, found);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if supported.contains(&ext) {
                found.insert(ext.to_string());
            }
        }
    }
}

/// Runs `sentinel init` in `project_root`.
/// Returns Err if config already exists and force == false.
pub fn run_init(project_root: &Path, force: bool) -> anyhow::Result<()> {
    let sentinel_dir = project_root.join(".sentinel");
    let config_path = sentinel_dir.join("config.toml");

    if config_path.exists() && !force {
        anyhow::bail!(
            "Ya existe una configuración en {}. Usa --force para sobrescribir.",
            config_path.display()
        );
    }

    std::fs::create_dir_all(&sentinel_dir)?;

    let extensions = detect_project_extensions(project_root);
    let ext_list = if extensions.is_empty() {
        vec!["ts".to_string(), "js".to_string()]
    } else {
        extensions.clone()
    };

    let ext_toml = ext_list
        .iter()
        .map(|e| format!("\"{}\"", e))
        .collect::<Vec<_>>()
        .join(", ");

    let config_content = format!(
        r#"# Sentinel Pro — Configuración del Proyecto
# Generado por `sentinel init`

[sentinel]
file_extensions = [{ext_list}]
test_patterns = ["**/*.spec.ts", "**/*.test.ts", "**/*.spec.js", "**/*.test.js"]

[rule_config]
complexity_threshold = 10
function_length_threshold = 50
dead_code_enabled = true
unused_imports_enabled = true
"#,
        ext_list = ext_toml
    );

    std::fs::write(&config_path, &config_content)?;
    Ok(())
}

pub fn handle_init_command(project_root: &Path, force: bool) {
    println!("\n{}", "🚀 Sentinel Init".bold().green());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let extensions = detect_project_extensions(project_root);
    if extensions.is_empty() {
        println!("   ℹ️  No se detectaron lenguajes soportados. Usando TypeScript por defecto.");
    } else {
        println!("   🔍 Lenguajes detectados: {}", extensions.join(", ").cyan());
    }

    match run_init(project_root, force) {
        Ok(()) => {
            let config_path = project_root.join(".sentinel/config.toml");
            println!("   ✅ Configuración creada en: {}", config_path.display().to_string().cyan());
            println!("\n   {} Próximos pasos:", "💡".yellow());
            println!("      sentinel pro check src/    # análisis estático");
            println!("      sentinel pro audit src/    # auditoría interactiva");
            println!("      sentinel pro review        # review arquitectónico con IA");
        }
        Err(e) => {
            eprintln!("   ❌ {}", e);
            eprintln!("   💡 Usa --force para sobrescribir la configuración existente.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_languages_from_extensions() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("app.ts"), "").unwrap();
        std::fs::write(tmp.path().join("util.go"), "").unwrap();

        let exts = detect_project_extensions(tmp.path());
        assert!(exts.contains(&"ts".to_string()), "should detect .ts");
        assert!(exts.contains(&"go".to_string()), "should detect .go");
        assert!(!exts.contains(&"py".to_string()), "should not detect .py (none present)");
    }

    #[test]
    fn test_init_creates_config_file() {
        let tmp = TempDir::new().unwrap();
        run_init(tmp.path(), false).unwrap();
        let config_path = tmp.path().join(".sentinel/config.toml");
        assert!(config_path.exists(), "init should create .sentinel/config.toml");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("file_extensions"), "config must contain file_extensions");
        assert!(content.contains("rule_config"), "config must contain rule_config section");
    }

    #[test]
    fn test_init_does_not_overwrite_without_force() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".sentinel")).unwrap();
        let config_path = tmp.path().join(".sentinel/config.toml");
        std::fs::write(&config_path, "existing = true").unwrap();

        let result = run_init(tmp.path(), false);
        assert!(result.is_err(), "init without force should fail if config exists");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing = true", "content must be unchanged");
    }

    #[test]
    fn test_init_with_force_overwrites() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".sentinel")).unwrap();
        let config_path = tmp.path().join(".sentinel/config.toml");
        std::fs::write(&config_path, "old = true").unwrap();

        run_init(tmp.path(), true).unwrap();
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("file_extensions"), "force should overwrite with new config");
    }

    #[test]
    fn test_init_defaults_to_ts_when_no_files() {
        let tmp = TempDir::new().unwrap();
        // No source files at all
        run_init(tmp.path(), false).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".sentinel/config.toml")).unwrap();
        assert!(content.contains("\"ts\""), "should default to ts when no files detected");
    }

    #[test]
    fn test_detect_ignores_node_modules() {
        let tmp = TempDir::new().unwrap();
        let nm = tmp.path().join("node_modules");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join("lib.ts"), "").unwrap();
        // Only ts file is inside node_modules — should NOT be detected
        let exts = detect_project_extensions(tmp.path());
        assert!(!exts.contains(&"ts".to_string()), "node_modules must be skipped");
    }
}
