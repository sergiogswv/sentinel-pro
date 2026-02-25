use std::path::Path;

#[test]
fn test_distribution_scripts_exist() {
    assert!(Path::new("scripts/extract-version.sh").exists());
    assert!(Path::new("tools/generate-checksums.sh").exists());
    assert!(Path::new("tools/homebrew/sentinel-pro.rb").exists());
    assert!(Path::new(".github/workflows/release.yml").exists());
}

#[test]
fn test_documentation_structure() {
    assert!(Path::new("website/docs/getting-started.md").exists());
    assert!(Path::new("website/docs/features/custom-rules.md").exists());
    assert!(Path::new("website/docs/api/commands.md").exists());
}

#[test]
fn test_cargo_toml_metadata() {
    let content = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(content.contains("publish = true"));
    assert!(content.contains("homepage"));
    assert!(content.contains("repository"));
}

#[test]
fn test_telemetry_module_exists() {
    assert!(Path::new("src/telemetry/mod.rs").exists());
    assert!(Path::new("src/telemetry/event.rs").exists());
    assert!(Path::new("src/telemetry/client.rs").exists());
    assert!(Path::new("src/telemetry/storage.rs").exists());
}

#[test]
fn test_update_module_exists() {
    assert!(Path::new("src/update.rs").exists());
}

#[test]
fn test_extract_version_script_is_executable() {
    let metadata = std::fs::metadata("scripts/extract-version.sh").unwrap();
    let permissions = metadata.permissions();
    assert!(!permissions.readonly());
}
