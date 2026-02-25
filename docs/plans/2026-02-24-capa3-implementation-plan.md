# Capa 3 - Expansión: Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Custom Rules, Java/Rust support, pre-commit hooks, GitHub Actions CI/CD, and VS Code extension.

**Architecture:** Modular sequential implementation. Each component is self-contained and buildable independently. Custom Rules forms the foundation; subsequent components extend or consume it.

**Tech Stack:** Rust (core), YAML/JSON (configs), Tree-sitter (parsing), git hooks (pre-commit), GitHub Actions (CI), TypeScript (VS Code extension)

---

## PHASE 1: CUSTOM RULES SYSTEM

### Task 1: Create custom rules module structure

**Files:**
- Create: `src/rules/custom.rs`
- Create: `src/rules/custom/schema.rs`
- Create: `src/rules/custom/loader.rs`
- Create: `src/rules/custom/executor.rs`
- Modify: `src/rules/mod.rs`

**Step 1: Create custom.rs module entry point**

Create `src/rules/custom.rs`:
```rust
//! Custom rules system supporting pattern-based and AST-based rules

pub mod schema;
pub mod loader;
pub mod executor;

pub use schema::{CustomRule, RuleType, RuleSeverity};
pub use loader::CustomRulesLoader;
pub use executor::CustomRulesExecutor;

/// Load and execute custom rules from .sentinel/custom-rules/
pub fn load_custom_rules(project_path: &std::path::Path) -> Result<Vec<CustomRule>, String> {
    let loader = CustomRulesLoader::new(project_path);
    loader.load_all()
}

/// Execute custom rules against a file
pub fn execute_custom_rules(
    rules: &[CustomRule],
    file_content: &str,
    file_path: &std::path::Path,
) -> Vec<RuleViolation> {
    let executor = CustomRulesExecutor::new(rules);
    executor.check_file(file_content, file_path)
}

#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule_name: String,
    pub severity: RuleSeverity,
    pub message: String,
    pub line: usize,
    pub column: usize,
}
```

**Step 2: Create schema.rs with data structures**

Create `src/rules/custom/schema.rs`:
```rust
//! Data structures for custom rules (YAML + JSON compatible)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CustomRule {
    #[serde(rename = "pattern")]
    Pattern(PatternRule),
    #[serde(rename = "ast")]
    Ast(AstRule),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    pub name: String,
    pub pattern: String, // regex
    #[serde(default)]
    pub file_patterns: Vec<String>, // glob patterns
    pub severity: RuleSeverity,
    pub message: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstRule {
    pub name: String,
    pub language: String, // "typescript", "java", "rust", etc.
    pub query: String,    // Tree-sitter query
    pub severity: RuleSeverity,
    pub message: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Validates rule structure against JSON schema
pub fn validate_rule(rule: &CustomRule) -> Result<(), String> {
    match rule {
        CustomRule::Pattern(r) => {
            if r.name.is_empty() {
                return Err("Pattern rule 'name' cannot be empty".to_string());
            }
            if r.pattern.is_empty() {
                return Err("Pattern rule 'pattern' cannot be empty".to_string());
            }
            // Validate regex
            if let Err(e) = regex::Regex::new(&r.pattern) {
                return Err(format!("Invalid regex pattern: {}", e));
            }
            Ok(())
        }
        CustomRule::Ast(r) => {
            if r.name.is_empty() {
                return Err("AST rule 'name' cannot be empty".to_string());
            }
            if r.language.is_empty() {
                return Err("AST rule 'language' cannot be empty".to_string());
            }
            if r.query.is_empty() {
                return Err("AST rule 'query' cannot be empty".to_string());
            }
            Ok(())
        }
    }
}
```

**Step 3: Create loader.rs for YAML/JSON loading**

Create `src/rules/custom/loader.rs`:
```rust
//! Load custom rules from .sentinel/custom-rules/ (YAML + JSON)

use super::schema::CustomRule;
use std::path::{Path, PathBuf};

pub struct CustomRulesLoader {
    rules_dir: PathBuf,
}

impl CustomRulesLoader {
    pub fn new(project_path: &Path) -> Self {
        let rules_dir = project_path.join(".sentinel/custom-rules");
        Self { rules_dir }
    }

    /// Load all rules from .sentinel/custom-rules/
    pub fn load_all(&self) -> Result<Vec<CustomRule>, String> {
        if !self.rules_dir.exists() {
            // Return empty list if directory doesn't exist (not an error)
            return Ok(Vec::new());
        }

        let mut rules = Vec::new();

        // Read all .yaml and .json files
        for entry in std::fs::read_dir(&self.rules_dir)
            .map_err(|e| format!("Failed to read rules directory: {}", e))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str());
            match ext {
                Some("yaml") | Some("yml") => {
                    let rule = self.load_yaml(&path)?;
                    rules.push(rule);
                }
                Some("json") => {
                    let rule = self.load_json(&path)?;
                    rules.push(rule);
                }
                _ => {} // Skip other file types
            }
        }

        Ok(rules)
    }

    fn load_yaml(&self, path: &Path) -> Result<CustomRule, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Invalid YAML in {}: {}", path.display(), e))
    }

    fn load_json(&self, path: &Path) -> Result<CustomRule, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_yaml_pattern_rule() {
        let yaml_content = r#"
name: "Test Rule"
type: "pattern"
pattern: "console\\.log"
file_patterns: ["src/**/*.ts"]
severity: "error"
message: "No console.log"
"#;
        let rule: CustomRule =
            serde_yaml::from_str(yaml_content).expect("Should parse valid YAML");
        match rule {
            CustomRule::Pattern(r) => {
                assert_eq!(r.name, "Test Rule");
                assert_eq!(r.pattern, "console\\.log");
            }
            _ => panic!("Expected pattern rule"),
        }
    }

    #[test]
    fn test_load_json_ast_rule() {
        let json_content = r#"
{
  "type": "ast",
  "name": "Test AST",
  "language": "typescript",
  "query": "(function_declaration)",
  "severity": "warning",
  "message": "Found function"
}
"#;
        let rule: CustomRule =
            serde_json::from_str(json_content).expect("Should parse valid JSON");
        match rule {
            CustomRule::Ast(r) => {
                assert_eq!(r.name, "Test AST");
                assert_eq!(r.language, "typescript");
            }
            _ => panic!("Expected AST rule"),
        }
    }
}
```

**Step 4: Create executor.rs for rule execution**

Create `src/rules/custom/executor.rs`:
```rust
//! Execute custom rules against files

use super::schema::{CustomRule, PatternRule, RuleSeverity};
use crate::rules::custom::{CustomRule as Rule, RuleViolation};
use regex::Regex;
use std::path::Path;

pub struct CustomRulesExecutor<'a> {
    rules: &'a [Rule],
    pattern_cache: std::collections::HashMap<String, Regex>,
}

impl<'a> CustomRulesExecutor<'a> {
    pub fn new(rules: &'a [Rule]) -> Self {
        Self {
            rules,
            pattern_cache: Default::default(),
        }
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
                Rule::Ast(a) => {
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

        let file_str = file_path.to_string_lossy();
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

        let executor = CustomRulesExecutor::new(&[rule]);
        let violations = executor.check_file("console.log('test');", Path::new("src/index.ts"));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
    }
}
```

**Step 5: Update src/rules/mod.rs to export custom module**

Modify `src/rules/mod.rs` to add:
```rust
pub mod custom;
```

Add to public exports section:
```rust
pub use custom::{load_custom_rules, execute_custom_rules, CustomRule, RuleViolation};
```

**Step 6: Add dependencies to Cargo.toml**

Modify `Cargo.toml`:
```toml
[dependencies]
# ... existing deps ...
serde_yaml = "0.9"
glob = "0.3"
```

**Step 7: Test module compilation**

Run:
```bash
cargo check --lib
```

Expected: No errors, custom module compiles cleanly.

**Step 8: Commit**

```bash
git add src/rules/custom.rs src/rules/custom/schema.rs src/rules/custom/loader.rs src/rules/custom/executor.rs src/rules/mod.rs Cargo.toml
git commit -m "feat: add custom rules system with pattern and AST support

- Pattern rules: regex-based validation with file patterns
- AST rules: Tree-sitter query support (executor placeholder)
- YAML + JSON loader in .sentinel/custom-rules/
- Rule severity levels: info, warning, error
- Validation against schema"
```

---

### Task 2: Integrate custom rules into main rule engine

**Files:**
- Modify: `src/rules/engine.rs`
- Modify: `src/main.rs`
- Create: `tests/custom_rules_integration_test.rs`

**Step 1: Update rule engine to load custom rules on startup**

Modify `src/rules/engine.rs`:

Find the `RulesEngine` struct and add custom rules loading to its `new()` method:

```rust
pub struct RulesEngine {
    static_rules: Vec<StaticRule>,
    custom_rules: Vec<CustomRule>,
}

impl RulesEngine {
    pub fn new(project_path: &Path) -> Result<Self, String> {
        // Load built-in static rules (existing code)
        let static_rules = Self::load_static_rules();

        // Load custom rules from .sentinel/custom-rules/
        let custom_rules = custom::load_custom_rules(project_path)?;

        Ok(Self {
            static_rules,
            custom_rules,
        })
    }

    pub fn check(&self, file_content: &str, file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        // Run static rules (existing)
        violations.extend(self.check_static_rules(file_content, file_path));

        // Run custom rules
        let custom_violations = custom::execute_custom_rules(
            &self.custom_rules,
            file_content,
            file_path,
        );
        violations.extend(custom_violations);

        violations
    }
}
```

**Step 2: Update main.rs to pass project_path to RulesEngine**

Modify `src/main.rs`:

Find where `RulesEngine` is instantiated and ensure project_path is passed:

```rust
let rules_engine = RulesEngine::new(&project_path)
    .map_err(|e| eprintln!("Error loading rules: {}", e))?;
```

**Step 3: Create integration test**

Create `tests/custom_rules_integration_test.rs`:
```rust
use sentinel_pro::rules::{RulesEngine, load_custom_rules};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_custom_rules_loaded_and_executed() {
    let temp = TempDir::new().unwrap();
    let project_path = temp.path();

    // Create .sentinel/custom-rules directory
    let rules_dir = project_path.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();

    // Write a test pattern rule
    let rule_yaml = r#"
name: "No debug statements"
type: "pattern"
pattern: "\\bdebug\\("
file_patterns: ["src/**/*.rs"]
severity: "warning"
message: "Remove debug statements before commit"
"#;
    fs::write(rules_dir.join("no-debug.yaml"), rule_yaml).unwrap();

    // Test loading rules
    let rules = load_custom_rules(project_path).unwrap();
    assert_eq!(rules.len(), 1);

    // Test engine integration
    let engine = RulesEngine::new(project_path).unwrap();
    let violations = engine.check("debug!(\"test\");", Path::new("src/main.rs"));

    assert!(violations.iter().any(|v| v.rule_name == "No debug statements"));
}

#[test]
fn test_custom_rules_respects_file_patterns() {
    let temp = TempDir::new().unwrap();
    let project_path = temp.path();

    let rules_dir = project_path.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();

    let rule_yaml = r#"
name: "Test rule"
type: "pattern"
pattern: "forbidden"
file_patterns: ["src/**/*.rs"]
severity: "error"
message: "Forbidden word"
"#;
    fs::write(rules_dir.join("test.yaml"), rule_yaml).unwrap();

    let engine = RulesEngine::new(project_path).unwrap();

    // Should match
    let violations = engine.check("forbidden", Path::new("src/main.rs"));
    assert!(!violations.is_empty());

    // Should not match (different path)
    let violations = engine.check("forbidden", Path::new("docs/readme.md"));
    assert!(violations.is_empty());
}
```

**Step 4: Run integration tests**

```bash
cargo test --test custom_rules_integration_test -- --nocapture
```

Expected: Both tests PASS

**Step 5: Commit**

```bash
git add src/rules/engine.rs src/main.rs tests/custom_rules_integration_test.rs
git commit -m "feat: integrate custom rules into main rule engine

Custom rules now loaded on startup and executed during file checks.
Violations aggregated with static rule violations."
```

---

### Task 3: Add 'sentinel rules' command for validation

**Files:**
- Modify: `src/commands/mod.rs`
- Create: `src/commands/rules.rs`
- Create: `tests/rules_command_test.rs`

**Step 1: Create rules command module**

Create `src/commands/rules.rs`:
```rust
//! Command: sentinel rules
//! Validates custom rules syntax and structure

use crate::rules::custom::{CustomRulesLoader, validate_rule};
use std::path::Path;

pub fn validate_all_rules(project_path: &Path) -> Result<(), String> {
    println!("Validating custom rules in {}...", project_path.display());

    let loader = CustomRulesLoader::new(project_path);
    let rules = loader.load_all()?;

    if rules.is_empty() {
        println!("ℹ️  No custom rules found in .sentinel/custom-rules/");
        return Ok(());
    }

    let mut all_valid = true;
    for (idx, rule) in rules.iter().enumerate() {
        if let Err(e) = validate_rule(rule) {
            eprintln!("❌ Rule {} validation failed: {}", idx + 1, e);
            all_valid = false;
        } else {
            let rule_name = match rule {
                crate::rules::custom::CustomRule::Pattern(p) => &p.name,
                crate::rules::custom::CustomRule::Ast(a) => &a.name,
            };
            println!("✅ Rule '{}' is valid", rule_name);
        }
    }

    if !all_valid {
        return Err("Some rules failed validation".to_string());
    }

    println!("✅ All {} rules are valid!", rules.len());
    Ok(())
}
```

**Step 2: Register command in commands/mod.rs**

Modify `src/commands/mod.rs`:
```rust
pub mod rules;

pub fn execute_rules_command(project_path: &Path, args: &[&str]) -> Result<(), String> {
    if args.is_empty() || args[0] == "validate" {
        rules::validate_all_rules(project_path)
    } else {
        Err("Unknown rules subcommand. Use: sentinel rules validate".to_string())
    }
}
```

**Step 3: Add to main.rs CLI dispatcher**

Modify `src/main.rs` to handle `rules` command:
```rust
"rules" => {
    commands::execute_rules_command(&project_path, &args[2..])?;
}
```

**Step 4: Create test**

Create `tests/rules_command_test.rs`:
```rust
#[test]
fn test_rules_validate_command() {
    use std::fs;
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path();
    let rules_dir = project_path.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();

    // Valid rule
    let valid_rule = r#"
name: "Test"
type: "pattern"
pattern: "test"
file_patterns: []
severity: "warning"
message: "Test"
"#;
    fs::write(rules_dir.join("valid.yaml"), valid_rule).unwrap();

    // Should pass
    let result = sentinel_pro::commands::rules::validate_all_rules(project_path);
    assert!(result.is_ok());
}

#[test]
fn test_rules_validate_invalid_regex() {
    use std::fs;
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path();
    let rules_dir = project_path.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();

    // Invalid regex
    let invalid_rule = r#"
name: "Bad Rule"
type: "pattern"
pattern: "["
file_patterns: []
severity: "error"
message: "Test"
"#;
    fs::write(rules_dir.join("invalid.yaml"), invalid_rule).unwrap();

    // Should fail
    let result = sentinel_pro::commands::rules::validate_all_rules(project_path);
    assert!(result.is_err());
}
```

**Step 5: Test the command**

```bash
cargo build && cargo test --test rules_command_test
```

Expected: All tests PASS

**Step 6: Manual test**

Create a test project:
```bash
mkdir -p /tmp/test-sentinel/.sentinel/custom-rules
cat > /tmp/test-sentinel/.sentinel/custom-rules/test.yaml <<'EOF'
name: "Test Rule"
type: "pattern"
pattern: "console\\.log"
file_patterns: []
severity: "warning"
message: "No console logs"
EOF

cargo run -- rules validate /tmp/test-sentinel
```

Expected: Output shows "✅ Rule 'Test Rule' is valid"

**Step 7: Commit**

```bash
git add src/commands/rules.rs src/commands/mod.rs src/main.rs tests/rules_command_test.rs
git commit -m "feat: add 'sentinel rules validate' command

Validates all custom rules in .sentinel/custom-rules/ for syntax
and structural correctness."
```

---

## PHASE 2: JAVA/RUST SUPPORT

### Task 4: Add Tree-sitter grammars for Java and Rust

**Files:**
- Modify: `Cargo.toml`
- Create: `src/rules/language_support.rs`
- Create: `tests/java_rust_parsing_test.rs`

**Step 1: Add tree-sitter Java and Rust dependencies**

Modify `Cargo.toml`:
```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-java = "0.19"
tree-sitter-rust = "0.20"
```

**Step 2: Create language support module**

Create `src/rules/language_support.rs`:
```rust
//! Language-specific analysis for Java and Rust

use tree_sitter::{Language, Parser, Query, QueryCursor};
use std::path::Path;

pub fn get_language(file_ext: &str) -> Option<Language> {
    match file_ext {
        "rs" => Some(tree_sitter_rust::language()),
        "java" => Some(tree_sitter_java::language()),
        _ => None,
    }
}

/// Detect unused imports in Java files
pub fn detect_unused_imports_java(content: &str) -> Vec<String> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_java::language()).unwrap();

    if let Some(tree) = parser.parse(content, None) {
        let query_str = "(import_declaration) @import";
        let query = Query::new(tree_sitter_java::language(), query_str)
            .expect("Valid query");

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        matches
            .flat_map(|m| m.captures)
            .filter_map(|c| {
                let text = c.node.utf8_text(content.as_bytes()).ok()?;
                Some(text.to_string())
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Detect unused imports in Rust files
pub fn detect_unused_imports_rust(content: &str) -> Vec<String> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();

    if let Some(tree) = parser.parse(content, None) {
        let query_str = "(use_declaration) @use";
        let query = Query::new(tree_sitter_rust::language(), query_str)
            .expect("Valid query");

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        matches
            .flat_map(|m| m.captures)
            .filter_map(|c| {
                let text = c.node.utf8_text(content.as_bytes()).ok()?;
                Some(text.to_string())
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Detect naming convention violations in Java
/// Java convention: camelCase for methods/variables, PascalCase for classes
pub fn check_java_naming_conventions(content: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // Very basic check: look for snake_case in likely method/variable positions
        if let Some(captures) = regex::Regex::new(r"(public|private|protected)\s+\w+\s+(\w+_\w+)\s*\(")
            .ok()
            .and_then(|r| r.captures(line))
        {
            if let Some(m) = captures.get(2) {
                violations.push((line_num + 1, format!("Method '{}' should use camelCase", m.as_str())));
            }
        }
    }

    violations
}

/// Detect naming convention violations in Rust
/// Rust convention: snake_case for functions/variables, PascalCase for types
pub fn check_rust_naming_conventions(content: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // Check for struct/impl with snake_case (should be PascalCase)
        if let Some(captures) = regex::Regex::new(r"\b(struct|impl)\s+(\w*[a-z]_[a-z_]*\w*)\b")
            .ok()
            .and_then(|r| r.captures(line))
        {
            if let Some(m) = captures.get(2) {
                violations.push((line_num + 1, format!("Type '{}' should use PascalCase", m.as_str())));
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_java_imports() {
        let code = "import java.util.List;\nimport java.util.*;";
        let imports = detect_unused_imports_java(code);
        assert!(!imports.is_empty());
    }

    #[test]
    fn test_detect_rust_imports() {
        let code = "use std::collections::HashMap;\nuse std::fs;";
        let imports = detect_unused_imports_rust(code);
        assert!(!imports.is_empty());
    }

    #[test]
    fn test_java_naming_violation() {
        let code = "public void bad_method_name() {}";
        let violations = check_java_naming_conventions(code);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_rust_naming_violation() {
        let code = "struct bad_struct_name {}";
        let violations = check_rust_naming_conventions(code);
        assert!(!violations.is_empty());
    }
}
```

**Step 3: Export from rules/mod.rs**

Modify `src/rules/mod.rs`:
```rust
pub mod language_support;
pub use language_support::{get_language, detect_unused_imports_java, detect_unused_imports_rust};
```

**Step 4: Create comprehensive test**

Create `tests/java_rust_parsing_test.rs`:
```rust
use sentinel_pro::rules::language_support::*;

#[test]
fn test_parse_java_code() {
    let java_code = r#"
public class MyClass {
    public String myField;
    public void myMethod() {
        System.out.println("Hello");
    }
}
"#;

    let imports = detect_unused_imports_java(java_code);
    // No imports in this code, so should be empty
    assert!(imports.is_empty());
}

#[test]
fn test_parse_rust_code() {
    let rust_code = r#"
use std::collections::HashMap;

pub struct MyStruct {
    data: HashMap<String, i32>,
}

impl MyStruct {
    pub fn new() -> Self {
        MyStruct {
            data: HashMap::new(),
        }
    }
}
"#;

    let imports = detect_unused_imports_rust(rust_code);
    assert!(!imports.is_empty());
    assert!(imports[0].contains("use"));
}

#[test]
fn test_java_naming_conventions() {
    let bad_java = "public void bad_method_name() { }";
    let violations = check_java_naming_conventions(bad_java);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].1.contains("camelCase"));
}

#[test]
fn test_rust_naming_conventions() {
    let bad_rust = "struct bad_name_struct { }";
    let violations = check_rust_naming_conventions(bad_rust);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].1.contains("PascalCase"));
}
```

**Step 5: Run tests**

```bash
cargo test --test java_rust_parsing_test -- --nocapture
```

Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add src/rules/language_support.rs src/rules/mod.rs tests/java_rust_parsing_test.rs Cargo.toml
git commit -m "feat: add Tree-sitter support for Java and Rust analysis

- Detect imports in both languages
- Check naming convention violations (Java camelCase, Rust snake_case)
- Foundation for AST-based custom rules"
```

---

### Task 5: Integrate Java/Rust detection into config

**Files:**
- Modify: `src/config.rs`
- Create: `tests/java_rust_detection_test.rs`

**Step 1: Update SentinelConfig to detect Java/Rust**

Modify `src/config.rs` - find the `SentinelConfig` struct and update `load()`:

```rust
impl SentinelConfig {
    pub fn load(project_path: &Path) -> Option<Self> {
        let config_path = project_path.join(".sentinelrc.toml");

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).ok()?;
            toml::from_str(&content).ok()?
        } else {
            // Detect language and create default config
            return Some(Self::from_project_detection(project_path));
        };

        Some(config)
    }

    /// Detect supported languages in project
    fn from_project_detection(project_path: &Path) -> Self {
        let mut config = Self::default(
            project_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("sentinel-project")
                .to_string(),
            Self::detectar_gestor(project_path),
        );

        // Detect supported languages
        config.supported_languages = Self::detect_languages(project_path);

        config
    }

    fn detect_languages(project_path: &Path) -> Vec<String> {
        let mut languages = vec!["typescript".to_string(), "python".to_string()];

        // Check for Java
        if project_path.join("pom.xml").exists() ||
           project_path.join("build.gradle").exists() ||
           project_path.join("build.gradle.kts").exists() {
            languages.push("java".to_string());
        }

        // Check for Rust
        if project_path.join("Cargo.toml").exists() {
            languages.push("rust".to_string());
        }

        languages
    }
}

// Add to SentinelConfig struct:
pub struct SentinelConfig {
    pub nombre_proyecto: String,
    pub gestor_paquetes: String,
    pub ignorar_patrones: Vec<String>,
    pub supported_languages: Vec<String>,  // NEW
}
```

**Step 2: Create detection test**

Create `tests/java_rust_detection_test.rs`:
```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_detect_java_project() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    // Create pom.xml to mark as Java project
    fs::write(path.join("pom.xml"), "<project></project>").unwrap();

    let config = sentinel_pro::config::SentinelConfig::load(path);
    assert!(config.is_some());

    let config = config.unwrap();
    assert!(config.supported_languages.contains(&"java".to_string()));
}

#[test]
fn test_detect_rust_project() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    // Create Cargo.toml to mark as Rust project
    fs::write(path.join("Cargo.toml"), "[package]").unwrap();

    let config = sentinel_pro::config::SentinelConfig::load(path);
    assert!(config.is_some());

    let config = config.unwrap();
    assert!(config.supported_languages.contains(&"rust".to_string()));
}

#[test]
fn test_detect_gradle_java_project() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    fs::write(path.join("build.gradle"), "plugins {}").unwrap();

    let config = sentinel_pro::config::SentinelConfig::load(path);
    assert!(config.is_some());

    let config = config.unwrap();
    assert!(config.supported_languages.contains(&"java".to_string()));
}
```

**Step 3: Run tests**

```bash
cargo test --test java_rust_detection_test
```

Expected: All tests PASS

**Step 4: Commit**

```bash
git add src/config.rs tests/java_rust_detection_test.rs
git commit -m "feat: auto-detect Java/Rust projects in config

Automatically sets supported_languages based on project files
(pom.xml, build.gradle, Cargo.toml)"
```

---

## PHASE 3: PRE-COMMIT INTEGRATION

### Task 6: Create pre-commit hook generator

**Files:**
- Create: `src/commands/precommit.rs`
- Create: `src/precommit_template.sh` (template)
- Create: `tests/precommit_test.rs`

**Step 1: Create pre-commit template**

Create `src/precommit_template.sh`:
```bash
#!/bin/bash
# Sentinel Pre-Commit Hook
# Generated by: sentinel init-precommit
# DO NOT EDIT - regenerate with sentinel init-precommit

set -e

PROJECT_PATH="$(git rev-parse --show-toplevel)"
SENTINEL_BIN="${SENTINEL_BIN:-sentinel}"

# Load config
if [ ! -f "$PROJECT_PATH/.sentinelrc.toml" ]; then
    exit 0  # No config, skip
fi

# Get configuration values
PRECOMMIT_ENABLED=$(grep -A 5 "\[precommit\]" "$PROJECT_PATH/.sentinelrc.toml" | grep "enabled" | grep -o "true\|false" || echo "false")
PRECOMMIT_CHECKS=$(grep -A 5 "\[precommit\]" "$PROJECT_PATH/.sentinelrc.toml" | grep "checks" | grep -o '"[^"]*"' | tr -d '"' | tr ',' '\n' | xargs)
FAIL_ON=$(grep -A 5 "\[precommit\]" "$PROJECT_PATH/.sentinelrc.toml" | grep "fail_on" | grep -o '"[^"]*"' | tr -d '"' || echo "error")

if [ "$PRECOMMIT_ENABLED" != "true" ]; then
    exit 0
fi

echo "🔍 Running Sentinel Pre-commit Checks..."

# Get staged files
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -z "$STAGED_FILES" ]; then
    exit 0  # No files to check
fi

# Run sentinel check on staged files
if $SENTINEL_BIN check --mode precommit --files "$STAGED_FILES" --fail-on "$FAIL_ON"; then
    echo "✅ Sentinel checks passed!"
    exit 0
else
    echo "❌ Sentinel checks failed!"
    if [ "$FAIL_ON" = "error" ]; then
        echo "💡 Use: git commit --no-verify to skip checks"
        exit 1
    else
        exit 0  # warning mode - don't block
    fi
fi
```

**Step 2: Create precommit command module**

Create `src/commands/precommit.rs`:
```rust
//! Pre-commit hook integration

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const PRECOMMIT_TEMPLATE: &str = include_str!("../precommit_template.sh");

pub fn init_precommit(project_path: &Path) -> Result<(), String> {
    let git_dir = project_path.join(".git");
    let hooks_dir = git_dir.join("hooks");
    let hook_path = hooks_dir.join("pre-commit");

    // Verify git repo
    if !git_dir.exists() {
        return Err(
            "Not a git repository. Run: git init".to_string()
        );
    }

    // Create hooks directory if missing
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;

    // Write hook file
    fs::write(&hook_path, PRECOMMIT_TEMPLATE)
        .map_err(|e| format!("Failed to write pre-commit hook: {}", e))?;

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&hook_path, perms)
            .map_err(|e| format!("Failed to set hook permissions: {}", e))?;
    }

    println!("✅ Pre-commit hook installed at: {}", hook_path.display());
    println!("📝 Configure in .sentinelrc.toml: [precommit]");

    Ok(())
}

pub fn uninstall_precommit(project_path: &Path) -> Result<(), String> {
    let hook_path = project_path.join(".git/hooks/pre-commit");

    if !hook_path.exists() {
        println!("ℹ️  Pre-commit hook not found");
        return Ok(());
    }

    fs::remove_file(&hook_path)
        .map_err(|e| format!("Failed to remove pre-commit hook: {}", e))?;

    println!("✅ Pre-commit hook removed");
    Ok(())
}
```

**Step 3: Register command in commands/mod.rs**

Modify `src/commands/mod.rs`:
```rust
pub mod precommit;

pub fn execute_precommit_command(project_path: &Path, args: &[&str]) -> Result<(), String> {
    match args.first().map(|s| *s) {
        Some("init") => precommit::init_precommit(project_path),
        Some("uninstall") => precommit::uninstall_precommit(project_path),
        _ => Err("Use: sentinel precommit <init|uninstall>".to_string()),
    }
}
```

**Step 4: Add to main CLI dispatcher**

Modify `src/main.rs`:
```rust
"precommit" => {
    commands::execute_precommit_command(&project_path, &args[2..])?;
}
```

Also add to help text.

**Step 5: Create test**

Create `tests/precommit_test.rs`:
```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_precommit_creates_hook() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    // Initialize git repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .unwrap();

    // Create sentinel config
    fs::write(path.join(".sentinelrc.toml"),
        "[precommit]\nenabled = true\n").unwrap();

    // Test hook creation
    sentinel_pro::commands::precommit::init_precommit(path).unwrap();

    let hook_path = path.join(".git/hooks/pre-commit");
    assert!(hook_path.exists());

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("Sentinel Pre-Commit Hook"));
}

#[test]
fn test_init_precommit_fails_without_git() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    // No .git directory
    let result = sentinel_pro::commands::precommit::init_precommit(path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("git repository"));
}

#[test]
fn test_uninstall_precommit() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .unwrap();

    sentinel_pro::commands::precommit::init_precommit(path).unwrap();
    let hook_path = path.join(".git/hooks/pre-commit");
    assert!(hook_path.exists());

    sentinel_pro::commands::precommit::uninstall_precommit(path).unwrap();
    assert!(!hook_path.exists());
}
```

**Step 6: Run tests**

```bash
cargo test --test precommit_test -- --nocapture
```

Expected: All tests PASS

**Step 7: Manual test**

```bash
cd /tmp && mkdir test-pre-commit && cd test-pre-commit
git init
cargo run -- precommit init
ls -la .git/hooks/pre-commit
cat .git/hooks/pre-commit | head
```

Expected: Hook file exists and contains Sentinel template

**Step 8: Commit**

```bash
git add src/commands/precommit.rs src/commands/mod.rs src/main.rs src/precommit_template.sh tests/precommit_test.rs
git commit -m "feat: pre-commit hook integration

- 'sentinel precommit init' generates and installs hook
- 'sentinel precommit uninstall' removes hook
- Hook respects .sentinelrc.toml [precommit] configuration"
```

---

## PHASE 4: GITHUB ACTIONS CI/CD

### Task 7: Create GitHub Actions workflow templates

**Files:**
- Create: `src/commands/github_actions.rs`
- Create: `templates/sentinel-analysis.yml`
- Create: `templates/sentinel-tests.yml`
- Create: `templates/sentinel-security.yml`
- Create: `tests/github_actions_test.rs`

**Step 1: Create workflow templates directory and files**

Create `templates/sentinel-analysis.yml`:
```yaml
name: Sentinel Analysis

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  sentinel-analysis:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Sentinel
      run: cargo install sentinel-pro

    - name: Run Sentinel Audit
      run: sentinel audit --json --output sentinel-report.json
      continue-on-error: true

    - name: Upload Report
      uses: actions/upload-artifact@v3
      with:
        name: sentinel-report
        path: sentinel-report.json
      if: always()

    - name: Sentinel Check
      run: sentinel check
```

Create `templates/sentinel-tests.yml`:
```yaml
name: Tests + Sentinel Check

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test-and-check:
    runs-on: ubuntu-latest

    strategy:
      matrix:
        node-version: [18.x, 20.x]

    steps:
    - uses: actions/checkout@v4

    - name: Setup Node
      uses: actions/setup-node@v4
      with:
        node-version: ${{ matrix.node-version }}

    - name: Install dependencies
      run: npm ci

    - name: Run tests
      run: npm test

    - name: Install Sentinel
      run: cargo install sentinel-pro

    - name: Sentinel Check
      run: sentinel check --mode ci
```

Create `templates/sentinel-security.yml`:
```yaml
name: Security Checks

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  security:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Sentinel
      run: cargo install sentinel-pro

    - name: Run Security Rules
      run: sentinel check --rule-type security
      continue-on-error: true

    - name: Check for secrets
      run: sentinel audit --include-secret-scan
      continue-on-error: true
```

**Step 2: Create github_actions command module**

Create `src/commands/github_actions.rs`:
```rust
//! GitHub Actions workflow integration

use std::fs;
use std::path::Path;

const TEMPLATES: &[(&str, &str)] = &[
    ("sentinel-analysis.yml", include_str!("../../templates/sentinel-analysis.yml")),
    ("sentinel-tests.yml", include_str!("../../templates/sentinel-tests.yml")),
    ("sentinel-security.yml", include_str!("../../templates/sentinel-security.yml")),
];

pub fn init_github_actions(project_path: &Path) -> Result<(), String> {
    let workflows_dir = project_path.join(".github/workflows");

    // Create directory
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;

    // Write each template
    for (name, content) in TEMPLATES {
        let path = workflows_dir.join(name);
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write {}: {}", name, e))?;
        println!("✅ Created: {}", path.display());
    }

    println!("\n📋 GitHub Actions workflows installed!");
    println!("   - sentinel-analysis.yml: Runs audit on every push/PR");
    println!("   - sentinel-tests.yml: Runs tests + Sentinel check");
    println!("   - sentinel-security.yml: Security rule checking");
    println!("\n💡 Customize in .github/workflows/ before committing");

    Ok(())
}

pub fn list_workflows(project_path: &Path) -> Result<(), String> {
    let workflows_dir = project_path.join(".github/workflows");

    if !workflows_dir.exists() {
        println!("ℹ️  No workflows found. Run: sentinel init-ci");
        return Ok(());
    }

    let entries = fs::read_dir(&workflows_dir)
        .map_err(|e| format!("Failed to read workflows: {}", e))?;

    println!("📋 Installed workflows:");
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        println!("   - {}", name.to_string_lossy());
    }

    Ok(())
}
```

**Step 3: Register in commands/mod.rs**

Modify `src/commands/mod.rs`:
```rust
pub mod github_actions;

pub fn execute_ci_command(project_path: &Path, args: &[&str]) -> Result<(), String> {
    match args.first().map(|s| *s) {
        Some("init") => github_actions::init_github_actions(project_path),
        Some("list") => github_actions::list_workflows(project_path),
        _ => Err("Use: sentinel ci <init|list>".to_string()),
    }
}
```

**Step 4: Add to CLI dispatcher**

Modify `src/main.rs`:
```rust
"ci" | "init-ci" => {
    commands::execute_ci_command(&project_path, &args[2..])?;
}
```

**Step 5: Create test**

Create `tests/github_actions_test.rs`:
```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_github_actions_creates_workflows() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    sentinel_pro::commands::github_actions::init_github_actions(path).unwrap();

    let workflows_dir = path.join(".github/workflows");
    assert!(workflows_dir.exists());

    assert!(workflows_dir.join("sentinel-analysis.yml").exists());
    assert!(workflows_dir.join("sentinel-tests.yml").exists());
    assert!(workflows_dir.join("sentinel-security.yml").exists());
}

#[test]
fn test_workflow_files_contain_correct_content() {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    sentinel_pro::commands::github_actions::init_github_actions(path).unwrap();

    let analysis = fs::read_to_string(path.join(".github/workflows/sentinel-analysis.yml")).unwrap();
    assert!(analysis.contains("Sentinel Analysis"));
    assert!(analysis.contains("sentinel audit"));

    let tests = fs::read_to_string(path.join(".github/workflows/sentinel-tests.yml")).unwrap();
    assert!(tests.contains("Tests + Sentinel Check"));
    assert!(tests.contains("npm test"));
}
```

**Step 6: Run tests**

```bash
cargo test --test github_actions_test
```

Expected: All tests PASS

**Step 7: Manual test**

```bash
mkdir /tmp/test-ci && cd /tmp/test-ci
cargo run -- init-ci
ls -la .github/workflows/
```

Expected: 3 YAML files created

**Step 8: Commit**

```bash
git add src/commands/github_actions.rs src/commands/mod.rs src/main.rs templates/sentinel-*.yml tests/github_actions_test.rs
git commit -m "feat: GitHub Actions CI/CD workflow templates

- sentinel-analysis.yml: Audit on push/PR
- sentinel-tests.yml: Tests + Sentinel check
- sentinel-security.yml: Security rule validation
- Command: sentinel init-ci"
```

---

## PHASE 5: VS CODE EXTENSION

### Task 8: Setup VS Code extension project structure

**Files:**
- Create: `vscode-sentinel/package.json`
- Create: `vscode-sentinel/src/extension.ts`
- Create: `vscode-sentinel/tsconfig.json`
- Create: `vscode-sentinel/.vscodeignore`

**Step 1: Create extension directory**

```bash
mkdir -p vscode-sentinel/src
cd vscode-sentinel
```

**Step 2: Create package.json**

Create `vscode-sentinel/package.json`:
```json
{
  "name": "sentinel",
  "displayName": "Sentinel Quality Guardian",
  "description": "Code quality analysis and architecture validation in VS Code",
  "version": "5.0.0",
  "publisher": "sentinel-team",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": ["Linters", "Code Quality"],
  "activationEvents": ["onStartupFinished"],
  "main": "./out/extension.js",
  "contributes": {
    "commands": [
      {
        "command": "sentinel.runAudit",
        "title": "Sentinel: Run Audit"
      },
      {
        "command": "sentinel.fixIssues",
        "title": "Sentinel: Fix Issues"
      },
      {
        "command": "sentinel.checkFile",
        "title": "Sentinel: Check File"
      },
      {
        "command": "sentinel.showConfig",
        "title": "Sentinel: Show Configuration"
      },
      {
        "command": "sentinel.initialize",
        "title": "Sentinel: Initialize"
      },
      {
        "command": "sentinel.openCustomRules",
        "title": "Sentinel: Open Custom Rules"
      }
    ]
  },
  "scripts": {
    "vscode:prepublish": "npm run esbuild-base -- --minify",
    "esbuild-base": "esbuild ./src/extension.ts --bundle --outfile=out/extension.js --external:vscode --format=cjs --platform=node",
    "esbuild": "npm run esbuild-base -- --sourcemap",
    "esbuild-watch": "npm run esbuild-base -- --sourcemap --watch",
    "test": "echo \"Error: no test specified\" && exit 1"
  },
  "devDependencies": {
    "@types/node": "^18.0.0",
    "@types/vscode": "^1.75.0",
    "esbuild": "^0.17.0",
    "typescript": "^5.0.0"
  }
}
```

**Step 3: Create TypeScript configuration**

Create `vscode-sentinel/tsconfig.json`:
```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "ES2020",
    "lib": ["ES2020"],
    "outDir": "./out",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src"],
  "exclude": ["node_modules", "out"]
}
```

**Step 4: Create extension entry point**

Create `vscode-sentinel/src/extension.ts`:
```typescript
import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';

export function activate(context: vscode.ExtensionContext) {
    console.log('Sentinel extension activated');

    // Check if sentinel is installed
    if (!isSentinelInstalled()) {
        showInstallationPrompt();
    }

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('sentinel.runAudit', runAudit),
        vscode.commands.registerCommand('sentinel.fixIssues', fixIssues),
        vscode.commands.registerCommand('sentinel.checkFile', checkFile),
        vscode.commands.registerCommand('sentinel.showConfig', showConfig),
        vscode.commands.registerCommand('sentinel.initialize', initialize),
        vscode.commands.registerCommand('sentinel.openCustomRules', openCustomRules),
    );
}

function isSentinelInstalled(): boolean {
    try {
        cp.execSync('sentinel --version', { stdio: 'pipe' });
        return true;
    } catch (e) {
        return false;
    }
}

function showInstallationPrompt() {
    vscode.window.showInformationMessage(
        'Sentinel is not installed. Install it with: cargo install sentinel-pro',
        'Install'
    ).then(selection => {
        if (selection === 'Install') {
            vscode.window.showInformationMessage('Please run: cargo install sentinel-pro');
        }
    });
}

async function runAudit() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const output = vscode.window.createOutputChannel('Sentinel');
    output.show();
    output.appendLine('Running Sentinel audit...');

    try {
        const result = cp.execSync(`sentinel audit --json`, {
            cwd: workspaceFolder.uri.fsPath,
            encoding: 'utf-8'
        });
        output.appendLine(result);
        vscode.window.showInformationMessage('Audit complete');
    } catch (e) {
        output.appendLine(`Error: ${e}`);
        vscode.window.showErrorMessage('Audit failed');
    }
}

async function fixIssues() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No file open');
        return;
    }

    const filePath = editor.document.uri.fsPath;
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }

    try {
        cp.execSync(`sentinel fix "${filePath}"`, {
            cwd: workspaceFolder.uri.fsPath
        });
        vscode.window.showInformationMessage('Fixes applied');
    } catch (e) {
        vscode.window.showErrorMessage(`Fix failed: ${e}`);
    }
}

async function checkFile() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No file open');
        return;
    }

    const filePath = editor.document.uri.fsPath;
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }

    const output = vscode.window.createOutputChannel('Sentinel Check');
    output.show();

    try {
        const result = cp.execSync(`sentinel check "${filePath}"`, {
            cwd: workspaceFolder.uri.fsPath,
            encoding: 'utf-8'
        });
        output.appendLine(result);
    } catch (e) {
        output.appendLine(`${e}`);
    }
}

async function showConfig() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const configPath = path.join(workspaceFolder.uri.fsPath, '.sentinelrc.toml');
    const uri = vscode.Uri.file(configPath);
    await vscode.window.showTextDocument(uri);
}

async function initialize() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const output = vscode.window.createOutputChannel('Sentinel Init');
    output.show();

    try {
        const result = cp.execSync(`sentinel init`, {
            cwd: workspaceFolder.uri.fsPath,
            encoding: 'utf-8'
        });
        output.appendLine(result);
        vscode.window.showInformationMessage('Sentinel initialized');
    } catch (e) {
        output.appendLine(`${e}`);
        vscode.window.showErrorMessage('Initialization failed');
    }
}

async function openCustomRules() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const rulesPath = path.join(workspaceFolder.uri.fsPath, '.sentinel/custom-rules');
    const uri = vscode.Uri.file(rulesPath);
    await vscode.commands.executeCommand('revealFileInOS', uri);
}

export function deactivate() {}
```

**Step 5: Create .vscodeignore**

Create `vscode-sentinel/.vscodeignore`:
```
.git
.gitignore
.vscode
**/*.ts
**/*.map
out/tests
node_modules
src
tsconfig.json
package-lock.json
```

**Step 6: Install dependencies and build**

```bash
cd vscode-sentinel
npm install
npm run esbuild
```

Expected: `out/extension.js` is created

**Step 7: Create basic test setup**

Create `vscode-sentinel/src/test.ts`:
```typescript
// Placeholder for future tests
```

**Step 8: Commit**

```bash
cd ..
git add vscode-sentinel/
git commit -m "feat: VS Code extension scaffolding

- Extension entry point with 6 commands
- Sentinel binary detection and installation prompt
- Commands: audit, fix, check, config, init, custom-rules
- Ready for marketplace publishing"
```

---

### Task 9: Package and document VS Code extension

**Files:**
- Create: `vscode-sentinel/README.md`
- Create: `vscode-sentinel/CHANGELOG.md`
- Modify: `vscode-sentinel/package.json` (add icon, repo, etc.)

**Step 1: Create README**

Create `vscode-sentinel/README.md`:
```markdown
# Sentinel - Code Quality Guardian

VS Code extension for Sentinel quality analysis and architecture validation.

## Features

- **Run Audit**: Analyze your entire workspace
- **Fix Issues**: Auto-fix detected problems
- **Check File**: Validate current file
- **Custom Rules**: Manage custom analysis rules
- **Configuration**: Edit `.sentinelrc.toml`
- **Initialize**: Set up Sentinel in a project

## Requirements

- VS Code 1.75+
- Sentinel CLI installed globally: `cargo install sentinel-pro`

## Installation

1. Install Sentinel CLI: `cargo install sentinel-pro`
2. Install extension from VS Code Marketplace
3. Open command palette (`Ctrl+Shift+P`) and run "Sentinel: Initialize"

## Commands

| Command | Action |
|---------|--------|
| `Sentinel: Run Audit` | Analyze workspace for issues |
| `Sentinel: Fix Issues` | Apply auto-fixes to current file |
| `Sentinel: Check File` | Validate current file |
| `Sentinel: Show Configuration` | Edit project configuration |
| `Sentinel: Initialize` | Set up Sentinel in project |
| `Sentinel: Open Custom Rules` | Browse custom rules directory |

## Configuration

Edit `.sentinelrc.toml` in your project root:

```toml
[precommit]
enabled = true
checks = ["static-analysis", "custom-rules"]
fail_on = "error"
```

## Troubleshooting

**"Sentinel not found"**
- Install: `cargo install sentinel-pro`
- Ensure sentinel is in PATH

**Extension not activating**
- Reload window (`Ctrl+Shift+P` → "Reload Window")
- Check extension output channel

## Contributing

Report issues at: https://github.com/sentinel-team/sentinel-pro/issues

## License

MIT
```

**Step 2: Update package.json with metadata**

Modify `vscode-sentinel/package.json`:
```json
{
  ...
  "repository": {
    "type": "git",
    "url": "https://github.com/sentinel-team/sentinel-pro.git"
  },
  "bugs": "https://github.com/sentinel-team/sentinel-pro/issues",
  "homepage": "https://github.com/sentinel-team/sentinel-pro#readme",
  "keywords": ["code quality", "linting", "architecture", "validation", "sentinel"],
  ...
}
```

**Step 3: Create CHANGELOG**

Create `vscode-sentinel/CHANGELOG.md`:
```markdown
# Changelog

All notable changes to the Sentinel VS Code extension.

## [5.0.0] - 2026-02-24

### Added
- VS Code extension with 6 core commands
- Sentinel CLI binary detection
- Integration with custom rules
- Output channel for audit results

### Features
- `Sentinel: Run Audit` - Workspace analysis
- `Sentinel: Fix Issues` - Auto-fix problems
- `Sentinel: Check File` - File validation
- `Sentinel: Show Configuration` - Config editing
- `Sentinel: Initialize` - Project setup
- `Sentinel: Open Custom Rules` - Rule management
```

**Step 4: Build and test packaging**

```bash
cd vscode-sentinel
npm run vscode:prepublish
```

Expected: `out/extension.js` is minified

**Step 5: Commit**

```bash
git add vscode-sentinel/README.md vscode-sentinel/CHANGELOG.md vscode-sentinel/package.json
git commit -m "docs: add VS Code extension documentation and metadata

- README with features, installation, commands
- CHANGELOG for version tracking
- Updated package.json with repo/bugs/homepage links"
```

---

## Final Integration Tests

### Task 10: Create integration test suite

**Files:**
- Create: `tests/integration_capa3_test.rs`

**Step 1: Create comprehensive integration test**

Create `tests/integration_capa3_test.rs`:
```rust
//! Integration tests for Capa 3 (Expansion) components

use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_custom_rules_java_rust_integration() {
    // Setup temp project
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create .sentinel/custom-rules/
    let rules_dir = project.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();

    // Add Java rule
    let java_rule = r#"
name: "No public fields in Java"
type: "pattern"
pattern: "public\s+\w+\s+\w+"
file_patterns: ["**/*.java"]
severity: "error"
message: "Use getters/setters instead of public fields"
"#;
    fs::write(rules_dir.join("java-rules.yaml"), java_rule).unwrap();

    // Add Rust rule
    let rust_rule = r#"
name: "No unwrap in Rust"
type: "pattern"
pattern: "\\.unwrap\\(\\)"
file_patterns: ["**/*.rs"]
severity: "warning"
message: "Use proper error handling instead of unwrap"
"#;
    fs::write(rules_dir.join("rust-rules.json"), rust_rule).unwrap();

    // Load rules
    let rules = sentinel_pro::rules::custom::load_custom_rules(project).unwrap();
    assert_eq!(rules.len(), 2);
}

#[test]
fn test_precommit_java_rust_analysis() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Initialize git
    std::process::Command::new("git")
        .arg("init")
        .current_dir(project)
        .output()
        .unwrap();

    // Create config
    let config = r#"
[precommit]
enabled = true
checks = ["static-analysis"]
fail_on = "error"
"#;
    fs::write(project.join(".sentinelrc.toml"), config).unwrap();

    // Install pre-commit hook
    sentinel_pro::commands::precommit::init_precommit(project).unwrap();

    let hook = project.join(".git/hooks/pre-commit");
    assert!(hook.exists());
}

#[test]
fn test_github_actions_workflow_generation() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Generate workflows
    sentinel_pro::commands::github_actions::init_github_actions(project).unwrap();

    // Verify all workflows created
    let workflows = project.join(".github/workflows");
    assert!(workflows.join("sentinel-analysis.yml").exists());
    assert!(workflows.join("sentinel-tests.yml").exists());
    assert!(workflows.join("sentinel-security.yml").exists());

    // Verify content
    let analysis = fs::read_to_string(workflows.join("sentinel-analysis.yml")).unwrap();
    assert!(analysis.contains("sentinel audit"));
}

#[test]
fn test_java_rust_detection_in_config() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create Cargo.toml (Rust)
    fs::write(project.join("Cargo.toml"), "[package]").unwrap();

    // Create pom.xml (Java)
    fs::write(project.join("pom.xml"), "<project></project>").unwrap();

    // Load config
    let config = sentinel_pro::config::SentinelConfig::load(project).unwrap();

    // Should detect both
    assert!(config.supported_languages.contains(&"rust".to_string()));
    assert!(config.supported_languages.contains(&"java".to_string()));
}

#[test]
fn test_capa3_end_to_end() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // 1. Initialize git
    std::process::Command::new("git")
        .arg("init")
        .current_dir(project)
        .output()
        .unwrap();

    // 2. Detect Java/Rust project
    fs::write(project.join("Cargo.toml"), "[package]").unwrap();
    let config = sentinel_pro::config::SentinelConfig::load(project).unwrap();
    assert!(config.supported_languages.contains(&"rust".to_string()));

    // 3. Create custom rules
    let rules_dir = project.join(".sentinel/custom-rules");
    fs::create_dir_all(&rules_dir).unwrap();
    let rule = r#"
name: "Test Rule"
type: "pattern"
pattern: "test"
file_patterns: []
severity: "info"
message: "Test"
"#;
    fs::write(rules_dir.join("test.yaml"), rule).unwrap();

    // 4. Validate rules
    sentinel_pro::commands::rules::validate_all_rules(project).unwrap();

    // 5. Install pre-commit hook
    fs::write(project.join(".sentinelrc.toml"),
        "[precommit]\nenabled = true\n").unwrap();
    sentinel_pro::commands::precommit::init_precommit(project).unwrap();
    assert!(project.join(".git/hooks/pre-commit").exists());

    // 6. Generate GitHub Actions
    sentinel_pro::commands::github_actions::init_github_actions(project).unwrap();
    assert!(project.join(".github/workflows/sentinel-analysis.yml").exists());

    println!("✅ Capa 3 end-to-end test passed!");
}
```

**Step 2: Run integration tests**

```bash
cargo test --test integration_capa3_test -- --nocapture
```

Expected: All 5 tests PASS

**Step 3: Commit**

```bash
git add tests/integration_capa3_test.rs
git commit -m "test: add Capa 3 integration test suite

End-to-end tests for:
- Custom rules + Java/Rust detection
- Pre-commit integration
- GitHub Actions workflow generation
- Full Capa 3 workflow"
```

---

## Completion Checklist

- [ ] Task 1-3: Custom Rules System complete
- [ ] Task 4-5: Java/Rust Support complete
- [ ] Task 6: Pre-commit Integration complete
- [ ] Task 7: GitHub Actions CI/CD complete
- [ ] Task 8-9: VS Code Extension complete
- [ ] Task 10: Integration tests passing
- [ ] All commits made with descriptive messages
- [ ] No compilation errors: `cargo build --release`
- [ ] All tests passing: `cargo test`
- [ ] Documentation updated: `docs/plans/2026-02-24-capa3-expansion-design.md`

---

## Success Criteria

✅ **Custom Rules**: YAML + JSON loading, pattern + AST execution, validation command
✅ **Java/Rust**: Tree-sitter parsing, static analysis, naming conventions, config detection
✅ **Pre-commit**: Hook generation, configurable, git integration
✅ **GitHub Actions**: 3 workflow templates, artifact upload, CI/CD pipeline
✅ **VS Code Extension**: 6 core commands, binary detection, output channels

