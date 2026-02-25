//! Load custom rules from .sentinel/custom-rules/ (YAML + JSON)

use super::schema::CustomRule;
use std::path::{Path, PathBuf};

pub struct CustomRulesLoader {
    rules_dir: PathBuf,
}

impl CustomRulesLoader {
    pub fn new(project_path: &Path) -> Self {
        let rules_dir = project_path.join(".sentinel/custom-rules");
        Self { rules_dir }
    }

    /// Load all rules from .sentinel/custom-rules/
    pub fn load_all(&self) -> Result<Vec<CustomRule>, String> {
        if !self.rules_dir.exists() {
            // Return empty list if directory doesn't exist (not an error)
            return Ok(Vec::new());
        }

        let mut rules = Vec::new();

        // Read all .yaml and .json files
        for entry in std::fs::read_dir(&self.rules_dir)
            .map_err(|e| format!("Failed to read rules directory: {}", e))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str());
            match ext {
                Some("yaml") | Some("yml") => {
                    let rule = self.load_yaml(&path)?;
                    rules.push(rule);
                }
                Some("json") => {
                    let rule = self.load_json(&path)?;
                    rules.push(rule);
                }
                _ => {} // Skip other file types
            }
        }

        Ok(rules)
    }

    fn load_yaml(&self, path: &Path) -> Result<CustomRule, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Invalid YAML in {}: {}", path.display(), e))
    }

    fn load_json(&self, path: &Path) -> Result<CustomRule, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_yaml_pattern_rule() {
        let yaml_content = r#"
name: "Test Rule"
type: "pattern"
pattern: "console\\.log"
file_patterns: ["src/**/*.ts"]
severity: "error"
message: "No console.log"
"#;
        let rule: CustomRule =
            serde_yaml::from_str(yaml_content).expect("Should parse valid YAML");
        match rule {
            CustomRule::Pattern(r) => {
                assert_eq!(r.name, "Test Rule");
                assert_eq!(r.pattern, "console\\.log");
            }
            _ => panic!("Expected pattern rule"),
        }
    }

    #[test]
    fn test_load_json_ast_rule() {
        let json_content = r#"
{
  "type": "ast",
  "name": "Test AST",
  "language": "typescript",
  "query": "(function_declaration)",
  "severity": "warning",
  "message": "Found function"
}
"#;
        let rule: CustomRule =
            serde_json::from_str(json_content).expect("Should parse valid JSON");
        match rule {
            CustomRule::Ast(r) => {
                assert_eq!(r.name, "Test AST");
                assert_eq!(r.language, "typescript");
            }
            _ => panic!("Expected AST rule"),
        }
    }
}
