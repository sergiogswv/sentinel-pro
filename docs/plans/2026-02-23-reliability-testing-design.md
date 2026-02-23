# Design: Reliability Testing — Real Project Verification

**Date:** 2026-02-23
**Status:** Approved
**Context:** Post reliability-pass (word-boundary matching, line numbers, audit CI mode, review context)

---

## Objective

Verify that the reliability fixes shipped today work correctly against a real NestJS project,
and leave that verification as a reproducible artefact for future regression testing.

## Scope

**In scope:**
- `scripts/test-real-project.sh` — install + run 4 key commands with PASS/FAIL assertions
- 3 new unit tests in `src/rules/static_analysis.rs` — permanent regression coverage

**Out of scope:** new features, new commands, behavior changes.

---

## Deliverable 1 — `scripts/test-real-project.sh`

Accepts `$1` = path to NestJS project. Runs 4 checks in sequence:

| Step | Command | Assertions |
|------|---------|------------|
| 1 | `cargo install --path .` | `sentinel` binary responds |
| 2 | `sentinel pro check $PROJECT/src --format text` | output contains `":N"` (line number); no false positives on substring names |
| 3 | `sentinel pro audit $PROJECT/src --no-fix --format text` | exits without user input; exit code 0 or 1, never crash |
| 4 | `sentinel pro review` (from `$PROJECT`) | output contains `"archivo(s)"` and `"líneas de código cargadas"`; file count ≥ 1 |

- Pure bash + grep/awk, no extra dependencies.
- On failure: prints raw output for diagnosis.
- Returns exit code 1 if any check fails (CI compatible).

---

## Deliverable 2 — Unit Tests in `static_analysis.rs`

Three regression tests covering the exact bugs fixed today:

### `test_unused_import_not_flagged_when_used`
```
code: "import { Injectable } from '@nestjs/common';\n@Injectable()\nclass Svc {}"
assert: violations.is_empty()
```
Verifies: word-boundary prevents false positive when import IS used.

### `test_dead_code_no_false_positive_on_substring`
```
code: "const user = 1;\nconsole.log(username);"
assert: violations contains DEAD_CODE for `user`
```
Verifies: `"username"` does not count as a usage of `"user"` (word-boundary works).

### `test_function_too_long_has_line_number`
```
code: function of 55 lines starting at line 1
assert: violation.line == Some(1)
```
Verifies: `RuleViolation.line` is populated by tree-sitter position.

---

## Success Criteria

- Script exits 0 against a real NestJS project with ≥ 1 TypeScript file.
- All 3 new unit tests pass alongside existing 25 tests.
- `cargo build` clean after adding tests.
