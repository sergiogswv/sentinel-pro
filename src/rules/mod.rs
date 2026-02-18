pub mod engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameworkRule {
    pub name: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub forbidden_patterns: Vec<String>,
    pub required_imports: Vec<String>,
    pub level: RuleLevel,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    Error,
    Warning,
    Info,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrameworkDefinition {
    pub framework: String,
    pub language: String,
    pub rules: Vec<FrameworkRule>,
    pub architecture_patterns: Vec<ArchitecturePattern>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArchitecturePattern {
    pub name: String,
    pub selector: String, // e.g., "**/*.service.ts"
    pub expected_parent: Option<String>,
    pub expected_layer: String,
}
