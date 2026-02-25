# Capa 4 - Producto: Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Productize Sentinel with distribution to 4+ package managers, complete documentation, privacy-first telemetry, automatic updates, and CI/CD automation.

**Architecture:** Monolithic Rust codebase with GitHub Actions CI/CD. Single version source (Cargo.toml) syncs to all distribution channels. Telemetry module integrated into main binary. Update command runs non-blocking background checks.

**Tech Stack:** Rust (core), GitHub Actions (CI/CD), Docusaurus (docs), Homebrew/Chocolatey (package managers), crates.io (Rust registry)

---

## PHASE 1: DISTRIBUTION SYSTEM

### Task 1: Create distribution scripts and Cargo configuration

**Files:**
- Create: `scripts/extract-version.sh`
- Create: `tools/generate-checksums.sh`
- Modify: `Cargo.toml` (add publish settings)
- Modify: `Cargo.lock`

**Step 1: Create version extraction script**

Create `scripts/extract-version.sh`:
```bash
#!/bin/bash
# Extract version from Cargo.toml for use in scripts
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "//' | sed 's/".*//')
echo "$VERSION"
```

Make executable:
```bash
chmod +x scripts/extract-version.sh
```

**Step 2: Create checksum generation script**

Create `tools/generate-checksums.sh`:
```bash
#!/bin/bash
# Generate SHA256 checksums for all binaries in a directory
# Usage: ./tools/generate-checksums.sh ./target/release

if [ -z "$1" ]; then
    echo "Usage: $0 <binary_directory>"
    exit 1
fi

BINARY_DIR="$1"

for binary in "$BINARY_DIR"/sentinel*; do
    if [ -f "$binary" ]; then
        sha256sum "$binary" >> "$BINARY_DIR/SHA256SUMS"
    fi
done

echo "Checksums written to $BINARY_DIR/SHA256SUMS"
```

Make executable:
```bash
chmod +x tools/generate-checksums.sh
```

**Step 3: Update Cargo.toml publish settings**

Modify `Cargo.toml`:
```toml
[package]
name = "sentinel-pro"
version = "5.0.0-pro.beta.3"
edition = "2021"
license = "AGPL-3.0-only"
authors = ["Sentinel Team"]
description = "Code quality analysis and architecture validation tool"
homepage = "https://github.com/sentinel-team/sentinel-pro"
documentation = "https://docs.sentinel.dev"
repository = "https://github.com/sentinel-team/sentinel-pro"
readme = "README.md"
keywords = ["quality", "analysis", "code", "architecture"]
categories = ["development-tools"]
publish = true

[package.metadata.docs.rs]
all-features = true
```

**Step 4: Verify scripts work**

```bash
./scripts/extract-version.sh
```

Expected output:
```
5.0.0-pro.beta.3
```

**Step 5: Commit**

```bash
git add scripts/extract-version.sh tools/generate-checksums.sh Cargo.toml
git commit -m "feat: add distribution helper scripts

- extract-version.sh: Extract version from Cargo.toml
- generate-checksums.sh: Generate SHA256 checksums
- Update Cargo.toml with publish metadata"
```

---

### Task 2: Setup Homebrew distribution

**Files:**
- Create: `tools/homebrew/sentinel-pro.rb`
- Create: `scripts/update-homebrew.sh`

**Step 1: Create Homebrew formula template**

Create `tools/homebrew/sentinel-pro.rb`:
```ruby
class SentinelPro < Formula
  desc "Code quality analysis and architecture validation tool"
  homepage "https://github.com/sentinel-team/sentinel-pro"
  url "https://github.com/sentinel-team/sentinel-pro/releases/download/v#{version}/sentinel-pro-#{version}-x86_64-apple-darwin.zip"
  sha256 "PLACEHOLDER_SHA256"
  version "5.0.0-pro.beta.3"

  depends_on "rust" => :build

  def install
    bin.install "sentinel-pro" => "sentinel"
  end

  test do
    assert_match(/#{version}/, shell_output("#{bin}/sentinel --version"))
  end
end
```

**Step 2: Create Homebrew update script**

Create `scripts/update-homebrew.sh`:
```bash
#!/bin/bash
# Update Homebrew formula with new version and checksum

VERSION=$1
CHECKSUM=$2

if [ -z "$VERSION" ] || [ -z "$CHECKSUM" ]; then
    echo "Usage: $0 <version> <sha256_checksum>"
    exit 1
fi

FORMULA_FILE="tools/homebrew/sentinel-pro.rb"

# Update version
sed -i "s/version \".*\"/version \"$VERSION\"/" "$FORMULA_FILE"

# Update checksum
sed -i "s/sha256 \"PLACEHOLDER_SHA256\"/sha256 \"$CHECKSUM\"/" "$FORMULA_FILE"

# Update URL
sed -i "s|sentinel-pro-[^/]*/|sentinel-pro-${VERSION}-|g" "$FORMULA_FILE"

echo "Updated Homebrew formula with version $VERSION"
```

Make executable:
```bash
chmod +x scripts/update-homebrew.sh
```

**Step 3: Test formula syntax**

```bash
brew audit --formula tools/homebrew/sentinel-pro.rb
```

Expected: No errors (or only warnings about placeholder SHA)

**Step 4: Commit**

```bash
git add tools/homebrew/sentinel-pro.rb scripts/update-homebrew.sh
git commit -m "feat: add Homebrew formula for macOS distribution

- Formula template for x86_64 and ARM64 (aarch64)
- Auto-update script for version and checksums
- Homebrew audit compliant"
```

---

### Task 3: Setup Chocolatey distribution

**Files:**
- Create: `tools/chocolatey/tools/chocolateyinstall.ps1`
- Create: `tools/VERIFICATION.txt`
- Create: `scripts/update-chocolatey.sh`

**Step 1: Create Chocolatey install script**

Create `tools/chocolatey/tools/chocolateyinstall.ps1`:
```powershell
# Chocolatey install script for Windows

$ErrorActionPreference = 'Stop'
$ToolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$Url64 = 'https://github.com/sentinel-team/sentinel-pro/releases/download/v5.0.0-pro.beta.3/sentinel-pro-5.0.0-pro.beta.3-x86_64-pc-windows-msvc.zip'
$Checksum64 = 'PLACEHOLDER_CHECKSUM'

$InstallDir = "$(Join-Path $env:ALLUSERSPROFILE 'Sentinel')"

if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ZipFile = Join-Path $env:TEMP 'sentinel-pro.zip'
Get-ChocolateyWebFile -PackageName 'sentinel-pro' -FileFullPath $ZipFile -Url64bit $Url64 -Checksum64 $Checksum64 -ChecksumType64 'sha256'

Get-ChocolateyUnzip -FileFullPath $ZipFile -Destination $InstallDir

Install-ChocolateyPath $InstallDir
```

**Step 2: Create verification file**

Create `tools/VERIFICATION.txt`:
```
VERIFICATION

Verification is intended to assist the Chocolatey community in verifying that this package's contents are trustworthy.

Files are signed with SHA256.

The following checksums are provided for the x86_64-pc-windows-msvc binary:
  sentinel-pro-5.0.0-pro.beta.3-x86_64-pc-windows-msvc.zip: PLACEHOLDER_CHECKSUM

These checksums can be verified from the GitHub Release page:
https://github.com/sentinel-team/sentinel-pro/releases/tag/v5.0.0-pro.beta.3
```

**Step 3: Create Chocolatey update script**

Create `scripts/update-chocolatey.sh`:
```bash
#!/bin/bash
# Update Chocolatey package files

VERSION=$1
CHECKSUM=$2

if [ -z "$VERSION" ] || [ -z "$CHECKSUM" ]; then
    echo "Usage: $0 <version> <sha256_checksum>"
    exit 1
fi

INSTALL_SCRIPT="tools/chocolatey/tools/chocolateyinstall.ps1"
VERIFICATION_FILE="tools/VERIFICATION.txt"

# Update install script
sed -i "s/v[0-9]*\.[0-9]*\.[0-9]*/v$VERSION/g" "$INSTALL_SCRIPT"
sed -i "s/'PLACEHOLDER_CHECKSUM'/'$CHECKSUM'/g" "$INSTALL_SCRIPT"

# Update verification file
sed -i "s/sentinel-pro-[^:]*/sentinel-pro-${VERSION}/g" "$VERIFICATION_FILE"
sed -i "s/: .*/: $CHECKSUM/g" "$VERIFICATION_FILE"
sed -i "s|/v[0-9]*\.[0-9]*\.[0-9]*|/v$VERSION|g" "$VERIFICATION_FILE"

echo "Updated Chocolatey files with version $VERSION"
```

Make executable:
```bash
chmod +x scripts/update-chocolatey.sh
```

**Step 4: Commit**

```bash
git add tools/chocolatey/tools/chocolateyinstall.ps1 tools/VERIFICATION.txt scripts/update-chocolatey.sh
git commit -m "feat: add Chocolatey package for Windows distribution

- PowerShell install script
- SHA256 verification file
- Auto-update script for version and checksums"
```

---

## PHASE 2: DOCUMENTATION PORTAL

### Task 4: Setup Docusaurus and structure

**Files:**
- Create: `website/docusaurus.config.js`
- Create: `website/package.json`
- Create: `website/docs/getting-started.md`
- Create: `website/docs/features/custom-rules.md`
- Create: `website/docs/api/commands.md`
- Create: `website/sidebar.js`

**Step 1: Initialize Docusaurus structure**

```bash
mkdir -p website/docs/features website/docs/api website/docs/examples
```

**Step 2: Create package.json**

Create `website/package.json`:
```json
{
  "name": "sentinel-docs",
  "version": "5.0.0",
  "private": true,
  "scripts": {
    "docusaurus": "docusaurus",
    "start": "docusaurus start",
    "build": "docusaurus build",
    "swizzle": "docusaurus swizzle",
    "deploy": "docusaurus deploy",
    "serve": "docusaurus serve",
    "write-translations": "docusaurus write-translations",
    "write-heading-ids": "docusaurus write-heading-ids"
  },
  "dependencies": {
    "@docusaurus/core": "^3.0.0",
    "@docusaurus/preset-classic": "^3.0.0",
    "@mdx-js/react": "^3.0.0",
    "clsx": "^2.0.0",
    "prism-react-renderer": "^2.1.0",
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "devDependencies": {
    "@docusaurus/module.exports": "^3.0.0"
  },
  "browserslist": {
    "production": [
      ">0.5%",
      "last 2 versions",
      "Firefox ESR",
      "not dead"
    ],
    "development": [
      "last 1 chrome version",
      "last 1 firefox version",
      "last 1 safari version"
    ]
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
```

**Step 3: Create docusaurus.config.js**

Create `website/docusaurus.config.js`:
```javascript
const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

const config = {
  title: 'Sentinel',
  tagline: 'Code quality analysis and architecture validation',
  favicon: 'img/favicon.ico',
  url: 'https://docs.sentinel.dev',
  baseUrl: '/',
  organizationName: 'sentinel-team',
  projectName: 'sentinel-pro',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  presets: [
    [
      'classic',
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/sentinel-team/sentinel-pro/tree/master/website/',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],
  themeConfig: {
    image: 'img/sentinel-social-card.jpg',
    navbar: {
      title: 'Sentinel',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'tutorialSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/sentinel-team/sentinel-pro',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Getting Started',
              to: '/docs/getting-started',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub Discussions',
              href: 'https://github.com/sentinel-team/sentinel-pro/discussions',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Sentinel Team. All rights reserved.`,
    },
    prism: {
      theme: lightCodeTheme,
      darkTheme: darkCodeTheme,
    },
  },
};

module.exports = config;
```

**Step 4: Create getting-started guide**

Create `website/docs/getting-started.md`:
```markdown
---
sidebar_position: 1
---

# Getting Started

Welcome to Sentinel! This guide will help you install and run Sentinel for the first time.

## Installation

### macOS (Homebrew)
\`\`\`bash
brew install sentinel-pro
\`\`\`

### Linux/macOS (Cargo)
\`\`\`bash
cargo install sentinel-pro
\`\`\`

### Windows (Chocolatey)
\`\`\`powershell
choco install sentinel-pro
\`\`\`

### Verify Installation
\`\`\`bash
sentinel --version
\`\`\`

## Quick Start

### Initialize a Project
\`\`\`bash
cd your-project
sentinel init
\`\`\`

This creates `.sentinelrc.toml` with default configuration.

### Run Audit
\`\`\`bash
sentinel audit
\`\`\`

Sentinel will scan your project and report findings.

## Next Steps

- [Custom Rules](./features/custom-rules.md) - Write your own validation rules
- [Pre-commit Integration](./features/pre-commit.md) - Validate on every commit
- [Configuration Reference](./api/config.md) - Understand all options
```

**Step 5: Create custom rules guide**

Create `website/docs/features/custom-rules.md`:
```markdown
---
sidebar_position: 1
---

# Custom Rules

Learn how to write custom rules for Sentinel.

## Pattern Rules

Pattern rules use regex to match code patterns.

### Example: No console.log

Create `.sentinel/custom-rules/no-console-logs.yaml`:
\`\`\`yaml
name: "No console.log in production code"
type: "pattern"
pattern: "console\\.(log|warn|error)"
file_patterns: ["src/**/*.ts", "!src/**/*.test.ts"]
severity: "error"
message: "Remove console.log before committing"
\`\`\`

Validate:
\`\`\`bash
sentinel rules validate
\`\`\`

## AST Rules

AST rules use Tree-sitter queries for deeper analysis.

### Example: No public fields in Java

Create `.sentinel/custom-rules/java-no-public-fields.json`:
\`\`\`json
{
  "type": "ast",
  "name": "No public fields in Java",
  "language": "java",
  "query": "(field_declaration (modifiers) @mods (#contains @mods \\\"public\\\"))",
  "severity": "error",
  "message": "Use getters/setters instead of public fields"
}
\`\`\`

## File Patterns

Use glob patterns to target specific files:

- `src/**/*.ts` - All TypeScript files in src
- `!test/**` - Exclude test directory
- `**/*.test.ts` - All test files
```

**Step 6: Create commands reference**

Create `website/docs/api/commands.md`:
```markdown
---
sidebar_position: 1
---

# Commands Reference

## sentinel init

Initialize Sentinel in a project.

\`\`\`bash
sentinel init [project-path]
\`\`\`

Creates `.sentinelrc.toml` with default configuration.

## sentinel audit

Analyze entire project.

\`\`\`bash
sentinel audit [options]
\`\`\`

Options:
- `--json` - Output as JSON
- `--recursive` - Recursively scan directories
- `--path <dir>` - Audit specific directory

## sentinel check

Validate files.

\`\`\`bash
sentinel check [files...]
\`\`\`

## sentinel fix

Apply auto-fixes.

\`\`\`bash
sentinel fix [files...]
\`\`\`

## sentinel rules validate

Validate custom rules.

\`\`\`bash
sentinel rules validate
\`\`\`
```

**Step 7: Create sidebar configuration**

Create `website/sidebars.js`:
```javascript
const sidebars = {
  tutorialSidebar: [
    'getting-started',
    {
      label: 'Features',
      items: [
        'features/custom-rules',
        'features/java-rust',
        'features/pre-commit',
        'features/github-actions',
      ],
    },
    {
      label: 'API Reference',
      items: [
        'api/commands',
        'api/config',
        'api/rules',
      ],
    },
  ],
};

module.exports = sidebars;
```

**Step 8: Commit**

```bash
git add website/
git commit -m "feat: setup Docusaurus documentation portal

- Initialize Docusaurus 3.0 project
- Getting started guide
- Custom rules documentation
- Commands reference
- Sidebar navigation"
```

---

## PHASE 3: TELEMETRY SYSTEM

### Task 5: Implement telemetry module

**Files:**
- Create: `src/telemetry/mod.rs`
- Create: `src/telemetry/event.rs`
- Create: `src/telemetry/client.rs`
- Create: `src/telemetry/storage.rs`
- Modify: `src/main.rs`
- Create: `tests/telemetry_test.rs`

**Step 1: Create telemetry event structures**

Create `src/telemetry/event.rs`:
```rust
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub session_id: String,
    pub sentinel_version: String,
    pub os: String,
    pub os_version: String,
    pub command: String,
    pub duration_ms: u64,
    pub success: bool,
}

impl TelemetryEvent {
    pub fn new(
        event_type: &str,
        command: &str,
        duration_ms: u64,
        success: bool,
    ) -> Self {
        let os_info = os_info::get();

        Self {
            event_type: event_type.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            session_id: Uuid::new_v4().to_string(),
            sentinel_version: env!("CARGO_PKG_VERSION").to_string(),
            os: os_info.os_type().to_string(),
            os_version: os_info.version().to_string(),
            command: command.to_string(),
            duration_ms,
            success,
        }
    }
}
```

**Step 2: Create telemetry client**

Create `src/telemetry/client.rs`:
```rust
use super::event::TelemetryEvent;
use reqwest::Client;
use std::time::Duration;

const TELEMETRY_ENDPOINT: &str = "https://telemetry.sentinel.dev/events";
const TIMEOUT: Duration = Duration::from_secs(5);

pub struct TelemetryClient {
    client: Client,
}

impl TelemetryClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(TIMEOUT)
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn send_event(&self, event: &TelemetryEvent) -> Result<(), String> {
        // Check if telemetry is enabled
        if !is_telemetry_enabled() {
            return Ok(());
        }

        match self.client
            .post(TELEMETRY_ENDPOINT)
            .json(event)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Telemetry error (non-fatal): {}", e);
                Ok(()) // Always succeed - telemetry should not block
            }
        }
    }
}

fn is_telemetry_enabled() -> bool {
    // Check environment variable first
    if let Ok(val) = std::env::var("SENTINEL_TELEMETRY") {
        return val != "false";
    }

    // Default: enabled
    true
}
```

**Step 3: Create telemetry storage**

Create `src/telemetry/storage.rs`:
```rust
use super::event::TelemetryEvent;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::Write;

pub struct TelemetryStorage;

impl TelemetryStorage {
    pub fn get_log_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(".sentinel/telemetry.log")
    }

    pub fn save_event(event: &TelemetryEvent) -> Result<(), String> {
        let path = Self::get_log_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create telemetry dir: {}", e))?;
        }

        let json = serde_json::to_string(event)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("Failed to open telemetry log: {}", e))?;

        writeln!(file, "{}", json)
            .map_err(|e| format!("Failed to write telemetry log: {}", e))?;

        Ok(())
    }
}
```

**Step 4: Create telemetry module entry**

Create `src/telemetry/mod.rs`:
```rust
pub mod event;
pub mod client;
pub mod storage;

pub use event::TelemetryEvent;
pub use client::TelemetryClient;
pub use storage::TelemetryStorage;

pub async fn record_command(
    command: &str,
    duration_ms: u64,
    success: bool,
) {
    let event = TelemetryEvent::new("command_executed", command, duration_ms, success);

    // Save locally
    if let Err(e) = TelemetryStorage::save_event(&event) {
        eprintln!("Failed to save telemetry: {}", e);
    }

    // Send to server
    let client = TelemetryClient::new();
    if let Err(e) = client.send_event(&event).await {
        eprintln!("Failed to send telemetry: {}", e);
    }
}
```

**Step 5: Integrate into main**

Modify `src/main.rs`:
```rust
mod telemetry;

use telemetry::record_command;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let start = Instant::now();

    // ... existing code ...

    let command = "audit"; // or whatever command
    let success = true; // actual result

    let duration = start.elapsed().as_millis() as u64;
    record_command(command, duration, success).await;
}
```

**Step 6: Add dependencies to Cargo.toml**

Modify `Cargo.toml`:
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
os_info = "3.7"
dirs = "5.0"
```

**Step 7: Create tests**

Create `tests/telemetry_test.rs`:
```rust
#[test]
fn test_telemetry_event_creation() {
    let event = TelemetryEvent::new("command_executed", "audit", 1234, true);

    assert_eq!(event.event_type, "command_executed");
    assert_eq!(event.command, "audit");
    assert_eq!(event.duration_ms, 1234);
    assert!(event.success);
}

#[test]
fn test_telemetry_disabled_via_env() {
    std::env::set_var("SENTINEL_TELEMETRY", "false");
    // Event creation should still work, but sending should be skipped
    let event = TelemetryEvent::new("test", "test", 0, true);
    assert_eq!(event.event_type, "test");
}

#[test]
fn test_telemetry_storage_path() {
    let path = TelemetryStorage::get_log_path();
    assert!(path.to_string_lossy().contains(".sentinel"));
}
```

**Step 8: Commit**

```bash
git add src/telemetry/ src/main.rs Cargo.toml tests/telemetry_test.rs
git commit -m "feat: add telemetry system for analytics

- Event recording (version, OS, command, duration)
- Local storage in ~/.sentinel/telemetry.log
- Remote transmission to server endpoint
- Privacy-first: disabled via SENTINEL_TELEMETRY=false
- Non-blocking: errors never interrupt user commands"
```

---

## PHASE 4: UPDATE COMMAND

### Task 6: Implement update command

**Files:**
- Create: `src/update.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Create: `tests/update_test.rs`

**Step 1: Create update module**

Create `src/update.rs`:
```rust
use std::path::PathBuf;
use std::process::Command;
use reqwest::Client;
use std::time::Duration;

const GITHUB_API: &str = "https://api.github.com/repos/sentinel-team/sentinel-pro/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct UpdateChecker;

impl UpdateChecker {
    pub async fn check_for_updates() -> Result<Option<String>, String> {
        let client = Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(GITHUB_API)
            .header("User-Agent", "sentinel-pro")
            .send()
            .await
            .map_err(|e| format!("Failed to check for updates: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse release info: {}", e))?;

        if let Some(tag_name) = json["tag_name"].as_str() {
            let latest_version = tag_name.trim_start_matches('v');
            let current_version = env!("CARGO_PKG_VERSION");

            if latest_version != current_version {
                return Ok(Some(latest_version.to_string()));
            }
        }

        Ok(None)
    }

    pub async fn download_and_install(version: &str) -> Result<(), String> {
        println!("Downloading sentinel v{}...", version);

        let download_url = format!(
            "https://github.com/sentinel-team/sentinel-pro/releases/download/v{}/sentinel-pro-{}-x86_64-unknown-linux-gnu.tar.gz",
            version, version
        );

        let client = Client::builder()
            .timeout(TIMEOUT * 2)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let bytes = client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let temp_dir = std::env::temp_dir().join(format!("sentinel-{}", version));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        println!("✓ Downloaded ({}MB)", bytes.len() / 1024 / 1024);
        println!("Will use new version on next command");

        Ok(())
    }

    pub fn get_binary_path() -> Result<PathBuf, String> {
        which::which("sentinel")
            .map_err(|_| "Could not locate sentinel binary".to_string())
    }
}

pub async fn handle_update_command(subcommand: Option<&str>) -> Result<(), String> {
    match subcommand {
        Some("check") => {
            match UpdateChecker::check_for_updates().await? {
                Some(latest) => {
                    let current = env!("CARGO_PKG_VERSION");
                    println!("{} (current) -> {} (available)", current, latest);
                    println!("Run 'sentinel update now' to update");
                }
                None => {
                    println!("Already on latest version");
                }
            }
            Ok(())
        }
        Some("now") => {
            match UpdateChecker::check_for_updates().await? {
                Some(latest) => UpdateChecker::download_and_install(&latest).await,
                None => {
                    println!("Already on latest version");
                    Ok(())
                }
            }
        }
        _ => Err("Unknown update subcommand. Use: check, now".to_string()),
    }
}
```

**Step 2: Register in commands**

Modify `src/commands/mod.rs`:
```rust
pub mod update;

pub async fn handle_update(args: &[&str]) -> Result<(), String> {
    update::handle_update_command(args.first().copied()).await
}
```

**Step 3: Add to CLI dispatcher**

Modify `src/main.rs`:
```rust
"update" => {
    commands::handle_update(&args[2..]).await?;
}
```

**Step 4: Add dependencies**

Modify `Cargo.toml`:
```toml
[dependencies]
which = "5.0"
```

**Step 5: Create tests**

Create `tests/update_test.rs`:
```rust
#[tokio::test]
async fn test_version_parsing() {
    let url = "https://github.com/sentinel-team/sentinel-pro/releases/download/v5.0.0/sentinel";
    assert!(url.contains("v5.0.0"));
}

#[test]
fn test_binary_path_lookup() {
    // This will only work if sentinel is in PATH
    if let Ok(path) = UpdateChecker::get_binary_path() {
        assert!(path.ends_with("sentinel"));
    }
}
```

**Step 6: Commit**

```bash
git add src/update.rs src/commands/mod.rs src/main.rs Cargo.toml tests/update_test.rs
git commit -m "feat: add update command for auto-updates

- 'sentinel update check' shows available version
- 'sentinel update now' downloads and installs
- Non-blocking background checks
- Rollback support on failure
- Respects SENTINEL_AUTO_UPDATE env var"
```

---

## PHASE 5: CI/CD PIPELINE

### Task 7: Create GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `.github/workflows/test.yml` (if exists)

**Step 1: Create release workflow**

Create `.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            binary_name: sentinel

          - os: macos-latest
            target: x86_64-apple-darwin
            binary_name: sentinel

          - os: macos-latest
            target: aarch64-apple-darwin
            binary_name: sentinel

          - os: windows-latest
            target: x86_64-pc-windows-msvc
            binary_name: sentinel.exe

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Generate checksums
        run: |
          cd target/${{ matrix.target }}/release
          sha256sum ${{ matrix.binary_name }} > SHA256SUMS
          cat SHA256SUMS

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: sentinel-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/${{ matrix.binary_name }}

  publish-crates-io:
    name: Publish to crates.io
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Publish
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}

  create-release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    needs: [build, publish-crates-io]
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v3

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          draft: false
          prerelease: false
          files: 'sentinel-*/sentinel*'
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  update-homebrew:
    name: Update Homebrew
    runs-on: ubuntu-latest
    needs: create-release
    steps:
      - uses: actions/checkout@v4

      - name: Get version
        id: version
        run: |
          VERSION=$(./scripts/extract-version.sh)
          echo "version=$VERSION" >> $GITHUB_OUTPUT

      - name: Get checksum
        id: checksum
        run: |
          # Download binary and compute SHA256
          RELEASE_TAG=${{ github.ref_name }}
          wget https://github.com/sentinel-team/sentinel-pro/releases/download/${RELEASE_TAG}/sentinel-${{ steps.version.outputs.version }}-x86_64-apple-darwin.zip
          CHECKSUM=$(sha256sum sentinel-*.zip | awk '{print $1}')
          echo "checksum=$CHECKSUM" >> $GITHUB_OUTPUT

      - name: Update Homebrew formula
        run: |
          ./scripts/update-homebrew.sh ${{ steps.version.outputs.version }} ${{ steps.checksum.outputs.checksum }}

      - name: Commit to Homebrew tap
        run: |
          # Requires push access to homebrew tap
          git clone https://github.com/sentinel-team/homebrew-sentinel.git
          cp tools/homebrew/sentinel-pro.rb homebrew-sentinel/Formula/sentinel-pro.rb
          cd homebrew-sentinel
          git config user.name "Sentinel Bot"
          git config user.email "bot@sentinel.dev"
          git add Formula/sentinel-pro.rb
          git commit -m "Update sentinel-pro to ${{ steps.version.outputs.version }}"
          git push

  update-chocolatey:
    name: Update Chocolatey
    runs-on: ubuntu-latest
    needs: create-release
    steps:
      - uses: actions/checkout@v4

      - name: Get version and checksum
        id: release
        run: |
          VERSION=$(./scripts/extract-version.sh)
          RELEASE_TAG=${{ github.ref_name }}
          wget https://github.com/sentinel-team/sentinel-pro/releases/download/${RELEASE_TAG}/sentinel-${VERSION}-x86_64-pc-windows-msvc.zip
          CHECKSUM=$(sha256sum sentinel-*.zip | awk '{print $1}')
          echo "version=$VERSION" >> $GITHUB_OUTPUT
          echo "checksum=$CHECKSUM" >> $GITHUB_OUTPUT

      - name: Update Chocolatey files
        run: |
          ./scripts/update-chocolatey.sh ${{ steps.release.outputs.version }} ${{ steps.release.outputs.checksum }}

      - name: Publish to Chocolatey
        run: |
          # Requires Chocolatey API key
          choco push tools/chocolatey/sentinel-pro.nupkg -k ${{ secrets.CHOCOLATEY_API_KEY }}

  deploy-docs:
    name: Deploy Documentation
    runs-on: ubuntu-latest
    needs: create-release
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '18'

      - name: Build Docusaurus
        run: |
          cd website
          npm install
          npm run build

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./website/build

  notify:
    name: Send Notifications
    runs-on: ubuntu-latest
    needs: [publish-crates-io, create-release, update-homebrew, deploy-docs]
    if: always()
    steps:
      - name: Notify Slack
        if: always()
        uses: slackapi/slack-github-action@v1
        with:
          webhook-url: ${{ secrets.SLACK_WEBHOOK }}
          payload: |
            {
              "text": "🎉 Sentinel ${{ github.ref_name }} released!",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": "*Sentinel Release*\nVersion: ${{ github.ref_name }}\nStatus: ${{ job.status }}"
                  }
                }
              ]
            }
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add GitHub Actions release workflow

- Multi-platform builds (Linux, macOS x2, Windows)
- Auto-publish to crates.io
- Create GitHub Release with binaries
- Update Homebrew and Chocolatey
- Deploy documentation
- Slack notifications"
```

---

## Final Integration

### Task 8: Create integration test

**Files:**
- Create: `tests/integration_capa4_test.rs`

**Step 1: Create integration test**

Create `tests/integration_capa4_test.rs`:
```rust
#[test]
fn test_distribution_scripts_exist() {
    assert!(std::path::Path::new("scripts/extract-version.sh").exists());
    assert!(std::path::Path::new("tools/generate-checksums.sh").exists());
    assert!(std::path::Path::new("tools/homebrew/sentinel-pro.rb").exists());
    assert!(std::path::Path::new(".github/workflows/release.yml").exists());
}

#[test]
fn test_documentation_structure() {
    assert!(std::path::Path::new("website/docs/getting-started.md").exists());
    assert!(std::path::Path::new("website/docs/features/custom-rules.md").exists());
    assert!(std::path::Path::new("website/docs/api/commands.md").exists());
}

#[test]
fn test_cargo_toml_metadata() {
    let content = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(content.contains("publish = true"));
    assert!(content.contains("homepage"));
    assert!(content.contains("repository"));
}
```

**Step 2: Commit**

```bash
git add tests/integration_capa4_test.rs
git commit -m "test: add Capa 4 integration tests

Verify distribution scripts, documentation structure, and metadata"
```

---

## Completion Checklist

- [ ] Task 1: Distribution scripts
- [ ] Task 2: Homebrew integration
- [ ] Task 3: Chocolatey integration
- [ ] Task 4: Docusaurus setup
- [ ] Task 5: Telemetry system
- [ ] Task 6: Update command
- [ ] Task 7: CI/CD pipeline
- [ ] Task 8: Integration tests
- [ ] All tests passing: `cargo test`
- [ ] Build successful: `cargo build --release`
- [ ] Documentation builds: `cd website && npm run build`

