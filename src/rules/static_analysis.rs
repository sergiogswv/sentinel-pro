use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};
use crate::rules::{RuleViolation, RuleLevel};

pub trait StaticAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation>;
}

/// Analizador de código muerto (funciones/variables no utilizadas)
pub struct DeadCodeAnalyzer;

impl DeadCodeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl StaticAnalyzer for DeadCodeAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        parser.set_language(language).expect("Error al cargar lenguaje");
        
        let tree = parser.parse(source_code, None).expect("Error al parsear");
        let root_node = tree.root_node();

        let query_str = r#"
            (lexical_declaration (variable_declarator name: (identifier) @var_name))
            (variable_declaration (variable_declarator name: (identifier) @var_name))
            (function_declaration name: (identifier) @func_name)
        "#;

        let query = Query::new(language, query_str).unwrap_or_else(|_| Query::new(language, "(function_declaration) @f").unwrap());
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root_node, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let node = capture.node;
                let name = node.utf8_text(source_code.as_bytes()).unwrap_or("");
                
                if name.is_empty() {
                    continue;
                }
                
                let count = source_code.matches(name).count();
                if count == 1 {
                    violations.push(RuleViolation {
                        rule_name: "DEAD_CODE".to_string(),
                        message: format!("La entidad '{}' parece estar declarada pero nunca utilizada.", name),
                        level: RuleLevel::Warning,
                    });
                }
            }
        }

        violations
    }
}

/// Analizador de importaciones no utilizadas
pub struct UnusedImportsAnalyzer;

impl UnusedImportsAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl StaticAnalyzer for UnusedImportsAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        parser.set_language(language).ok();
        
        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        let query_str = r#"
            (import_specifier name: (identifier) @import_name)
            (import_clause (identifier) @import_default)
        "#;

        let query = Query::new(language, query_str).unwrap_or_else(|_| Query::new(language, "(function_declaration) @f").unwrap());
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root_node, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let node = capture.node;
                let name = node.utf8_text(source_code.as_bytes()).unwrap_or("");
                
                if source_code.matches(name).count() == 1 {
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no se está utilizando en este archivo.", name),
                        level: RuleLevel::Warning,
                    });
                }
            }
        }

        violations
    }
}

/// Analizador de complejidad ciclomática
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl StaticAnalyzer for ComplexityAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        parser.set_language(language).ok();
        
        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        let query_str = r#"
            (if_statement) @branch
            (for_statement) @branch
            (while_statement) @branch
            (catch_clause) @branch
            (binary_expression operator: "&&") @branch
            (binary_expression operator: "||") @branch
            (ternary_expression) @branch
        "#;

        let query = match Query::new(language, query_str) {
            Ok(q) => q,
            Err(_) => {
                // Fallback sin el operador ternario si falla
                let fallback_query = r#"
                    (if_statement) @branch
                    (for_statement) @branch
                    (while_statement) @branch
                    (catch_clause) @branch
                "#;
                Query::new(language, fallback_query).unwrap()
            }
        };
        
        let func_query = r#"
            (function_declaration) @func
            (method_definition) @func
            (arrow_function) @func
            (function_expression) @func
        "#;
        let func_q = match Query::new(language, func_query) {
            Ok(q) => q,
            Err(_) => Query::new(language, "(function_declaration) @func").unwrap()
        };
        let mut f_cursor = QueryCursor::new();
        let mut funcs = f_cursor.captures(&func_q, root_node, source_code.as_bytes());

        while let Some((m, _)) = funcs.next() {
            for capture in m.captures {
                let func_node = capture.node;
                let mut b_cursor = QueryCursor::new();
                let mut branches = b_cursor.captures(&query, func_node, source_code.as_bytes());
                
                let mut complexity = 1;
                while let Some(_) = branches.next() {
                    complexity += 1;
                }

                if complexity > 10 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("La función tiene una complejidad ciclomática de {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                    });
                }
            }
        }

        // Detectar funciones demasiado largas (> 50 líneas)
        let mut f_cursor2 = QueryCursor::new();
        let func_q2 = match Query::new(language, func_query) {
            Ok(q) => q,
            Err(_) => Query::new(language, "(function_declaration) @func").unwrap(),
        };
        let mut funcs2 = f_cursor2.captures(&func_q2, root_node, source_code.as_bytes());
        while let Some((m, _)) = funcs2.next() {
            for capture in m.captures {
                let node = capture.node;
                let start_line = node.range().start_point.row;
                let end_line = node.range().end_point.row;
                let line_count = end_line.saturating_sub(start_line);
                if line_count > 50 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!(
                            "Función de {} líneas (máximo recomendado: 50). Considera dividirla en funciones más pequeñas.",
                            line_count
                        ),
                        level: RuleLevel::Warning,
                    });
                }
            }
        }

        violations
    }
}

/// Analizador de convenciones de nombres (framework-aware)
pub struct NamingAnalyzer;

impl NamingAnalyzer {
    pub fn new() -> Self { Self }
}

pub struct NamingAnalyzerWithFramework {
    /// "nestjs" | "django" | "laravel" | "spring" | "typescript" | "javascript"
    framework: String,
}

impl NamingAnalyzerWithFramework {
    pub fn new(framework: &str) -> Self {
        Self { framework: framework.to_lowercase() }
    }

    fn expects_snake_case(&self) -> bool {
        matches!(self.framework.as_str(), "django" | "python" | "laravel" | "php")
    }

    pub fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let mut parser = Parser::new();
        parser.set_language(language).ok();

        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => return violations,
        };
        let root_node = tree.root_node();

        let query_str = r#"
            (variable_declarator name: (identifier) @var_name)
            (function_declaration name: (identifier) @func_name)
        "#;
        let query = Query::new(language, query_str)
            .unwrap_or_else(|_| Query::new(language, "(function_declaration) @f").unwrap());
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root_node, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                let has_snake = name.contains('_') && !name.chars().next().unwrap_or(' ').is_uppercase();

                if self.expects_snake_case() {
                    // Python/PHP: camelCase ES la violación
                    let has_camel = name.chars().any(|c| c.is_uppercase())
                        && !name.chars().next().unwrap_or(' ').is_uppercase();
                    if has_camel {
                        violations.push(RuleViolation {
                            rule_name: "NAMING_CONVENTION".to_string(),
                            message: format!(
                                "'{}' usa camelCase. Se recomienda snake_case para {}.",
                                name, self.framework
                            ),
                            level: RuleLevel::Info,
                        });
                    }
                } else {
                    // TypeScript/JS: snake_case ES la violación
                    if has_snake {
                        violations.push(RuleViolation {
                            rule_name: "NAMING_CONVENTION".to_string(),
                            message: format!(
                                "'{}' usa snake_case. Se recomienda camelCase para {}.",
                                name, self.framework
                            ),
                            level: RuleLevel::Info,
                        });
                    }
                }
            }
        }
        violations
    }
}

impl StaticAnalyzer for NamingAnalyzer {
    fn analyze(&self, language: &Language, source_code: &str) -> Vec<RuleViolation> {
        // Default: TypeScript/camelCase
        NamingAnalyzerWithFramework::new("typescript").analyze(language, source_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Language;

    fn ts_lang() -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    #[test]
    fn test_complexity_flags_long_function() {
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        // Función de 55 líneas (sobre el límite de 50)
        let long_fn = format!(
            "function longFn() {{\n{}\n}}",
            "  const x = 1;\n".repeat(54)
        );
        let violations = analyzer.analyze(&lang, &long_fn);
        let has_length_violation = violations.iter().any(|v| v.rule_name == "FUNCTION_TOO_LONG");
        assert!(has_length_violation, "Debería detectar función de más de 50 líneas");
    }

    #[test]
    fn test_complexity_ok_for_short_function() {
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        let short_fn = "function shortFn() {\n  return 1;\n}";
        let violations = analyzer.analyze(&lang, short_fn);
        let has_length_violation = violations.iter().any(|v| v.rule_name == "FUNCTION_TOO_LONG");
        assert!(!has_length_violation);
    }

    #[test]
    fn test_dead_code_detects_unused_function() {
        let lang = ts_lang();
        let analyzer = DeadCodeAnalyzer::new();
        let code = "function unusedFn() { return 42; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "Debería detectar unusedFn como dead code");
    }

    #[test]
    fn test_unused_import_detected() {
        let lang = ts_lang();
        let analyzer = UnusedImportsAnalyzer::new();
        let code = "import { Injectable } from '@nestjs/common';\n\nfunction foo() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "Debería detectar Injectable como import no usado");
    }

    #[test]
    fn test_naming_snake_case_ok_in_python_context() {
        let analyzer = NamingAnalyzerWithFramework::new("django");
        let lang = ts_lang(); // usamos TS como proxy para el test
        let code = "function my_function() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(violations.is_empty(), "En Django/Python, snake_case no debería ser violación");
    }

    #[test]
    fn test_naming_snake_case_bad_in_typescript() {
        let analyzer = NamingAnalyzerWithFramework::new("nestjs");
        let lang = ts_lang();
        let code = "function my_function() { return 1; }";
        let violations = analyzer.analyze(&lang, code);
        assert!(!violations.is_empty(), "En NestJS/TS, snake_case sí es violación");
    }
}
