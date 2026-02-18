use crate::rules::{FrameworkDefinition, FrameworkRule};
use std::fs;
use std::path::Path;

pub struct RuleEngine {
    pub framework_def: Option<FrameworkDefinition>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            framework_def: None,
        }
    }

    pub fn load_from_yaml(&mut self, yaml_path: &Path) -> anyhow::Result<()> {
        let content = fs::read_to_string(yaml_path)?;
        let def: FrameworkDefinition = serde_yaml::from_str(&content)?;
        self.framework_def = Some(def);
        Ok(())
    }

    pub fn validate_file(&self, _file_path: &Path, content: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        if let Some(ref def) = self.framework_def {
            for rule in &def.rules {
                if self.check_rule(rule, content) {
                    violations.push(RuleViolation {
                        rule_name: rule.name.clone(),
                        message: rule.description.clone(),
                        level: rule.level.clone(),
                    });
                }
            }
        }

        violations
    }

    fn check_rule(&self, rule: &FrameworkRule, content: &str) -> bool {
        // En una implementación real, aquí se usarían regex o parsers de AST.
        // Por ahora, implementamos una búsqueda de patrones simple.
        for forbidden in &rule.forbidden_patterns {
            if content.contains(forbidden) {
                return true;
            }
        }

        // Verificación de imports requeridos
        for required in &rule.required_imports {
            if !content.contains(required) {
                return true;
            }
        }

        false
    }
}

pub struct RuleViolation {
    pub rule_name: String,
    pub message: String,
    pub level: crate::rules::RuleLevel,
}
