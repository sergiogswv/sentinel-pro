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

pub fn load_ignore_entries(project_root: &Path) -> Vec<IgnoreEntry> {
    let path = ignore_path(project_root);
    if !path.exists() {
        return vec![];
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str::<IgnoreFile>(&content)
        .map(|f| f.entries)
        .unwrap_or_default()
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
) {
    let project_root = std::env::current_dir().unwrap();
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
    use super::normalize_symbol;

    #[test]
    fn test_normalize_strips_suffix_and_lowercases() {
        assert_eq!(normalize_symbol("AuthService"),    "auth");
        assert_eq!(normalize_symbol("UserController"), "user");
        assert_eq!(normalize_symbol("auth_service"),   "auth");
        assert_eq!(normalize_symbol("userId"),         "userid");
        assert_eq!(normalize_symbol("getUser"),        "getuser");
        assert_eq!(normalize_symbol("SomethingElse"),  "somethingelse");
    }
}
