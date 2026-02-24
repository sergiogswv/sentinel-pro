use crate::rules::{FrameworkDefinition, FrameworkRule, RuleViolation, RuleLevel};
use crate::rules::static_analysis::NamingAnalyzerWithFramework;
use crate::rules::languages;
use std::fs;
use std::path::Path;

pub struct RuleEngine {
    pub framework_def: Option<FrameworkDefinition>,
    pub index_db: Option<std::sync::Arc<crate::index::IndexDb>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            framework_def: None,
            index_db: None,
        }
    }

    pub fn with_index_db(mut self, db: std::sync::Arc<crate::index::IndexDb>) -> Self {
        self.index_db = Some(db);
        self
    }

    pub fn load_from_yaml(&mut self, yaml_path: &Path) -> anyhow::Result<()> {
        let content = fs::read_to_string(yaml_path)?;
        let def: FrameworkDefinition = serde_yaml::from_str(&content)?;
        self.framework_def = Some(def);
        Ok(())
    }

    pub fn validate_file(&self, _file_path: &Path, content: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        // 1. Capa de Análisis Estático (Layer 1 - Automática)
        let ext = _file_path.extension().and_then(|e: &std::ffi::OsStr| e.to_str()).unwrap_or("");
        if let Some((lang, analyzers)) = languages::get_language_and_analyzers(ext) {
            for analyzer in &analyzers {
                violations.extend(analyzer.analyze(&lang, content));
            }

            // NamingAnalyzer: only for TS/JS (framework naming conventions)
            if matches!(ext, "ts" | "tsx" | "js" | "jsx") {
                let framework = self.framework_def.as_ref()
                    .map(|f| f.framework.as_str())
                    .unwrap_or("typescript");
                let naming_violations = NamingAnalyzerWithFramework::new(framework)
                    .analyze(&lang, content);
                violations.extend(naming_violations);
            }
        }

        // --- Análisis de Proyecto Cruzado (SI hay DB disponible) ---
        if let Some(ref db) = self.index_db {
            let rel_path = _file_path.to_string_lossy();
            let call_graph = crate::index::call_graph::CallGraph::new(db);

            // Post-filter: remove DEAD_CODE violations for symbols called from other files
            violations.retain(|v| {
                if v.rule_name != "DEAD_CODE" {
                    return true;
                }
                if let Some(ref sym) = v.symbol {
                    !call_graph.is_called_from_other_file(sym, &rel_path)
                } else {
                    true
                }
            });

            // 1. Dead Code de Proyecto (DEAD_CODE_GLOBAL from call graph)
            if let Ok(dead_symbols) = call_graph.get_dead_code(Some(&rel_path)) {
                for symbol in dead_symbols {
                    violations.push(RuleViolation {
                        rule_name: "DEAD_CODE_GLOBAL".to_string(),
                        message: format!("El símbolo '{}' no tiene llamadas registradas en todo el proyecto.", symbol),
                        level: RuleLevel::Warning,
                        line: None,
                        symbol: None,
                        value: None,
                    });
                }
            }
        }

        // 2. Reglas basadas en Patrones (Legacy/Configurable)
        if let Some(ref def) = self.framework_def {
            for rule in &def.rules {
                if self.check_rule(rule, content) {
                    violations.push(RuleViolation {
                        rule_name: rule.name.clone(),
                        message: rule.description.clone(),
                        level: rule.level.clone(),
                        line: None,
                        symbol: None,
                        value: None,
                    });
                }
            }
        }

        violations
    }

    /// Post-filter violations based on RuleConfig thresholds and enabled flags.
    pub fn filter_by_config(
        mut violations: Vec<crate::rules::RuleViolation>,
        config: &crate::config::RuleConfig,
    ) -> Vec<crate::rules::RuleViolation> {
        violations.retain(|v| match v.rule_name.as_str() {
            "HIGH_COMPLEXITY" => {
                v.value.map(|n| n > config.complexity_threshold).unwrap_or(true)
            }
            "FUNCTION_TOO_LONG" => {
                v.value.map(|n| n > config.function_length_threshold).unwrap_or(true)
            }
            "DEAD_CODE" | "DEAD_CODE_GLOBAL" => config.dead_code_enabled,
            "UNUSED_IMPORT" => config.unused_imports_enabled,
            _ => true,
        });
        violations
    }

    fn check_rule(&self, rule: &FrameworkRule, content: &str) -> bool {
        for forbidden in &rule.forbidden_patterns {
            if content.contains(forbidden) {
                return true;
            }
        }

        for required in &rule.required_imports {
            if !content.contains(required) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuleConfig;
    use crate::rules::{RuleViolation, RuleLevel};

    #[test]
    fn test_filter_by_config_complexity_threshold() {
        let violations = vec![
            RuleViolation {
                rule_name: "HIGH_COMPLEXITY".to_string(),
                message: "complexity 12".to_string(),
                level: RuleLevel::Error,
                line: None,
                symbol: None,
                value: Some(12),
            },
            RuleViolation {
                rule_name: "HIGH_COMPLEXITY".to_string(),
                message: "complexity 7".to_string(),
                level: RuleLevel::Error,
                line: None,
                symbol: None,
                value: Some(7),
            },
        ];
        let config = RuleConfig { complexity_threshold: 15, ..RuleConfig::default() };
        let filtered = RuleEngine::filter_by_config(violations, &config);
        assert!(filtered.is_empty(), "both below threshold 15 should be filtered");
    }

    #[test]
    fn test_filter_by_config_dead_code_disabled() {
        let violations = vec![
            RuleViolation {
                rule_name: "DEAD_CODE".to_string(),
                message: "unused".to_string(),
                level: RuleLevel::Warning,
                line: None,
                symbol: None,
                value: None,
            },
            RuleViolation {
                rule_name: "UNUSED_IMPORT".to_string(),
                message: "unused import".to_string(),
                level: RuleLevel::Warning,
                line: None,
                symbol: None,
                value: None,
            },
        ];
        let config = RuleConfig { dead_code_enabled: false, unused_imports_enabled: false, ..RuleConfig::default() };
        let filtered = RuleEngine::filter_by_config(violations, &config);
        assert!(filtered.is_empty(), "both rules disabled, should filter all");
    }
}
