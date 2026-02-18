# Sentinel Pro - Phase 1: Fundamentos Pro Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish Sentinel Pro foundation with CLI dispatcher, basic command infrastructure, and first 3 working commands (analyze, generate, refactor).

**Architecture:** Extend existing Sentinel CLI with new `pro` subcommand namespace. Use Clap 4.4 for CLI parsing, establish module structure for agents/ML/framework-engine, implement basic file operations with backup/safety mechanisms.

**Tech Stack:** Rust 2024 (1.75+), Clap 4.4, Tokio, tree-sitter 0.20, colored 2.0, indicatif 0.17

**Timeline:** 4-6 weeks

**Scope:** This plan covers Phase 1 only. Subsequent phases (Multi-Agent, ML, Framework Engine, etc.) will have separate implementation plans.

---

## Prerequisites Verification

### Task 0: Environment Setup

**Step 1: Verify Rust toolchain**

Run:
```bash
rustc --version
cargo --version
```

Expected: Rust 1.75.0 or higher

**Step 2: Review existing codebase**

Read:
- `Cargo.toml` - understand current dependencies
- `src/main.rs` - understand entry point
- `src/commands/` - understand existing command structure

**Step 3: Create development branch**

```bash
git checkout -b feature/phase1-fundamentos-pro
git push -u origin feature/phase1-fundamentos-pro
```

---

## Part 1: Project Structure Setup

### Task 1: Update Cargo.toml with Phase 1 Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add Phase 1 dependencies**

Add to `[dependencies]`:

```toml
# CLI (upgrade if needed)
clap = { version = "4.4", features = ["derive", "cargo"] }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Parsing (for future tree-sitter integration)
tree-sitter = "0.20"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# File operations
walkdir = "2.4"
regex = "1.10"

# UI/UX
colored = "2.0"
indicatif = "0.17"
console = "0.15"

# Testing
mockall = "0.12"
```

**Step 2: Build to verify dependencies**

Run:
```bash
cargo build
```

Expected: Clean build with new dependencies downloaded

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add Phase 1 dependencies for Sentinel Pro"
```

---

### Task 2: Create Module Directory Structure

**Files:**
- Create: `src/commands/pro/mod.rs`
- Create: `src/commands/pro/analyze.rs`
- Create: `src/commands/pro/generate.rs`
- Create: `src/commands/pro/refactor.rs`
- Create: `src/agents/mod.rs`
- Create: `src/agents/base.rs`
- Create: `src/ml/mod.rs`
- Create: `src/framework_engine/mod.rs`
- Create: `src/knowledge/mod.rs`

**Step 1: Create pro commands module structure**

Create `src/commands/pro/mod.rs`:

```rust
//! Sentinel Pro advanced commands
//!
//! This module contains all "sentinel pro" subcommands.

pub mod analyze;
pub mod generate;
pub mod refactor;

pub use analyze::AnalyzeCommand;
pub use generate::GenerateCommand;
pub use refactor::RefactorCommand;
```

**Step 2: Create placeholder command files**

Create `src/commands/pro/analyze.rs`:

```rust
use clap::Args;
use anyhow::Result;

#[derive(Debug, Args)]
pub struct AnalyzeCommand {
    /// File or directory to analyze
    pub path: String,

    /// Deep analysis with all checks
    #[arg(long)]
    pub deep: bool,

    /// Focus on security issues
    #[arg(long)]
    pub security: bool,

    /// Focus on performance issues
    #[arg(long)]
    pub performance: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl AnalyzeCommand {
    pub async fn execute(&self) -> Result<()> {
        // TODO: Implement in later tasks
        println!("🔍 Analyzing: {}", self.path);
        println!("⏳ Implementation pending...");
        Ok(())
    }
}
```

Create `src/commands/pro/generate.rs`:

```rust
use clap::Args;
use anyhow::Result;

#[derive(Debug, Args)]
pub struct GenerateCommand {
    /// File to generate
    pub path: String,

    /// Description of what to generate
    #[arg(long)]
    pub prompt: Option<String>,

    /// Specification file (YAML)
    #[arg(long)]
    pub spec: Option<String>,

    /// Interactive mode
    #[arg(long)]
    pub interactive: bool,

    /// Auto-generate tests
    #[arg(long)]
    pub with_tests: bool,

    /// Show without applying
    #[arg(long)]
    pub dry_run: bool,
}

impl GenerateCommand {
    pub async fn execute(&self) -> Result<()> {
        println!("✨ Generating: {}", self.path);
        println!("⏳ Implementation pending...");
        Ok(())
    }
}
```

Create `src/commands/pro/refactor.rs`:

```rust
use clap::Args;
use anyhow::Result;

#[derive(Debug, Args)]
pub struct RefactorCommand {
    /// File to refactor
    pub path: String,

    /// Extract long functions
    #[arg(long)]
    pub extract_functions: bool,

    /// Rename variables semantically
    #[arg(long)]
    pub rename_variables: bool,

    /// Remove dead code
    #[arg(long)]
    pub remove_dead: bool,

    /// Simplify complex logic
    #[arg(long)]
    pub simplify: bool,

    /// Maximum verification (slower)
    #[arg(long)]
    pub safety_first: bool,

    /// Create backup before refactoring
    #[arg(long)]
    pub backup: bool,
}

impl RefactorCommand {
    pub async fn execute(&self) -> Result<()> {
        println!("♻️  Refactoring: {}", self.path);
        println!("⏳ Implementation pending...");
        Ok(())
    }
}
```

**Step 3: Create placeholder module stubs**

Create `src/agents/mod.rs`:

```rust
//! Multi-agent system (Phase 2)
//!
//! Placeholder for future agent implementation.

pub mod base;
```

Create `src/agents/base.rs`:

```rust
//! Agent base traits and structures
//!
//! Phase 2 implementation

use anyhow::Result;

#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult>;
}

pub struct AgentContext {
    // TODO: Phase 2
}

pub struct AgentResult {
    // TODO: Phase 2
}
```

Create `src/ml/mod.rs`:

```rust
//! Machine Learning components (Phase 3)
//!
//! Placeholder for ML implementation.

// Phase 3: embeddings, similarity, predictor, patterns, models
```

Create `src/framework_engine/mod.rs`:

```rust
//! Framework rules engine (Phase 4)
//!
//! Placeholder for framework validation.

// Phase 4: rules, loader, versions, registry
```

Create `src/knowledge/mod.rs`:

```rust
//! Knowledge base and vector store (Phase 5)
//!
//! Placeholder for codebase indexing.

// Phase 5: codebase, vector_store, search, context
```

**Step 4: Update src/commands/mod.rs**

Modify `src/commands/mod.rs` to add:

```rust
pub mod pro;
```

**Step 5: Build to verify structure**

Run:
```bash
cargo check
```

Expected: Clean compilation (with async_trait added if needed)

**Step 6: Commit**

```bash
git add src/commands/pro/ src/agents/ src/ml/ src/framework_engine/ src/knowledge/
git add src/commands/mod.rs
git commit -m "feat: create Phase 1 module structure with placeholders"
```

---

### Task 3: Update Main CLI Entry Point

**Files:**
- Modify: `src/main.rs`

**Step 1: Add Pro subcommand enum**

Add to `src/main.rs` (adapt based on existing structure):

```rust
use clap::{Parser, Subcommand};
use commands::pro::{AnalyzeCommand, GenerateCommand, RefactorCommand};

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "Sentinel - Intelligent file watcher and code assistant")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Sentinel in current project
    Init,

    /// Watch files and run tests (classic mode)
    Watch,

    /// Sentinel Pro - Advanced AI-powered commands
    Pro {
        #[command(subcommand)]
        command: ProCommands,
    },
}

#[derive(Subcommand)]
enum ProCommands {
    /// Deep analysis of code
    Analyze(AnalyzeCommand),

    /// Generate new code with AI
    Generate(GenerateCommand),

    /// Refactor code automatically
    Refactor(RefactorCommand),
}
```

**Step 2: Update main function**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            // Existing init logic
            println!("Initializing Sentinel...");
            Ok(())
        }
        Some(Commands::Watch) => {
            // Existing watch logic
            println!("Starting file watcher...");
            Ok(())
        }
        Some(Commands::Pro { command }) => {
            match command {
                ProCommands::Analyze(cmd) => cmd.execute().await,
                ProCommands::Generate(cmd) => cmd.execute().await,
                ProCommands::Refactor(cmd) => cmd.execute().await,
            }
        }
        None => {
            // Default: watch mode
            println!("Starting default watch mode...");
            Ok(())
        }
    }
}
```

**Step 3: Add async-trait dependency if needed**

Add to `Cargo.toml`:

```toml
async-trait = "0.1"
```

**Step 4: Test CLI parsing**

Run:
```bash
cargo build
./target/debug/sentinel pro --help
```

Expected: Help text showing analyze, generate, refactor commands

**Step 5: Test each subcommand**

Run:
```bash
./target/debug/sentinel pro analyze test.rs
./target/debug/sentinel pro generate src/new.rs --prompt "test"
./target/debug/sentinel pro refactor src/old.rs
```

Expected: Each prints placeholder message

**Step 6: Commit**

```bash
git add src/main.rs Cargo.toml Cargo.lock
git commit -m "feat: wire up Pro subcommands to main CLI dispatcher"
```

---

## Part 2: Core Utilities and Infrastructure

### Task 4: File Backup System

**Files:**
- Create: `src/utils/mod.rs`
- Create: `src/utils/backup.rs`
- Create: `tests/utils/backup_test.rs`

**Step 1: Write failing test**

Create `tests/utils/backup_test.rs`:

```rust
use sentinel_pro::utils::backup::BackupManager;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_create_backup() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, "original content").unwrap();

    let backup_manager = BackupManager::new(temp.path().join(".sentinel-backups"));
    let backup_path = backup_manager.create_backup(&file_path).unwrap();

    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, "original content");
}

#[test]
fn test_restore_backup() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, "original").unwrap();

    let backup_manager = BackupManager::new(temp.path().join(".sentinel-backups"));
    let backup_path = backup_manager.create_backup(&file_path).unwrap();

    fs::write(&file_path, "modified").unwrap();
    backup_manager.restore_backup(&file_path, &backup_path).unwrap();

    let restored = fs::read_to_string(&file_path).unwrap();
    assert_eq!(restored, "original");
}
```

**Step 2: Add tempfile dependency**

Add to `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3.8"
```

**Step 3: Run test to verify it fails**

Run:
```bash
cargo test test_create_backup
```

Expected: FAIL - module not found

**Step 4: Implement BackupManager**

Create `src/utils/mod.rs`:

```rust
pub mod backup;
```

Create `src/utils/backup.rs`:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    pub fn create_backup(&self, file_path: &Path) -> Result<PathBuf> {
        // Create backup directory if it doesn't exist
        fs::create_dir_all(&self.backup_dir)
            .context("Failed to create backup directory")?;

        // Generate backup filename with timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let file_name = file_path
            .file_name()
            .context("Invalid file path")?
            .to_str()
            .context("Non-UTF8 filename")?;

        let backup_filename = format!("{}_{}", timestamp, file_name);
        let backup_path = self.backup_dir.join(backup_filename);

        // Copy file to backup location
        fs::copy(file_path, &backup_path)
            .context("Failed to create backup")?;

        Ok(backup_path)
    }

    pub fn restore_backup(&self, target_path: &Path, backup_path: &Path) -> Result<()> {
        fs::copy(backup_path, target_path)
            .context("Failed to restore backup")?;
        Ok(())
    }

    pub fn list_backups(&self, file_name: &str) -> Result<Vec<PathBuf>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(file_name) {
                    backups.push(path);
                }
            }
        }

        backups.sort();
        Ok(backups)
    }
}
```

**Step 5: Add chrono dependency**

Add to `Cargo.toml`:

```toml
chrono = "0.4"
```

**Step 6: Update src/lib.rs**

Create or modify `src/lib.rs`:

```rust
pub mod utils;
pub mod commands;
pub mod agents;
// Export for testing
```

**Step 7: Run tests**

Run:
```bash
cargo test backup
```

Expected: PASS

**Step 8: Commit**

```bash
git add src/utils/ tests/utils/ src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: implement backup system for safe file operations"
```

---

### Task 5: File Analysis Foundation

**Files:**
- Create: `src/analysis/mod.rs`
- Create: `src/analysis/file_info.rs`
- Create: `tests/analysis/file_info_test.rs`

**Step 1: Write failing test**

Create `tests/analysis/file_info_test.rs`:

```rust
use sentinel_pro::analysis::file_info::FileAnalyzer;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_count_lines() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

    let analyzer = FileAnalyzer::new();
    let info = analyzer.analyze(&file_path).unwrap();

    assert_eq!(info.total_lines, 3);
}

#[test]
fn test_detect_language() {
    let temp = TempDir::new().unwrap();

    let rs_file = temp.path().join("test.rs");
    fs::write(&rs_file, "fn main() {}").unwrap();

    let analyzer = FileAnalyzer::new();
    let info = analyzer.analyze(&rs_file).unwrap();

    assert_eq!(info.language, "rust");
}

#[test]
fn test_count_functions() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, r#"
fn foo() {}
fn bar() {}
pub fn baz() {}
"#).unwrap();

    let analyzer = FileAnalyzer::new();
    let info = analyzer.analyze(&file_path).unwrap();

    assert_eq!(info.function_count, 3);
}
```

**Step 2: Run test to verify failure**

Run:
```bash
cargo test test_count_lines
```

Expected: FAIL - module not found

**Step 3: Implement FileAnalyzer**

Create `src/analysis/mod.rs`:

```rust
pub mod file_info;
```

Create `src/analysis/file_info.rs`:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub function_count: usize,
}

pub struct FileAnalyzer {
    function_regex: Regex,
}

impl FileAnalyzer {
    pub fn new() -> Self {
        Self {
            // Simple regex for function detection (Rust, TypeScript, Python)
            function_regex: Regex::new(r"(?m)^\s*(fn|function|def|async\s+fn)\s+\w+").unwrap(),
        }
    }

    pub fn analyze(&self, file_path: &Path) -> Result<FileInfo> {
        let content = fs::read_to_string(file_path)
            .context("Failed to read file")?;

        let language = self.detect_language(file_path);
        let lines: Vec<&str> = content.lines().collect();

        let total_lines = lines.len();
        let blank_lines = lines.iter().filter(|l| l.trim().is_empty()).count();
        let comment_lines = self.count_comment_lines(&lines, &language);
        let code_lines = total_lines - blank_lines - comment_lines;

        let function_count = self.count_functions(&content);

        Ok(FileInfo {
            path: file_path.to_string_lossy().to_string(),
            language,
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            function_count,
        })
    }

    fn detect_language(&self, file_path: &Path) -> String {
        match file_path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("py") => "python".to_string(),
            Some("go") => "go".to_string(),
            Some("php") => "php".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn count_comment_lines(&self, lines: &[&str], language: &str) -> usize {
        let mut count = 0;
        let mut in_block_comment = false;

        for line in lines {
            let trimmed = line.trim();

            match language {
                "rust" | "typescript" | "javascript" | "go" => {
                    if trimmed.starts_with("/*") {
                        in_block_comment = true;
                    }
                    if in_block_comment || trimmed.starts_with("//") {
                        count += 1;
                    }
                    if trimmed.ends_with("*/") {
                        in_block_comment = false;
                    }
                }
                "python" => {
                    if trimmed.starts_with("#") {
                        count += 1;
                    }
                }
                "php" => {
                    if trimmed.starts_with("//") || trimmed.starts_with("#") {
                        count += 1;
                    }
                }
                _ => {}
            }
        }

        count
    }

    fn count_functions(&self, content: &str) -> usize {
        self.function_regex.find_iter(content).count()
    }
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod analysis;
```

**Step 5: Run tests**

Run:
```bash
cargo test file_info
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/analysis/ tests/analysis/ src/lib.rs
git commit -m "feat: implement file analysis foundation"
```

---

### Task 6: AI Client Abstraction Layer

**Files:**
- Create: `src/ai/mod.rs`
- Create: `src/ai/client.rs`
- Create: `src/ai/types.rs`
- Create: `tests/ai/client_test.rs`

**Step 1: Define AI types**

Create `src/ai/mod.rs`:

```rust
pub mod client;
pub mod types;

pub use client::{AiClient, LocalAiClient};
pub use types::{AiRequest, AiResponse, AiProvider};
```

Create `src/ai/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    Ollama,
    LmStudio,
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<usize>,
}

impl AiRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            system_prompt: None,
            temperature: 0.7,
            max_tokens: None,
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}
```

**Step 2: Write failing test**

Create `tests/ai/client_test.rs`:

```rust
use sentinel_pro::ai::{AiClient, LocalAiClient, AiRequest};

#[tokio::test]
async fn test_client_creation() {
    let client = LocalAiClient::new("http://localhost:11434".to_string());
    assert_eq!(client.base_url(), "http://localhost:11434");
}

#[tokio::test]
async fn test_request_building() {
    let request = AiRequest::new("Test prompt")
        .with_system_prompt("You are a code assistant")
        .with_temperature(0.5);

    assert_eq!(request.prompt, "Test prompt");
    assert_eq!(request.system_prompt, Some("You are a code assistant".to_string()));
    assert_eq!(request.temperature, 0.5);
}
```

**Step 3: Run test to verify failure**

Run:
```bash
cargo test test_client_creation
```

Expected: FAIL - module not found

**Step 4: Implement AI client (stub)**

Create `src/ai/client.rs`:

```rust
use anyhow::{Context, Result};
use crate::ai::types::{AiRequest, AiResponse};

#[async_trait::async_trait]
pub trait AiClient: Send + Sync {
    async fn generate(&self, request: AiRequest) -> Result<AiResponse>;
    fn base_url(&self) -> &str;
}

pub struct LocalAiClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl LocalAiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AiClient for LocalAiClient {
    async fn generate(&self, request: AiRequest) -> Result<AiResponse> {
        // Stub implementation - will connect to Ollama in later tasks
        Ok(AiResponse {
            content: format!("Mock response to: {}", request.prompt),
            model: "mock-model".to_string(),
            tokens_used: Some(100),
        })
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}
```

**Step 5: Add reqwest dependency**

Add to `Cargo.toml`:

```toml
reqwest = { version = "0.11", features = ["json"] }
```

**Step 6: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod ai;
```

**Step 7: Run tests**

Run:
```bash
cargo test ai::client
```

Expected: PASS

**Step 8: Commit**

```bash
git add src/ai/ tests/ai/ src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: implement AI client abstraction layer"
```

---

## Part 3: Implement First Command - Analyze

### Task 7: Implement Basic Analyze Command

**Files:**
- Modify: `src/commands/pro/analyze.rs`
- Create: `tests/commands/analyze_test.rs`

**Step 1: Write integration test**

Create `tests/commands/analyze_test.rs`:

```rust
use sentinel_pro::commands::pro::AnalyzeCommand;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_analyze_basic() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, r#"
fn main() {
    println!("Hello");
}

fn helper() {
    // comment
}
"#).unwrap();

    let cmd = AnalyzeCommand {
        path: file_path.to_string_lossy().to_string(),
        deep: false,
        security: false,
        performance: false,
        json: false,
    };

    let result = cmd.execute().await;
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify current behavior**

Run:
```bash
cargo test test_analyze_basic
```

Expected: PASS (but only prints placeholder)

**Step 3: Implement analyze command logic**

Modify `src/commands/pro/analyze.rs`:

```rust
use clap::Args;
use anyhow::{Context, Result};
use colored::Colorize;
use crate::analysis::file_info::FileAnalyzer;
use std::path::Path;

#[derive(Debug, Args)]
pub struct AnalyzeCommand {
    /// File or directory to analyze
    pub path: String,

    /// Deep analysis with all checks
    #[arg(long)]
    pub deep: bool,

    /// Focus on security issues
    #[arg(long)]
    pub security: bool,

    /// Focus on performance issues
    #[arg(long)]
    pub performance: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl AnalyzeCommand {
    pub async fn execute(&self) -> Result<()> {
        let path = Path::new(&self.path);

        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", self.path);
        }

        println!("🔍 Analyzing: {}", self.path.bright_cyan());
        println!("{}", "━".repeat(50).bright_black());

        let analyzer = FileAnalyzer::new();
        let info = analyzer.analyze(path)
            .context("Failed to analyze file")?;

        if self.json {
            self.print_json(&info)?;
        } else {
            self.print_human_readable(&info)?;
        }

        Ok(())
    }

    fn print_human_readable(&self, info: &crate::analysis::file_info::FileInfo) -> Result<()> {
        println!("\n{}", "📊 OVERVIEW".bright_green().bold());
        println!("  {} Lines: {}", "•".bright_black(), info.total_lines);
        println!("    {} Code: {}", "└─".bright_black(), info.code_lines);
        println!("    {} Comments: {}", "└─".bright_black(), info.comment_lines);
        println!("    {} Blank: {}", "└─".bright_black(), info.blank_lines);
        println!("  {} Functions: {}", "•".bright_black(), info.function_count);
        println!("  {} Language: {}", "•".bright_black(), info.language);

        let complexity = self.calculate_complexity(info);
        let complexity_label = match complexity {
            c if c < 5.0 => "Low".bright_green(),
            c if c < 10.0 => "Medium".bright_yellow(),
            _ => "High".bright_red(),
        };
        println!("  {} Complexity: {} ({:.1})", "•".bright_black(), complexity_label, complexity);

        println!("\n{}", "✅ Analysis complete".bright_green());

        Ok(())
    }

    fn print_json(&self, info: &crate::analysis::file_info::FileInfo) -> Result<()> {
        let json = serde_json::json!({
            "path": info.path,
            "language": info.language,
            "metrics": {
                "total_lines": info.total_lines,
                "code_lines": info.code_lines,
                "comment_lines": info.comment_lines,
                "blank_lines": info.blank_lines,
                "function_count": info.function_count,
            }
        });

        println!("{}", serde_json::to_string_pretty(&json)?);
        Ok(())
    }

    fn calculate_complexity(&self, info: &crate::analysis::file_info::FileInfo) -> f64 {
        if info.function_count == 0 {
            return 0.0;
        }
        info.code_lines as f64 / info.function_count as f64
    }
}
```

**Step 4: Export FileInfo in analysis module**

Modify `src/analysis/file_info.rs` to make FileInfo public (already is).

**Step 5: Run tests**

Run:
```bash
cargo test analyze
```

Expected: PASS

**Step 6: Manual test**

Create a test file:
```bash
echo 'fn main() { println!("test"); }' > /tmp/test.rs
cargo run -- pro analyze /tmp/test.rs
```

Expected: Shows formatted analysis output

**Step 7: Commit**

```bash
git add src/commands/pro/analyze.rs tests/commands/
git commit -m "feat: implement basic analyze command with metrics"
```

---

### Task 8: Add Issue Detection to Analyze

**Files:**
- Create: `src/analysis/issues.rs`
- Modify: `src/analysis/mod.rs`
- Modify: `src/commands/pro/analyze.rs`
- Create: `tests/analysis/issues_test.rs`

**Step 1: Write failing test**

Create `tests/analysis/issues_test.rs`:

```rust
use sentinel_pro::analysis::issues::{IssueDetector, IssueSeverity};
use sentinel_pro::analysis::file_info::FileInfo;

#[test]
fn test_detect_long_function() {
    let info = FileInfo {
        path: "test.rs".to_string(),
        language: "rust".to_string(),
        total_lines: 100,
        code_lines: 80,
        comment_lines: 10,
        blank_lines: 10,
        function_count: 1,
    };

    let detector = IssueDetector::new();
    let issues = detector.detect(&info);

    assert!(issues.iter().any(|i| i.code == "LONG_FUNCTION"));
}

#[test]
fn test_detect_no_functions() {
    let info = FileInfo {
        path: "test.rs".to_string(),
        language: "rust".to_string(),
        total_lines: 50,
        code_lines: 45,
        comment_lines: 5,
        blank_lines: 0,
        function_count: 0,
    };

    let detector = IssueDetector::new();
    let issues = detector.detect(&info);

    assert!(issues.iter().any(|i| i.code == "NO_FUNCTIONS"));
}
```

**Step 2: Run test to verify failure**

Run:
```bash
cargo test test_detect_long_function
```

Expected: FAIL - module not found

**Step 3: Implement issue detector**

Create `src/analysis/issues.rs`:

```rust
use crate::analysis::file_info::FileInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub code: String,
    pub severity: IssueSeverity,
    pub message: String,
    pub line: Option<usize>,
}

pub struct IssueDetector {
    max_avg_function_length: usize,
    min_comment_ratio: f64,
}

impl IssueDetector {
    pub fn new() -> Self {
        Self {
            max_avg_function_length: 50,
            min_comment_ratio: 0.1,
        }
    }

    pub fn detect(&self, info: &FileInfo) -> Vec<Issue> {
        let mut issues = Vec::new();

        // Check for no functions
        if info.code_lines > 20 && info.function_count == 0 {
            issues.push(Issue {
                code: "NO_FUNCTIONS".to_string(),
                severity: IssueSeverity::Low,
                message: "File contains no functions, consider organizing code".to_string(),
                line: None,
            });
        }

        // Check for long functions (average)
        if info.function_count > 0 {
            let avg_length = info.code_lines / info.function_count;
            if avg_length > self.max_avg_function_length {
                issues.push(Issue {
                    code: "LONG_FUNCTION".to_string(),
                    severity: IssueSeverity::Medium,
                    message: format!(
                        "Average function length is {} lines (max recommended: {})",
                        avg_length, self.max_avg_function_length
                    ),
                    line: None,
                });
            }
        }

        // Check for low comment ratio
        if info.total_lines > 0 {
            let comment_ratio = info.comment_lines as f64 / info.total_lines as f64;
            if comment_ratio < self.min_comment_ratio && info.code_lines > 50 {
                issues.push(Issue {
                    code: "LOW_COMMENTS".to_string(),
                    severity: IssueSeverity::Low,
                    message: format!(
                        "Comment ratio is {:.1}% (recommended: {:.0}%+)",
                        comment_ratio * 100.0,
                        self.min_comment_ratio * 100.0
                    ),
                    line: None,
                });
            }
        }

        // Check for very large files
        if info.code_lines > 500 {
            issues.push(Issue {
                code: "LARGE_FILE".to_string(),
                severity: IssueSeverity::Medium,
                message: format!("File has {} lines of code, consider splitting", info.code_lines),
                line: None,
            });
        }

        issues
    }
}

impl Default for IssueDetector {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update analysis module**

Modify `src/analysis/mod.rs`:

```rust
pub mod file_info;
pub mod issues;
```

**Step 5: Update analyze command to show issues**

Modify `src/commands/pro/analyze.rs` to add issue detection:

```rust
use crate::analysis::issues::{IssueDetector, IssueSeverity};

// In print_human_readable method, add before final message:

        // Detect and display issues
        let detector = IssueDetector::new();
        let issues = detector.detect(info);

        if !issues.is_empty() {
            println!("\n{}", "⚠️  ISSUES DETECTED".bright_yellow().bold());
            for (i, issue) in issues.iter().enumerate() {
                let severity_icon = match issue.severity {
                    IssueSeverity::Low => "🟢".to_string(),
                    IssueSeverity::Medium => "🟡".to_string(),
                    IssueSeverity::High => "🟠".to_string(),
                    IssueSeverity::Critical => "🔴".to_string(),
                };
                println!("  {}. {} [{}] {}",
                    i + 1,
                    severity_icon,
                    issue.code.bright_black(),
                    issue.message
                );
            }
        }
```

**Step 6: Run tests**

Run:
```bash
cargo test issues
```

Expected: PASS

**Step 7: Manual test with issues**

Create test file with issues:
```bash
cat > /tmp/long.rs << 'EOF'
fn very_long_function() {
    let x = 1;
    let y = 2;
    // ... many lines
    for i in 0..100 {
        println!("{}", i);
    }
    // Add 80+ lines here
}
EOF

cargo run -- pro analyze /tmp/long.rs
```

Expected: Shows issues detected

**Step 8: Commit**

```bash
git add src/analysis/issues.rs src/analysis/mod.rs src/commands/pro/analyze.rs tests/analysis/issues_test.rs
git commit -m "feat: add issue detection to analyze command"
```

---

## Part 4: Implement Generate Command (Basic)

### Task 9: Template-Based Code Generation

**Files:**
- Create: `src/generation/mod.rs`
- Create: `src/generation/templates.rs`
- Modify: `src/commands/pro/generate.rs`
- Create: `tests/generation/templates_test.rs`

**Step 1: Write failing test**

Create `tests/generation/templates_test.rs`:

```rust
use sentinel_pro::generation::templates::{TemplateEngine, TemplateContext};

#[test]
fn test_render_simple_template() {
    let engine = TemplateEngine::new();
    let template = "fn {{name}}() {\n    // TODO\n}";

    let mut context = TemplateContext::new();
    context.insert("name", "test_function");

    let result = engine.render(template, &context).unwrap();
    assert!(result.contains("fn test_function()"));
}

#[test]
fn test_rust_function_template() {
    let engine = TemplateEngine::new();
    let template = engine.get_template("rust_function").unwrap();

    let mut context = TemplateContext::new();
    context.insert("name", "calculate");
    context.insert("params", "x: i32, y: i32");
    context.insert("return_type", "i32");

    let result = engine.render(template, &context).unwrap();
    assert!(result.contains("pub fn calculate"));
}
```

**Step 2: Run test to verify failure**

Run:
```bash
cargo test test_render_simple_template
```

Expected: FAIL - module not found

**Step 3: Add handlebars dependency**

Add to `Cargo.toml`:

```toml
handlebars = "4.5"
```

**Step 4: Implement template engine**

Create `src/generation/mod.rs`:

```rust
pub mod templates;
```

Create `src/generation/templates.rs`:

```rust
use anyhow::{Context, Result};
use handlebars::Handlebars;
use std::collections::HashMap;

pub type TemplateContext = HashMap<String, String>;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            handlebars: Handlebars::new(),
        };
        engine.register_builtin_templates();
        engine
    }

    pub fn render(&self, template: &str, context: &TemplateContext) -> Result<String> {
        self.handlebars
            .render_template(template, context)
            .context("Failed to render template")
    }

    pub fn get_template(&self, name: &str) -> Result<&str> {
        self.handlebars
            .get_template(name)
            .map(|t| t.source.as_str())
            .context(format!("Template not found: {}", name))
    }

    fn register_builtin_templates(&mut self) {
        // Rust templates
        self.handlebars
            .register_template_string(
                "rust_function",
                r#"pub fn {{name}}({{params}}) -> {{return_type}} {
    // TODO: Implement {{name}}
    unimplemented!()
}
"#,
            )
            .unwrap();

        self.handlebars
            .register_template_string(
                "rust_struct",
                r#"#[derive(Debug, Clone)]
pub struct {{name}} {
    {{#each fields}}
    pub {{this}},
    {{/each}}
}
"#,
            )
            .unwrap();

        // TypeScript templates
        self.handlebars
            .register_template_string(
                "ts_function",
                r#"export function {{name}}({{params}}): {{return_type}} {
    // TODO: Implement {{name}}
    throw new Error('Not implemented');
}
"#,
            )
            .unwrap();

        self.handlebars
            .register_template_string(
                "ts_class",
                r#"export class {{name}} {
    constructor({{constructor_params}}) {
        // TODO: Initialize
    }

    {{#each methods}}
    {{this}}() {
        // TODO: Implement
    }
    {{/each}}
}
"#,
            )
            .unwrap();
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 5: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod generation;
```

**Step 6: Run tests**

Run:
```bash
cargo test templates
```

Expected: PASS (some tests may need adjustment for handlebars syntax)

**Step 7: Fix test for get_template**

The `get_template` method needs adjustment. Update the test:

```rust
#[test]
fn test_rust_function_template() {
    let engine = TemplateEngine::new();

    let mut context = TemplateContext::new();
    context.insert("name".to_string(), "calculate".to_string());
    context.insert("params".to_string(), "x: i32, y: i32".to_string());
    context.insert("return_type".to_string(), "i32".to_string());

    let template = engine.get_template("rust_function").unwrap();
    let result = engine.render(template, &context).unwrap();
    assert!(result.contains("pub fn calculate"));
}
```

**Step 8: Re-run tests**

Run:
```bash
cargo test templates
```

Expected: PASS

**Step 9: Commit**

```bash
git add src/generation/ tests/generation/ src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: implement template-based code generation"
```

---

### Task 10: Implement Generate Command Logic

**Files:**
- Modify: `src/commands/pro/generate.rs`
- Create: `tests/commands/generate_test.rs`

**Step 1: Write integration test**

Create `tests/commands/generate_test.rs`:

```rust
use sentinel_pro::commands::pro::GenerateCommand;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_generate_rust_function() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("generated.rs");

    let cmd = GenerateCommand {
        path: file_path.to_string_lossy().to_string(),
        prompt: Some("create a function named add that takes two integers".to_string()),
        spec: None,
        interactive: false,
        with_tests: false,
        dry_run: false,
    };

    let result = cmd.execute().await;
    assert!(result.is_ok());

    // Check file was created
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("fn add") || content.contains("function"));
}
```

**Step 2: Run test to verify current behavior**

Run:
```bash
cargo test test_generate_rust_function
```

Expected: FAIL - file not created

**Step 3: Implement generate command**

Modify `src/commands/pro/generate.rs`:

```rust
use clap::Args;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use crate::generation::templates::{TemplateEngine, TemplateContext};
use crate::utils::backup::BackupManager;

#[derive(Debug, Args)]
pub struct GenerateCommand {
    /// File to generate
    pub path: String,

    /// Description of what to generate
    #[arg(long)]
    pub prompt: Option<String>,

    /// Specification file (YAML)
    #[arg(long)]
    pub spec: Option<String>,

    /// Interactive mode
    #[arg(long)]
    pub interactive: bool,

    /// Auto-generate tests
    #[arg(long)]
    pub with_tests: bool,

    /// Show without applying
    #[arg(long)]
    pub dry_run: bool,
}

impl GenerateCommand {
    pub async fn execute(&self) -> Result<()> {
        println!("✨ Generating: {}", self.path.bright_cyan());

        let path = Path::new(&self.path);
        let language = self.detect_language(path);

        // Parse prompt for intent
        let content = if let Some(prompt) = &self.prompt {
            self.generate_from_prompt(prompt, &language)?
        } else if let Some(spec_path) = &self.spec {
            self.generate_from_spec(spec_path, &language)?
        } else {
            anyhow::bail!("Either --prompt or --spec must be provided");
        };

        if self.dry_run {
            println!("\n{}", "📄 Generated code (dry-run):".bright_green());
            println!("{}", "─".repeat(50).bright_black());
            println!("{}", content);
            println!("{}", "─".repeat(50).bright_black());
            return Ok(());
        }

        // Backup if file exists
        if path.exists() {
            let backup_dir = path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".sentinel-backups");
            let backup_mgr = BackupManager::new(backup_dir);
            let backup_path = backup_mgr.create_backup(path)?;
            println!("💾 Backup created: {}", backup_path.display().to_string().bright_black());
        }

        // Write file
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &content)
            .context("Failed to write generated file")?;

        println!("✅ Generated: {}", path.display().to_string().bright_green());

        if self.with_tests {
            self.generate_tests(path, &language)?;
        }

        Ok(())
    }

    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("py") => "python".to_string(),
            _ => "rust".to_string(), // default
        }
    }

    fn generate_from_prompt(&self, prompt: &str, language: &str) -> Result<String> {
        let engine = TemplateEngine::new();

        // Simple pattern matching for common cases
        let prompt_lower = prompt.to_lowercase();

        if prompt_lower.contains("function") || prompt_lower.contains("fn") {
            let name = self.extract_function_name(prompt);
            let mut context = TemplateContext::new();
            context.insert("name".to_string(), name);
            context.insert("params".to_string(), "/* TODO: add parameters */".to_string());

            let template_name = match language {
                "rust" => {
                    context.insert("return_type".to_string(), "()".to_string());
                    "rust_function"
                }
                "typescript" | "javascript" => {
                    context.insert("return_type".to_string(), "void".to_string());
                    "ts_function"
                }
                _ => "rust_function",
            };

            let template = engine.get_template(template_name)?;
            engine.render(template, &context)
        } else if prompt_lower.contains("struct") || prompt_lower.contains("class") {
            let name = self.extract_type_name(prompt);
            let mut context = TemplateContext::new();
            context.insert("name".to_string(), name);

            let template_name = match language {
                "rust" => "rust_struct",
                "typescript" | "javascript" => "ts_class",
                _ => "rust_struct",
            };

            let template = engine.get_template(template_name)?;
            engine.render(template, &context)
        } else {
            // Generic template
            Ok(format!(
                "// Generated from prompt: {}\n// TODO: Implement\n\n",
                prompt
            ))
        }
    }

    fn generate_from_spec(&self, _spec_path: &str, _language: &str) -> Result<String> {
        // TODO: Implement YAML spec parsing
        Ok("// TODO: Implement spec-based generation\n".to_string())
    }

    fn extract_function_name(&self, prompt: &str) -> String {
        // Simple extraction: look for "named X" or "called X"
        let words: Vec<&str> = prompt.split_whitespace().collect();

        for i in 0..words.len() {
            if (words[i] == "named" || words[i] == "called") && i + 1 < words.len() {
                return words[i + 1].trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            }
        }

        "generated_function".to_string()
    }

    fn extract_type_name(&self, prompt: &str) -> String {
        let words: Vec<&str> = prompt.split_whitespace().collect();

        for i in 0..words.len() {
            if (words[i] == "named" || words[i] == "called") && i + 1 < words.len() {
                return words[i + 1]
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string();
            }
        }

        "GeneratedType".to_string()
    }

    fn generate_tests(&self, _target_path: &Path, _language: &str) -> Result<()> {
        println!("🧪 Test generation not yet implemented");
        Ok(())
    }
}
```

**Step 4: Run tests**

Run:
```bash
cargo test test_generate_rust_function
```

Expected: PASS

**Step 5: Manual test**

```bash
cargo run -- pro generate /tmp/test.rs --prompt "create a function named add that takes two integers" --dry-run
```

Expected: Shows generated code

**Step 6: Test actual generation**

```bash
cargo run -- pro generate /tmp/calculator.rs --prompt "create a function named multiply"
cat /tmp/calculator.rs
```

Expected: File created with function stub

**Step 7: Commit**

```bash
git add src/commands/pro/generate.rs tests/commands/generate_test.rs
git commit -m "feat: implement basic generate command with template-based generation"
```

---

## Part 5: Implement Refactor Command (Basic)

### Task 11: Code Diff and Safety Utilities

**Files:**
- Create: `src/refactor/mod.rs`
- Create: `src/refactor/safety.rs`
- Create: `tests/refactor/safety_test.rs`

**Step 1: Add similar dependency**

Add to `Cargo.toml`:

```toml
similar = "2.3"
```

**Step 2: Write failing test**

Create `tests/refactor/safety_test.rs`:

```rust
use sentinel_pro::refactor::safety::SafetyChecker;

#[test]
fn test_detect_breaking_change() {
    let original = r#"
pub fn calculate(x: i32) -> i32 {
    x * 2
}
"#;

    let modified = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x * y
}
"#;

    let checker = SafetyChecker::new();
    let is_safe = checker.is_signature_compatible(original, modified);

    assert!(!is_safe); // Signature changed - not safe
}

#[test]
fn test_allow_implementation_change() {
    let original = r#"
pub fn calculate(x: i32) -> i32 {
    x * 2
}
"#;

    let modified = r#"
pub fn calculate(x: i32) -> i32 {
    x << 1  // Same result, different implementation
}
"#;

    let checker = SafetyChecker::new();
    let is_safe = checker.is_signature_compatible(original, modified);

    assert!(is_safe); // Signature same - safe
}
```

**Step 3: Run test to verify failure**

Run:
```bash
cargo test test_detect_breaking_change
```

Expected: FAIL - module not found

**Step 4: Implement safety checker**

Create `src/refactor/mod.rs`:

```rust
pub mod safety;
```

Create `src/refactor/safety.rs`:

```rust
use regex::Regex;

pub struct SafetyChecker {
    fn_signature_regex: Regex,
}

impl SafetyChecker {
    pub fn new() -> Self {
        Self {
            // Matches: pub fn name(params) -> return_type
            fn_signature_regex: Regex::new(
                r"(?m)^\s*(pub\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\\s*([^\{]+))?"
            ).unwrap(),
        }
    }

    pub fn is_signature_compatible(&self, original: &str, modified: &str) -> bool {
        let original_sigs = self.extract_signatures(original);
        let modified_sigs = self.extract_signatures(modified);

        // Check if any function signature changed
        for (name, orig_sig) in &original_sigs {
            if let Some(mod_sig) = modified_sigs.get(name) {
                if orig_sig != mod_sig {
                    return false; // Signature changed
                }
            } else {
                return false; // Function removed
            }
        }

        true
    }

    fn extract_signatures(&self, code: &str) -> std::collections::HashMap<String, String> {
        let mut signatures = std::collections::HashMap::new();

        for cap in self.fn_signature_regex.captures_iter(code) {
            let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let params = cap.get(3).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(4).map(|m| m.as_str()).unwrap_or("()");

            let signature = format!("{}({})->{}", name, params.trim(), return_type.trim());
            signatures.insert(name.to_string(), signature);
        }

        signatures
    }

    pub fn calculate_diff(&self, original: &str, modified: &str) -> Vec<DiffLine> {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(original, modified);
        let mut result = Vec::new();

        for change in diff.iter_all_changes() {
            let line = DiffLine {
                tag: match change.tag() {
                    ChangeTag::Equal => DiffTag::Equal,
                    ChangeTag::Delete => DiffTag::Delete,
                    ChangeTag::Insert => DiffTag::Insert,
                },
                content: change.to_string(),
            };
            result.push(line);
        }

        result
    }
}

impl Default for SafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffTag {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub content: String,
}
```

**Step 5: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod refactor;
```

**Step 6: Run tests**

Run:
```bash
cargo test safety
```

Expected: PASS

**Step 7: Commit**

```bash
git add src/refactor/ tests/refactor/ src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: implement safety checker for refactoring"
```

---

### Task 12: Implement Basic Refactor Command

**Files:**
- Modify: `src/commands/pro/refactor.rs`
- Create: `tests/commands/refactor_test.rs`

**Step 1: Write integration test**

Create `tests/commands/refactor_test.rs`:

```rust
use sentinel_pro::commands::pro::RefactorCommand;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_refactor_with_backup() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.rs");
    fs::write(&file_path, "fn old() { let x = 1; }").unwrap();

    let cmd = RefactorCommand {
        path: file_path.to_string_lossy().to_string(),
        extract_functions: false,
        rename_variables: false,
        remove_dead: false,
        simplify: false,
        safety_first: true,
        backup: true,
    };

    let result = cmd.execute().await;
    assert!(result.is_ok());

    // Check backup was created
    let backup_dir = temp.path().join(".sentinel-backups");
    assert!(backup_dir.exists());
}
```

**Step 2: Implement refactor command**

Modify `src/commands/pro/refactor.rs`:

```rust
use clap::Args;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use crate::utils::backup::BackupManager;
use crate::refactor::safety::SafetyChecker;

#[derive(Debug, Args)]
pub struct RefactorCommand {
    /// File to refactor
    pub path: String,

    /// Extract long functions
    #[arg(long)]
    pub extract_functions: bool,

    /// Rename variables semantically
    #[arg(long)]
    pub rename_variables: bool,

    /// Remove dead code
    #[arg(long)]
    pub remove_dead: bool,

    /// Simplify complex logic
    #[arg(long)]
    pub simplify: bool,

    /// Maximum verification (slower)
    #[arg(long)]
    pub safety_first: bool,

    /// Create backup before refactoring
    #[arg(long)]
    pub backup: bool,
}

impl RefactorCommand {
    pub async fn execute(&self) -> Result<()> {
        let path = Path::new(&self.path);

        if !path.exists() {
            anyhow::bail!("File does not exist: {}", self.path);
        }

        println!("♻️  Refactoring: {}", self.path.bright_cyan());

        // Read original content
        let original_content = fs::read_to_string(path)
            .context("Failed to read file")?;

        // Create backup if requested
        if self.backup {
            let backup_dir = path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".sentinel-backups");
            let backup_mgr = BackupManager::new(backup_dir);
            let backup_path = backup_mgr.create_backup(path)?;
            println!("💾 Backup: {}", backup_path.display().to_string().bright_black());
        }

        // Apply refactorings
        let mut refactored_content = original_content.clone();

        if self.remove_dead {
            refactored_content = self.remove_dead_code(&refactored_content)?;
        }

        if self.simplify {
            refactored_content = self.simplify_code(&refactored_content)?;
        }

        if self.extract_functions {
            println!("⏳ Extract functions: Not yet implemented");
        }

        if self.rename_variables {
            println!("⏳ Rename variables: Not yet implemented");
        }

        // Safety check
        if self.safety_first {
            let checker = SafetyChecker::new();
            if !checker.is_signature_compatible(&original_content, &refactored_content) {
                anyhow::bail!("Refactoring would break function signatures. Aborting.");
            }
            println!("✅ Safety check passed");
        }

        // Show diff
        self.show_diff(&original_content, &refactored_content)?;

        // Write if changed
        if original_content != refactored_content {
            fs::write(path, &refactored_content)
                .context("Failed to write refactored file")?;
            println!("✅ Refactoring complete");
        } else {
            println!("ℹ️  No changes needed");
        }

        Ok(())
    }

    fn remove_dead_code(&self, content: &str) -> Result<String> {
        // Simple implementation: remove commented-out code
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            // Skip lines that are only comments with code-like content
            if trimmed.starts_with("// fn ") ||
               trimmed.starts_with("// let ") ||
               trimmed.starts_with("// pub ") {
                continue; // This looks like commented-out code
            }
            result.push(line);
        }

        Ok(result.join("\n"))
    }

    fn simplify_code(&self, content: &str) -> Result<String> {
        // Simple implementation: Remove unnecessary braces, consolidate imports
        let mut result = content.to_string();

        // Example: if x { true } else { false } -> x
        result = result.replace("if x { true } else { false }", "x");

        Ok(result)
    }

    fn show_diff(&self, original: &str, modified: &str) -> Result<()> {
        if original == modified {
            return Ok(());
        }

        let checker = SafetyChecker::new();
        let diff = checker.calculate_diff(original, modified);

        println!("\n{}", "📝 Changes:".bright_yellow());
        println!("{}", "─".repeat(50).bright_black());

        for line in diff.iter().take(20) {  // Limit output
            match line.tag {
                crate::refactor::safety::DiffTag::Delete => {
                    print!("{}", format!("- {}", line.content).bright_red());
                }
                crate::refactor::safety::DiffTag::Insert => {
                    print!("{}", format!("+ {}", line.content).bright_green());
                }
                crate::refactor::safety::DiffTag::Equal => {
                    // Skip showing unchanged lines for brevity
                }
            }
        }

        Ok(())
    }
}
```

**Step 3: Run tests**

Run:
```bash
cargo test refactor
```

Expected: PASS

**Step 4: Manual test**

Create a test file:
```bash
cat > /tmp/refactor_test.rs << 'EOF'
fn main() {
    println!("hello");
}

// fn old_function() {
//     // This is dead code
// }

fn active() {
    let x = true;
    if x { true } else { false }
}
EOF

cargo run -- pro refactor /tmp/refactor_test.rs --remove-dead --simplify --backup
cat /tmp/refactor_test.rs
```

Expected: Dead code removed, backup created

**Step 5: Commit**

```bash
git add src/commands/pro/refactor.rs tests/commands/refactor_test.rs
git commit -m "feat: implement basic refactor command with safety checks"
```

---

## Part 6: Configuration and Documentation

### Task 13: Configuration File Support

**Files:**
- Create: `src/config/pro.rs`
- Modify: `src/config.rs` (or create if doesn't exist)
- Create: `.sentinelrc-pro.toml` (example)
- Create: `tests/config/pro_test.rs`

**Step 1: Write failing test**

Create `tests/config/pro_test.rs`:

```rust
use sentinel_pro::config::pro::ProConfig;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_config() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join(".sentinelrc-pro.toml");

    fs::write(&config_path, r#"
[general]
version = "1.0"
framework = "rust"

[features]
enable_ml = false
enable_agents = false
"#).unwrap();

    let config = ProConfig::load_from_file(&config_path).unwrap();
    assert_eq!(config.general.framework, "rust");
    assert_eq!(config.features.enable_ml, false);
}

#[test]
fn test_default_config() {
    let config = ProConfig::default();
    assert!(config.features.enable_ml);
}
```

**Step 2: Run test to verify failure**

Run:
```bash
cargo test test_load_config
```

Expected: FAIL - module not found

**Step 3: Add toml dependency**

Add to `Cargo.toml`:

```toml
toml = "0.8"
```

**Step 4: Implement ProConfig**

Create or modify `src/config.rs`:

```rust
pub mod pro;
```

Create `src/config/pro.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProConfig {
    pub general: GeneralConfig,
    pub features: FeaturesConfig,
    pub local_llm: Option<LocalLlmConfig>,
    pub ml: Option<MlConfig>,
    pub knowledge_base: Option<KnowledgeBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub version: String,
    pub framework: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub enable_ml: bool,
    pub enable_agents: bool,
    pub enable_knowledge_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLlmConfig {
    pub provider: String,
    pub model_path: String,
    pub api_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlConfig {
    pub models_path: String,
    pub embeddings_model: String,
    pub bug_predictor_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    pub vector_db_url: String,
    pub index_on_start: bool,
}

impl ProConfig {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;

        toml::from_str(&content)
            .context("Failed to parse config file")
    }

    pub fn load_or_default() -> Self {
        let config_paths = vec![
            ".sentinelrc-pro.toml",
            "sentinel-pro.toml",
            "~/.config/sentinel-pro/config.toml",
        ];

        for path_str in config_paths {
            let path = Path::new(path_str);
            if path.exists() {
                if let Ok(config) = Self::load_from_file(path) {
                    return config;
                }
            }
        }

        Self::default()
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(path, content)
            .context("Failed to write config file")?;

        Ok(())
    }
}

impl Default for ProConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                version: "1.0".to_string(),
                framework: "auto".to_string(),
            },
            features: FeaturesConfig {
                enable_ml: true,
                enable_agents: true,
                enable_knowledge_base: true,
            },
            local_llm: Some(LocalLlmConfig {
                provider: "ollama".to_string(),
                model_path: "~/.ollama/models".to_string(),
                api_port: 11434,
            }),
            ml: Some(MlConfig {
                models_path: ".sentinel/models".to_string(),
                embeddings_model: "codebert".to_string(),
                bug_predictor_model: "bug-predictor-v1".to_string(),
            }),
            knowledge_base: Some(KnowledgeBaseConfig {
                vector_db_url: "http://localhost:6333".to_string(),
                index_on_start: true,
            }),
        }
    }
}
```

**Step 5: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod config;
```

**Step 6: Run tests**

Run:
```bash
cargo test test_load_config
```

Expected: PASS

**Step 7: Create example config**

Create `.sentinelrc-pro.toml`:

```toml
[general]
version = "1.0"
framework = "auto"  # auto-detect or specify: nestjs, laravel, rust, etc.

[features]
enable_ml = true
enable_agents = true
enable_knowledge_base = true

[local_llm]
provider = "ollama"  # ollama, lmstudio, openai
model_path = "~/.ollama/models"
api_port = 11434

[ml]
models_path = ".sentinel/models"
embeddings_model = "codebert"
bug_predictor_model = "bug-predictor-v1"

[knowledge_base]
vector_db_url = "http://localhost:6333"
index_on_start = true
```

**Step 8: Commit**

```bash
git add src/config/ tests/config/ src/lib.rs .sentinelrc-pro.toml Cargo.toml Cargo.lock
git commit -m "feat: add configuration file support for Sentinel Pro"
```

---

### Task 14: Update README and Documentation

**Files:**
- Create: `README.md` (or update existing)
- Create: `docs/PHASE1_GUIDE.md`

**Step 1: Create comprehensive README**

Create or update `README.md`:

```markdown
# Sentinel Pro

> Intelligent CLI for automated code analysis, generation, and refactoring

## Quick Start

### Installation

\`\`\`bash
cargo build --release
cargo install --path .
\`\`\`

### Basic Usage

\`\`\`bash
# Analyze a file
sentinel pro analyze src/main.rs

# Generate new code
sentinel pro generate src/utils.rs --prompt "create a logger function"

# Refactor code safely
sentinel pro refactor src/old.rs --remove-dead --backup
\`\`\`

## Commands

### `sentinel pro analyze`

Deep code analysis with issue detection.

\`\`\`bash
sentinel pro analyze <file> [options]

Options:
  --deep              Deep analysis with all checks
  --security          Focus on security issues
  --performance       Focus on performance issues
  --json              Output as JSON
\`\`\`

### `sentinel pro generate`

AI-powered code generation.

\`\`\`bash
sentinel pro generate <file> [options]

Options:
  --prompt <text>     Description of what to generate
  --spec <file>       YAML specification file
  --with-tests        Auto-generate tests
  --dry-run           Preview without writing
\`\`\`

### `sentinel pro refactor`

Safe automatic refactoring.

\`\`\`bash
sentinel pro refactor <file> [options]

Options:
  --remove-dead       Remove dead code
  --simplify          Simplify complex logic
  --backup            Create backup before changes
  --safety-first      Maximum safety verification
\`\`\`

## Configuration

Create `.sentinelrc-pro.toml` in your project root:

\`\`\`toml
[general]
framework = "rust"

[features]
enable_ml = true
enable_agents = true
\`\`\`

## Development Status

**Phase 1: Fundamentos Pro** ✅ (Current)
- ✅ CLI dispatcher and command structure
- ✅ Basic analyze command
- ✅ Template-based generation
- ✅ Safe refactoring with backups

**Phase 2-7**: See [docs/plans/2025-02-17-sentinel-pro-cli-design.md](docs/plans/2025-02-17-sentinel-pro-cli-design.md)

## Architecture

- **Rust 2024** for performance and safety
- **Clap 4.4** for CLI parsing
- **Tree-sitter** for code parsing
- **Modular design** for future ML/AI integration

## License

MIT
```

**Step 2: Create Phase 1 guide**

Create `docs/PHASE1_GUIDE.md`:

```markdown
# Phase 1 Implementation Guide

## What's Implemented

### Module Structure

\`\`\`
src/
├── commands/pro/       # Pro commands (analyze, generate, refactor)
├── analysis/           # File analysis and issue detection
├── generation/         # Template-based code generation
├── refactor/           # Refactoring with safety checks
├── utils/              # Backup system
├── ai/                 # AI client abstraction (stub)
├── config/             # Configuration management
└── agents/             # Agent system (stub for Phase 2)
\`\`\`

### Features

1. **Analyze Command**
   - File metrics (lines, functions, complexity)
   - Issue detection (long functions, low comments, etc.)
   - JSON output support

2. **Generate Command**
   - Template-based generation
   - Prompt parsing for function/struct generation
   - Backup system integration
   - Dry-run mode

3. **Refactor Command**
   - Dead code removal
   - Code simplification
   - Safety checks (signature compatibility)
   - Automatic backups
   - Diff visualization

4. **Infrastructure**
   - Backup management
   - Config file support
   - Safe file operations

## Testing

\`\`\`bash
# Run all tests
cargo test

# Run specific module tests
cargo test analysis
cargo test generation
cargo test refactor

# Run with output
cargo test -- --nocapture
\`\`\`

## Next Steps (Phase 2)

- Implement multi-agent system
- Add CoderAgent, TesterAgent, RefactorAgent, ReviewerAgent
- Create agent orchestration
- Implement workflows

See full plan in [2025-02-17-sentinel-pro-cli-design.md](plans/2025-02-17-sentinel-pro-cli-design.md)
```

**Step 3: Commit**

```bash
git add README.md docs/PHASE1_GUIDE.md
git commit -m "docs: add README and Phase 1 implementation guide"
```

---

## Part 7: Final Testing and Polish

### Task 15: Integration Tests

**Files:**
- Create: `tests/integration_test.rs`

**Step 1: Write comprehensive integration test**

Create `tests/integration_test.rs`:

```rust
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_analyze_command_integration() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("test.rs");
    fs::write(&file, "fn main() { println!(\"test\"); }").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "pro", "analyze", file.to_str().unwrap()])
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Analyzing"));
}

#[test]
fn test_generate_command_integration() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("generated.rs");

    let output = Command::new("cargo")
        .args(&[
            "run", "--", "pro", "generate",
            file.to_str().unwrap(),
            "--prompt", "create a function named test",
            "--dry-run"
        ])
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Generating"));
}

#[test]
fn test_help_output() {
    let output = Command::new("cargo")
        .args(&["run", "--", "pro", "--help"])
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("analyze"));
    assert!(stdout.contains("generate"));
    assert!(stdout.contains("refactor"));
}
```

**Step 2: Run integration tests**

Run:
```bash
cargo test integration_test
```

Expected: PASS

**Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: add integration tests for Phase 1 commands"
```

---

### Task 16: Error Handling Improvements

**Files:**
- Create: `src/errors.rs`
- Modify relevant command files

**Step 1: Create custom error types**

Create `src/errors.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SentinelError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Refactoring failed: {0}")]
    RefactorError(String),

    #[error("Generation failed: {0}")]
    GenerationError(String),

    #[error("Analysis failed: {0}")]
    AnalysisError(String),

    #[error("AI client error: {0}")]
    AiClientError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, SentinelError>;
```

**Step 2: Update src/lib.rs**

Add to `src/lib.rs`:

```rust
pub mod errors;
pub use errors::{SentinelError, Result};
```

**Step 3: Commit**

```bash
git add src/errors.rs src/lib.rs
git commit -m "feat: add custom error types for better error handling"
```

---

### Task 17: Final Polish and Version Tag

**Files:**
- Modify: `Cargo.toml`
- Create: `CHANGELOG.md`

**Step 1: Update Cargo.toml version**

Modify `Cargo.toml`:

```toml
[package]
name = "sentinel-pro"
version = "0.1.0"  # Phase 1 release
edition = "2024"
authors = ["Sergio Guadarrama"]
description = "Intelligent CLI for automated code analysis, generation, and refactoring"
license = "MIT"
repository = "https://github.com/yourusername/sentinel-pro"
```

**Step 2: Create CHANGELOG**

Create `CHANGELOG.md`:

```markdown
# Changelog

All notable changes to Sentinel Pro will be documented in this file.

## [0.1.0] - 2025-02-17

### Added - Phase 1: Fundamentos Pro

#### Commands
- **`sentinel pro analyze`** - Deep code analysis with metrics and issue detection
- **`sentinel pro generate`** - Template-based code generation from prompts
- **`sentinel pro refactor`** - Safe automatic refactoring with backups

#### Features
- File analysis with metrics (lines, functions, complexity)
- Issue detection (long functions, low comment ratio, large files)
- Template engine for Rust, TypeScript, JavaScript
- Backup system for safe file operations
- Safety checker for signature compatibility
- Configuration file support (`.sentinelrc-pro.toml`)
- JSON output support for analyze command
- Diff visualization for refactoring

#### Infrastructure
- Module structure for future phases (agents, ML, framework engine)
- AI client abstraction layer
- Comprehensive error handling
- Integration and unit tests

### Technical
- Rust 2024 Edition (1.75+)
- Clap 4.4 for CLI
- Tree-sitter for parsing
- Async runtime with Tokio

## [Unreleased]

### Planned - Phase 2: Sistema Multi-Agent
- CoderAgent for code generation
- TesterAgent for test generation
- RefactorAgent for advanced refactoring
- ReviewerAgent for code review
- Agent orchestration and workflows

See [docs/plans/2025-02-17-sentinel-pro-cli-design.md](docs/plans/2025-02-17-sentinel-pro-cli-design.md) for full roadmap.
```

**Step 3: Run final test suite**

Run:
```bash
cargo test
cargo clippy
cargo fmt --check
```

Expected: All pass

**Step 4: Build release binary**

Run:
```bash
cargo build --release
./target/release/sentinel pro --help
```

Expected: Clean build, help displays correctly

**Step 5: Commit and tag**

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: prepare v0.1.0 release - Phase 1 complete"
git tag -a v0.1.0 -m "Phase 1: Fundamentos Pro"
```

---

## Summary and Completion

### Phase 1 Deliverables ✅

1. **CLI Infrastructure**
   - Pro command namespace
   - 3 working commands (analyze, generate, refactor)
   - Configuration system

2. **Core Features**
   - File analysis and metrics
   - Issue detection
   - Template-based generation
   - Safe refactoring

3. **Safety & Quality**
   - Backup system
   - Safety checks
   - Comprehensive tests
   - Error handling

4. **Documentation**
   - README
   - Phase 1 guide
   - CHANGELOG
   - Inline docs

### Verification Checklist

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] All commands work manually
- [ ] Documentation complete
- [ ] Version tagged

### Next Phase

After Phase 1 completion, create new plan for **Phase 2: Sistema Multi-Agent** using this skill again.

---

## Execution Options

Plan complete and saved to `docs/plans/2025-02-17-phase1-fundamentos-pro.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
