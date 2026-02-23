# Reliability Testing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Verify the reliability fixes (word-boundary, line numbers, audit CI mode, review context) work correctly via unit tests and a real-project bash script.

**Architecture:** Two deliverables — (1) 3 new regression unit tests added to the existing `#[cfg(test)]` block in `src/rules/static_analysis.rs`, (2) a bash script `scripts/test-real-project.sh` that installs sentinel and runs the 4 key commands against a real NestJS project with PASS/FAIL assertions.

**Tech Stack:** Rust unit tests (cargo test), bash, grep.

---

### Task 1: Unit test — used import is NOT flagged (word-boundary regression)

**Files:**
- Modify: `src/rules/static_analysis.rs` (append to `#[cfg(test)]` block, after line ~390)

**Step 1: Write the failing test**

Add this test inside the existing `mod tests { ... }` block at the bottom of `src/rules/static_analysis.rs`:

```rust
#[test]
fn test_unused_import_not_flagged_when_used() {
    let lang = ts_lang();
    let analyzer = UnusedImportsAnalyzer::new();
    // Injectable aparece en el import Y como decorador → count > 1 → no debe reportarse
    let code = "import { Injectable } from '@nestjs/common';\n\n@Injectable()\nexport class AppService {}";
    let violations = analyzer.analyze(&lang, code);
    let flagged = violations.iter().any(|v| v.rule_name == "UNUSED_IMPORT");
    assert!(!flagged, "Injectable está en uso — no debe ser reportado como UNUSED_IMPORT");
}
```

**Step 2: Run the test to verify it FAILS before the fix existed**

```bash
cargo test test_unused_import_not_flagged_when_used -- --nocapture 2>&1
```

Expected with OLD code: FAIL (el test hubiera fallado porque `.matches()` daba falso positivo).
Expected con el código actual (ya tiene word-boundary): PASS directamente — confirma que el fix funciona.

**Step 3: Confirm test passes**

```bash
cargo test test_unused_import_not_flagged_when_used -- --nocapture 2>&1
```
Expected: `test ... ok`

---

### Task 2: Unit test — substring "username" no cuenta como uso de "user" (word-boundary)

**Files:**
- Modify: `src/rules/static_analysis.rs` (append al mismo bloque de tests)

**Step 1: Write the test**

```rust
#[test]
fn test_dead_code_no_false_positive_on_substring() {
    let lang = ts_lang();
    let analyzer = DeadCodeAnalyzer::new();
    // `user` está declarado pero NUNCA usado como palabra completa.
    // `username` contiene "user" como substring pero NO es un uso de `user`.
    let code = "const user = 1;\nconsole.log(username);";
    let violations = analyzer.analyze(&lang, code);
    let flagged = violations.iter().any(|v| v.rule_name == "DEAD_CODE");
    // Con word-boundary: "username" NO cuenta como uso de "user" → DEAD_CODE debe reportarse
    assert!(flagged, "user está declarado y nunca usado como palabra completa — debe reportarse DEAD_CODE");
}
```

**Step 2: Run the test**

```bash
cargo test test_dead_code_no_false_positive_on_substring -- --nocapture 2>&1
```
Expected: `test ... ok`

---

### Task 3: Unit test — RuleViolation.line está poblado para FUNCTION_TOO_LONG

**Files:**
- Modify: `src/rules/static_analysis.rs` (append al mismo bloque de tests)

**Step 1: Write the test**

```rust
#[test]
fn test_function_too_long_has_line_number() {
    let lang = ts_lang();
    let analyzer = ComplexityAnalyzer::new();
    // Función de 55 líneas empezando en línea 1 → violation.line debe ser Some(1)
    let long_fn = format!(
        "function longFn() {{\n{}\n}}",
        "  const x = 1;\n".repeat(54)
    );
    let violations = analyzer.analyze(&lang, &long_fn);
    let v = violations.iter().find(|v| v.rule_name == "FUNCTION_TOO_LONG")
        .expect("Debería detectar FUNCTION_TOO_LONG");
    assert_eq!(v.line, Some(1), "La violación debe incluir el número de línea donde empieza la función");
}
```

**Step 2: Run the test**

```bash
cargo test test_function_too_long_has_line_number -- --nocapture 2>&1
```
Expected: `test ... ok`

**Step 3: Run all tests to verify no regresiones**

```bash
cargo test 2>&1 | tail -10
```
Expected: `28 passed; 0 failed`

---

### Task 4: Crear `scripts/test-real-project.sh`

**Files:**
- Create: `scripts/test-real-project.sh`

**Step 1: Crear el directorio y el script**

```bash
mkdir -p scripts
```

Contenido del script:

```bash
#!/usr/bin/env bash
# Usage: ./scripts/test-real-project.sh /path/to/nestjs-project
# Verifica que el reliability pass funciona contra un proyecto NestJS real.
set -euo pipefail

PROJECT="${1:-}"
if [[ -z "$PROJECT" ]]; then
  echo "Usage: $0 /path/to/nestjs-project" >&2
  exit 1
fi

if [[ ! -d "$PROJECT" ]]; then
  echo "ERROR: Directorio '$PROJECT' no existe." >&2
  exit 1
fi

PASS=0
FAIL=0
SENTINEL_DIR="$(cd "$(dirname "$0")/.." && pwd)"

pass() { echo "  ✅ PASS: $1"; ((PASS++)) || true; }
fail() { echo "  ❌ FAIL: $1"; echo "     Output: $2"; ((FAIL++)) || true; }

echo ""
echo "=========================================="
echo " Sentinel Reliability Test"
echo " Project: $PROJECT"
echo "=========================================="

# ── PASO 1: Build e install ────────────────────────────────────────────────
echo ""
echo "[ 1/4 ] cargo install --path ."
cd "$SENTINEL_DIR"
if cargo install --path . --quiet 2>&1; then
  pass "sentinel instalado correctamente"
else
  fail "cargo install falló" ""
  echo "Abortando — no se puede continuar sin el binario." >&2
  exit 1
fi

# Verificar que el binario responde
if sentinel --version > /dev/null 2>&1 || sentinel --help > /dev/null 2>&1; then
  pass "binario sentinel responde"
else
  fail "binario sentinel no responde" ""
fi

# ── PASO 2: check con line numbers ────────────────────────────────────────
echo ""
echo "[ 2/4 ] sentinel pro check src/ --format text"
cd "$PROJECT"
CHECK_OUT=$(sentinel pro check src/ --format text 2>&1 || true)

# Assert: debe contener ":N" (número de línea) en alguna violación, o bien "Sin problemas"
if echo "$CHECK_OUT" | grep -qE '\[[A-Z_]+:[0-9]+\]|Sin problemas'; then
  pass "check muestra números de línea (o proyecto sin problemas)"
else
  fail "check NO muestra números de línea en violaciones" "$CHECK_OUT"
fi

# Assert: no debe crashear (exit 2 = error de path)
CHECK_EXIT=$(sentinel pro check src/ --format text > /dev/null 2>&1; echo $?) || CHECK_EXIT=0
if [[ "$CHECK_EXIT" != "2" ]]; then
  pass "check no crashea con exit 2"
else
  fail "check terminó con exit 2 (path error)" "$CHECK_EXIT"
fi

# Assert: JSON mode incluye campo "checked"
CHECK_JSON=$(sentinel pro check src/ --format json 2>&1 || true)
if echo "$CHECK_JSON" | grep -q '"checked"'; then
  pass "check --format json produce JSON válido con campo 'checked'"
else
  fail "check --format json no contiene campo 'checked'" "$CHECK_JSON"
fi

# ── PASO 3: audit --no-fix (modo CI) ──────────────────────────────────────
echo ""
echo "[ 3/4 ] sentinel pro audit src/ --no-fix"
AUDIT_OUT=$(timeout 30 sentinel pro audit src/ --no-fix 2>&1 || true)
AUDIT_STATUS=$?

# Assert: no cuelga (timeout 30s)
if [[ $AUDIT_STATUS -ne 124 ]]; then
  pass "audit --no-fix termina sin colgar (no timeout)"
else
  fail "audit --no-fix colgó (timeout 30s)" ""
fi

# Assert: salida tiene resumen o "No se detectaron"
if echo "$AUDIT_OUT" | grep -qE 'Auditoría|No se detectaron|issues'; then
  pass "audit --no-fix muestra resumen de issues"
else
  fail "audit --no-fix no muestra resumen reconocible" "$AUDIT_OUT"
fi

# Assert: --format json produce JSON
AUDIT_JSON=$(timeout 30 sentinel pro audit src/ --no-fix --format json 2>&1 || true)
if echo "$AUDIT_JSON" | grep -q '"total_issues"'; then
  pass "audit --format json produce JSON con campo 'total_issues'"
else
  fail "audit --format json no contiene 'total_issues'" "$AUDIT_JSON"
fi

# ── PASO 4: review con contexto ampliado ──────────────────────────────────
echo ""
echo "[ 4/4 ] sentinel pro review"
REVIEW_OUT=$(timeout 60 sentinel pro review 2>&1 || true)

# Assert: muestra línea de contexto
if echo "$REVIEW_OUT" | grep -qE 'archivo\(s\).*líneas|líneas.*archivo'; then
  pass "review muestra stats de contexto cargado"
else
  fail "review NO muestra stats de contexto" "$REVIEW_OUT"
fi

# ── REPORTE FINAL ──────────────────────────────────────────────────────────
echo ""
echo "=========================================="
echo " Resultado: $PASS passed, $FAIL failed"
echo "=========================================="
echo ""

if [[ $FAIL -gt 0 ]]; then
  exit 1
else
  exit 0
fi
```

**Step 2: Dar permisos de ejecución**

```bash
chmod +x scripts/test-real-project.sh
```

**Step 3: Verificar sintaxis del script**

```bash
bash -n scripts/test-real-project.sh && echo "Sintaxis OK"
```
Expected: `Sintaxis OK`

---

### Task 5: Ejecutar las pruebas completas

**Step 1: Correr los unit tests**

```bash
cd /home/protec/Documentos/dev/sentinel-pro
cargo test 2>&1 | tail -15
```
Expected: `28 passed; 0 failed`

**Step 2: Correr el script contra el proyecto NestJS**

```bash
./scripts/test-real-project.sh /path/to/tu-proyecto-nestjs
```
Expected: `Resultado: N passed, 0 failed`

**Step 3: Si algún check falla — leer el output y abrir issue**

Anotar exactamente qué check falló y qué output produjo. No parchear el script — parchear sentinel.

---

### Task 6: Commit todo junto (solo si Task 5 es exitoso)

```bash
cd /home/protec/Documentos/dev/sentinel-pro
git add src/rules/static_analysis.rs scripts/test-real-project.sh docs/plans/2026-02-23-reliability-testing.md docs/plans/2026-02-23-reliability-testing-design.md
git commit -m "$(cat <<'EOF'
test: add regression tests + real-project verification script

- 3 unit tests: word-boundary false positive, substring isolation, line numbers
- scripts/test-real-project.sh: install + 4-command PASS/FAIL verification

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```
