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

/// Dead code: top-level functions and classes used only once (declaration only).
pub struct PythonDeadCodeAnalyzer;

impl StaticAnalyzer for PythonDeadCodeAnalyzer {
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
            (function_definition name: (identifier) @func_name)
            (class_definition name: (identifier) @class_name)
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
                if name.is_empty() || name == "main" { continue; }
                // Skip dunder methods
                if name.starts_with("__") && name.ends_with("__") { continue; }
                if count_word_occurrences(source_code, name) <= 1 {
                    violations.push(RuleViolation {
                        rule_name: "DEAD_CODE".to_string(),
                        message: format!("'{}' se declara pero no parece usarse en este archivo.", name),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, name),
                        symbol: Some(name.to_string()),
                        value: None,
                    });
                }
            }
        }
        violations
    }
}

/// Unused imports: import names that never appear again in the source.
pub struct PythonUnusedImportsAnalyzer;

impl StaticAnalyzer for PythonUnusedImportsAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        // Query 1: `import X` style — capture module name X
        let q1_str = r#"(import_statement name: (dotted_name (identifier) @module))"#;
        // Query 2: `from X import Y [as Z]` style — capture imported name Y (or alias Z)
        let q2_str = r#"(import_from_statement names: (import_list [(identifier) @symbol (aliased_import alias: (identifier) @symbol)]))"#;

        let q1 = match Query::new(language, q1_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        // Fallback: if from-import query fails, skip from-imports entirely (no false positives)
        let q2 = match Query::new(language, q2_str) {
            Ok(q) => Some(q),
            Err(_) => None, // skip from-imports
        };

        // Check `import X` imports
        let mut cursor1 = QueryCursor::new();
        let mut captures1 = cursor1.captures(&q1, root, source_code.as_bytes());
        while let Some((m, _)) = captures1.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                if name.is_empty() { continue; }
                if count_word_occurrences(source_code, name) <= 1 {
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no parece usarse en este archivo.", name),
                        level: RuleLevel::Warning,
                        line: find_line_of(source_code, name),
                        symbol: Some(name.to_string()),
                        value: None,
                    });
                }
            }
        }

        // Check `from X import Y` imports — only if query compiled successfully
        if let Some(q2) = q2 {
            let mut cursor2 = QueryCursor::new();
            let mut captures2 = cursor2.captures(&q2, root, source_code.as_bytes());
            while let Some((m, _)) = captures2.next() {
                for capture in m.captures {
                    let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                    if name.is_empty() { continue; }
                    if count_word_occurrences(source_code, name) <= 1 {
                        violations.push(RuleViolation {
                            rule_name: "UNUSED_IMPORT".to_string(),
                            message: format!("El import '{}' no parece usarse en este archivo.", name),
                            level: RuleLevel::Warning,
                            line: find_line_of(source_code, name),
                            symbol: Some(name.to_string()),
                            value: None,
                        });
                    }
                }
            }
        }

        violations
    }
}

/// Complexity + function length analyzer for Python.
pub struct PythonComplexityAnalyzer;

impl StaticAnalyzer for PythonComplexityAnalyzer {
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
            (elif_clause) @branch
            (for_statement) @branch
            (while_statement) @branch
            (except_clause) @branch
            (with_statement) @branch
            (conditional_expression) @branch
            (boolean_operator) @branch
        "#;
        let branch_query = match Query::new(language, branch_query_str) {
            Ok(q) => q,
            Err(_) => {
                match Query::new(language, "(if_statement) @b (for_statement) @b (while_statement) @b") {
                    Ok(q) => q,
                    Err(_) => return violations,
                }
            }
        };

        let func_query_str = r#"(function_definition) @func"#;
        let func_query = match Query::new(language, func_query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };

        let mut f_cursor = QueryCursor::new();
        let mut funcs = f_cursor.captures(&func_query, root, source_code.as_bytes());

        while let Some((m, _)) = funcs.next() {
            for capture in m.captures {
                let func_node = capture.node;
                let mut b_cursor = QueryCursor::new();
                let mut branches = b_cursor.captures(&branch_query, func_node, source_code.as_bytes());
                let mut complexity = 1usize;
                while branches.next().is_some() { complexity += 1; }
                // NOTE: 5 is the absolute generation floor. The configured complexity_threshold
                // can suppress violations above this floor but cannot lower it below 5.
                if complexity > 5 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("Función con complejidad ciclomática {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                        value: Some(complexity),
                    });
                }
                let line_count = func_node.range().end_point.row.saturating_sub(func_node.range().start_point.row);
                // NOTE: 10 is the absolute generation floor for function length.
                if line_count > 10 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!("Función de {} líneas (máximo recomendado: 50). Considera dividirla.", line_count),
                        level: RuleLevel::Warning,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                        value: Some(line_count),
                    });
                }
            }
        }
        violations
    }
}

/// Returns the set of static analyzers for Python files.
pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(PythonDeadCodeAnalyzer),
        Box::new(PythonUnusedImportsAnalyzer),
        Box::new(PythonComplexityAnalyzer),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn py_lang() -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    #[test]
    fn test_python_dead_code_detects_unused_function() {
        let src = r#"
def unused_helper():
    return "hello"

def main():
    print("hi")
"#;
        let violations = PythonDeadCodeAnalyzer.analyze(&py_lang(), src);
        assert!(
            violations.iter().any(|v| v.rule_name == "DEAD_CODE" && v.symbol.as_deref() == Some("unused_helper")),
            "should detect unused_helper, got: {:?}", violations
        );
    }

    #[test]
    fn test_python_dead_code_ignores_dunder_methods() {
        let src = r#"
class Foo:
    def __init__(self):
        pass
    def __str__(self):
        return "foo"
"#;
        let violations = PythonDeadCodeAnalyzer.analyze(&py_lang(), src);
        assert!(
            !violations.iter().any(|v| v.symbol.as_deref() == Some("__init__") || v.symbol.as_deref() == Some("__str__")),
            "dunder methods should not be flagged, got: {:?}", violations
        );
    }

    #[test]
    fn test_python_unused_import_detected() {
        let src = r#"
import os
import sys

def main():
    print(sys.argv)
"#;
        let violations = PythonUnusedImportsAnalyzer.analyze(&py_lang(), src);
        assert!(
            violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT" && v.symbol.as_deref() == Some("os")),
            "should detect os as unused, got: {:?}", violations
        );
        assert!(
            !violations.iter().any(|v| v.symbol.as_deref() == Some("sys")),
            "sys is used and should not be flagged"
        );
    }

    #[test]
    fn test_python_complexity_detects_deeply_nested() {
        let src = r#"
def complex_func(x):
    if x > 0:
        if x > 1:
            if x > 2:
                if x > 3:
                    if x > 4:
                        if x > 5:
                            if x > 6:
                                if x > 7:
                                    if x > 8:
                                        if x > 9:
                                            return x
    return 0
"#;
        let violations = PythonComplexityAnalyzer.analyze(&py_lang(), src);
        assert!(
            violations.iter().any(|v| v.rule_name == "HIGH_COMPLEXITY"),
            "deeply nested should be HIGH_COMPLEXITY, got: {:?}", violations
        );
    }

    #[test]
    fn test_python_registry_returns_three_analyzers() {
        let result = super::super::get_language_and_analyzers("py");
        assert!(result.is_some(), "registry must return analyzers for .py files");
        let (_, analyzers) = result.unwrap();
        assert_eq!(analyzers.len(), 3, "Python should have 3 analyzers");
    }

    #[test]
    fn test_python_from_import_checks_symbol_not_module() {
        let src = r#"
from os import path

def main():
    print(path.join("/tmp", "foo"))
"#;
        let lang = py_lang();
        let violations = PythonUnusedImportsAnalyzer.analyze(&lang, src);
        // `path` IS used, so no UNUSED_IMPORT should fire
        assert!(
            !violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT"),
            "from os import path where path is used should not be flagged, got: {:?}", violations
        );
    }
}
