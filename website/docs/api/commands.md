---
sidebar_position: 1
---

# Commands Reference

## sentinel init

Initialize Sentinel in a project.

```bash
sentinel init [project-path]
```

Creates `.sentinelrc.toml` with default configuration.

## sentinel audit

Analyze entire project.

```bash
sentinel audit [options]
```

Options:
- `--json` - Output as JSON
- `--recursive` - Recursively scan directories
- `--path <dir>` - Audit specific directory

## sentinel check

Validate files.

```bash
sentinel check [files...]
```

## sentinel fix

Apply auto-fixes.

```bash
sentinel fix [files...]
```

## sentinel rules validate

Validate custom rules.

```bash
sentinel rules validate
```
