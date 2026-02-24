use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};
use crate::rules::{RuleViolation, RuleLevel};

/// Cuenta ocurrencias de `word` como palabra completa (word-boundary) en `text`.
/// Fallback a 2 (no reportar) si el patrón no compila.
fn count_word_occurrences(text: &str, word: &str) -> usize {
    let pattern = format!(r"\b{}\b", regex::escape(word));
    match regex::Regex::new(&pattern) {
        Ok(re) => re.find_iter(text).count(),
        Err(_) => 2, // safe: no reportar falso positivo
    }
}

/// Primera línea (1-based) donde aparece `word` en `source_code`, o None.
fn find_line_of(source_code: &str, word: &str) -> Option<usize> {
    source_code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(word))
        .map(|(i, _)| i + 1)
}

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
                
                let count = count_word_occurrences(source_code, name);
                if count == 1 {
                    violations.push(RuleViolation {
                        rule_name: "DEAD_CODE".to_string(),
                        message: format!("La entidad '{}' parece estar declarada pero nunca utilizada.", name),
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
                
                if count_word_occurrences(source_code, name) == 1 {
                    // Skip if used as a decorator: @Name or @Name()
                    let decorator_pattern = format!(r"@{}\b", regex::escape(name));
                    let used_as_decorator = regex::Regex::new(&decorator_pattern)
                        .map(|re| re.is_match(source_code))
                        .unwrap_or(false);
                    if used_as_decorator {
                        continue;
                    }
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no se está utilizando en este archivo.", name),
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

                // NOTE: 5 is the absolute generation floor. The configured complexity_threshold
                // can suppress violations above this floor but cannot lower it below 5.
                if complexity > 5 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("La función tiene una complejidad ciclomática de {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                        value: Some(complexity),
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
                // NOTE: 10 is the absolute generation floor for function length.
                if line_count > 10 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!(
                            "Función de {} líneas (máximo recomendado: 50). Considera dividirla en funciones más pequeñas.",
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
                let node = capture.node;
                let name = node.utf8_text(source_code.as_bytes()).unwrap_or("");
                let has_snake = name.contains('_') && !name.chars().next().unwrap_or(' ').is_uppercase();
                let node_line = Some(node.start_position().row + 1);

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
                            line: node_line,
                            symbol: None,
                            value: None,
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
                            line: node_line,
                            symbol: None,
                            value: None,
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

    #[test]
    fn test_unused_import_not_flagged_when_used() {
        let lang = ts_lang();
        let analyzer = UnusedImportsAnalyzer::new();
        // Injectable aparece en el import Y como decorador → count > 1 → no debe reportarse
        let code = "import { Injectable } from '@nestjs/common';

@Injectable()
export class AppService {}";
        let violations = analyzer.analyze(&lang, code);
        let flagged = violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT");
        assert!(!flagged, "Injectable está en uso — no debe ser reportado como UNUSED_IMPORT");
    }

    #[test]
    fn test_dead_code_no_false_positive_on_substring() {
        let lang = ts_lang();
        let analyzer = DeadCodeAnalyzer::new();
        // `user` está declarado pero NUNCA usado como palabra completa.
        // `username` contiene "user" como substring pero NO es un uso de `user`.
        let code = "const user = 1;\nconsole.log(username);";
        let violations = analyzer.analyze(&lang, code);
        let flagged = violations.iter().any(|v| v.rule_name == "DEAD_CODE");
        // Con word-boundary: "username" NO cuenta como uso de "user" → DEAD_CODE debe reportarse
        assert!(flagged, "user está declarado y nunca usado como palabra completa — debe reportarse DEAD_CODE");
    }

    #[test]
    fn test_dead_code_symbol_field_populated() {
        let lang = ts_lang();
        let analyzer = DeadCodeAnalyzer::new();
        let code = "function unusedFn() { return 42; }";
        let violations = analyzer.analyze(&lang, code);
        let v = violations.iter().find(|v| v.rule_name == "DEAD_CODE")
            .expect("Should detect DEAD_CODE");
        assert_eq!(v.symbol, Some("unusedFn".to_string()),
            "symbol field must be populated for DEAD_CODE violations");
    }

    #[test]
    fn test_unused_import_not_flagged_when_decorator() {
        let lang = ts_lang();
        let analyzer = UnusedImportsAnalyzer::new();
        // ApiProperty only appears as @ApiProperty() decorator — must NOT be flagged
        let code = "import { ApiProperty } from '@nestjs/swagger';\n\nexport class UserDto {\n  @ApiProperty()\n  name: string;\n}";
        let violations = analyzer.analyze(&lang, code);
        let flagged = violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT");
        assert!(!flagged, "@ApiProperty() is used as decorator — must not be UNUSED_IMPORT");
    }

    #[test]
    fn test_function_too_long_has_line_number() {
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        // Función de 55 líneas empezando en línea 1 → violation.line debe ser Some(1)
        let long_fn = format!(
            "function longFn() {{\n{}\n}}",
            "  const x = 1;\n".repeat(54)
        );
        let violations = analyzer.analyze(&lang, &long_fn);
        let v = violations.iter().find(|v| v.rule_name == "FUNCTION_TOO_LONG")
            .expect("Debería detectar FUNCTION_TOO_LONG");
        assert_eq!(v.line, Some(1), "La violación debe incluir el número de línea donde empieza la función");
    }

    #[test]
    fn test_complexity_generates_above_floor_5() {
        // 5 if statements = complexity 6. With old floor > 10 this was never generated.
        // After fix the floor is > 5, so complexity 6 must be reported.
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        let code = "function f(x) {
                      if (x>0) { return 1; }
                      if (x>1) { return 2; }
                      if (x>2) { return 3; }
                      if (x>3) { return 4; }
                      if (x>4) { return 5; }
                      return 0;
                    }";
        let violations = analyzer.analyze(&lang, code);
        let v = violations.iter().find(|v| v.rule_name == "HIGH_COMPLEXITY");
        assert!(v.is_some(), "complexity 6 (above new floor 5) should be flagged, got: {:?}", violations);
        assert_eq!(v.unwrap().value, Some(6));
    }

    #[test]
    fn test_function_length_generates_above_floor_10() {
        // A 12-line function should be flagged after lowering floor to > 10.
        let lang = ts_lang();
        let analyzer = ComplexityAnalyzer::new();
        let code = format!("function f() {{
{}}}", "  const x = 1;
".repeat(12));
        let violations = analyzer.analyze(&lang, &code);
        let v = violations.iter().find(|v| v.rule_name == "FUNCTION_TOO_LONG");
        assert!(v.is_some(), "12-line function (above new floor 10) should be flagged, got: {:?}", violations);
    }
}
