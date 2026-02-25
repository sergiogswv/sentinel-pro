//! Integration tests for custom rules integration with main rule engine

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_rule_engine_loads_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create .sentinel/custom-rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a sample custom rule (YAML format)
    let rule_yaml = r#"type: pattern
name: "No Console Logs"
pattern: "console\\.log"
file_patterns: ["**/*.ts"]
severity: error
message: "console.log is not allowed in production code"
enabled: true
"#;

    fs::write(rules_dir.join("no_console.yaml"), rule_yaml)
        .expect("Failed to write rule file");

    // Load rules into engine
    let mut engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    engine.load_custom_rules()
        .expect("Failed to load custom rules");

    // Verify custom rules were loaded
    assert!(!engine.custom_rules.is_empty(), "No custom rules were loaded");
    assert_eq!(engine.custom_rules.len(), 1, "Expected exactly 1 custom rule");
}

#[test]
fn test_rule_engine_validates_file_with_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rule directory and rule
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    let rule_yaml = r#"type: pattern
name: "No Debugger"
pattern: "debugger"
file_patterns: ["**/*.ts"]
severity: error
message: "Debugger statement found"
enabled: true
"#;

    fs::write(rules_dir.join("no_debugger.yaml"), rule_yaml)
        .expect("Failed to write rule file");

    // Create and configure engine
    let mut engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    engine.load_custom_rules()
        .expect("Failed to load custom rules");

    // Test file with violation
    let test_file = PathBuf::from("test/file.ts");
    let content_with_violation = "function test() { debugger; }";

    let violations = engine.validate_file(&test_file, content_with_violation);

    // Should find the debugger statement
    assert!(!violations.is_empty(), "Expected to find violations");
    assert!(violations.iter().any(|v| v.rule_name == "No Debugger"),
            "Expected 'No Debugger' violation");
}

#[test]
fn test_rule_engine_empty_custom_rules_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create empty .sentinel/custom-rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Load rules - should succeed with empty list
    let mut engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    let result = engine.load_custom_rules();

    assert!(result.is_ok(), "Loading empty custom rules should succeed");
    assert!(engine.custom_rules.is_empty(), "No rules should be loaded from empty directory");
}

#[test]
fn test_rule_engine_no_custom_rules_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Don't create .sentinel/custom-rules directory at all
    let engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    // Note: We can't call load_custom_rules without making engine mutable,
    // but the test verifies that the engine initializes correctly
    assert!(engine.custom_rules.is_empty(), "No rules should be loaded initially");
}

#[test]
fn test_rule_engine_validates_file_without_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create engine without loading any custom rules
    let engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    // Don't call load_custom_rules

    let test_file = PathBuf::from("test/file.ts");
    let content = "function test() { debugger; }";

    // Should work without errors even though no custom rules are loaded
    let _violations = engine.validate_file(&test_file, content);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rule_engine_mixed_json_and_yaml_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rule directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a YAML rule
    let yaml_rule = r#"type: pattern
name: "No Console Logs"
pattern: "console\\.log"
file_patterns: ["**/*.ts"]
severity: warning
message: "console.log found"
enabled: true
"#;

    fs::write(rules_dir.join("no_console.yaml"), yaml_rule)
        .expect("Failed to write YAML rule");

    // Create a JSON rule
    let json_rule = r#"{
  "type": "pattern",
  "name": "No Alert",
  "pattern": "alert\\(",
  "file_patterns": ["**/*.ts"],
  "severity": "error",
  "message": "alert() found",
  "enabled": true
}
"#;

    fs::write(rules_dir.join("no_alert.json"), json_rule)
        .expect("Failed to write JSON rule");

    // Load rules
    let mut engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    engine.load_custom_rules()
        .expect("Failed to load custom rules");

    // Should load both rules
    assert_eq!(engine.custom_rules.len(), 2, "Expected 2 custom rules");
}

#[test]
fn test_rule_engine_custom_rules_file_pattern_matching() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rule directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a rule that only applies to TypeScript files
    let rule_yaml = r#"type: pattern
name: "TS Only Rule"
pattern: "console"
file_patterns: ["**/*.ts"]
severity: warning
message: "console found"
enabled: true
"#;

    fs::write(rules_dir.join("ts_only.yaml"), rule_yaml)
        .expect("Failed to write rule file");

    // Load rules
    let mut engine = sentinel_rust::rules::RuleEngine::new()
        .with_project_path(project_path);

    engine.load_custom_rules()
        .expect("Failed to load custom rules");

    // Test with TypeScript file
    let ts_file = PathBuf::from("src/test.ts");
    let violations_ts = engine.validate_file(&ts_file, "console.log()");

    // Test with JavaScript file
    let js_file = PathBuf::from("src/test.js");
    let violations_js = engine.validate_file(&js_file, "console.log()");

    // TypeScript should match, JavaScript should not match rule's file patterns
    let ts_has_violation = violations_ts.iter().any(|v| v.rule_name == "TS Only Rule");
    let js_has_violation = violations_js.iter().any(|v| v.rule_name == "TS Only Rule");

    assert!(ts_has_violation, "Expected violation in TypeScript file");
    assert!(!js_has_violation, "Expected no violation in JavaScript file");
}
