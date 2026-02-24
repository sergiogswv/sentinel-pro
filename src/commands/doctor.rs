use colored::Colorize;
use std::path::Path;

/// Check if the config file (.sentinelrc.toml) exists and loads correctly
pub fn check_config(project_root: &Path) -> anyhow::Result<crate::config::SentinelConfig> {
    let config_path = project_root.join(".sentinelrc.toml");

    if !config_path.exists() {
        anyhow::bail!(".sentinelrc.toml not found at {}", config_path.display());
    }

    // Try to load the config
    crate::config::SentinelConfig::load(project_root)
        .ok_or_else(|| anyhow::anyhow!("Failed to load .sentinelrc.toml"))
}

/// Check if the ANTHROPIC_API_KEY environment variable is set and non-empty
pub fn check_api_key() -> bool {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
}

/// Check if the SQLite index exists and has content
pub fn check_index(project_root: &Path) -> bool {
    let index_path = project_root.join(".sentinel/index.db");

    if !index_path.exists() {
        return false;
    }

    // Check if the file has content (size > 0 bytes)
    std::fs::metadata(&index_path)
        .ok()
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
}

/// Main handler for the doctor command with colored output
pub fn handle_doctor_command(project_root: &Path) {
    println!("\n{}", "🏥 Sentinel Doctor".bold().cyan());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut issues = 0;

    // Check 1: Config file
    print!("   ");
    match check_config(project_root) {
        Ok(config) => {
            println!("{} Config file", "✅".green());
            println!("      └─ {}", config.project_name.cyan());
        }
        Err(e) => {
            println!("{} Config file", "❌".red());
            println!("      └─ Error: {}", e.to_string().red());
            issues += 1;
        }
    }

    // Check 2: API Key
    print!("   ");
    if check_api_key() {
        println!("{} ANTHROPIC_API_KEY", "✅".green());
    } else {
        println!("{} ANTHROPIC_API_KEY", "❌".red());
        println!("      └─ {}", "Required for AI features (audit, check, analyze, review)".red());
        issues += 1;
    }

    // Check 3: Index database
    print!("   ");
    if check_index(project_root) {
        println!("{} SQLite index", "✅".green());
    } else {
        println!("{} SQLite index", "⚠️ ".yellow());
        println!("      └─ {}", "Run 'sentinel index --rebuild' to create it".yellow());
    }

    // Check 4: Languages detected
    print!("   ");
    let languages = crate::commands::init::detect_project_extensions(project_root);
    if !languages.is_empty() {
        println!("{} Languages detected", "✅".green());
        println!("      └─ {}", languages.join(", ").cyan());
    } else {
        println!("{} Languages detected", "⚠️ ".yellow());
        println!("      └─ {}", "No supported files found in project".yellow());
    }

    // Summary
    println!();
    if issues == 0 {
        println!("{}", "✅ All critical checks passed!".green().bold());
    } else if issues == 1 {
        println!("{}", format!("⚠️  {} critical issue found", issues).yellow().bold());
    } else {
        println!("{}", format!("⚠️  {} critical issues found", issues).yellow().bold());
    }

    println!();

    // Exit with error code if issues > 0
    if issues > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_config_returns_ok_when_config_exists() {
        let tmp = TempDir::new().unwrap();
        let project_root = tmp.path();

        // Create a valid config file
        let config_path = project_root.join(".sentinelrc.toml");
        let config_content = r#"
version = "5.0.0"
project_name = "test-project"
framework = "nestjs"
manager = "npm"
test_command = "npm run test"
architecture_rules = []
file_extensions = ["ts"]
code_language = "typescript"
parent_patterns = []
test_patterns = []
ignore_patterns = []
use_cache = true

[primary_model]
name = "claude-3-5-sonnet-20241022"
url = "https://api.anthropic.com"
api_key = ""
provider = "anthropic"

[rule_config]
complexity_threshold = 10
function_length_threshold = 50
dead_code_enabled = true
unused_imports_enabled = true
"#;
        std::fs::write(&config_path, config_content).unwrap();

        // Should succeed
        let result = check_config(project_root);
        assert!(result.is_ok(), "check_config should succeed when config exists");
    }

    #[test]
    fn test_check_config_returns_err_when_missing() {
        let tmp = TempDir::new().unwrap();
        let project_root = tmp.path();

        // No config file created
        let result = check_config(project_root);
        assert!(result.is_err(), "check_config should fail when config missing");
    }

    #[test]
    fn test_check_index_returns_false_when_missing() {
        let tmp = TempDir::new().unwrap();
        let project_root = tmp.path();

        // No index directory or file
        assert!(
            !check_index(project_root),
            "check_index should return false when index missing"
        );
    }

    #[test]
    fn test_check_api_key_returns_bool() {
        // This test verifies that check_api_key function exists and returns a bool
        // The function checks if ANTHROPIC_API_KEY env var is set and non-empty
        let result = check_api_key();

        // Verify the function returns a boolean (not panicking)
        // The result will be true or false depending on whether ANTHROPIC_API_KEY is set
        let _: bool = result;
    }
}
