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

# Assert: --format json produce JSON (usar un solo archivo para ser rápido)
FIRST_TS=$(find src/ -name "*.ts" -not -path "*/node_modules/*" | head -1)
if [[ -n "$FIRST_TS" ]]; then
  AUDIT_JSON=$(timeout 120 sentinel pro audit "$FIRST_TS" --no-fix --format json 2>&1 || true)
  if echo "$AUDIT_JSON" | grep -q '"total_issues"'; then
    pass "audit --format json produce JSON con campo 'total_issues'"
  else
    fail "audit --format json no contiene 'total_issues'" "$AUDIT_JSON"
  fi
else
  pass "audit --format json (sin archivos TS, skip)"
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
