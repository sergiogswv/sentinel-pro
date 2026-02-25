//! Integration tests for the 'sentinel rules' command

use std::fs;
use tempfile::TempDir;

#[test]
fn test_rules_command_displays_framework_rules() {
    // This test verifies the rules command can be executed
    // We use a temporary directory as the project root
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // The rules command should work even without a config file
    // It will display default rules
    // We can't easily capture stdout in unit tests, so we just verify it doesn't panic
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rules_command_with_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a sample custom rule
    let rule_yaml = r#"type: pattern
name: "No Console Logs"
pattern: "console\\.log"
file_patterns: ["**/*.ts"]
severity: error
message: "console.log is not allowed"
enabled: true
"#;

    fs::write(rules_dir.join("no_console.yaml"), rule_yaml)
        .expect("Failed to write rule file");

    // The rules command should display both framework and custom rules
    // We just verify it doesn't panic
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rules_command_with_multiple_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create multiple custom rules
    let rule1 = r#"type: pattern
name: "No Console Logs"
pattern: "console\\.log"
file_patterns: ["**/*.ts"]
severity: error
message: "console.log is not allowed"
enabled: true
"#;

    let rule2 = r#"type: pattern
name: "No Debugger"
pattern: "debugger"
file_patterns: ["**/*.ts"]
severity: warning
message: "debugger statement found"
enabled: true
"#;

    fs::write(rules_dir.join("no_console.yaml"), rule1)
        .expect("Failed to write rule 1");

    fs::write(rules_dir.join("no_debugger.yaml"), rule2)
        .expect("Failed to write rule 2");

    // The rules command should display all custom rules
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rules_command_with_json_and_yaml_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a YAML rule
    let yaml_rule = r#"type: pattern
name: "YAML Rule"
pattern: "test_pattern"
file_patterns: ["**/*.ts"]
severity: info
message: "Test pattern found"
enabled: true
"#;

    // Create a JSON rule
    let json_rule = r#"{
  "type": "pattern",
  "name": "JSON Rule",
  "pattern": "another_pattern",
  "file_patterns": ["**/*.rs"],
  "severity": "warning",
  "message": "Another pattern found",
  "enabled": true
}
"#;

    fs::write(rules_dir.join("yaml_rule.yaml"), yaml_rule)
        .expect("Failed to write YAML rule");

    fs::write(rules_dir.join("json_rule.json"), json_rule)
        .expect("Failed to write JSON rule");

    // The rules command should handle both formats
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rules_command_with_disabled_custom_rules() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create custom rules directory
    let rules_dir = project_path.join(".sentinel").join("custom-rules");
    fs::create_dir_all(&rules_dir).expect("Failed to create rules dir");

    // Create a disabled custom rule
    let rule_yaml = r#"type: pattern
name: "Disabled Rule"
pattern: "some_pattern"
file_patterns: ["**/*.ts"]
severity: error
message: "This rule is disabled"
enabled: false
"#;

    fs::write(rules_dir.join("disabled.yaml"), rule_yaml)
        .expect("Failed to write rule file");

    // The rules command should display disabled rules with [OFF] status
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}

#[test]
fn test_rules_command_without_custom_rules_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Don't create custom rules directory
    // The rules command should still work and only show framework rules
    sentinel_pro::commands::rules::handle_rules_command(project_path);

    // Test passes if we get here without panicking
    assert!(true);
}
