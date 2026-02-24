# Sentinel Power Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add SARIF/GitHub Actions integration, git diff review context, configurable rule thresholds, Python static analysis, monitor daemon mode, and richer Go rules to Sentinel.

**Architecture:** Six independent tasks. Tasks 1-2 are purely additive (new analyzers/new language). Task 3 adds a `RuleConfig` struct + new subcommand. Tasks 4-5 enhance existing handlers. Task 6 adds OS-level daemon management.

**Tech Stack:** Rust, tree-sitter (Go/Python grammars), SARIF 2.1.0 JSON, POSIX signals via nix crate, TOML config, cargo test.

---

## Context — Key files

- `src/rules/languages/go.rs` — 3 existing Go analyzers + `analyzers()` pub fn
- `src/rules/languages/mod.rs` — registry: `get_language_and_analyzers(ext)`
- `src/rules/mod.rs` — `RuleViolation` struct (rule_name, message, level, line, symbol)
- `src/rules/engine.rs` — `RuleEngine::validate_file()`, uses registry
- `src/commands/pro.rs` — check handler (lines ~291-530), review handler (lines ~1612+)
- `src/commands/mod.rs` — `Commands` and `ProCommands` enums
- `src/commands/monitor.rs` — `start_monitor()` fn
- `src/config.rs` — `SentinelConfig` struct (TOML-serializable)
- `src/index/builder.rs` — language map for indexing (`"go" => Some(tree_sitter_go::LANGUAGE.into())`)
- `src/main.rs` — routes `Commands::*` to handlers
- `Cargo.toml` — `tree-sitter-go = "0.25.0"` already present

---

## Task 1: Go Richer Rules

**Files:**
- Modify: `src/rules/languages/go.rs`

Add three new analyzer structs before the existing `pub fn analyzers()`. Then add them to the returned vec. The test counts will change from 3 to 6 analyzers.

**Step 1: Write failing tests**

Add to the `#[cfg(test)]` block in `src/rules/languages/go.rs`:

```rust
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
fn test_go_registry_now_has_six_analyzers() {
    let (_, analyzers) = super::super::get_language_and_analyzers("go").unwrap();
    assert_eq!(analyzers.len(), 6, "Go should now have 6 analyzers");
}
```

**Step 2: Run tests to see them fail**

```bash
cd /home/protec/Documentos/dev/sentinel-pro
cargo test go_unchecked_error 2>&1 | tail -5
```

Expected: error — `GoUncheckedErrorAnalyzer` not found.

**Step 3: Implement the three analyzers**

In `src/rules/languages/go.rs`, before the `pub fn analyzers()` line, insert:

```rust
/// UNCHECKED_ERROR: detects `_, _ = call()` or `_, _ := call()` patterns.
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

        // Match short var declarations where all left-hand identifiers are blank
        let query_str = r#"(short_var_declaration left: (expression_list) @lhs right: (expression_list (call_expression) @call))"#;
        let query = match Query::new(language, query_str) {
            Ok(q) => q,
            Err(_) => return violations,
        };
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, root, source_code.as_bytes());

        while let Some((m, _)) = captures.next() {
            let lhs_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "lhs");
            let call_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "call");
            if let (Some(lhs), Some(call)) = (lhs_node, call_node) {
                let lhs_text = lhs.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                // All identifiers in lhs are blanks
                if lhs_text.split(',').map(|s| s.trim()).all(|s| s == "_") {
                    let callee = call.node.utf8_text(source_code.as_bytes()).unwrap_or("unknown");
                    violations.push(RuleViolation {
                        rule_name: "UNCHECKED_ERROR".to_string(),
                        message: format!("Resultado de error descartado en llamada a {}.", callee),
                        level: RuleLevel::Warning,
                        line: Some(call.node.start_position().row + 1),
                        symbol: None,
                    });
                }
            }
        }
        violations
    }
}

/// NAMING_CONVENTION_GO: constants in ALL_CAPS format (violates Go naming).
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

        let all_caps_re = regex::Regex::new(r"^[A-Z][A-Z0-9_]{1,}$").unwrap();

        while let Some((m, _)) = captures.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source_code.as_bytes()).unwrap_or("");
                if all_caps_re.is_match(name) {
                    violations.push(RuleViolation {
                        rule_name: "NAMING_CONVENTION_GO".to_string(),
                        message: format!("Constante Go en formato ALL_CAPS: '{}'. Usar PascalCase según convención Go.", name),
                        level: RuleLevel::Info,
                        line: Some(capture.node.start_position().row + 1),
                        symbol: Some(name.to_string()),
                    });
                }
            }
        }
        violations
    }
}

/// DEFER_IN_LOOP: defer statement inside a for loop — resource not freed until function returns.
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
                    });
                }
            }
        }
        violations
    }
}
```

**Step 4: Update `analyzers()` to include the new 3**

In `src/rules/languages/go.rs`, change `pub fn analyzers()`:

```rust
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
```

**Step 5: Run tests**

```bash
cargo test languages::go 2>&1 | tail -15
```

Expected: all 8 tests in `go.rs` pass (3 old + 4 new + registry count updated to 6).

If `test_go_registry_now_has_six_analyzers` fails because the OLD test `test_go_registry_returns_analyzers_for_go_extension` asserts `len() == 3`, update that assertion to `len() == 6` too.

**Step 6: Full build**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no output.

---

## Task 2: Python Static Analysis

**Files:**
- Modify: `Cargo.toml`
- Create: `src/rules/languages/python.rs`
- Modify: `src/rules/languages/mod.rs`
- Modify: `src/index/builder.rs`

**Step 1: Add tree-sitter-python to Cargo.toml**

In `Cargo.toml`, after the `tree-sitter-go` line, add:

```toml
tree-sitter-python = "0.23"
```

**Step 2: Write failing tests**

Create `src/rules/languages/python.rs` with ONLY the test module first:

```rust
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

pub struct PythonDeadCodeAnalyzer;
pub struct PythonUnusedImportsAnalyzer;
pub struct PythonComplexityAnalyzer;

pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(PythonDeadCodeAnalyzer),
        Box::new(PythonUnusedImportsAnalyzer),
        Box::new(PythonComplexityAnalyzer),
    ]
}

// Placeholder impls (will fail tests)
impl StaticAnalyzer for PythonDeadCodeAnalyzer {
    fn analyze(&self, _: &Language, _: &str) -> Vec<RuleViolation> { vec![] }
}
impl StaticAnalyzer for PythonUnusedImportsAnalyzer {
    fn analyze(&self, _: &Language, _: &str) -> Vec<RuleViolation> { vec![] }
}
impl StaticAnalyzer for PythonComplexityAnalyzer {
    fn analyze(&self, _: &Language, _: &str) -> Vec<RuleViolation> { vec![] }
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
}
```

Register in `src/rules/languages/mod.rs`:

```rust
pub mod typescript;
pub mod go;
pub mod python;   // ← add this line

// ... in get_language_and_analyzers match:
"py" => Some((
    tree_sitter_python::LANGUAGE.into(),
    python::analyzers(),
)),
```

**Step 3: Run tests to verify they fail**

```bash
cargo test python 2>&1 | grep -E "FAILED|error\["
```

Expected: `test_python_dead_code_detects_unused_function` FAILED (returns empty vec).

**Step 4: Implement PythonDeadCodeAnalyzer**

Replace the placeholder `PythonDeadCodeAnalyzer` impl:

```rust
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
                if name.is_empty() || name == "__init__" || name == "main" { continue; }
                // Skip dunder methods and private convention
                if name.starts_with("__") && name.ends_with("__") { continue; }
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
```

**Step 5: Implement PythonUnusedImportsAnalyzer**

```rust
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

        // Match: import os / import sys (module name is the identifier)
        let query_str = r#"
            (import_statement name: (dotted_name (identifier) @module))
            (import_from_statement name: (dotted_name (identifier) @module))
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
                if name.is_empty() { continue; }
                if count_word_occurrences(source_code, name) <= 1 {
                    violations.push(RuleViolation {
                        rule_name: "UNUSED_IMPORT".to_string(),
                        message: format!("El import '{}' no parece usarse en este archivo.", name),
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
```

**Step 6: Implement PythonComplexityAnalyzer**

```rust
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
            Err(_) => return violations,
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
                if complexity > 10 {
                    violations.push(RuleViolation {
                        rule_name: "HIGH_COMPLEXITY".to_string(),
                        message: format!("Función con complejidad ciclomática {} (máximo recomendado: 10).", complexity),
                        level: RuleLevel::Error,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                    });
                }
                let line_count = func_node.range().end_point.row.saturating_sub(func_node.range().start_point.row);
                if line_count > 50 {
                    violations.push(RuleViolation {
                        rule_name: "FUNCTION_TOO_LONG".to_string(),
                        message: format!("Función de {} líneas (máximo recomendado: 50).", line_count),
                        level: RuleLevel::Warning,
                        line: Some(func_node.start_position().row + 1),
                        symbol: None,
                    });
                }
            }
        }
        violations
    }
}
```

**Step 7: Add "py" to index builder**

In `src/index/builder.rs`, find the match arm that maps extensions to tree-sitter languages (look for `"go" => Some(tree_sitter_go::LANGUAGE.into())`). Add below it:

```rust
"py" => Some(tree_sitter_python::LANGUAGE.into()),
```

**Step 8: Run all Python tests**

```bash
cargo test python 2>&1 | tail -15
```

Expected: all 4 Python tests pass.

**Step 9: Full build**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no output.

---

## Task 3: RuleConfig + `sentinel rules list`

**Files:**
- Modify: `src/rules/mod.rs` — add `value: Option<usize>` to `RuleViolation`
- Modify: `src/rules/languages/go.rs` — set `value` in HIGH_COMPLEXITY and FUNCTION_TOO_LONG
- Modify: `src/rules/static_analysis.rs` — set `value` in HIGH_COMPLEXITY and FUNCTION_TOO_LONG
- Modify: `src/config.rs` — add `RuleConfig` struct + field in `SentinelConfig`
- Modify: `src/rules/engine.rs` — add `rule_config` field + threshold post-filter
- Create: `src/commands/rules.rs` — `handle_rules_command()`
- Modify: `src/commands/mod.rs` — add `Commands::Rules` + `pub mod rules`
- Modify: `src/main.rs` — wire `Commands::Rules`

**Step 1: Add `value` field to RuleViolation**

In `src/rules/mod.rs`, change `RuleViolation`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_name: String,
    pub message: String,
    pub level: RuleLevel,
    pub line: Option<usize>,
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<usize>,   // raw numeric value (complexity, line count) for threshold filtering
}
```

**Step 2: Update all existing RuleViolation constructors**

Search for `RuleViolation {` across all source files. Any struct literal that doesn't have `value:` needs `value: None` added. Run:

```bash
grep -rn "RuleViolation {" src/ | grep -v "value:"
```

For each hit, add `value: None,` as the last field before the closing `}`.

**Step 3: Write failing test for RuleConfig threshold filtering**

In `src/commands/pro.rs` (or a test module in `src/rules/engine.rs`), add test:

```rust
// In src/rules/engine.rs #[cfg(test)] block:
#[test]
fn test_rule_engine_respects_complexity_threshold() {
    use crate::config::RuleConfig;
    use crate::rules::RuleViolation;

    // Simulate violations with value set
    let violations = vec![
        RuleViolation {
            rule_name: "HIGH_COMPLEXITY".to_string(),
            message: "complexity 12".to_string(),
            level: crate::rules::RuleLevel::Error,
            line: None,
            symbol: None,
            value: Some(12),
        },
        RuleViolation {
            rule_name: "HIGH_COMPLEXITY".to_string(),
            message: "complexity 7".to_string(),
            level: crate::rules::RuleLevel::Error,
            line: None,
            symbol: None,
            value: Some(7),
        },
    ];

    let config = RuleConfig { complexity_threshold: 15, ..RuleConfig::default() };
    let filtered = RuleEngine::filter_by_config(violations, &config);
    assert!(filtered.is_empty(), "all below threshold 15 should be filtered");
}
```

**Step 4: Add `RuleConfig` to `src/config.rs`**

After the `FeaturesConfig` struct, add:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleConfig {
    #[serde(default = "default_complexity")]
    pub complexity_threshold: usize,
    #[serde(default = "default_function_length")]
    pub function_length_threshold: usize,
    #[serde(default = "default_true")]
    pub dead_code_enabled: bool,
    #[serde(default = "default_true")]
    pub unused_imports_enabled: bool,
}

fn default_complexity() -> usize { 10 }
fn default_function_length() -> usize { 50 }

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            complexity_threshold: 10,
            function_length_threshold: 50,
            dead_code_enabled: true,
            unused_imports_enabled: true,
        }
    }
}
```

Add field to `SentinelConfig`:

```rust
// In SentinelConfig struct, after the `ml` field:
#[serde(default)]
pub rule_config: RuleConfig,
```

**Step 5: Add `filter_by_config` to RuleEngine**

In `src/rules/engine.rs`, add after the `impl RuleEngine` opening:

```rust
/// Post-filter violations based on RuleConfig thresholds.
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
```

**Step 6: Set `value` in complexity analyzers**

In `src/rules/static_analysis.rs`, find the `HIGH_COMPLEXITY` push (search for `"HIGH_COMPLEXITY".to_string()`). Change to include `value: Some(complexity)`:

```rust
violations.push(RuleViolation {
    rule_name: "HIGH_COMPLEXITY".to_string(),
    message: format!("Complejidad ciclomática {} (máximo recomendado: {}).", complexity, 10),
    level: RuleLevel::Error,
    line: Some(func_node.start_position().row + 1),
    symbol: None,
    value: Some(complexity),
});
```

Do the same for `FUNCTION_TOO_LONG`: `value: Some(line_count)`.

Do the same in `src/rules/languages/go.rs` for `GoComplexityAnalyzer` — same two violations.

**Step 7: Apply filter in check handler**

In `src/commands/pro.rs`, in the `ProCommands::Check` handler, after loading violations (after line ~432 where ignore filtering happens), add:

```rust
// Apply rule config thresholds
let rule_violations: Vec<crate::rules::RuleViolation> = violations.drain(..).map(|fv| {
    crate::rules::RuleViolation {
        rule_name: fv.rule_name,
        message: fv.message,
        level: fv.level,
        line: fv.line,
        symbol: fv.symbol,
        value: None, // FileViolation doesn't carry value
    }
}).collect();
// Note: FileViolation needs to carry value too — add value: Option<usize> to FileViolation struct
```

Actually, the simpler approach: add `value: Option<usize>` to the local `FileViolation` struct in the check handler (around line 382), and pass it through from `validate_file` results. Then call `filter_by_config` on the `FileViolation` list.

In the `FileViolation` struct (local to check handler), add:
```rust
struct FileViolation {
    file_path: String,
    rule_name: String,
    symbol: Option<String>,
    message: String,
    level: crate::rules::RuleLevel,
    line: Option<usize>,
    value: Option<usize>,  // ← add this
}
```

In the loop that builds `violations` (around line 402):
```rust
violations.push(FileViolation {
    // ...existing fields...
    value: v.value,  // ← add this
});
```

After the ignore filter (`violations.retain(...)`), add:
```rust
let rule_cfg = &agent_context.config.rule_config;
violations.retain(|v| match v.rule_name.as_str() {
    "HIGH_COMPLEXITY" => v.value.map(|n| n > rule_cfg.complexity_threshold).unwrap_or(true),
    "FUNCTION_TOO_LONG" => v.value.map(|n| n > rule_cfg.function_length_threshold).unwrap_or(true),
    "DEAD_CODE" | "DEAD_CODE_GLOBAL" => rule_cfg.dead_code_enabled,
    "UNUSED_IMPORT" => rule_cfg.unused_imports_enabled,
    _ => true,
});
```

**Step 8: Create `src/commands/rules.rs`**

```rust
use crate::config::SentinelConfig;
use colored::Colorize;

pub fn handle_rules_command(project_root: &std::path::Path) {
    let config = SentinelConfig::load(project_root);
    let rule_cfg = config
        .as_ref()
        .map(|c| c.rule_config.clone())
        .unwrap_or_default();

    println!("\n{}", "📋 Reglas activas:".bold());
    println!(
        "  {:<30} {:<10} {}",
        "REGLA".bold().underline(),
        "NIVEL".bold().underline(),
        "DESCRIPCIÓN".bold().underline()
    );

    let rules = [
        ("DEAD_CODE",            "ERROR",   "Funciones/variables no referenciadas",
            if rule_cfg.dead_code_enabled { "✅" } else { "🚫" }, None),
        ("UNUSED_IMPORT",        "WARNING", "Imports sin uso en el archivo",
            if rule_cfg.unused_imports_enabled { "✅" } else { "🚫" }, None),
        ("HIGH_COMPLEXITY",      "ERROR",   "Complejidad ciclomática excede umbral",
            "✅", Some(format!("threshold: {}", rule_cfg.complexity_threshold))),
        ("FUNCTION_TOO_LONG",    "WARNING", "Funciones que exceden el límite de líneas",
            "✅", Some(format!("threshold: {} líneas", rule_cfg.function_length_threshold))),
        ("UNCHECKED_ERROR",      "WARNING", "Error de Go sin verificar (blank identifier)",
            "✅", None),
        ("NAMING_CONVENTION_GO", "INFO",    "Constante Go en formato ALL_CAPS",
            "✅", None),
        ("DEFER_IN_LOOP",        "WARNING", "defer dentro de bucle for",
            "✅", None),
    ];

    for (name, level, desc, status, threshold) in &rules {
        let threshold_info = threshold.as_deref().unwrap_or("");
        println!(
            "  {} {:<28} {:<10} {}  {}",
            status,
            name.yellow(),
            format!("[{}]", level),
            desc,
            threshold_info.dimmed()
        );
    }

    println!();
    if config.is_none() {
        println!("   ℹ️  No se encontró .sentinelrc.toml. Usando valores por defecto.");
    } else {
        println!("   ℹ️  Para cambiar umbrales, edita la sección [rule_config] en .sentinelrc.toml");
    }
    println!("   Ejemplo:");
    println!("   [rule_config]");
    println!("   complexity_threshold = 15");
    println!("   function_length_threshold = 80");
    println!("   dead_code_enabled = true");
}
```

**Step 9: Add `Commands::Rules` to mod.rs and main.rs**

In `src/commands/mod.rs`, add to `Commands` enum:

```rust
/// Lista las reglas activas con sus umbrales
Rules,
```

Add at the top of the file:
```rust
pub mod rules;
```

In `src/main.rs`, add to the match:

```rust
Some(Commands::Rules) => {
    let project_root = crate::config::SentinelConfig::find_project_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    commands::rules::handle_rules_command(&project_root);
}
```

**Step 10: Run tests**

```bash
cargo test rule_engine_respects_complexity 2>&1 | tail -5
cargo build 2>&1 | grep "^error"
```

Expected: test passes, no build errors.

---

## Task 4: SARIF Output

**Files:**
- Modify: `src/commands/pro.rs` — `render_sarif()` function + SARIF branch in Check handler
- Create: `docs/github-actions-example.yml`

**Step 1: Write failing test**

In `src/commands/pro.rs` or a new test in the same file's `#[cfg(test)]` section, add a unit test (this requires extracting `render_sarif` as a standalone function):

```rust
#[cfg(test)]
mod tests {
    // ...existing tests...

    #[test]
    fn test_render_sarif_produces_valid_structure() {
        use super::SarifIssue;
        let issues = vec![
            SarifIssue {
                file: "src/main.ts".to_string(),
                rule: "DEAD_CODE".to_string(),
                severity: "warning".to_string(),
                message: "userId no se usa".to_string(),
                line: Some(23),
            },
        ];
        let sarif = super::render_sarif(&issues);
        assert!(sarif.contains("\"$schema\""), "must have schema");
        assert!(sarif.contains("\"2.1.0\""), "must have version");
        assert!(sarif.contains("DEAD_CODE"), "must include rule");
        assert!(sarif.contains("\"startLine\": 23"), "must include line number");
    }
}
```

**Step 2: Add `SarifIssue` struct and `render_sarif()` function**

In `src/commands/pro.rs`, near the top (after `ReviewRecord` definition, before `handle_pro_command`):

```rust
/// Reusable structure for SARIF rendering.
#[derive(Debug)]
pub struct SarifIssue {
    pub file: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub line: Option<usize>,
}

/// Renders a SARIF 2.1.0 JSON string from a list of issues.
pub fn render_sarif(issues: &[SarifIssue]) -> String {
    // Collect unique rules for the driver.rules array
    let mut seen_rules: Vec<&str> = Vec::new();
    for issue in issues {
        if !seen_rules.contains(&issue.rule.as_str()) {
            seen_rules.push(&issue.rule);
        }
    }

    let rules_json: Vec<serde_json::Value> = seen_rules.iter().map(|r| {
        serde_json::json!({
            "id": r,
            "shortDescription": { "text": r }
        })
    }).collect();

    let results_json: Vec<serde_json::Value> = issues.iter().map(|i| {
        let level = match i.severity.as_str() {
            "error" => "error",
            "info"  => "note",
            _       => "warning",
        };
        let region = if let Some(line) = i.line {
            serde_json::json!({ "startLine": line })
        } else {
            serde_json::json!({ "startLine": 1 })
        };
        serde_json::json!({
            "ruleId": i.rule,
            "level": level,
            "message": { "text": i.message },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": i.file,
                        "uriBaseId": "%SRCROOT%"
                    },
                    "region": region
                }
            }]
        })
    }).collect();

    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "sentinel",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/your-org/sentinel",
                    "rules": rules_json
                }
            },
            "results": results_json
        }]
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_default()
}
```

**Step 3: Add SARIF branch in check handler**

In `ProCommands::Check { target, format }` handler, change the opening:

```rust
let json_mode = format.to_lowercase() == "json";
let sarif_mode = format.to_lowercase() == "sarif";
```

After `json_mode` is used to select json output, add a SARIF branch. At the point where `json_issues` is rendered (around line 495 where `json_mode` check is):

```rust
if sarif_mode {
    let sarif_issues: Vec<SarifIssue> = violations.iter().map(|v| {
        let sev = match v.level {
            RuleLevel::Error   => "error",
            RuleLevel::Warning => "warning",
            RuleLevel::Info    => "note",
        };
        SarifIssue {
            file: v.file_path.clone(),
            rule: v.rule_name.clone(),
            severity: sev.to_string(),
            message: v.message.clone(),
            line: v.line,
        }
    }).collect();
    println!("{}", render_sarif(&sarif_issues));
} else if json_mode {
    // ... existing json output ...
} else {
    // ... existing text output ...
}
```

Note: the `violations` iteration that currently builds `json_issues` (and does `n_errors++` counting) still needs to run before this branch. Move the counting loop before the format selection, collecting into a unified `Vec<SarifIssue>` that also populates `json_issues`, `n_errors`, etc.

**Practical refactor:** Change the format dispatch to run the counting loop first, then branch:

```rust
// Count loop (always runs):
for v in &violations {
    let sev_str = match v.level {
        RuleLevel::Error   => { n_errors   += 1; "error" }
        RuleLevel::Warning => { n_warnings += 1; "warning" }
        RuleLevel::Info    => { n_infos    += 1; "info" }
    };
    if json_mode || sarif_mode {
        json_issues.push(JsonIssue { ... });
    }
}

// Output branch:
if sarif_mode {
    let sarif_issues: Vec<SarifIssue> = json_issues.iter().map(|j| SarifIssue {
        file: j.file.clone(),
        rule: j.rule.clone(),
        severity: j.severity.clone(),
        message: j.message.clone(),
        line: j.line,
    }).collect();
    println!("{}", render_sarif(&sarif_issues));
} else if json_mode {
    // existing json output
} else {
    // existing text output (reprint from violations)
}
```

**Step 4: Create docs/github-actions-example.yml**

```yaml
# .github/workflows/sentinel.yml
name: Sentinel Code Quality

on: [push, pull_request]

jobs:
  sentinel-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Sentinel
        run: cargo install --path .

      - name: Run Sentinel check
        run: sentinel pro check src/ --format sarif > sentinel.sarif
        continue-on-error: true

      - name: Upload SARIF to GitHub Security tab
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: sentinel.sarif
```

**Step 5: Run tests and build**

```bash
cargo test render_sarif 2>&1 | tail -5
cargo build 2>&1 | grep "^error"
```

Expected: test passes, no build errors.

---

## Task 5: git diff in Review Context

**Files:**
- Modify: `src/commands/pro.rs` — add `get_changed_files()` + inject before candidates sort

**Step 1: Write failing test**

In `src/commands/pro.rs` test section:

```rust
#[test]
fn test_get_changed_files_returns_vec() {
    // Returns a Vec (possibly empty in non-git dirs), never panics
    let files = super::get_changed_files(&std::path::PathBuf::from("."));
    // Can't assert content (CI-dependent), just verify it doesn't crash
    let _ = files;
}
```

**Step 2: Implement `get_changed_files()`**

In `src/commands/pro.rs`, before `handle_pro_command`, add:

```rust
/// Returns relative paths of files changed in the current git working tree / last commit.
/// Silently returns empty Vec if not a git repo or git is unavailable.
pub fn get_changed_files(project_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    // Try staged + unstaged changes first, then fall back to committed changes
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok();

    let mut files = Vec::new();
    if let Some(out) = output {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let p = project_root.join(line.trim());
                if p.exists() {
                    files.push(p);
                }
            }
        }
    }
    files
}
```

**Step 3: Inject changed files before sort in review handler**

In the review handler, find the line:
```rust
// Priorizar archivos de arquitectura (NestJS, etc.) al frente
let priority_patterns = [
```

Before that line, inject changed files at the front of `candidates`:

```rust
// 0. Inject recently changed files (from git diff) at the front — they get priority slots
let changed_files = get_changed_files(&agent_context.project_root);
let mut changed_count = 0usize;
for cf in &changed_files {
    // Only include if it's a candidate (right extension, not ignored)
    let ext = cf.extension().and_then(|e| e.to_str()).unwrap_or("");
    if agent_context.config.file_extensions.contains(&ext.to_string()) {
        if !candidates.contains(cf) {
            candidates.insert(0, cf.clone());
        } else {
            // Move to front
            candidates.retain(|p| p != cf);
            candidates.insert(0, cf.clone());
        }
        changed_count += 1;
    }
}
```

After the coverage line is printed (the `📎 Contexto:` line that currently exists), update it to include git diff count. Search for `📎 Contexto:` and change it to show changed files count:

```rust
let diff_note = if changed_count > 0 {
    format!(" · {} del diff reciente", changed_count)
} else {
    String::new()
};
println!("   📎 Contexto: {} archivo(s) · {} líneas{}",
    muestras, total_lines_loaded, diff_note);
```

**Step 4: Run test and build**

```bash
cargo test get_changed_files 2>&1 | tail -5
cargo build 2>&1 | grep "^error"
```

---

## Task 6: Monitor Daemon (`--daemon`, `--stop`, `--status`)

**Files:**
- Modify: `Cargo.toml` — add `nix` crate
- Modify: `src/commands/mod.rs` — add flags to `Commands::Monitor`
- Modify: `src/commands/monitor.rs` — implement daemon/stop/status logic
- Modify: `src/main.rs` — pass new flags to monitor handler

**Step 1: Add nix to Cargo.toml**

```toml
nix = { version = "0.29", features = ["signal", "process"] }
```

**Step 2: Add flags to `Commands::Monitor`**

In `src/commands/mod.rs`, change:

```rust
/// Inicia el modo monitor (comportamiento clásico)
Monitor,
```

to:

```rust
/// Inicia el modo monitor (foreground) o gestiona el proceso daemon
Monitor {
    /// Iniciar como daemon en segundo plano (guarda PID en .sentinel/monitor.pid)
    #[arg(long)]
    daemon: bool,
    /// Detener el daemon en ejecución
    #[arg(long)]
    stop: bool,
    /// Mostrar estado del daemon
    #[arg(long)]
    status: bool,
},
```

**Step 3: Write tests for daemon helpers**

In `src/commands/monitor.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pid_file_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let sentinel_dir = tmp.path().join(".sentinel");
        std::fs::create_dir_all(&sentinel_dir).unwrap();
        let pid_path = sentinel_dir.join("monitor.pid");

        write_pid_file(&pid_path, 12345).unwrap();
        let pid = read_pid_file(&pid_path).unwrap();
        assert_eq!(pid, 12345);
    }

    #[test]
    fn test_read_pid_file_returns_none_if_missing() {
        let tmp = TempDir::new().unwrap();
        let pid_path = tmp.path().join(".sentinel/monitor.pid");
        assert!(read_pid_file(&pid_path).is_none());
    }
}
```

**Step 4: Implement daemon helpers in monitor.rs**

Add these helper functions to `src/commands/monitor.rs` (before `start_monitor`):

```rust
use std::path::Path;

pub fn write_pid_file(pid_path: &Path, pid: u32) -> anyhow::Result<()> {
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(pid_path, pid.to_string())?;
    Ok(())
}

pub fn read_pid_file(pid_path: &Path) -> Option<u32> {
    std::fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

pub fn handle_daemon(project_root: &Path) -> anyhow::Result<()> {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    if pid_path.exists() {
        if let Some(pid) = read_pid_file(&pid_path) {
            // Check if already running
            if is_process_alive(pid) {
                println!("⚠️  sentinel monitor ya está corriendo (PID {}). Usa --stop para detenerlo.", pid);
                return Ok(());
            }
        }
    }

    // Re-spawn self without --daemon flag
    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(exe)
        .arg("monitor")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let pid = child.id();
    write_pid_file(&pid_path, pid)?;
    println!("✅ sentinel monitor iniciado en background (PID {})", pid);
    println!("   Detener: sentinel monitor --stop");
    Ok(())
}

pub fn handle_stop(project_root: &Path) {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    match read_pid_file(&pid_path) {
        None => {
            println!("ℹ️  No hay PID file. sentinel monitor no está corriendo como daemon.");
        }
        Some(pid) => {
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                match signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                    Ok(_) => {
                        let _ = std::fs::remove_file(&pid_path);
                        println!("✅ sentinel monitor detenido (PID {})", pid);
                    }
                    Err(e) => {
                        println!("⚠️  No se pudo enviar SIGTERM a PID {}: {}. El proceso puede haber terminado.", pid, e);
                        let _ = std::fs::remove_file(&pid_path);
                    }
                }
            }
            #[cfg(not(unix))]
            {
                println!("⚠️  --stop solo está soportado en sistemas Unix.");
            }
        }
    }
}

pub fn handle_status(project_root: &Path) {
    let pid_path = project_root.join(".sentinel/monitor.pid");
    match read_pid_file(&pid_path) {
        None => println!("ℹ️  sentinel monitor no está corriendo como daemon."),
        Some(pid) => {
            if is_process_alive(pid) {
                println!("✅ sentinel monitor corriendo (PID {})", pid);
            } else {
                println!("⚠️  PID {} encontrado pero el proceso ya no existe. Limpiando PID file.", pid);
                let _ = std::fs::remove_file(&pid_path);
            }
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal;
        use nix::unistd::Pid;
        // kill(pid, 0) = check if process exists without sending a signal
        signal::kill(Pid::from_raw(pid as i32), None).is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}
```

**Step 5: Update `start_monitor` signature and main.rs**

In `src/commands/monitor.rs`, the existing `pub fn start_monitor()` stays as-is for the foreground case.

In `src/main.rs`, change the Monitor match arm:

```rust
Some(Commands::Monitor { daemon, stop, status }) => {
    let project_root = crate::config::SentinelConfig::find_project_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if stop {
        commands::monitor::handle_stop(&project_root);
    } else if status {
        commands::monitor::handle_status(&project_root);
    } else if daemon {
        if let Err(e) = commands::monitor::handle_daemon(&project_root) {
            eprintln!("❌ Error iniciando daemon: {}", e);
            std::process::exit(1);
        }
    } else {
        commands::monitor::start_monitor();
    }
}
```

**Step 6: Run tests and build**

```bash
cargo test pid_file 2>&1 | tail -5
cargo build 2>&1 | grep "^error"
```

Expected: 2 PID tests pass, no build errors.

**Step 7: Full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all existing 53 tests + new tests pass (minimum 63 total).

---

## Final Verification

```bash
# 1. Full test suite
cargo test 2>&1 | grep -E "^test result|FAILED"

# 2. Check format SARIF
cargo run --bin sentinel -- pro check src/ --format sarif 2>/dev/null | python3 -m json.tool | head -5

# 3. Rules list
cargo run --bin sentinel -- rules

# 4. Monitor status
cargo run --bin sentinel -- monitor --status

# 5. Build release
cargo build --release 2>&1 | grep "^error"
```

All should succeed cleanly.
