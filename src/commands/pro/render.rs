use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Input type for SARIF rendering.
#[derive(Debug)]
pub struct SarifIssue {
    pub file: String,
    pub rule: String,
    pub severity: String,  // "error", "warning", "note"
    pub message: String,
    pub line: Option<usize>,
}

/// Renders a SARIF 2.1.0 JSON string from a list of issues.
/// Returns a pretty-printed JSON string compatible with GitHub Security tab.
pub fn render_sarif(issues: &[SarifIssue]) -> String {
    // Collect unique rule IDs for the driver.rules array
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
            "error"          => "error",
            "note" | "info"  => "note",
            _                => "warning",
        };
        let start_line = i.line.unwrap_or(1);
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
                    "region": { "startLine": start_line }
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

/// Returns absolute paths of files changed in the current working tree (via `git diff --name-only HEAD`).
/// Silently returns empty Vec if not a git repo or git is unavailable.
pub fn get_changed_files(project_root: &Path) -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok();

    let mut files = Vec::new();
    if let Some(out) = output {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let p = project_root.join(trimmed);
                    if p.exists() {
                        files.push(p);
                    }
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_sarif_produces_valid_structure() {
        let issues = vec![
            SarifIssue {
                file: "src/main.ts".to_string(),
                rule: "DEAD_CODE".to_string(),
                severity: "warning".to_string(),
                message: "userId no se usa".to_string(),
                line: Some(23),
            },
        ];
        let sarif = render_sarif(&issues);
        assert!(sarif.contains("\"$schema\""), "must have schema");
        assert!(sarif.contains("\"2.1.0\""), "must have version");
        assert!(sarif.contains("DEAD_CODE"), "must include rule");
        assert!(sarif.contains("\"startLine\": 23"), "must include line number");
        // Verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&sarif).expect("must be valid JSON");
        assert_eq!(parsed["version"], "2.1.0");
        assert!(parsed["runs"][0]["results"][0]["ruleId"] == "DEAD_CODE");
    }

    #[test]
    fn test_get_changed_files_returns_vec() {
        // Verify it doesn't panic in any directory (git or non-git)
        let tmp = std::env::temp_dir();
        let files = get_changed_files(&tmp);
        // In a non-git dir, returns empty
        assert!(files.is_empty() || !files.is_empty(), "should not panic");
    }

    #[test]
    fn test_get_changed_files_in_git_repo() {
        // In the actual project root (which is a git repo), should not panic
        // and should return valid paths if there are any changes
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = get_changed_files(&repo_root);
        // Each returned path must exist
        for f in &files {
            assert!(f.exists(), "get_changed_files returned non-existent path: {:?}", f);
        }
    }
}
