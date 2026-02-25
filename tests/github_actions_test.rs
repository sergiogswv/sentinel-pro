//! Integration tests for GitHub Actions workflow templates

#[test]
fn test_analysis_workflow_template_exists() {
    let content = include_str!("../templates/sentinel-analysis.yml");
    assert!(!content.is_empty(), "Analysis workflow template should not be empty");
}

#[test]
fn test_tests_workflow_template_exists() {
    let content = include_str!("../templates/sentinel-tests.yml");
    assert!(!content.is_empty(), "Tests workflow template should not be empty");
}

#[test]
fn test_security_workflow_template_exists() {
    let content = include_str!("../templates/sentinel-security.yml");
    assert!(!content.is_empty(), "Security workflow template should not be empty");
}

#[test]
fn test_analysis_workflow_has_correct_structure() {
    let content = include_str!("../templates/sentinel-analysis.yml");

    assert!(
        content.contains("name: Sentinel Code Analysis"),
        "Should have correct workflow name"
    );
    assert!(content.contains("on:"), "Should have trigger section");
    assert!(
        content.contains("push:"),
        "Should trigger on push"
    );
    assert!(
        content.contains("pull_request:"),
        "Should trigger on pull request"
    );
    assert!(content.contains("jobs:"), "Should have jobs section");
    assert!(
        content.contains("sentinel-analysis:"),
        "Should have analysis job"
    );
}

#[test]
fn test_tests_workflow_has_correct_structure() {
    let content = include_str!("../templates/sentinel-tests.yml");

    assert!(
        content.contains("name: Sentinel Test Execution"),
        "Should have correct workflow name"
    );
    assert!(content.contains("on:"), "Should have trigger section");
    assert!(content.contains("jobs:"), "Should have jobs section");
    assert!(
        content.contains("sentinel-tests:"),
        "Should have tests job"
    );
}

#[test]
fn test_security_workflow_has_correct_structure() {
    let content = include_str!("../templates/sentinel-security.yml");

    assert!(
        content.contains("name: Sentinel Security Scanning"),
        "Should have correct workflow name"
    );
    assert!(content.contains("on:"), "Should have trigger section");
    assert!(
        content.contains("schedule:"),
        "Should have scheduled trigger"
    );
    assert!(
        content.contains("cron:"),
        "Should have cron expression"
    );
    assert!(content.contains("jobs:"), "Should have jobs section");
}

#[test]
fn test_analysis_workflow_references_sentinel() {
    let content = include_str!("../templates/sentinel-analysis.yml");
    assert!(
        content.contains("sentinel init") && content.contains("sentinel pro check"),
        "Should reference Sentinel commands"
    );
}

#[test]
fn test_tests_workflow_references_sentinel() {
    let content = include_str!("../templates/sentinel-tests.yml");
    assert!(
        content.contains("sentinel init") && content.contains("sentinel pro test-all"),
        "Should reference Sentinel commands"
    );
}

#[test]
fn test_security_workflow_references_sentinel() {
    let content = include_str!("../templates/sentinel-security.yml");
    assert!(
        content.contains("sentinel init") && content.contains("sentinel pro audit"),
        "Should reference Sentinel commands"
    );
}

#[test]
fn test_workflows_have_consistent_structure() {
    let analysis = include_str!("../templates/sentinel-analysis.yml");
    let tests = include_str!("../templates/sentinel-tests.yml");
    let security = include_str!("../templates/sentinel-security.yml");

    for workflow in &[analysis, tests, security] {
        assert!(workflow.starts_with("name:"), "Should start with name");
        assert!(workflow.contains("on:"), "Should have trigger");
        assert!(workflow.contains("jobs:"), "Should have jobs");
        assert!(workflow.contains("runs-on:"), "Should specify runner");
        assert!(workflow.contains("steps:"), "Should have steps");
    }
}

#[test]
fn test_analysis_workflow_has_pr_comment() {
    let content = include_str!("../templates/sentinel-analysis.yml");
    assert!(
        content.contains("Comment PR with Results"),
        "Should have PR comment step"
    );
    assert!(
        content.contains("github.rest.issues.createComment"),
        "Should create GitHub comment"
    );
}

#[test]
fn test_security_workflow_has_sarif_upload() {
    let content = include_str!("../templates/sentinel-security.yml");
    assert!(
        content.contains("Upload SARIF to GitHub Security"),
        "Should upload SARIF format"
    );
    assert!(
        content.contains("github/codeql-action/upload-sarif"),
        "Should use GitHub's SARIF upload action"
    );
}

#[test]
fn test_security_workflow_has_schedule() {
    let content = include_str!("../templates/sentinel-security.yml");
    assert!(
        content.contains("schedule:"),
        "Should have scheduled runs"
    );
    assert!(
        content.contains("cron:"),
        "Should have cron schedule"
    );
}

#[test]
fn test_workflows_reference_main_develop_branches() {
    let analysis = include_str!("../templates/sentinel-analysis.yml");
    let tests = include_str!("../templates/sentinel-tests.yml");
    let security = include_str!("../templates/sentinel-security.yml");

    for workflow in &[analysis, tests, security] {
        assert!(
            (workflow.contains("main") || workflow.contains("master"))
                && workflow.contains("develop"),
            "Should reference main/develop branches"
        );
    }
}

#[test]
fn test_workflows_have_failure_handling() {
    let analysis = include_str!("../templates/sentinel-analysis.yml");
    let security = include_str!("../templates/sentinel-security.yml");

    assert!(
        analysis.contains("exit 1") || analysis.contains("Fail if Critical"),
        "Analysis should fail on critical issues"
    );
    assert!(
        security.contains("exit 1") || security.contains("Fail on Critical Security"),
        "Security should fail on critical issues"
    );
}

#[test]
fn test_tests_workflow_has_artifact_upload() {
    let content = include_str!("../templates/sentinel-tests.yml");
    assert!(
        content.contains("upload-artifact"),
        "Should upload test artifacts"
    );
}

#[test]
fn test_workflows_install_sentinel() {
    let analysis = include_str!("../templates/sentinel-analysis.yml");
    let tests = include_str!("../templates/sentinel-tests.yml");
    let security = include_str!("../templates/sentinel-security.yml");

    for workflow in &[analysis, tests, security] {
        assert!(
            workflow.contains("Install Sentinel"),
            "Should have Sentinel installation step"
        );
    }
}

#[test]
fn test_workflows_are_valid_yaml_structure() {
    let analysis = include_str!("../templates/sentinel-analysis.yml");
    let tests = include_str!("../templates/sentinel-tests.yml");
    let security = include_str!("../templates/sentinel-security.yml");

    // Basic YAML structure validation
    for workflow in &[analysis, tests, security] {
        // Check for proper indentation and structure
        assert!(
            workflow.lines().filter(|l| l.starts_with("  ")).count() > 5,
            "Should have proper YAML indentation"
        );
        // Check for no tabs (YAML doesn't allow tabs)
        assert!(
            !workflow.contains('\t'),
            "YAML should not contain tabs"
        );
    }
}
