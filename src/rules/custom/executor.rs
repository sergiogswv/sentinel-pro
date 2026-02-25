//! Execute custom rules against files

use super::schema::PatternRule;
use crate::rules::custom::{CustomRule as Rule, RuleViolation};
use regex::Regex;
use std::path::Path;

pub struct CustomRulesExecutor<'a> {
    rules: &'a [Rule],
}

impl<'a> CustomRulesExecutor<'a> {
    pub fn new(rules: &'a [Rule]) -> Self {
        Self { rules }
    }

    /// Check a file against all custom rules
    pub fn check_file(&self, content: &str, file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        for rule in self.rules {
            match rule {
                Rule::Pattern(p) => {
                    if self.matches_file_pattern(file_path, &p.file_patterns) {
                        violations.extend(self.check_pattern_rule(p, content));
                    }
                }
                Rule::Ast(_) => {
                    // AST rules will be implemented in Phase 1 Task 3
                    // For now, skip
                }
            }
        }

        violations
    }

    fn matches_file_pattern(&self, _file_path: &Path, patterns: &[String]) -> bool {
        if patterns.is_empty() {
            return true; // No patterns = match all files
        }

        for pattern in patterns {
            if pattern.starts_with('!') {
                // Exclude pattern
                if glob::glob_with(&pattern[1..], Default::default())
                    .is_ok()
                {
                    return false;
                }
            } else {
                // Include pattern
                if glob::glob_with(pattern, Default::default())
                    .is_ok()
                {
                    return true;
                }
            }
        }

        false
    }

    fn check_pattern_rule(&self, rule: &PatternRule, content: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        if let Ok(regex) = Regex::new(&rule.pattern) {
            for (line_num, line) in content.lines().enumerate() {
                for cap in regex.captures_iter(line) {
                    if let Some(m) = cap.get(0) {
                        violations.push(RuleViolation {
                            rule_name: rule.name.clone(),
                            severity: rule.severity,
                            message: rule.message.clone(),
                            line: line_num + 1,
                            column: m.start() + 1,
                        });
                    }
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::custom::RuleSeverity;

    #[test]
    fn test_pattern_rule_detection() {
        let rule = Rule::Pattern(PatternRule {
            name: "No console.log".to_string(),
            pattern: "console\\.log".to_string(),
            file_patterns: vec!["src/**/*.ts".to_string()],
            severity: RuleSeverity::Error,
            message: "Remove console.log".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);
        let violations = executor.check_file("console.log('test');", Path::new("src/index.ts"));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
    }
}
