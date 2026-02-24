use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};
use crate::rules::{RuleViolation, RuleLevel};
use crate::rules::static_analysis::StaticAnalyzer;

fn count_word_occurrences(text: &str, word: &str) -> usize {
    let pattern = format!(r"\b{}\b", regex::escape(word));
    match regex::Regex::new(&pattern) {
        Ok(re) => re.find_iter(text).count(),
        Err(_) => 2,
    }
}

fn find_line_of(source_code: &str, word: &str) -> Option<usize> {
    source_code.lines().enumerate()
        .find(|(_, line)| line.contains(word))
        .map(|(i, _)| i + 1)
}

/// Dead code: top-level functions and methods used only once (declaration only).
pub struct GoDeadCodeAnalyzer;

impl StaticAnalyzer for GoDeadCodeAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        let query_str = r#"
            (function_declaration name: (identifier) @func_name)
            (method_declaration name: (field_identifier) @method_name)
        "#;
        let query = match Query::new(language, query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                if name.is_empty() || name == "init" || name == "main" { continue; }
                // Skip exported functions (uppercase) — may be used by external packages
                if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) { continue; }
                if count_word_occurrences(source_code, name) <= 1 {
                    violations.push(RuleViolation {
                        rule_name: "DEAD_CODE".to_string(),
                        message: format!("'{}' se declara pero no parece usarse en este archivo.", name),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, name),
                        symbol: Some(name.to_string()),
                    });
                }
            }
        }
        violations
    }
}

/// Unused imports: import paths whose package alias never appears in source.
pub struct GoUnusedImportsAnalyzer;

impl StaticAnalyzer for GoUnusedImportsAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        let query_str = r#"(import_spec path: (interpreted_string_literal) @import_path)"#;
        let query = match Query::new(language, query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let raw = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                // raw is `"pkg/path"` with quotes; extract last path segment as package name
                let path = raw.trim_matches('"');
                let pkg_name = path.split('/').last().unwrap_or(path);
                // Strip version suffix e.g. "v2"
                let pkg_name = if pkg_name.starts_with('v') && pkg_name[1..].parse::<u32>().is_ok() {
                    path.split('/').nth_back(1).unwrap_or(pkg_name)
                } else {
                    pkg_name
                };
                if pkg_name == "_" || pkg_name == "." { continue; }
                if count_word_occurrences(source_code, pkg_name) <= 1 {
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no parece usarse en este archivo.", path),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, raw),
                        symbol: Some(pkg_name.to_string()),
                    });
                }
            }
        }
        violations
    }
}

/// Complexity + function length analyzer for Go.
pub struct GoComplexityAnalyzer;

impl StaticAnalyzer for GoComplexityAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        let branch_query_str = r#"
            (if_statement) @branch
            (for_statement) @branch
            (expression_switch_statement) @branch
            (type_switch_statement) @branch
            (select_statement) @branch
            (expression_case) @branch
            (type_case) @branch
            (binary_expression operator: "&&") @branch
            (binary_expression operator: "||") @branch
        "#;
        let branch_query = match Query::new(language, branch_query_str) {
            Ok(q) => q,
            Err(_) => {
                match Query::new(language, "(if_statement) @b (for_statement) @b (expression_switch_statement) @b") {
                    Ok(q) => q,
                    Err(_) => return violations,
                }
            }
        };

        let func_query_str = r#"
            (function_declaration) @func
            (method_declaration) @func
        "#;
        let func_query = match Query::new(language, func_query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };

        let mut f_cursor = QueryCursor::new();
        let mut funcs = f_cursor.captures(&func_query, root, source_code.as_bytes());

        while let Some((m, _)) = funcs.next() {
            for capture in m.captures {
                let func_node = capture.node;

                // Complexity
                let mut b_cursor = QueryCursor::new();
                let mut branches = b_cursor.captures(&branch_query, func_node, source_code.as_bytes());
                let mut complexity = 1usize;
                while branches.next().is_some() { complexity += 1; }
                if complexity > 10 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("Función con complejidad ciclomática {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                    });
                }

                // Function length
                let start_line = func_node.range().start_point.row;
                let end_line = func_node.range().end_point.row;
                let line_count = end_line.saturating_sub(start_line);
                if line_count > 50 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!(
                            "Función de {} líneas (máximo recomendado: 50). Considera dividirla.",
                            line_count
                        ),
                        level: RuleLevel::Warning,
                        line: Some(start_line + 1),
                        symbol: None,
                    });
                }
            }
        }
        violations
    }
}

/// Returns the set of static analyzers for Go files.
pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(GoDeadCodeAnalyzer),
        Box::new(GoUnusedImportsAnalyzer),
        Box::new(GoComplexityAnalyzer),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_lang() -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
    }

    #[test]
    fn test_go_dead_code_detects_unused_function() {
        let src = r#"package main

func unusedHelper() string {
    return "hello"
}

func main() {
    println("hi")
}
"#;
        let violations = GoDeadCodeAnalyzer.analyze(&go_lang(), src);
        assert!(
            violations.iter().any(|v| v.rule_name == "DEAD_CODE" && v.symbol.as_deref() == Some("unusedHelper")),
            "should detect unusedHelper as dead code, got: {:?}", violations
        );
    }

    #[test]
    fn test_go_dead_code_ignores_main_and_exported() {
        let src = r#"package main

func main() {}
func ExportedFunc() {}
"#;
        let violations = GoDeadCodeAnalyzer.analyze(&go_lang(), src);
        assert!(violations.is_empty(), "main and exported functions should not be flagged, got: {:?}", violations);
    }

    #[test]
    fn test_go_complexity_flags_complex_function() {
        let src = r#"package main

func complex(x int) int {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    if x > 4 {
                        if x > 5 {
                            if x > 6 {
                                if x > 7 {
                                    if x > 8 {
                                        if x > 9 {
                                            return x
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return 0
}
"#;
        let violations = GoComplexityAnalyzer.analyze(&go_lang(), src);
        assert!(
            violations.iter().any(|v| v.rule_name == "HIGH_COMPLEXITY"),
            "deeply nested function should have HIGH_COMPLEXITY, got: {:?}", violations
        );
    }

    #[test]
    fn test_go_registry_returns_analyzers_for_go_extension() {
        let result = super::super::get_language_and_analyzers("go");
        assert!(result.is_some(), "registry must return analyzers for .go files");
        let (_, analyzers) = result.unwrap();
        assert_eq!(analyzers.len(), 3, "Go should have 3 analyzers");
    }

    #[test]
    fn test_go_registry_returns_none_for_unknown() {
        assert!(super::super::get_language_and_analyzers("rb").is_none());
        assert!(super::super::get_language_and_analyzers("java").is_none());
    }
}
