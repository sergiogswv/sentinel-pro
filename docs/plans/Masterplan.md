# Sentinel Pro — Quality Guardian Pivot Design

**Date:** 2026-02-20
**Version target:** 6.0.0
**Status:** Approved

---

## Vision

Sentinel Pro is **not** a code generator. It is the quality guardian that runs alongside AI coding tools (Claude Code, Gemini CLI, Cursor, Copilot) and ensures that AI-generated code respects standards, has no dead code, and does not break business logic.

**Tagline:** "The AI Code Quality Guardian — the layer that makes AI-generated code production-ready."

**Target users:**
- Individual devs (free tier): real-time quality feedback while using AI coding tools
- Development teams (paid tier): custom rules, quality reports, enforcement config

---

## Architecture

### Two-Layer Analysis

```
AI Coding Tool (Claude Code / Gemini CLI / Cursor)
       ↓ (writes/modifies files)
File Watcher (notify crate) — already exists
       ↓
┌─────────────────────────────────────┐
│  LAYER 1: Static Analysis (fast)    │
│  Engine: tree-sitter (real AST)     │
│  - DeadCodeAnalyzer                 │
│  - UnusedImportsAnalyzer            │
│  - ComplexityAnalyzer               │
│  - NamingAnalyzer                   │
│  - FrameworkLayerRules              │
│  Cost: $0, latency: <100ms          │
└──────────────┬──────────────────────┘
               ↓ (only if issues found OR significant change)
┌─────────────────────────────────────┐
│  LAYER 2: AI Semantic Analysis      │
│  - BusinessLogicGuard               │
│  - SecurityContextReviewer          │
│  - ArchitectureViolationDetector    │
│  - TestAnalyzer (why tests fail)    │
│  Cost: tokens (targeted, not bulk)  │
└──────────────┬──────────────────────┘
               ↓
┌─────────────────────────────────────┐
│  FixSuggesterAgent                  │
│  Proposes diffs for detected issues │
│  User approves with Y/n             │
└─────────────────────────────────────┘
```

---

## What Changes

### Remove

| Feature | Reason |
|---------|--------|
| `CoderAgent` (code generation) | Claude Code's job. Creates confusion about Sentinel's role. |
| `sentinel pro generate` | Out of scope |
| `sentinel pro migrate` | Out of scope |
| `sentinel pro chat` | Better tools exist for this |

### Refocus

| Feature | New focus |
|---------|-----------|
| `CoderAgent` → `FixSuggesterAgent` | Only proposes diffs for detected issues, never generates new code |
| Rules engine | String matching → real AST analysis with tree-sitter |
| `ReviewerAgent` | Prompt refocused on business logic regression + contextual security |
| `sentinel pro analyze` | Structured output with severity levels (error/warning/info) |
| `sentinel pro audit` | Full project quality report |

### Add

| Feature | Description |
|---------|-------------|
| `DeadCodeAnalyzer` | tree-sitter: finds declared functions/vars with zero call sites in project |
| `UnusedImportsAnalyzer` | tree-sitter: imports declared vs actually used in file |
| `ComplexityAnalyzer` | Real cyclomatic complexity, function length, nesting depth |
| `NamingAnalyzer` | Framework-specific conventions (camelCase, PascalCase, snake_case) |
| `BusinessLogicGuard` | AI diff: compares before/after to detect logic regressions |
| `TestRunner` (improved) | Executes tests AND analyzes failure reasons with AI |
| Framework profiles | NestJS, Django, Laravel, Spring — specific rules per framework |
| `sentinel pro report` | JSON/HTML quality report for CI/CD or team dashboards |

---

## Framework Support

Multi-framework from v6.0:

| Framework | Language | Priority |
|-----------|----------|----------|
| NestJS | TypeScript | Phase 1 (exists, improve) |
| Django | Python | Phase 2 |
| Laravel | PHP | Phase 2 |
| Spring | Java | Phase 2 |

Framework is auto-detected. Rules are profile-based (YAML per framework).

---

## User Experience

```bash
$ sentinel monitor

👁  Sentinel watching: /my-project [NestJS/TypeScript]
📊 Static analyzer: ready | AI layer: Claude (fallback: Ollama)

# Claude Code modifies user.service.ts — Sentinel reacts automatically:

⚡ [STATIC]  user.service.ts
   ⚠  3 unused imports (lines 1, 4, 7)
   ⚠  getUserById() declared but never called
   ✗  validateEmail() cyclomatic complexity: 14 (max: 10)
   ✗  variable 'temp' assigned but never read (line 34)

🤖 [AI] Business logic check...
   ✗  REGRESSION: updateUser() no longer validates email format
   ⚠  No error handling for DB connection failure

🧪 [TESTS] Running affected tests...
   ✗  user.service.spec.ts — 2 tests failing
      └─ "should validate email" — assertion error

💡 [FIX] Auto-fix available for 3 issues. Apply? [Y/n]
```

---

## Implementation Roadmap

### Phase 1 — Static Foundation (highest priority)
Goal: Layer 1 works correctly with real tree-sitter AST.

- Implement `DeadCodeAnalyzer` with tree-sitter
- Implement `UnusedImportsAnalyzer` with tree-sitter
- Implement `ComplexityAnalyzer` (real cyclomatic complexity)
- Implement `NamingAnalyzer` (per-framework conventions)
- Remove `generate`, `migrate`, `chat` commands
- Rename `CoderAgent` → `FixSuggesterAgent`, remove generation logic
- Upgrade rules engine from string matching to AST-based

### Phase 2 — Multi-Framework Profiles
Goal: Works well beyond NestJS.

- Python/Django framework profile
- PHP/Laravel framework profile
- Java/Spring framework profile
- Naming convention rules per framework
- Architecture layer detection per framework

### Phase 3 — AI Layer Refocused
Goal: AI adds semantic value, not redundant with Layer 1.

- `BusinessLogicGuard`: before/after diff analysis
- `ReviewerAgent` refocused: contextual security + architecture
- `TestRunner` improved: failure analysis with AI
- `FixSuggesterAgent`: Y/n approvable diffs

### Phase 4 — Reporting & Monetization
Goal: Team-ready product.

- `sentinel pro report` → JSON/HTML output
- Quality trend tracking over time
- Custom rules config for team tier
- Integration docs for Claude Code / Gemini CLI workflows

---

## Non-Goals

- Code generation (that's Claude Code / Gemini CLI)
- IDE plugin (daemon approach is sufficient)
- Git hosting or PR management
- Replacing linters (ESLint, Pylint) — complements them

---

## Success Criteria

- Dead code detection catches >90% of unused functions/vars in TypeScript AST test suite
- Layer 1 analysis completes in <200ms per file
- AI layer triggered only when Layer 1 finds issues or diff is semantically significant
- Multi-framework profiles work correctly for NestJS, Django, Laravel, Spring
- `sentinel monitor` output is actionable: dev knows exactly what to fix and why
