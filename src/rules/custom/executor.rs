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
                    // Skip disabled rules
                    if !p.enabled {
                        continue;
                    }
                    if self.matches_file_pattern(file_path, &p.file_patterns) {
                        violations.extend(self.check_pattern_rule(p, content));
                    }
                }
                Rule::Ast(a) => {
                    // Skip disabled rules
                    if !a.enabled {
                        continue;
                    }
                    // AST rules will be implemented in Phase 1 Task 3
                    // For now, skip
                }
            }
        }

        violations
    }

    fn matches_file_pattern(&self, file_path: &Path, patterns: &[String]) -> bool {
        if patterns.is_empty() {
            return true; // No patterns = match all files
        }

        let file_path_str = file_path.to_string_lossy().replace('\\', "/");

        // Separate include and exclude patterns
        let mut include_patterns = Vec::new();
        let mut exclude_patterns = Vec::new();

        for pattern in patterns {
            if pattern.starts_with('!') {
                exclude_patterns.push(&pattern[1..]);
            } else {
                include_patterns.push(pattern.as_str());
            }
        }

        // Check if file matches any include pattern (if any)
        let matches_include = if include_patterns.is_empty() {
            true // No includes = match all (unless excluded)
        } else {
            include_patterns
                .iter()
                .any(|p| self.glob_matches(&file_path_str, p))
        };

        if !matches_include {
            return false;
        }

        // Check if file matches any exclude pattern
        for exclude_pattern in exclude_patterns {
            if self.glob_matches(&file_path_str, exclude_pattern) {
                return false; // Excluded
            }
        }

        true
    }

    /// Simple glob pattern matching implementation
    /// Supports * (any characters in a single directory) and ** (any directories)
    fn glob_matches(&self, path: &str, pattern: &str) -> bool {
        self.match_glob(path, pattern)
    }

    /// Recursive glob matching
    fn match_glob(&self, path: &str, pattern: &str) -> bool {
        // Handle ** pattern
        if pattern == "**" {
            return true;
        }

        if pattern.starts_with("**/") {
            // **/ matches zero or more directories
            let rest = &pattern[3..];
            // Try matching from the current position
            if self.match_glob(path, rest) {
                return true;
            }
            // Try matching from one directory deeper
            if let Some(pos) = path.find('/') {
                return self.match_glob(&path[pos + 1..], pattern);
            }
            return false;
        }

        if pattern.contains("/**/") {
            // Pattern has **/ in the middle
            if let Some(pos) = pattern.find("/**/") {
                let prefix = &pattern[..pos];
                let suffix = &pattern[pos + 4..];

                // Match the prefix, then try matching suffix at each point after prefix
                let path_parts: Vec<&str> = path.split('/').collect();
                let prefix_parts: Vec<&str> = prefix.split('/').collect();

                if path_parts.len() < prefix_parts.len() {
                    return false;
                }

                // Check if prefix matches
                for i in 0..prefix_parts.len() {
                    if !self.segment_matches(path_parts[i], prefix_parts[i]) {
                        return false;
                    }
                }

                // Now try matching suffix from each position after prefix
                for i in prefix_parts.len()..=path_parts.len() {
                    let remaining = path_parts[i..].join("/");
                    if self.match_glob(&remaining, suffix) {
                        return true;
                    }
                }
                return false;
            }
        }

        // No **, use simple path matching
        self.match_path_simple(path, pattern)
    }

/// Simple path matching without ** patterns
    fn match_path_simple(&self, path: &str, pattern: &str) -> bool {
        let path_parts: Vec<&str> = path.split('/').collect();
        let pattern_parts: Vec<&str> = pattern.split('/').collect();

        if path_parts.len() != pattern_parts.len() {
            return false;
        }

        for (path_part, pattern_part) in path_parts.iter().zip(pattern_parts.iter()) {
            if !self.segment_matches(path_part, pattern_part) {
                return false;
            }
        }

        true
    }

    /// Match a single path segment against a pattern segment (single filename/dirname)
    fn segment_matches(&self, segment: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true; // * matches any single segment
        }

        let mut s = segment.chars().peekable();
        let mut p = pattern.chars().peekable();

        while let Some(&pc) = p.peek() {
            match pc {
                '*' => {
                    p.next();
                    if p.peek().is_none() {
                        // * at end, matches rest of segment
                        return true;
                    }
                    // * in middle, match greedily
                    let rest_pattern: String = p.clone().collect();
                    while s.peek().is_some() {
                        if self.segment_matches(&s.clone().collect::<String>(), &rest_pattern) {
                            return true;
                        }
                        s.next();
                    }
                    // Try matching what remains
                    return self.segment_matches(&s.clone().collect::<String>(), &rest_pattern);
                }
                '?' => {
                    p.next();
                    if s.next().is_none() {
                        return false;
                    }
                }
                _ => {
                    p.next();
                    if let Some(sc) = s.next() {
                        if sc != pc {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        // Both exhausted
        s.peek().is_none()
    }

    fn check_pattern_rule(&self, rule: &PatternRule, content: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        match Regex::new(&rule.pattern) {
            Ok(regex) => {
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
            Err(e) => {
                // Report regex compilation error as a violation
                violations.push(RuleViolation {
                    rule_name: rule.name.clone(),
                    severity: crate::rules::custom::RuleSeverity::Error,
                    message: format!("Invalid regex pattern: {}", e),
                    line: 1,
                    column: 1,
                });
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

    #[test]
    fn test_glob_pattern_matching() {
        let rule = Rule::Pattern(PatternRule {
            name: "Test Rule".to_string(),
            pattern: "test".to_string(),
            file_patterns: vec!["src/**/*.ts".to_string()],
            severity: RuleSeverity::Warning,
            message: "Found test".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);

        // Should match files in src/**/*.ts
        let violations = executor.check_file("test", Path::new("src/utils/helper.ts"));
        assert_eq!(violations.len(), 1);

        // Should not match files outside src/**/*.ts
        let violations = executor.check_file("test", Path::new("tests/helper.ts"));
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_exclusion_pattern() {
        let rule = Rule::Pattern(PatternRule {
            name: "Test Rule".to_string(),
            pattern: "test".to_string(),
            file_patterns: vec!["src/**/*.ts".to_string(), "!src/excluded/**/*.ts".to_string()],
            severity: RuleSeverity::Warning,
            message: "Found test".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);

        // Should match files in src/**/*.ts
        let violations = executor.check_file("test", Path::new("src/index.ts"));
        assert_eq!(violations.len(), 1);

        // Should not match excluded files
        let violations = executor.check_file("test", Path::new("src/excluded/file.ts"));
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_disabled_rule_skipped() {
        let rule = Rule::Pattern(PatternRule {
            name: "Disabled Rule".to_string(),
            pattern: "test".to_string(),
            file_patterns: vec!["src/**/*.ts".to_string()],
            severity: RuleSeverity::Warning,
            message: "Found test".to_string(),
            enabled: false,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);
        let violations = executor.check_file("test", Path::new("src/index.ts"));

        // Should be skipped because enabled=false
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_invalid_regex_reported_as_error() {
        let rule = Rule::Pattern(PatternRule {
            name: "Invalid Regex Rule".to_string(),
            pattern: "[invalid(".to_string(), // Invalid regex
            file_patterns: vec!["src/**/*.ts".to_string()],
            severity: RuleSeverity::Warning,
            message: "Found match".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);
        let violations = executor.check_file("test", Path::new("src/index.ts"));

        // Should report the invalid regex as an error
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, RuleSeverity::Error);
        assert!(violations[0].message.contains("Invalid regex pattern"));
    }

    #[test]
    fn test_empty_patterns_match_all_files() {
        let rule = Rule::Pattern(PatternRule {
            name: "Match All".to_string(),
            pattern: "match".to_string(),
            file_patterns: vec![], // Empty patterns
            severity: RuleSeverity::Info,
            message: "Found match".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);

        // Should match any file when patterns is empty
        let violations = executor.check_file("match", Path::new("any/file.txt"));
        assert_eq!(violations.len(), 1);

        let violations = executor.check_file("match", Path::new("src/index.ts"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_multiple_violations_on_single_line() {
        let rule = Rule::Pattern(PatternRule {
            name: "Multiple Matches".to_string(),
            pattern: "bad".to_string(),
            file_patterns: vec!["**/*.rs".to_string()],
            severity: RuleSeverity::Error,
            message: "Found bad".to_string(),
            enabled: true,
        });

        let rules = [rule];
        let executor = CustomRulesExecutor::new(&rules);
        let content = "bad code bad practice";
        let violations = executor.check_file(content, Path::new("src/main.rs"));

        // Should find both occurrences on the same line
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[1].line, 1);
    }
}
