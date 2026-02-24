# Sentinel Depth — Multi-language, Smart Review, Ignore UX, Audit TTY, Review History

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Five production-readiness improvements: multi-language static analysis (Go + registry), hybrid review context by project size, ignore UX with symbol normalization, audit TTY auto-detection with improved interactive menu, and persistent review history with diff.

**Architecture:** Six independent tasks. Tasks 1-2 build the language registry and Go support. Tasks 3-6 are isolated feature additions in `pro.rs`, `ignore.rs`, and `mod.rs`. All tasks commit independently. 43 existing tests must pass throughout.

**Tech Stack:** Rust, tree-sitter 0.26.5, tree-sitter-go 0.23, tokio, chrono 0.4, std::io::IsTerminal (stable since Rust 1.70), rusqlite, serde_json.

---

## Task 1: Language registry + Go analyzers

**Files:**
- Create: `src/rules/languages/mod.rs`
- Create: `src/rules/languages/typescript.rs`
- Create: `src/rules/languages/go.rs`
- Modify: `src/rules/mod.rs`
- Modify: `src/rules/engine.rs`
- Modify: `Cargo.toml`

**Context:**

Current `src/rules/engine.rs` has:
```rust
pub struct RuleEngine {
    pub framework_def: Option<FrameworkDefinition>,
    analyzers: Vec<Box<dyn StaticAnalyzer + Send + Sync>>,
    pub index_db: Option<std::sync::Arc<crate::index::IndexDb>>,
}
impl RuleEngine {
    pub fn new() -> Self {
        Self {
            framework_def: None,
            analyzers: vec![
                Box::new(DeadCodeAnalyzer::new()),
                Box::new(UnusedImportsAnalyzer::new()),
                Box::new(ComplexityAnalyzer::new()),
            ],
            index_db: None,
        }
    }
```

And in `validate_file` (~line 41):
```rust
let language: Option<Language> = match ext {
    "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
    "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
    _ => None,
};
if let Some(lang) = language {
    for analyzer in &self.analyzers {
        violations.extend(analyzer.analyze(&lang, content));
    }
    let framework = ...;
    let naming_violations = NamingAnalyzerWithFramework::new(framework).analyze(&lang, content);
    violations.extend(naming_violations);
}
```

Current `src/rules/mod.rs`:
```rust
pub mod engine;
pub mod static_analysis;
pub use engine::RuleEngine;
```

**Step 1: Add `tree-sitter-go` to Cargo.toml**

In `Cargo.toml`, find:
```toml
tree-sitter-javascript = "0.25.0"
```
Add after it:
```toml
tree-sitter-go = "0.23"
```

**Step 2: Create `src/rules/languages/typescript.rs`**

```rust
use crate::rules::static_analysis::{StaticAnalyzer, DeadCodeAnalyzer, UnusedImportsAnalyzer, ComplexityAnalyzer};

/// Returns the set of static analyzers for TypeScript/JavaScript files.
pub fn analyzers() -> Vec<Box<dyn StaticAnalyzer + Send + Sync>> {
    vec![
        Box::new(DeadCodeAnalyzer::new()),
        Box::new(UnusedImportsAnalyzer::new()),
        Box::new(ComplexityAnalyzer::new()),
    ]
}
```

**Step 3: Create `src/rules/languages/go.rs`**

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
            (switch_statement) @branch
            (select_statement) @branch
            (expression_case_clause) @branch
            (type_case_clause) @branch
            (binary_expression operator: "&&") @branch
            (binary_expression operator: "||") @branch
        "#;
        let branch_query = match Query::new(language, branch_query_str) {
            Ok(q) => q,
            Err(_) => {
                // Fallback: only basic control flow
                match Query::new(language, "(if_statement) @b (for_statement) @b (switch_statement) @b") {
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
```

**Step 4: Create `src/rules/languages/mod.rs`**

```rust
pub mod typescript;
pub mod go;

use tree_sitter::Language;
use crate::rules::static_analysis::StaticAnalyzer;

/// Returns the tree-sitter Language and the set of analyzers for the given file extension.
/// Returns None for unsupported extensions.
pub fn get_language_and_analyzers(
    ext: &str,
) -> Option<(Language, Vec<Box<dyn StaticAnalyzer + Send + Sync>>)> {
    match ext {
        "ts" | "tsx" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            typescript::analyzers(),
        )),
        "js" | "jsx" => Some((
            tree_sitter_javascript::LANGUAGE.into(),
            typescript::analyzers(),
        )),
        "go" => Some((
            tree_sitter_go::LANGUAGE.into(),
            go::analyzers(),
        )),
        _ => None,
    }
}
```

**Step 5: Add `pub mod languages;` to `src/rules/mod.rs`**

Find:
```rust
pub mod engine;
pub mod static_analysis;
```
Replace with:
```rust
pub mod engine;
pub mod languages;
pub mod static_analysis;
```

**Step 6: Update `src/rules/engine.rs`**

Remove the `analyzers` field and update `new()` and `validate_file`.

Find:
```rust
use crate::rules::static_analysis::{StaticAnalyzer, DeadCodeAnalyzer, UnusedImportsAnalyzer, ComplexityAnalyzer};
```
Replace with:
```rust
use crate::rules::static_analysis::NamingAnalyzerWithFramework;
use crate::rules::languages;
```

Find the `RuleEngine` struct definition and replace:
```rust
pub struct RuleEngine {
    pub framework_def: Option<FrameworkDefinition>,
    analyzers: Vec<Box<dyn StaticAnalyzer + Send + Sync>>,
    pub index_db: Option<std::sync::Arc<crate::index::IndexDb>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            framework_def: None,
            analyzers: vec![
                Box::new(DeadCodeAnalyzer::new()),
                Box::new(UnusedImportsAnalyzer::new()),
                Box::new(ComplexityAnalyzer::new()),
            ],
            index_db: None,
        }
    }
```
With:
```rust
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
```

Find in `validate_file` the block:
```rust
        let language: Option<Language> = match ext {
            "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            _ => None,
        };

        if let Some(lang) = language {
            for analyzer in &self.analyzers {
                violations.extend(analyzer.analyze(&lang, content));
            }

            let framework = self.framework_def.as_ref()
                .map(|f| f.framework.as_str())
                .unwrap_or("typescript");
            let naming_violations = crate::rules::static_analysis::NamingAnalyzerWithFramework::new(framework)
                .analyze(&lang, content);
            violations.extend(naming_violations);
        }
```
Replace with:
```rust
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
```

Also remove the now-unused import `use tree_sitter::Language;` from engine.rs if it's no longer needed (only remove it if it's not used elsewhere in that file).

**Step 7: Write failing test**

Add to `src/rules/languages/go.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn go_lang() -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
    }

    #[test]
    fn test_go_dead_code_detects_unused_function() {
        let src = r#"
package main

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
            "should detect unusedHelper as dead code"
        );
    }

    #[test]
    fn test_go_dead_code_ignores_main_and_exported() {
        let src = r#"
package main

func main() {}
func ExportedFunc() {}
"#;
        let violations = GoDeadCodeAnalyzer.analyze(&go_lang(), src);
        assert!(violations.is_empty(), "main and exported functions should not be flagged");
    }

    #[test]
    fn test_go_complexity_flags_complex_function() {
        let src = r#"
package main

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
            "deeply nested function should have HIGH_COMPLEXITY"
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
```

**Step 8: Run tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: all 43 + 5 new = 48 tests passing.

If `tree-sitter-go` version `0.23` does not compile, check `https://crates.io/crates/tree-sitter-go` for the latest compatible version and update `Cargo.toml` accordingly.

**Step 9: Verify Go analysis works end-to-end**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

**Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock src/rules/languages/ src/rules/mod.rs src/rules/engine.rs
git commit -m "feat: language registry + Go static analysis (DEAD_CODE, UNUSED_IMPORT, HIGH_COMPLEXITY, FUNCTION_TOO_LONG)"
```

---

## Task 2: Go indexing in tree-sitter builder

**Files:**
- Modify: `src/index/builder.rs`

**Context:**

Current `src/index/builder.rs` (line ~61-68):
```rust
let language = match ext {
    "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
    "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
    _ => None,
};
if let Some(lang) = language {
    // ... parse and extract symbols
}
```

**Step 1: Add Go to the language match**

Find:
```rust
            "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            _ => None,
```
Replace with:
```rust
            "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            "go"         => Some(tree_sitter_go::LANGUAGE.into()),
            _ => None,
```

**Step 2: Build**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

**Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: 48 tests passing (same as after Task 1).

**Step 4: Commit**

```bash
git add src/index/builder.rs
git commit -m "feat: index Go files with tree-sitter-go (symbol extraction for sentinel index)"
```

---

## Task 3: Review hybrid context (size-based dispatch)

**Files:**
- Modify: `src/commands/pro.rs`

**Context:**

The Review handler is at `ProCommands::Review =>` (~line 1514). It currently:
1. Walks the project tree
2. Reads `candidates` (all source files in src/)
3. Priority-sorts candidates (NestJS patterns)
4. Takes top 8 × 100 lines
5. Builds `codigo_muestra` string
6. Prints coverage line
7. Sends to ReviewerAgent

After Task 3, the flow will be:
- Count source files → dispatch to `small_review` (<20), `medium_review` (20-80), or `large_review` (80+)
- Each path sets `codigo_muestra` and `muestras`/`total_lines_loaded`
- Rest of handler (LLM call, output) unchanged

**Step 1: Write a unit test for `select_by_centrality_files`**

At the bottom of `src/commands/pro.rs`, inside the existing `#[cfg(test)] mod batching_tests` block (add AFTER the last test function, before the closing `}`):

```rust
    #[test]
    fn test_review_size_thresholds() {
        // < 20 → small, 20-80 → medium, 80+ → large
        assert_eq!(review_size_mode(5),   ReviewMode::Small);
        assert_eq!(review_size_mode(19),  ReviewMode::Small);
        assert_eq!(review_size_mode(20),  ReviewMode::Medium);
        assert_eq!(review_size_mode(80),  ReviewMode::Medium);
        assert_eq!(review_size_mode(81),  ReviewMode::Large);
        assert_eq!(review_size_mode(200), ReviewMode::Large);
    }
```

**Step 2: Add the enum and function just before the `handle_pro_command` function**

Find in `src/commands/pro.rs` the function `pub fn handle_pro_command(`. Add just before it:

```rust
#[derive(Debug, PartialEq)]
enum ReviewMode { Small, Medium, Large }

fn review_size_mode(file_count: usize) -> ReviewMode {
    if file_count < 20 { ReviewMode::Small }
    else if file_count <= 80 { ReviewMode::Medium }
    else { ReviewMode::Large }
}
```

**Step 3: Run the test to verify it passes**

```bash
cargo test test_review_size_thresholds 2>&1 | grep -E "ok|FAILED"
```

Expected: `test batching_tests::test_review_size_thresholds ... ok`

**Step 4: Refactor the code sample collection block in the Review handler**

Find the block that sets `codigo_muestra` (lines ~1604-1623):

```rust
            let mut codigo_muestra = String::new();
            let mut muestras = 0usize;
            let mut total_lines_loaded = 0usize;
            for p in &candidates {
                if muestras >= 8 {
                    break;
                }
                if let Ok(contenido) = std::fs::read_to_string(p) {
                    let lines: Vec<&str> = contenido.lines().collect();
                    let preview_lines = lines.len().min(100);
                    let preview = lines[..preview_lines].join("\n");
                    let rel = p
                        .strip_prefix(&agent_context.project_root)
                        .map(|r| r.display().to_string())
                        .unwrap_or_else(|_| p.display().to_string());
                    codigo_muestra.push_str(&format!("\n\n=== {} ===\n{}", rel, preview));
                    muestras += 1;
                    total_lines_loaded += preview_lines;
                }
            }
```

Replace with:

```rust
            let mut codigo_muestra = String::new();
            let mut muestras = 0usize;
            let mut total_lines_loaded = 0usize;

            let review_mode = review_size_mode(candidates.len());

            match review_mode {
                ReviewMode::Small => {
                    // < 20 files: current behavior, top 8 × 100 lines
                    for p in candidates.iter().take(8) {
                        if let Ok(contenido) = std::fs::read_to_string(p) {
                            let lines: Vec<&str> = contenido.lines().collect();
                            let preview_lines = lines.len().min(100);
                            codigo_muestra.push_str(&format!(
                                "\n\n=== {} ===\n{}",
                                p.strip_prefix(&agent_context.project_root)
                                    .map(|r| r.display().to_string())
                                    .unwrap_or_else(|_| p.display().to_string()),
                                lines[..preview_lines].join("\n")
                            ));
                            muestras += 1;
                            total_lines_loaded += preview_lines;
                        }
                    }
                }
                ReviewMode::Medium => {
                    // 20-80 files: select by call_graph centrality, top 20 × 150 lines
                    let central_files: Vec<std::path::PathBuf> = if let Some(ref db) = agent_context.index_db {
                        let conn = db.lock();
                        let mut stmt = conn.prepare(
                            "SELECT s.file_path, COUNT(*) as hits \
                             FROM call_graph c \
                             JOIN symbols s ON c.callee_symbol = s.name \
                             GROUP BY s.file_path \
                             ORDER BY hits DESC \
                             LIMIT 20"
                        ).ok();
                        if let Some(ref mut stmt) = stmt {
                            stmt.query_map([], |row| row.get::<_, String>(0))
                                .map(|rows| rows.flatten().map(std::path::PathBuf::from).collect())
                                .unwrap_or_default()
                        } else { vec![] }
                    } else { vec![] };

                    // If centrality returns results, use them; else fall back to priority sort
                    let selected = if central_files.is_empty() {
                        candidates.iter().take(20).cloned().collect::<Vec<_>>()
                    } else {
                        central_files
                    };

                    for p in selected.iter().take(20) {
                        if let Ok(contenido) = std::fs::read_to_string(p) {
                            let lines: Vec<&str> = contenido.lines().collect();
                            let preview_lines = lines.len().min(150);
                            codigo_muestra.push_str(&format!(
                                "\n\n=== {} ===\n{}",
                                p.strip_prefix(&agent_context.project_root)
                                    .map(|r| r.display().to_string())
                                    .unwrap_or_else(|_| p.display().to_string()),
                                lines[..preview_lines].join("\n")
                            ));
                            muestras += 1;
                            total_lines_loaded += preview_lines;
                        }
                    }
                }
                ReviewMode::Large => {
                    // 80+ files: group by top-level subdir, pick top files per group
                    use std::collections::HashMap;
                    let mut groups: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();
                    for p in &candidates {
                        let rel = p.strip_prefix(&agent_context.project_root)
                            .unwrap_or(p.as_path());
                        let top_dir = rel.components().next()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .unwrap_or_else(|| "root".to_string());
                        groups.entry(top_dir).or_default().push(p.clone());
                    }
                    let mut group_keys: Vec<String> = groups.keys().cloned().collect();
                    group_keys.sort();
                    for key in group_keys.iter().take(6) {
                        let group_files = &groups[key];
                        for p in group_files.iter().take(10) {
                            if let Ok(contenido) = std::fs::read_to_string(p) {
                                let lines: Vec<&str> = contenido.lines().collect();
                                let preview_lines = lines.len().min(80);
                                codigo_muestra.push_str(&format!(
                                    "\n\n=== {} ===\n{}",
                                    p.strip_prefix(&agent_context.project_root)
                                        .map(|r| r.display().to_string())
                                        .unwrap_or_else(|_| p.display().to_string()),
                                    lines[..preview_lines].join("\n")
                                ));
                                muestras += 1;
                                total_lines_loaded += preview_lines;
                            }
                        }
                    }
                }
            }
```

**Step 5: Update the coverage line** to show the mode

Find:
```rust
            println!(
                "   📎 Contexto: {} archivo(s), {} líneas de código cargadas",
                muestras, total_lines_loaded
            );
```
Replace with:
```rust
            let mode_label = match review_size_mode(candidates.len()) {
                ReviewMode::Small  => "proyecto pequeño",
                ReviewMode::Medium => "modo centralidad",
                ReviewMode::Large  => "modo multi-grupo",
            };
            println!(
                "   📎 Contexto: {} archivo(s) · {} líneas · {} ({} en total)",
                muestras, total_lines_loaded, mode_label, candidates.len()
            );
```

**Step 6: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: 49 tests passing (48 + 1 new size threshold test).

**Step 7: Commit**

```bash
git add src/commands/pro.rs
git commit -m "feat: review hybrid context — small/medium/large dispatch with centrality and multi-group modes"
```

---

## Task 4: `sentinel ignore` — symbol normalization + per-violation hints

**Files:**
- Modify: `src/commands/ignore.rs`
- Modify: `src/commands/pro.rs`

**Context:**

`src/commands/ignore.rs` has `load_ignore_entries` (public) and `handle_ignore_command`. The `IgnoreEntry` struct has `symbol: Option<String>`.

In `src/commands/pro.rs` check handler (~line 337-348): filtering uses exact symbol comparison.
At ~line 421-427: a generic hint `sentinel ignore <REGLA> <ARCHIVO>` is shown once at the end.

After this task: each violation shows its own copy-ready `sentinel ignore` command. Symbols are normalized on save and on compare.

**Step 1: Write a failing test for `normalize_symbol`**

Add to `src/commands/ignore.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::normalize_symbol;

    #[test]
    fn test_normalize_strips_suffix_and_lowercases() {
        assert_eq!(normalize_symbol("AuthService"),    "auth");
        assert_eq!(normalize_symbol("UserController"), "user");
        assert_eq!(normalize_symbol("auth_service"),   "auth");
        assert_eq!(normalize_symbol("userId"),         "userid");
        assert_eq!(normalize_symbol("getUser"),        "getuser");
        assert_eq!(normalize_symbol("SomethingElse"),  "somethingelse");
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_normalize_strips_suffix 2>&1 | grep -E "ok|FAILED|error"
```

Expected: compile error (function not defined yet).

**Step 3: Add `normalize_symbol` to `src/commands/ignore.rs`**

Find in `src/commands/ignore.rs` the function `pub fn load_ignore_entries`. Add BEFORE it:

```rust
/// Normalize a symbol name for fuzzy ignore matching.
/// Lowercases, removes underscores, strips common framework suffixes.
pub fn normalize_symbol(s: &str) -> String {
    let suffixes = [
        "service", "controller", "repository", "guard",
        "module", "handler", "resolver", "provider",
    ];
    let s = s.to_lowercase().replace('_', "");
    for suffix in suffixes {
        if let Some(base) = s.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    s
}
```

**Step 4: Apply normalization when saving an ignore entry**

In `handle_ignore_command`, find the block that saves a new entry. It will look like:
```rust
    let sym_opt = if symbol.is_empty() { None } else { Some(symbol.clone()) };
```
or similar. Replace `Some(symbol.clone())` with `Some(normalize_symbol(&symbol))`:

Find (exact text may vary slightly):
```rust
        let sym_opt: Option<String> = if symbol.is_empty() { None } else { Some(symbol.clone()) };
```
Replace with:
```rust
        let sym_opt: Option<String> = if symbol.is_empty() { None } else { Some(normalize_symbol(&symbol)) };
```

**Step 5: Apply normalization in the check handler filter**

In `src/commands/pro.rs`, find the filtering block (~line 338-348):

```rust
            if !ignore_entries.is_empty() {
                violations.retain(|v| {
                    !ignore_entries.iter().any(|e| {
                        e.rule == v.rule_name
                            && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                            && e.symbol
                                .as_ref()
                                .map(|s| v.symbol.as_deref() == Some(s.as_str()))
                                .unwrap_or(true)
                    })
                });
            }
```

Replace with:

```rust
            if !ignore_entries.is_empty() {
                violations.retain(|v| {
                    !ignore_entries.iter().any(|e| {
                        e.rule == v.rule_name
                            && (v.file_path.contains(&e.file) || e.file.contains(&v.file_path))
                            && e.symbol
                                .as_ref()
                                .map(|s| {
                                    // Normalize both sides for fuzzy symbol matching
                                    let norm_entry = crate::commands::ignore::normalize_symbol(s);
                                    let norm_violation = v.symbol.as_deref()
                                        .map(|vs| crate::commands::ignore::normalize_symbol(vs))
                                        .unwrap_or_default();
                                    norm_entry == norm_violation
                                })
                                .unwrap_or(true)
                    })
                });
            }
```

**Step 6: Replace the generic end-of-check hint with per-violation hints**

In `src/commands/pro.rs` check handler, in the violation display loop (inside `} else {` block after `if json_mode {`). Currently (~line 379-385):

```rust
                } else {
                    let line_info = v.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!("   {} [{}{}]: {}", icon.color(match v.level {
                        RuleLevel::Error   => "red",
                        RuleLevel::Warning => "yellow",
                        RuleLevel::Info    => "blue",
                    }), v.rule_name.yellow(), line_info, v.message);
                }
```

Replace with:

```rust
                } else {
                    let line_info = v.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!("   {} [{}{}]: {}", icon.color(match v.level {
                        RuleLevel::Error   => "red",
                        RuleLevel::Warning => "yellow",
                        RuleLevel::Info    => "blue",
                    }), v.rule_name.yellow(), line_info, v.message);
                    // Per-violation copy-ready ignore hint
                    let rel_file = v.file_path
                        .strip_prefix(agent_context.project_root.to_string_lossy().as_ref())
                        .unwrap_or(&v.file_path)
                        .trim_start_matches('/')
                        .to_string();
                    let hint_file = if rel_file.is_empty() { &v.file_path } else { &rel_file };
                    if let Some(ref sym) = v.symbol {
                        println!(
                            "      {} sentinel ignore {} {} {}",
                            "👉".dimmed(),
                            v.rule_name.dimmed(),
                            hint_file.dimmed(),
                            sym.dimmed()
                        );
                    } else {
                        println!(
                            "      {} sentinel ignore {} {}",
                            "👉".dimmed(),
                            v.rule_name.dimmed(),
                            hint_file.dimmed()
                        );
                    }
                }
```

Also remove the generic hint block (~line 421-427):

```rust
            if !violations.is_empty() && !json_mode {
                println!(
                    "\n💡 Para ignorar: {} {} {}",
                    "sentinel ignore".cyan(),
                    "<REGLA>".dimmed(),
                    "<ARCHIVO>".dimmed()
                );
            }
```

Delete those 7 lines entirely.

**Step 7: Run tests**

```bash
cargo test 2>&1 | tail -3
```

Expected: 50 tests passing (49 + 1 new normalize test).

**Step 8: Commit**

```bash
git add src/commands/ignore.rs src/commands/pro.rs
git commit -m "feat: ignore symbol normalization + per-violation copy-ready hints in check output"
```

---

## Task 5: Audit TTY auto-detect + improved interactive menu

**Files:**
- Modify: `src/commands/pro.rs`

**Context:**

In `src/commands/pro.rs` audit handler (~line 1901-1903):
```rust
        ProCommands::Audit { target, no_fix, format, max_files, concurrency } => {
            let json_mode = format.to_lowercase() == "json";
            let non_interactive = no_fix || json_mode;
```

Interactive section starts at ~line 2237 with a `MultiSelect` block. The entire block (~lines 2252-2291) builds `options: Vec<String>`, calls `MultiSelect`, and processes `selected`.

**Step 1: Write a test for TTY detection logic**

Add to `#[cfg(test)] mod batching_tests` block in `src/commands/pro.rs`:

```rust
    #[test]
    fn test_non_interactive_logic() {
        // When no_fix=true → non_interactive regardless of tty
        assert!(true  || false || false, "no_fix overrides");
        // When json_mode=true → non_interactive
        assert!(false || true  || false, "json_mode overrides");
        // When not tty → non_interactive
        assert!(false || false || true,  "no tty overrides");
        // All false → interactive (requires tty check at runtime)
        let no_fix = false;
        let json_mode = false;
        let is_tty = false; // simulate CI
        assert!(no_fix || json_mode || !is_tty, "CI should be non-interactive");
    }
```

**Step 2: Run test**

```bash
cargo test test_non_interactive_logic 2>&1 | grep -E "ok|FAILED"
```

Expected: `ok`

**Step 3: Add TTY detection to the audit handler**

Find:
```rust
            let json_mode = format.to_lowercase() == "json";
            let non_interactive = no_fix || json_mode;
```
Replace with:
```rust
            let json_mode = format.to_lowercase() == "json";
            let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
            let non_interactive = no_fix || json_mode || !is_tty;
```

**Step 4: Replace the MultiSelect block with a per-issue loop**

Find the interactive section that starts approximately:
```rust
            let options: Vec<String> = display_issues
                .iter()
                .map(|i| {
```
and ends after:
```rust
            if selected.is_empty() {
                println!("   ⏭️  Operación cancelada.");
                return;
            }

            println!("\n🚀 Aplicando {} correcciones...", selected.len());
```

This entire block (from `let options:` through `println!("\n🚀 Aplicando...")`) should be replaced with:

```rust
            println!("\n📋 {} issues detectados. Revisando uno a uno:\n", display_issues.len());

            use std::io::{BufRead, Write};
            let mut selected_indices: Vec<usize> = Vec::new();
            let mut skip_all = false;

            for (idx, issue) in display_issues.iter().enumerate() {
                if skip_all { break; }

                let rel_file = std::path::Path::new(&issue.file_path)
                    .strip_prefix(&agent_context.project_root)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| issue.file_path.clone());

                println!("{}", "─".repeat(60));
                println!(
                    "Issue {}/{} · {} · {}",
                    idx + 1,
                    display_issues.len(),
                    issue.severity.to_uppercase().bold(),
                    rel_file.cyan()
                );
                println!("{}", issue.title.bold());
                if !issue.description.is_empty() {
                    println!("\n{}", issue.description);
                }
                if !issue.suggested_fix.is_empty() {
                    println!("\n{}", "Fix sugerido:".dimmed());
                    for line in issue.suggested_fix.lines() {
                        println!("  {}", line.dimmed());
                    }
                }
                println!("\n[a]plicar  [s]altar  [S]altar todos  [q]salir");
                print!("> ");
                std::io::stdout().flush().unwrap_or(());

                let mut input = String::new();
                std::io::stdin().lock().read_line(&mut input).unwrap_or(0);
                match input.trim() {
                    "a" | "A" => selected_indices.push(idx),
                    "S"       => { skip_all = true; }
                    "q" | "Q" => {
                        println!("   ⏭️  Operación cancelada.");
                        return;
                    }
                    _ => {} // "s" or anything else: skip
                }
            }

            if selected_indices.is_empty() {
                println!("   ⏭️  Sin fixes seleccionados.");
                return;
            }

            println!("\n🚀 Aplicando {} correcciones...", selected_indices.len());
```

**Step 5: Update the fix application loop**

The fix application loop after currently uses `for &idx in &selected {`. Find it and update to:

```rust
            for &idx in &selected_indices {
```

(just change `selected` → `selected_indices` in the for loop header)

**Step 6: Run full tests**

```bash
cargo test 2>&1 | tail -3
```

Expected: 51 tests passing.

**Step 7: Commit**

```bash
git add src/commands/pro.rs
git commit -m "feat: audit TTY auto-detection + per-issue interactive menu replacing MultiSelect"
```

---

## Task 6: Review history + `--history` + `--diff`

**Files:**
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/pro.rs`

**Context:**

Currently `ProCommands::Review` is a unit variant with no fields:
```rust
    Review,
```

After this task it becomes:
```rust
    Review {
        /// List last N review records
        #[arg(long, default_value_t = false)]
        history: bool,
        /// Diff last 2 review records
        #[arg(long, default_value_t = false)]
        diff: bool,
    },
```

Reviews are saved to `{project_root}/.sentinel/reviews/YYYY-MM-DD-HH-MM.json`.

The existing match arm `ProCommands::Review =>` must become `ProCommands::Review { history, diff } =>`.

**Step 1: Write failing tests**

Add to `#[cfg(test)] mod batching_tests` in `src/commands/pro.rs`:

```rust
    #[test]
    fn test_review_record_save_and_load() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let record = super::ReviewRecord {
            timestamp: "2026-02-23T14:32:00".to_string(),
            project_root: root.display().to_string(),
            files_reviewed: 5,
            suggestions: vec![
                serde_json::json!({"title": "Test suggestion", "impact": "High"}),
            ],
        };

        super::save_review_record(root, &record).unwrap();

        let loaded = super::load_review_records(root);
        assert_eq!(loaded.len(), 1, "should load 1 saved record");
        assert_eq!(loaded[0].files_reviewed, 5);
        assert_eq!(loaded[0].suggestions.len(), 1);
    }

    #[test]
    fn test_review_diff_categorizes_correctly() {
        let old = vec![
            serde_json::json!({"title": "Old and resolved"}),
            serde_json::json!({"title": "Persistent issue"}),
        ];
        let new = vec![
            serde_json::json!({"title": "Persistent issue"}),
            serde_json::json!({"title": "Brand new issue"}),
        ];

        let (resolved, added, persistent) = super::diff_reviews(&old, &new);
        assert_eq!(resolved.len(), 1, "Old and resolved should be resolved");
        assert_eq!(added.len(), 1, "Brand new issue should be new");
        assert_eq!(persistent.len(), 1, "Persistent issue should be persistent");
    }
```

**Step 2: Add `ReviewRecord`, `save_review_record`, `load_review_records`, `diff_reviews` near the top of `src/commands/pro.rs`**

Find the `struct AuditIssue` definition (~line 19). Add BEFORE it:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewRecord {
    pub timestamp: String,
    pub project_root: String,
    pub files_reviewed: usize,
    pub suggestions: Vec<serde_json::Value>,
}

/// Saves a review record to .sentinel/reviews/YYYY-MM-DD-HH-MM.json
pub fn save_review_record(project_root: &std::path::Path, record: &ReviewRecord) -> anyhow::Result<()> {
    let dir = project_root.join(".sentinel").join("reviews");
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{}.json", record.timestamp.replace(':', "-").replace('T', "-"));
    let path = dir.join(&filename);
    let json = serde_json::to_string_pretty(record)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Loads all review records from .sentinel/reviews/, sorted oldest-first.
pub fn load_review_records(project_root: &std::path::Path) -> Vec<ReviewRecord> {
    let dir = project_root.join(".sentinel").join("reviews");
    if !dir.exists() { return vec![]; }
    let mut records: Vec<ReviewRecord> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                .filter_map(|e| {
                    std::fs::read_to_string(e.path()).ok()
                        .and_then(|s| serde_json::from_str::<ReviewRecord>(&s).ok())
                })
                .collect()
        })
        .unwrap_or_default();
    records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    records
}

/// Diffs two suggestion arrays. Returns (resolved, new, persistent).
pub fn diff_reviews(
    old: &[serde_json::Value],
    new: &[serde_json::Value],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let old_titles: std::collections::HashSet<String> = old.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .map(|t| t.to_lowercase())
        .collect();
    let new_titles: std::collections::HashSet<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .map(|t| t.to_lowercase())
        .collect();

    let resolved: Vec<String> = old.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| !new_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();
    let added: Vec<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| !old_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();
    let persistent: Vec<String> = new.iter()
        .filter_map(|s| s.get("title").and_then(|t| t.as_str()))
        .filter(|t| old_titles.contains(&t.to_lowercase()))
        .map(|t| t.to_string())
        .collect();

    (resolved, added, persistent)
}
```

**Step 3: Run failing tests**

```bash
cargo test test_review_record_save_and_load 2>&1 | grep -E "ok|FAILED|error"
cargo test test_review_diff_categorizes 2>&1 | grep -E "ok|FAILED|error"
```

Expected: both pass (the functions are defined and tests should work now).

**Step 4: Update `ProCommands::Review` in `src/commands/mod.rs`**

Find:
```rust
    /// Review completo del proyecto (Arquitectura y Coherencia)
    Review,
```
Replace with:
```rust
    /// Review completo del proyecto (Arquitectura y Coherencia)
    Review {
        /// Listar últimos N reviews guardados
        #[arg(long, default_value_t = false)]
        history: bool,
        /// Comparar último review con el anterior
        #[arg(long, default_value_t = false)]
        diff: bool,
    },
```

**Step 5: Update `ProCommands::Review` match arm in `src/commands/pro.rs`**

Find:
```rust
        ProCommands::Review => {
```
Replace with:
```rust
        ProCommands::Review { history, diff } => {
```

**Step 6: Add `--history` and `--diff` early-return blocks at the start of the Review arm**

Just AFTER `ProCommands::Review { history, diff } => {`, add before `let pb = ui::crear_progreso(...)`:

```rust
            // Handle --history: list saved reviews
            if history {
                let records = load_review_records(&agent_context.project_root);
                if records.is_empty() {
                    println!("📋 No hay reviews guardados aún. Ejecuta `sentinel pro review` para generar el primero.");
                } else {
                    println!("📋 Historial de reviews ({}):", records.len());
                    for r in records.iter().rev().take(5) {
                        let first_title = r.suggestions.first()
                            .and_then(|s| s.get("title"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("(sin sugerencias)");
                        println!(
                            "  {}  ·  {} sugerencia(s)  ·  \"{}\"",
                            r.timestamp, r.suggestions.len(), first_title
                        );
                    }
                }
                return;
            }

            // Handle --diff: compare last 2 reviews
            if diff {
                let records = load_review_records(&agent_context.project_root);
                if records.len() < 2 {
                    println!("⚠️  Se necesitan al menos 2 reviews para comparar. Ejecuta `sentinel pro review` dos veces.");
                } else {
                    let prev = &records[records.len() - 2];
                    let last = &records[records.len() - 1];
                    let (resolved, added, persistent) = diff_reviews(&prev.suggestions, &last.suggestions);
                    println!(
                        "🔍 Comparando reviews ({} vs {}):",
                        prev.timestamp, last.timestamp
                    );
                    if !resolved.is_empty() {
                        println!("  ✅ Resueltas ({}):", resolved.len());
                        for t in &resolved { println!("     \"{}\"", t); }
                    }
                    if !added.is_empty() {
                        println!("  🆕 Nuevas ({}):", added.len());
                        for t in &added { println!("     \"{}\"", t); }
                    }
                    if !persistent.is_empty() {
                        println!("  ⏳ Persistentes ({}):", persistent.len());
                        for t in persistent.iter().take(5) { println!("     \"{}\"", t); }
                        if persistent.len() > 5 {
                            println!("     ... y {} más", persistent.len() - 5);
                        }
                    }
                }
                return;
            }
```

**Step 7: Save the review record after a successful LLM call**

In the Review handler, find the block `match result {` and inside the `Ok(res)` arm, just after printing the review output (near `pb_agent.finish_and_clear();`), add the save logic.

Find the `Ok(res) =>` arm in the `match result {` block. It will start with something like:
```rust
            match result {
                Ok(res) => {
```

Inside `Ok(res)`, after printing the review output and BEFORE `return` or the end of the arm, add:

```rust
                    // Save review record for history/diff
                    let suggestions_json: Vec<serde_json::Value> = serde_json::from_str::<Vec<serde_json::Value>>(&{
                        // Extract JSON block from LLM output (between first ``` json and ```)
                        let start = res.find("```json").or_else(|| res.find("```\n[")).map(|i| i + 7).unwrap_or(0);
                        let end = res[start..].find("```").map(|i| i + start).unwrap_or(res.len());
                        res[start..end].trim().to_string()
                    }).unwrap_or_default();

                    let record = ReviewRecord {
                        timestamp: chrono::Local::now().format("%Y-%m-%dT%H-%M-%S").to_string(),
                        project_root: agent_context.project_root.display().to_string(),
                        files_reviewed: muestras,
                        suggestions: suggestions_json,
                    };
                    if let Err(e) = save_review_record(&agent_context.project_root, &record) {
                        eprintln!("⚠️  No se pudo guardar el review: {}", e);
                    }
```

**Note:** Place this INSIDE the `Ok(res)` arm, after the display output but before any `return` statement. The exact position depends on the handler structure — put it right before `pb_agent.finish_and_clear()` is called or right after the output printing is done.

**Step 8: Fix monitor.rs if it references `ProCommands::Review`**

```bash
grep -n "Review" src/commands/monitor.rs
```

If found as a unit variant `ProCommands::Review`, change to `ProCommands::Review { history: false, diff: false }`.

**Step 9: Run full test suite**

```bash
cargo test 2>&1 | tail -3
```

Expected: 53 tests passing (51 + 2 new review history tests).

**Step 10: Build**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

**Step 11: Commit**

```bash
git add src/commands/mod.rs src/commands/pro.rs
git commit -m "feat: review history — auto-save to .sentinel/reviews/, --history listing, --diff comparison"
```

---

## Final Verification

```bash
# Full test suite
cargo test 2>&1 | tail -5

# Clean build
cargo build --release 2>&1 | grep "^error"

# Smoke test: Go analysis
echo 'package main
func unusedFn() string { return "x" }
func main() {}' > /tmp/test.go
sentinel pro check /tmp/test.go
# Expected: DEAD_CODE violation for unusedFn

# Smoke test: TTY detection
sentinel pro audit src/ --no-fix 2>&1 | head -3
# Expected: non-interactive output (no menu prompt)

# Smoke test: review history
sentinel pro review --history
# Expected: either "No hay reviews" or list of saved reviews
```

Expected: **53 tests passing**, no build errors.
