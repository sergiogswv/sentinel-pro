use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};
use crate::rules::{RuleViolation, RuleLevel};
use crate::rules::static_analysis::StaticAnalyzer;

static ALL_CAPS_RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"^[A-Z][A-Z0-9_]+$").unwrap());

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
                        value: None,
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
                        value: None,
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
                // NOTE: 10 is the hardcoded generation floor. The configured complexity_threshold
                // can suppress violations above this floor but cannot lower it below 10.
                if complexity > 10 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("Función con complejidad ciclomática {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                        value: Some(complexity),
                    });
                }

                // Function length
                let start_line = func_node.range().start_point.row;
                let end_line = func_node.range().end_point.row;
                let line_count = end_line.saturating_sub(start_line);
                // NOTE: 50 is the hardcoded generation floor for function length.
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
                        value: Some(line_count),
                    });
                }
            }
        }
        violations
    }
}

/// Unchecked error: detects `_, _ = call()` or `_, _ := call()` where all LHS are blank.
pub struct GoUncheckedErrorAnalyzer;

impl StaticAnalyzer for GoUncheckedErrorAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        // Helper closure to process captures from a query and collect violations.
        // We run two queries: one for `:=` (short_var_declaration) and one for `=` (assignment_statement).
        let process_query = |query_str: &str, violations: &mut Vec<RuleViolation>| {
            let query = match Query::new(language, query_str) {
                Ok(q) => q,
                Err(_) => return,
            };
            let mut cursor = QueryCursor::new();
            let mut captures = cursor.captures(&query, root, source_code.as_bytes());
            while let Some((m, _)) = captures.next() {
                let lhs_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "lhs");
                let call_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "callee");
                if let (Some(lhs), Some(call)) = (lhs_node, call_node) {
                    let lhs_text = lhs.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                    if lhs_text.split(',').map(|s| s.trim()).all(|s| s == "_") {
                        let callee = call.node.utf8_text(source_code.as_bytes()).unwrap_or("unknown");
                        violations.push(RuleViolation {
                            rule_name: "UNCHECKED_ERROR".to_string(),
                            message: format!("Resultado de error descartado en llamada a {}.", callee),
                            level: RuleLevel::Warning,
                            line: Some(call.node.start_position().row + 1),
                            symbol: None,
                            value: None,
                        });
                    }
                }
            }
        };

        // `:=` short var declaration pattern
        process_query(
            r#"(short_var_declaration left: (expression_list) @lhs right: (expression_list (call_expression function: _ @callee)))"#,
            &mut violations,
        );
        // `=` assignment statement pattern
        process_query(
            r#"(assignment_statement left: (expression_list) @lhs right: (expression_list (call_expression function: _ @callee)))"#,
            &mut violations,
        );
        violations
    }
}

/// Naming convention: detects Go constants in ALL_CAPS format (violates Go naming).
pub struct GoNamingConventionAnalyzer;

impl StaticAnalyzer for GoNamingConventionAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        let query_str = r#"(const_spec name: (identifier) @const_name)"#;
        let query = match Query::new(language, query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                if ALL_CAPS_RE.is_match(name) {
                    violations.push(RuleViolation {
                        rule_name: "NAMING_CONVENTION_GO".to_string(),
                        message: format!("Constante Go en formato ALL_CAPS: '{}'. Usar PascalCase según convención Go.", name),
                        level: RuleLevel::Info,
                        line: Some(capture.node.start_position().row + 1),
                        symbol: Some(name.to_string()),
                        value: None,
                    });
                }
            }
        }
        violations
    }
}

/// Defer in loop: detects `defer` statements inside `for` loops.
pub struct GoDeferInLoopAnalyzer;

impl StaticAnalyzer for GoDeferInLoopAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        if parser.set_language(language).is_err() { return violations; }
        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root = tree.root_node();

        let loop_query_str = r#"(for_statement) @loop"#;
        let loop_query = match Query::new(language, loop_query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        let defer_query_str = r#"(defer_statement) @defer"#;
        let defer_query = match Query::new(language, defer_query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };

        let mut l_cursor = QueryCursor::new();
        let mut loops = l_cursor.captures(&loop_query, root, source_code.as_bytes());

        while let Some((m, _)) = loops.next() {
            for loop_cap in m.captures {
                let loop_node = loop_cap.node;
                let mut d_cursor = QueryCursor::new();
                let mut defers = d_cursor.captures(&defer_query, loop_node, source_code.as_bytes());
                if defers.next().is_some() {
                    violations.push(RuleViolation {
                        rule_name: "DEFER_IN_LOOP".to_string(),
                        message: "defer dentro de un bucle: el recurso no se libera hasta que la función retorna.".to_string(),
                        level: RuleLevel::Warning,
                        line: Some(loop_node.start_position().row + 1),
                        symbol: None,
                        value: None,
                    });
                }
            }
        }

        // Deduplicate: only one violation per unique loop line
        let mut seen_lines = std::collections::HashSet::new();
        violations.retain(|v| {
            if let Some(line) = v.line {
                seen_lines.insert(line)
            } else {
                true
            }
        });

        violations
    }
}

/// Returns the set of static analyzers for Go files.
pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(GoDeadCodeAnalyzer),
        Box::new(GoUnusedImportsAnalyzer),
        Box::new(GoComplexityAnalyzer),
        Box::new(GoUncheckedErrorAnalyzer),
        Box::new(GoNamingConventionAnalyzer),
        Box::new(GoDeferInLoopAnalyzer),
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
        assert_eq!(analyzers.len(), 6, "Go should have 6 analyzers");
    }

    #[test]
    fn test_go_unchecked_error_detects_blank_error() {
        let src = r#"package main

import "os"

func main() {
    _, _ = os.Open("file.txt")
}
"#;
        let lang = go_lang();
        let analyzer = GoUncheckedErrorAnalyzer;
        let violations = analyzer.analyze(&lang, src);
        assert!(
            violations.iter().any(|v| v.rule_name == "UNCHECKED_ERROR"),
            "should detect blanked error, got: {:?}", violations
        );
    }

    #[test]
    fn test_go_naming_convention_detects_all_caps() {
        let src = r#"package main

const MY_CONSTANT = 42
const GoodName = 10
"#;
        let lang = go_lang();
        let analyzer = GoNamingConventionAnalyzer;
        let violations = analyzer.analyze(&lang, src);
        assert!(
            violations.iter().any(|v| v.rule_name == "NAMING_CONVENTION_GO" && v.symbol.as_deref() == Some("MY_CONSTANT")),
            "should detect MY_CONSTANT, got: {:?}", violations
        );
        assert!(
            !violations.iter().any(|v| v.symbol.as_deref() == Some("GoodName")),
            "GoodName (PascalCase) should not be flagged"
        );
    }

    #[test]
    fn test_go_defer_in_loop_detects_issue() {
        let src = r#"package main

import "os"

func leaky() {
    for i := 0; i < 10; i++ {
        f, _ := os.Open("file.txt")
        defer f.Close()
    }
}
"#;
        let lang = go_lang();
        let analyzer = GoDeferInLoopAnalyzer;
        let violations = analyzer.analyze(&lang, src);
        assert!(
            violations.iter().any(|v| v.rule_name == "DEFER_IN_LOOP"),
            "should detect defer inside for loop, got: {:?}", violations
        );
    }

    #[test]
    fn test_go_unchecked_error_only_blanks_trigger() {
        let src = r#"package main

import "os"

func main() {
    f, err := os.Open("file.txt")
    if err != nil { panic(err) }
    defer f.Close()
}
"#;
        let lang = go_lang();
        let analyzer = GoUncheckedErrorAnalyzer;
        let violations = analyzer.analyze(&lang, src);
        assert!(
            !violations.iter().any(|v| v.rule_name == "UNCHECKED_ERROR"),
            "should NOT flag when error is named (not blank), got: {:?}", violations
        );
    }

    #[test]
    fn test_go_registry_returns_none_for_unknown() {
        assert!(super::super::get_language_and_analyzers("rb").is_none());
        assert!(super::super::get_language_and_analyzers("java").is_none());
    }
}
