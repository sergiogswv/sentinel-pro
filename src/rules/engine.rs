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
                    });
                }
            }
        }

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
