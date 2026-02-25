---
sidebar_position: 1
---

# Custom Rules

Learn how to write custom rules for Sentinel.

## Pattern Rules

Pattern rules use regex to match code patterns.

### Example: No console.log

Create `.sentinel/custom-rules/no-console-logs.yaml`:
```yaml
name: "No console.log in production code"
type: "pattern"
pattern: "console\\.(log|warn|error)"
file_patterns: ["src/**/*.ts", "!src/**/*.test.ts"]
severity: "error"
message: "Remove console.log before committing"
```

Validate:
```bash
sentinel rules validate
```

## AST Rules

AST rules use Tree-sitter queries for deeper analysis.

### Example: No public fields in Java

Create `.sentinel/custom-rules/java-no-public-fields.json`:
```json
{
  "type": "ast",
  "name": "No public fields in Java",
  "language": "java",
  "query": "(field_declaration (modifiers) @mods (#contains @mods \\\"public\\\"))",
  "severity": "error",
  "message": "Use getters/setters instead of public fields"
}
```

## File Patterns

Use glob patterns to target specific files:

- `src/**/*.ts` - All TypeScript files in src
- `!test/**` - Exclude test directory
- `**/*.test.ts` - All test files
