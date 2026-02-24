use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreEntry {
    pub rule: String,
    pub file: String,
    pub symbol: Option<String>,
    pub added: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IgnoreFile {
    version: u32,
    entries: Vec<IgnoreEntry>,
}

fn ignore_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join(".sentinel/ignore.json")
}

/// Normalize a symbol name for fuzzy ignore matching.
/// Lowercases, removes underscores, strips common framework suffixes.
pub fn normalize_symbol(s: &str) -> String {
    let suffixes = [
        "service", "controller", "repository", "guard",
        "module", "handler", "resolver", "provider",
    ];
    let s = s.to_lowercase().replace('_', "");
    for suffix in suffixes {
        if let Some(base) = s.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    s
}

/// Parses a .sentinelignore file and returns entries
fn parse_sentinelignore_file(path: &Path) -> Vec<IgnoreEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            // Parse: RULE_NAME file/path.ts optional_symbol
            let mut parts = line.split_whitespace();
            let rule = parts.next()?;
            let file = parts.next()?;
            let symbol = parts.next();

            Some(IgnoreEntry {
                rule: rule.to_string(),
                file: file.to_string(),
                symbol: symbol.map(|s| normalize_symbol(s)),
                added: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            })
        })
        .collect()
}

/// Scans project_root recursively for .sentinelignore files and parses entries
pub fn load_directory_ignores(project_root: &Path) -> Vec<IgnoreEntry> {
    let mut entries = Vec::new();
    collect_sentinelignore_files(project_root, &mut entries);
    entries
}

fn collect_sentinelignore_files(dir: &Path, entries: &mut Vec<IgnoreEntry>) {
    const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "vendor", "dist", ".sentinel"];

    // Check for .sentinelignore in this dir
    let ignore_path = dir.join(".sentinelignore");
    if ignore_path.exists() {
        let file_entries = parse_sentinelignore_file(&ignore_path);
        entries.extend(file_entries);
    }

    // Recurse into subdirs
    if let Ok(entries_iter) = std::fs::read_dir(dir) {
        for entry in entries_iter.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if !SKIP_DIRS.contains(&name) {
                    collect_sentinelignore_files(&path, entries);
                }
            }
        }
    }
}

pub fn load_ignore_entries(project_root: &Path) -> Vec<IgnoreEntry> {
    let path = ignore_path(project_root);
    let mut entries = if !path.exists() {
        vec![]
    } else {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<IgnoreFile>(&content)
            .map(|f| f.entries)
            .unwrap_or_default()
    };

    // Merge per-directory .sentinelignore files
    let dir_entries = load_directory_ignores(project_root);
    entries.extend(dir_entries);

    entries
}

fn save_ignore_entries(project_root: &Path, entries: Vec<IgnoreEntry>) {
    let path = ignore_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = IgnoreFile {
        version: 1,
        entries,
    };
    let json = serde_json::to_string_pretty(&file).unwrap_or_default();
    let _ = std::fs::write(&path, json);
}

pub fn handle_ignore_command(
    rule: Option<String>,
    file: Option<String>,
    symbol: Option<String>,
    list: bool,
    clear: Option<String>,
    show_file: bool,
) {
    let project_root = std::env::current_dir().unwrap();

    if show_file {
        let ignore_file_path = project_root.join(".sentinel/ignores.json");
        println!("{}", ignore_file_path.display());
        return;
    }

    let mut entries = load_ignore_entries(&project_root);

    if list {
        if entries.is_empty() {
            println!("No hay ignores activos.");
        } else {
            println!("\n{}", "Ignores activos:".bold());
            for e in &entries {
                let sym = e.symbol.as_deref().unwrap_or("*");
                println!("  {} {} {}", e.rule.cyan(), e.file, sym.dimmed());
            }
        }
        return;
    }

    if let Some(ref clear_file) = clear {
        let before = entries.len();
        entries.retain(|e| &e.file != clear_file);
        let removed = before - entries.len();
        save_ignore_entries(&project_root, entries);
        println!(
            "{} {} ignore(s) eliminados para '{}'.",
            "✅".green(),
            removed,
            clear_file
        );
        return;
    }

    let (Some(rule), Some(file)) = (rule, file) else {
        println!("Uso: sentinel ignore <REGLA> <ARCHIVO> [--symbol <SÍMBOLO>]");
        println!("     sentinel ignore --list");
        println!("     sentinel ignore --clear <ARCHIVO>");
        return;
    };

    // Check for duplicate
    let already = entries.iter().any(|e| {
        e.rule == rule && e.file == file && e.symbol == symbol
    });
    if already {
        println!("{} Ya existe ese ignore.", "ℹ️".cyan());
        return;
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    entries.push(IgnoreEntry {
        rule: rule.clone(),
        file: file.clone(),
        symbol: symbol.as_deref().map(|s| normalize_symbol(s)),
        added: today,
    });
    save_ignore_entries(&project_root, entries);

    let sym_str = symbol
        .as_deref()
        .map(|s| format!(" (símbolo: {})", s))
        .unwrap_or_default();
    println!(
        "{} Ignorando {} en {}{} en próximas ejecuciones.",
        "✅".green(),
        rule.cyan(),
        file,
        sym_str
    );
}

#[cfg(test)]
mod tests {
    use super::{normalize_symbol, load_directory_ignores};

    #[test]
    fn test_normalize_strips_suffix_and_lowercases() {
        assert_eq!(normalize_symbol("AuthService"),    "auth");
        assert_eq!(normalize_symbol("UserController"), "user");
        assert_eq!(normalize_symbol("auth_service"),   "auth");
        assert_eq!(normalize_symbol("userId"),         "userid");
        assert_eq!(normalize_symbol("getUser"),        "getuser");
        assert_eq!(normalize_symbol("SomethingElse"),  "somethingelse");
    }

    #[test]
    fn test_load_directory_ignore_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sub_dir = tmp.path().join("src/services");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(
            sub_dir.join(".sentinelignore"),
            "DEAD_CODE src/services/user.service.ts processLegacy\n\
             UNUSED_IMPORT src/services/auth.service.ts Injectable\n",
        ).unwrap();
        let entries = load_directory_ignores(tmp.path());
        assert_eq!(entries.len(), 2, "should load 2 entries from .sentinelignore");
        assert!(
            entries.iter().any(|e| e.rule == "DEAD_CODE"
                && e.file == "src/services/user.service.ts"
                && e.symbol.as_deref() == Some("processlegacy")),
            "should load DEAD_CODE entry with normalized symbol"
        );
        assert!(
            entries.iter().any(|e| e.rule == "UNUSED_IMPORT"
                && e.file == "src/services/auth.service.ts"
                && e.symbol.as_deref() == Some("injectable")),
            "should load UNUSED_IMPORT entry with normalized symbol"
        );
    }

    #[test]
    fn test_sentinelignore_empty_lines_and_comments_ignored() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".sentinelignore"),
            "# This is a comment\n\
             \n\
             DEAD_CODE src/foo.ts bar\n",
        ).unwrap();
        let entries = load_directory_ignores(tmp.path());
        assert_eq!(entries.len(), 1, "comments and empty lines must be skipped");
        assert_eq!(entries[0].rule, "DEAD_CODE");
        assert_eq!(entries[0].file, "src/foo.ts");
        assert_eq!(entries[0].symbol.as_deref(), Some("bar"));
    }
}
