//! Integration tests for Java and Rust project detection

use std::fs;
use tempfile::TempDir;

#[test]
fn test_detect_java_project_with_pom_xml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create pom.xml (Maven project marker)
    fs::write(project_path.join("pom.xml"), "<project></project>")
        .expect("Failed to write pom.xml");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should detect Java project with pom.xml"
    );
}

#[test]
fn test_detect_java_project_with_gradle() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create build.gradle (Gradle project marker)
    fs::write(project_path.join("build.gradle"), "plugins { }")
        .expect("Failed to write build.gradle");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should detect Java project with build.gradle"
    );
}

#[test]
fn test_detect_java_project_with_gradle_kts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create build.gradle.kts (Gradle Kotlin DSL project marker)
    fs::write(project_path.join("build.gradle.kts"), "plugins { }")
        .expect("Failed to write build.gradle.kts");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should detect Java project with build.gradle.kts"
    );
}

#[test]
fn test_detect_java_project_with_src_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Maven/Gradle standard directory structure
    let src_main_java = project_path.join("src").join("main").join("java");
    fs::create_dir_all(&src_main_java).expect("Failed to create src/main/java");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should detect Java project with src/main/java structure"
    );
}

#[test]
fn test_detect_java_project_with_java_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create src directory with .java files
    let src = project_path.join("src");
    fs::create_dir_all(&src).expect("Failed to create src");
    fs::write(src.join("Main.java"), "public class Main { }")
        .expect("Failed to write Java file");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should detect Java project with .java files"
    );
}

#[test]
fn test_detect_rust_project_with_cargo_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Cargo.toml
    fs::write(project_path.join("Cargo.toml"), "[package]\n")
        .expect("Failed to write Cargo.toml");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_rust_project(project_path),
        "Should detect Rust project with Cargo.toml"
    );
}

#[test]
fn test_detect_rust_project_with_cargo_lock() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Cargo.lock
    fs::write(project_path.join("Cargo.lock"), "# This is a Cargo lock file\n")
        .expect("Failed to write Cargo.lock");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_rust_project(project_path),
        "Should detect Rust project with Cargo.lock"
    );
}

#[test]
fn test_detect_rust_project_with_rs_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create src directory with .rs files
    let src = project_path.join("src");
    fs::create_dir_all(&src).expect("Failed to create src");
    fs::write(src.join("main.rs"), "fn main() { }")
        .expect("Failed to write Rust file");

    assert!(
        sentinel_rust::config::SentinelConfig::detect_rust_project(project_path),
        "Should detect Rust project with .rs files"
    );
}

#[test]
fn test_detect_no_java_or_rust_project() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create neither Java nor Rust project markers
    assert!(
        !sentinel_rust::config::SentinelConfig::detect_java_project(project_path),
        "Should not detect Java project"
    );

    assert!(
        !sentinel_rust::config::SentinelConfig::detect_rust_project(project_path),
        "Should not detect Rust project"
    );
}

#[test]
fn test_detect_project_languages_mixed() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create markers for Java and Rust
    fs::write(project_path.join("pom.xml"), "<project></project>")
        .expect("Failed to write pom.xml");

    fs::write(project_path.join("Cargo.toml"), "[package]\n")
        .expect("Failed to write Cargo.toml");

    fs::write(project_path.join("package.json"), "{}\n")
        .expect("Failed to write package.json");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"java".to_string()),
        "Should detect Java language"
    );
    assert!(
        detected.contains(&"rust".to_string()),
        "Should detect Rust language"
    );
    assert!(
        detected.contains(&"typescript".to_string()),
        "Should detect TypeScript language"
    );
}

#[test]
fn test_detect_project_languages_typescript() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create TypeScript/Node.js project marker
    fs::write(project_path.join("package.json"), "{}\n")
        .expect("Failed to write package.json");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"typescript".to_string()),
        "Should detect TypeScript language"
    );
}

#[test]
fn test_detect_project_languages_python() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Python project with setup.py
    fs::write(project_path.join("setup.py"), "# setup\n")
        .expect("Failed to write setup.py");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"python".to_string()),
        "Should detect Python language"
    );
}

#[test]
fn test_detect_project_languages_python_pyproject() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Python project with pyproject.toml
    fs::write(project_path.join("pyproject.toml"), "[project]\n")
        .expect("Failed to write pyproject.toml");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"python".to_string()),
        "Should detect Python language with pyproject.toml"
    );
}

#[test]
fn test_detect_project_languages_python_requirements() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Python project with requirements.txt
    fs::write(project_path.join("requirements.txt"), "numpy==1.0\n")
        .expect("Failed to write requirements.txt");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"python".to_string()),
        "Should detect Python language with requirements.txt"
    );
}

#[test]
fn test_detect_project_languages_go() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create Go project with go.mod
    fs::write(project_path.join("go.mod"), "module example.com/app\n")
        .expect("Failed to write go.mod");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(
        detected.contains(&"go".to_string()),
        "Should detect Go language"
    );
}

#[test]
fn test_detect_project_languages_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // No project markers
    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert!(detected.is_empty(), "Should not detect any languages");
}

#[test]
fn test_detect_project_languages_comprehensive() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create a polyglot project with multiple languages
    fs::write(project_path.join("pom.xml"), "<project></project>")
        .expect("Failed to write pom.xml");

    fs::write(project_path.join("Cargo.toml"), "[package]\n")
        .expect("Failed to write Cargo.toml");

    fs::write(project_path.join("package.json"), "{}\n")
        .expect("Failed to write package.json");

    fs::write(project_path.join("go.mod"), "module example.com/app\n")
        .expect("Failed to write go.mod");

    fs::write(project_path.join("pyproject.toml"), "[project]\n")
        .expect("Failed to write pyproject.toml");

    let detected = sentinel_rust::config::SentinelConfig::detect_project_languages(project_path);

    assert_eq!(detected.len(), 5, "Should detect all 5 languages");
    assert!(detected.contains(&"java".to_string()));
    assert!(detected.contains(&"rust".to_string()));
    assert!(detected.contains(&"typescript".to_string()));
    assert!(detected.contains(&"go".to_string()));
    assert!(detected.contains(&"python".to_string()));
}
