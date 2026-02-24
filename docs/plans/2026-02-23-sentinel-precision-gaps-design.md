# Design: Sentinel Precision Gaps — Cold-Start Warning, Batching Tests, Flat-Project Grouping, TS-First Docs

**Date:** 2026-02-23
**Status:** Approved
**Context:** Post check/audit-precision pass. Four surgical fixes to close the remaining UX and reliability gaps
identified after testing against backend-proleads (NestJS, 142 files).

---

## Problem Statement

1. **Cold-start silent degradation** — When `sentinel pro check/audit/review` runs without a prior
   `sentinel monitor`, the SQLite call_graph is empty. Cross-file dead code filtering silently does
   nothing. Users see false positives with no explanation.

2. **Audit batching has zero unit tests** — The `build_audit_batches` logic (group by parent dir,
   split by 8-file / 800-line cap) is inline in the Audit handler. Any regression goes undetected.

3. **Flat-project grouping is random** — When all selected files share the same parent dir (e.g.
   a NestJS project with everything in `src/`), batches are split by cap only. Related files
   (`user.service.ts`, `user.controller.ts`) may end up in different batches.

4. **TypeScript-only scope is undocumented** — Static analysis (DEAD_CODE, UNUSED_IMPORT,
   FUNCTION_TOO_LONG, etc.) only works for TS/JS. Users running `check` on Go or Python projects
   get zero violations with no explanation.

---

## Solution: Four Independent Surgical Changes

### Change 1 — Cold-Start Warning

**File:** `src/index/db.rs` + `src/commands/pro.rs`

Add `IndexDb::is_populated() -> bool`:
```sql
SELECT COUNT(*) FROM call_graph
```
Returns `true` if count > 0.

In `handle_pro_command()`, after building `agent_context` (before any subcommand dispatch):
```
if index_db.is_some() && !index_db.is_populated():
    print warning once (text mode)
    or add "index_populated": false to JSON output (json mode)
    continue normally — analysis degrades gracefully
```

Warning text:
```
⚠️  ÍNDICE VACÍO — Ejecuta `sentinel monitor` primero para activar el análisis
    cross-file (dead code exportado, dependencias entre módulos).
    Continuando con análisis de archivo único...
```

- Printed once per command invocation, not per file
- JSON mode: `"index_populated": false` field in top-level output
- No `--skip-index-check` flag — YAGNI

### Change 2 — Extract and Test Audit Batching

**File:** `src/commands/pro.rs`

Extract batching logic into a free function:
```rust
pub fn build_audit_batches(
    files: &[PathBuf],
    max_files_per_batch: usize,
    max_lines_per_batch: usize,
) -> Vec<Vec<PathBuf>>
```

Three unit tests:
- `test_batch_groups_by_parent_dir` — files in different dirs go to separate batches
- `test_batch_splits_large_dir` — 10 files in same dir → 2 sub-batches of ≤8
- `test_batch_flat_project_prefix` — all files in `src/` → grouped by module prefix (see Change 3)

### Change 3 — Flat-Project Prefix Grouping

**File:** `src/commands/pro.rs` — inside `build_audit_batches`

When a directory produces more files than `MAX_FILES_PER_BATCH`, instead of random order,
group by module prefix (everything before the first `.` in the filename):

```
user.service.ts    → prefix: "user"  ┐
user.controller.ts → prefix: "user"  ├ batch together
user.module.ts     → prefix: "user"  ┘
auth.service.ts    → prefix: "auth"  ┐
auth.guard.ts      → prefix: "auth"  ├ batch together
auth.module.ts     → prefix: "auth"  ┘
```

Algorithm:
```
files → group by module_prefix(filename) → HashMap<String, Vec<PathBuf>>
for each prefix_group:
  apply 8-file / 800-line split as before
```

`module_prefix(filename)` = filename stem before first `.`, or full stem if no `.` in stem.

Fallback: files without a recognizable prefix (e.g. `index.ts`, `main.ts`) get their own group.

### Change 4 — TypeScript-First Documentation

**File:** `src/commands/pro.rs` (check handler) + `README.md`

In check handler, before the per-file loop:
```rust
let has_ts_js = files_to_check.iter().any(|f| {
    matches!(f.extension().and_then(|e| e.to_str()),
             Some("ts" | "js" | "tsx" | "jsx"))
});
if !has_ts_js && !json_mode {
    println!("ℹ️  Análisis estático optimizado para TypeScript/JavaScript.");
    println!("    Soporte para Go, Python, Java y otros: próxima versión.");
}
```

In README.md, add "Lenguajes soportados" section:
```markdown
| Lenguaje | Análisis estático | Monitor | Audit/Review |
|----------|-------------------|---------|--------------|
| TypeScript / JavaScript | ✅ Completo | ✅ | ✅ |
| Go, Python, Java, Rust, otros | 🔜 En desarrollo | ✅ | ✅ |
```

Note: Audit and Review use LLM directly and work with any language. Only static analysis
(DEAD_CODE, UNUSED_IMPORT, FUNCTION_TOO_LONG, etc.) is TS/JS-only.

---

## Success Criteria

- `sentinel pro check src/` on a cold project (no prior `sentinel monitor`) prints the index
  warning before results, does not exit, and continues with file-level analysis only.
- `sentinel pro check src/ --format json` includes `"index_populated": false` in output.
- `build_audit_batches()` has 3 unit tests, all passing.
- `sentinel pro audit src/` on a flat NestJS project groups `user.*` files together in the same batch.
- `sentinel pro check src/` on a Go/Python project shows the TS-first note.
- All 31 existing tests continue to pass.
- 3 new unit tests for batching (+1 for `is_populated`).

---

## Out of Scope

- Static analysis for Go, Python, Java (next major version)
- Parallel batch execution
- Auto-indexing on cold start (would require running tree-sitter indexer inline — adds latency)
- `--skip-index-check` flag
