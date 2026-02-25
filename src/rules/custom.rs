//! Custom rules system supporting pattern-based and AST-based rules

pub mod schema;
pub mod loader;
pub mod executor;

pub use schema::{CustomRule, RuleSeverity};
pub use loader::CustomRulesLoader;
pub use executor::CustomRulesExecutor;

/// Load all custom rules from the .sentinel/custom-rules/ directory.
///
/// Loads both YAML and JSON rule files from the custom rules directory.
/// If the directory doesn't exist, returns an empty vector.
///
/// # Arguments
///
/// * `project_path` - The root path of the project where .sentinel/custom-rules/ is located
///
/// # Returns
///
/// A Result containing a vector of loaded CustomRule variants, or an error message if loading fails.
pub fn load_custom_rules(project_path: &std::path::Path) -> Result<Vec<CustomRule>, String> {
    let loader = CustomRulesLoader::new(project_path);
    loader.load_all()
}

/// Execute custom rules against a file's content.
///
/// Checks the given file content against all provided custom rules, respecting the enabled
/// field of each rule. Returns violations found in the file.
///
/// # Arguments
///
/// * `rules` - Slice of custom rules to execute
/// * `file_content` - The content of the file to check
/// * `file_path` - The path to the file (used for pattern matching)
///
/// # Returns
///
/// A vector of RuleViolation instances found in the file.
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
