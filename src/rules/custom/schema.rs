//! Data structures for custom rules (YAML + JSON compatible)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CustomRule {
    #[serde(rename = "pattern")]
    Pattern(PatternRule),
    #[serde(rename = "ast")]
    Ast(AstRule),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    pub name: String,
    pub pattern: String, // regex
    #[serde(default)]
    pub file_patterns: Vec<String>, // glob patterns
    pub severity: RuleSeverity,
    pub message: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstRule {
    pub name: String,
    pub language: String, // "typescript", "java", "rust", etc.
    pub query: String,    // Tree-sitter query
    pub severity: RuleSeverity,
    pub message: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

// Re-export for convenience
pub use CustomRule as RuleType;

/// Validates rule structure against JSON schema
pub fn validate_rule(rule: &CustomRule) -> Result<(), String> {
    match rule {
        CustomRule::Pattern(r) => {
            if r.name.is_empty() {
                return Err("Pattern rule 'name' cannot be empty".to_string());
            }
            if r.pattern.is_empty() {
                return Err("Pattern rule 'pattern' cannot be empty".to_string());
            }
            // Validate regex
            if let Err(e) = regex::Regex::new(&r.pattern) {
                return Err(format!("Invalid regex pattern: {}", e));
            }
            Ok(())
        }
        CustomRule::Ast(r) => {
            if r.name.is_empty() {
                return Err("AST rule 'name' cannot be empty".to_string());
            }
            if r.language.is_empty() {
                return Err("AST rule 'language' cannot be empty".to_string());
            }
            if r.query.is_empty() {
                return Err("AST rule 'query' cannot be empty".to_string());
            }
            Ok(())
        }
    }
}
