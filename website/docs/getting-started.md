---
sidebar_position: 1
---

# Getting Started

Welcome to Sentinel! This guide will help you install and run Sentinel for the first time.

## Installation

### macOS (Homebrew)
```bash
brew install sentinel-pro
```

### Linux/macOS (Cargo)
```bash
cargo install sentinel-pro
```

### Windows (Chocolatey)
```powershell
choco install sentinel-pro
```

### Verify Installation
```bash
sentinel --version
```

## Quick Start

### Initialize a Project
```bash
cd your-project
sentinel init
```

This creates `.sentinelrc.toml` with default configuration.

### Run Audit
```bash
sentinel audit
```

Sentinel will scan your project and report findings.

## Next Steps

- [Custom Rules](./features/custom-rules.md) - Write your own validation rules
- [Pre-commit Integration](./features/pre-commit.md) - Validate on every commit
- [Configuration Reference](./api/config.md) - Understand all options
