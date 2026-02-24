# Design: Check/Audit Precision — Cross-file Dead Code, Decorator Imports, Audit Batching

**Date:** 2026-02-23
**Status:** Approved
**Context:** Post reliability-pass. Three surgical fixes to eliminate the most common false positives
in `check` and make `audit` usable on real NestJS projects (142+ files).

---

## Problem Statement

1. **check/DEAD_CODE** — `count == 1` heuristic is file-scoped. Exported symbols used in other
   modules are always flagged as dead code. Backend-proleads has dozens of these.

2. **check/UNUSED_IMPORT** — Decorator imports (`@Injectable`, `@ApiProperty`, `@Column`, etc.)
   appear only once in source as `@Name(...)` — regex word-count misses the `@` prefix, so they
   get flagged even though they're actively used.

3. **audit scalability** — 1 LLM call per file sequentially. 142 files = minutes of wall time.
   Unusable in any real workflow or CI pipeline.

---

## Solution: Three Independent Surgical Changes (Option A)

### Change 1 — DEAD_CODE Cross-file via SQLite Index

**File:** `src/rules/static_analysis.rs` — `DeadCodeAnalyzer`

`DeadCodeAnalyzer` gains an optional `index_db: Option<Arc<IndexDb>>`. When available, before
emitting a DEAD_CODE violation it queries the call graph for callers of the symbol in OTHER files.
If callers > 0 → skip. Fallback to current heuristic when index is unavailable or symbol not indexed.

```
analyze(language, source_code, file_path, index_db):
  for each declared symbol:
    if count_word_occurrences(source) == 1:
      if index_db available:
        callers = call_graph.get_callers(symbol, exclude_file=file_path)
        if callers > 0 → skip
      emit DEAD_CODE
```

**Interface change:** `StaticAnalyzer::analyze()` gains `file_path` and `index_db` parameters,
or `DeadCodeAnalyzer` gets a builder that accepts these. To minimize blast radius, prefer a
separate method `analyze_with_context()` that wraps the existing `analyze()`.

### Change 2 — UNUSED_IMPORT Decorator Recognition

**File:** `src/rules/static_analysis.rs` — `UnusedImportsAnalyzer`

Before emitting UNUSED_IMPORT, check if the import name appears as a decorator (`@Name` pattern).
Uses a second regex pass — fast, no structural change.

```
if count_word_occurrences(source, name) == 1:
  if Regex::new(&format!(r"@{}\b", regex::escape(name))).matches(source):
    → skip (decorator usage counts as used)
  emit UNUSED_IMPORT
```

Resolves all decorator-based imports in one shot: `@Injectable`, `@Controller`, `@ApiProperty`,
`@Column`, `@IsEmail`, `@Get`, `@Post`, etc.

### Change 3 — Audit `--max-files` + Module Batching

**Files:** `src/commands/mod.rs` (new flag) + `src/commands/pro.rs` (batching logic)

**New flag:** `--max-files N` (default: 20). Selects the N most recently modified files (by mtime)
when the project exceeds N files. Shows: `"ℹ️ Auditando 20 de 142 archivos (--max-files para más)"`.

**Module batching:** Group selected files by immediate parent directory. Send each group as one
LLM call with all file contents concatenated. Cap per batch: 8 files OR 800 lines (whichever
comes first — split if exceeded).

```
files → sort by mtime desc → take N → group by parent_dir → batches
for each batch:
  context = concat(file contents with === filename === headers)
  1 LLM call → parse JSON issues array
  tag each issue with its source file
```

**Result:** 20 files in 3 modules → 3 LLM calls instead of 20. A 10x reduction for typical projects.

---

## Success Criteria

- `sentinel pro check src/` on backend-proleads reports zero false positives for exported symbols
  that are imported by other modules (index must be populated).
- `@Injectable`, `@Controller`, `@ApiProperty` never appear in UNUSED_IMPORT output.
- `sentinel pro audit src/ --no-fix` on a 142-file project completes in under 60 seconds
  with default `--max-files 20`.
- All existing 28 unit tests continue to pass.
- 3 new unit tests cover each change.

---

## Out of Scope

- TypeScript `interface`/`type` cross-file analysis (next cycle)
- Audit result deduplication across batches
- Parallel batch execution (future optimization)
