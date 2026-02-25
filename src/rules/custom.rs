//! Custom rules system supporting pattern-based and AST-based rules

pub mod schema;
pub mod loader;
pub mod executor;

pub use schema::{CustomRule, RuleSeverity};
pub use loader::CustomRulesLoader;
pub use executor::CustomRulesExecutor;

/// Load and execute custom rules from .sentinel/custom-rules/
pub fn load_custom_rules(project_path: &std::path::Path) -> Result<Vec<CustomRule>, String> {
    let loader = CustomRulesLoader::new(project_path);
    loader.load_all()
}

/// Execute custom rules against a file
pub fn execute_custom_rules(
    rules: &[CustomRule],
    file_content: &str,
    file_path: &std::path::Path,
) -> Vec<RuleViolation> {
    let executor = CustomRulesExecutor::new(rules);
    executor.check_file(file_content, file_path)
}

#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule_name: String,
    pub severity: RuleSeverity,
    pub message: String,
    pub line: usize,
    pub column: usize,
}
