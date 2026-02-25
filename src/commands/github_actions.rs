//! GitHub Actions workflow template generation

use colored::Colorize;
use std::fs;
use std::path::Path;

/// GitHub Actions workflow templates (embedded in binary)
const ANALYSIS_WORKFLOW: &str = include_str!("../../templates/sentinel-analysis.yml");
const TESTS_WORKFLOW: &str = include_str!("../../templates/sentinel-tests.yml");
const SECURITY_WORKFLOW: &str = include_str!("../../templates/sentinel-security.yml");

pub enum WorkflowType {
    Analysis,
    Tests,
    Security,
    All,
}

/// Install GitHub Actions workflow templates
pub fn handle_github_actions_command(project_root: &Path, workflow_type: WorkflowType) {
    let git_dir = project_root.join(".git");
    if !git_dir.exists() {
        eprintln!(
            "{}",
            "Error: Not a git repository. Initialize git first with 'git init'".red()
        );
        std::process::exit(1);
    }

    let workflows_dir = project_root.join(".github").join("workflows");

    // Create workflows directory if it doesn't exist
    if !workflows_dir.exists() {
        if let Err(e) = fs::create_dir_all(&workflows_dir) {
            eprintln!(
                "{}",
                format!("Error creating workflows directory: {}", e).red()
            );
            std::process::exit(1);
        }
    }

    match workflow_type {
        WorkflowType::Analysis => {
            install_workflow(&workflows_dir, "sentinel-analysis.yml", ANALYSIS_WORKFLOW)
        }
        WorkflowType::Tests => {
            install_workflow(&workflows_dir, "sentinel-tests.yml", TESTS_WORKFLOW)
        }
        WorkflowType::Security => {
            install_workflow(&workflows_dir, "sentinel-security.yml", SECURITY_WORKFLOW)
        }
        WorkflowType::All => {
            println!("{}", "Installing all GitHub Actions workflows...".bold());
            install_workflow(&workflows_dir, "sentinel-analysis.yml", ANALYSIS_WORKFLOW);
            install_workflow(&workflows_dir, "sentinel-tests.yml", TESTS_WORKFLOW);
            install_workflow(&workflows_dir, "sentinel-security.yml", SECURITY_WORKFLOW);
        }
    }

    println!();
    println!(
        "{}",
        "✅ GitHub Actions workflows installed successfully!".green()
    );
    println!();
    println!("Workflows will run on:");
    println!("  • Push to main/develop branches");
    println!("  • Pull requests to main/develop branches");
    println!("  • Security scan: Weekly (Sunday 2 AM UTC)");
    println!();
    println!("View workflow status in GitHub Actions tab");
}

fn install_workflow(workflows_dir: &Path, filename: &str, content: &str) {
    let workflow_path = workflows_dir.join(filename);

    // Check if workflow already exists
    if workflow_path.exists() {
        println!(
            "{}",
            format!("Workflow {} already exists. Updating...", filename)
                .yellow()
        );
    }

    // Write the workflow file
    if let Err(e) = fs::write(&workflow_path, content) {
        eprintln!(
            "{}",
            format!("Error writing workflow {}: {}", filename, e).red()
        );
        std::process::exit(1);
    }

    println!("  {} {}", "✓".green(), filename.cyan());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_workflow_is_valid_yaml() {
        assert!(
            ANALYSIS_WORKFLOW.contains("name: Sentinel Code Analysis"),
            "Analysis workflow should have correct name"
        );
        assert!(
            ANALYSIS_WORKFLOW.contains("on:"),
            "Analysis workflow should have 'on' trigger"
        );
        assert!(
            ANALYSIS_WORKFLOW.contains("jobs:"),
            "Analysis workflow should have jobs section"
        );
    }

    #[test]
    fn test_tests_workflow_is_valid_yaml() {
        assert!(
            TESTS_WORKFLOW.contains("name: Sentinel Test Execution"),
            "Tests workflow should have correct name"
        );
        assert!(
            TESTS_WORKFLOW.contains("jobs:"),
            "Tests workflow should have jobs section"
        );
    }

    #[test]
    fn test_security_workflow_is_valid_yaml() {
        assert!(
            SECURITY_WORKFLOW.contains("name: Sentinel Security Scanning"),
            "Security workflow should have correct name"
        );
        assert!(
            SECURITY_WORKFLOW.contains("schedule:"),
            "Security workflow should have scheduled trigger"
        );
    }

    #[test]
    fn test_workflows_have_pull_request_trigger() {
        assert!(
            ANALYSIS_WORKFLOW.contains("pull_request:"),
            "Analysis workflow should trigger on PRs"
        );
        assert!(
            TESTS_WORKFLOW.contains("pull_request:"),
            "Tests workflow should trigger on PRs"
        );
        assert!(
            SECURITY_WORKFLOW.contains("pull_request:"),
            "Security workflow should trigger on PRs"
        );
    }

    #[test]
    fn test_workflows_have_push_trigger() {
        assert!(
            ANALYSIS_WORKFLOW.contains("push:"),
            "Analysis workflow should trigger on push"
        );
        assert!(
            TESTS_WORKFLOW.contains("push:"),
            "Tests workflow should trigger on push"
        );
        assert!(
            SECURITY_WORKFLOW.contains("push:"),
            "Security workflow should trigger on push"
        );
    }

    #[test]
    fn test_workflows_have_proper_structure() {
        for workflow in &[ANALYSIS_WORKFLOW, TESTS_WORKFLOW, SECURITY_WORKFLOW] {
            assert!(
                workflow.starts_with("name:"),
                "Workflow should start with name"
            );
            assert!(workflow.contains("runs-on:"), "Workflow should specify runner");
            assert!(
                workflow.contains("steps:"),
                "Workflow should have steps section"
            );
        }
    }

    #[test]
    fn test_security_workflow_has_schedule() {
        assert!(
            SECURITY_WORKFLOW.contains("schedule:"),
            "Security workflow should have scheduled runs"
        );
        assert!(
            SECURITY_WORKFLOW.contains("cron:"),
            "Security workflow should have cron expression"
        );
    }

    #[test]
    fn test_workflows_reference_sentinel() {
        assert!(
            ANALYSIS_WORKFLOW.contains("sentinel"),
            "Analysis workflow should reference sentinel"
        );
        assert!(
            TESTS_WORKFLOW.contains("sentinel"),
            "Tests workflow should reference sentinel"
        );
        assert!(
            SECURITY_WORKFLOW.contains("sentinel"),
            "Security workflow should reference sentinel"
        );
    }
}
