"""
tools.py — Wrappers HTTP hacia el Sentinel Core (Rust, :4001).

FILOSOFÍA:
  Estas funciones NO toman decisiones. Solo ejecutan la acción que
  Cerebro ya decidió y persisten el resultado crudo en memoria.
  El análisis/razonamiento lo hace el LLM en cerebro_bridge.py.

  Flujo:
    Cerebro solicita action  →  tool llama al Core Rust
    Core retorna raw JSON    →  tool persiste en SQLite
    raw JSON sube al bridge  →  LLM analiza y produce síntesis
    síntesis va a Cerebro    →  Cerebro decide qué hacer

Contratos (mismo que el Core Rust):
  - Input:  { action, target?, subcommand?, options? }
  - Output: CommandAck { status, result?, error? }
"""

import time
import httpx
from typing import Optional

from .settings import settings
from . import memory

CORE = settings.sentinel_core_url


# ──────────────────────────────────────────────
# Helper interno de llamada al Core Rust
# ──────────────────────────────────────────────

async def call_core(action: str, target: Optional[str] = None, subcommand: Optional[str] = None, options: Optional[dict] = None) -> dict:
    """
    Envía un OrchestratorCommand al Sentinel Core (Rust).
    Es el único punto de contacto con el proceso Rust.
    Maneja errores de red sin crashear el sidecar.
    """
    payload: dict = {"action": action}
    if target:
        payload["target"] = target
    if subcommand:
        payload["subcommand"] = subcommand
    if options:
        payload["options"] = options

    try:
        async with httpx.AsyncClient(timeout=120.0) as client:
            resp = await client.post(f"{CORE}/command", json=payload)
            resp.raise_for_status()
            return resp.json()
    except httpx.ConnectError:
        return {"status": "error", "error": f"Sentinel Core no disponible en {CORE}"}
    except httpx.TimeoutException:
        return {"status": "error", "error": "Timeout al llamar a Sentinel Core (>120s)"}
    except Exception as exc:
        return {"status": "error", "error": str(exc)}


# ──────────────────────────────────────────────
# Acciones Pro disponibles con persistencia
# Cada función = 1 acción del Core + persistencia en SQLite
# ──────────────────────────────────────────────

async def execute_check(target: str = ".") -> tuple[dict, int]:
    """Check rápido: dead code, unused imports, complejidad. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("pro", target=target, subcommand="check")
    duration_ms = int((time.monotonic() - start) * 1000)

    severity = "error" if result.get("status") == "error" else "info"
    mid = await memory.save_finding("check_completed", severity, result, target)
    await memory.save_analysis_run("check", target, {"status": result.get("status")}, duration_ms)

    # Actualizar perfiles de calidad si hay resultados
    if result.get("status") == "completed":
        res_data = result.get("result", {})
        files = res_data.get("files", [])
        for file_data in files:
            await memory.update_quality_profile(
                file_path=file_data.get("path", "unknown"),
                issues=len(file_data.get("issues", [])),
                complexity=file_data.get("complexity", 0.0),
                dead_code=len(file_data.get("dead_code", [])),
                unused_imports=len(file_data.get("unused_imports", [])),
            )

    return result, mid


async def execute_audit(target: str = ".") -> tuple[dict, int]:
    """Auditoría completa con ReviewerAgent. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("pro", target=target, subcommand="audit")
    duration_ms = int((time.monotonic() - start) * 1000)

    severity = "error" if result.get("status") == "error" else "warning"
    mid = await memory.save_finding("audit_completed", severity, result, target)
    await memory.save_analysis_run("audit", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_report(target: str = ".") -> tuple[dict, int]:
    """Reporte de calidad. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("pro", target=target, subcommand="report")
    duration_ms = int((time.monotonic() - start) * 1000)

    mid = await memory.save_finding("report_completed", "info", result, target)
    await memory.save_analysis_run("report", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_fix(target: str = ".", options: Optional[dict] = None) -> tuple[dict, int]:
    """Auto-fix de bugs. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("pro", target=target, subcommand="fix", options=options)
    duration_ms = int((time.monotonic() - start) * 1000)

    severity = "warning" if result.get("status") == "error" else "info"
    mid = await memory.save_finding("fix_completed", severity, result, target)
    await memory.save_analysis_run("fix", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_review(target: str = ".") -> tuple[dict, int]:
    """Review de arquitectura. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("pro", target=target, subcommand="review")
    duration_ms = int((time.monotonic() - start) * 1000)

    mid = await memory.save_finding("review_completed", "info", result, target)
    await memory.save_analysis_run("review", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_clean_cache(target: str = ".") -> tuple[dict, int]:
    """Limpieza de caché de IA. Retorna (raw_result, memory_id)."""
    result = await call_core("pro", target=target, subcommand="clean-cache")
    mid = await memory.save_finding("cache_cleaned", "info", result, target)
    return result, mid


# ──────────────────────────────────────────────
# Acciones de Monitor
# ──────────────────────────────────────────────

async def execute_monitor_pause(target: str = ".") -> tuple[dict, int]:
    """Pausa/reanuda monitoreo. Retorna (raw_result, memory_id)."""
    result = await call_core("monitor/pause", target=target)
    mid = await memory.save_finding("monitor_pause", "info", result, target)
    return result, mid


async def execute_monitor_daily_report(target: str = ".") -> tuple[dict, int]:
    """Reporte diario de productividad. Retorna (raw_result, memory_id)."""
    result = await call_core("monitor/daily-report", target=target)
    mid = await memory.save_finding("daily_report_completed", "info", result, target)
    return result, mid


async def execute_monitor_metrics(target: str = ".") -> tuple[dict, int]:
    """Métricas de Sentinel. Retorna (raw_result, memory_id)."""
    result = await call_core("monitor/metrics", target=target)
    mid = await memory.save_finding("metrics", "info", result, target)
    return result, mid


async def execute_monitor_testing(target: str = ".") -> tuple[dict, int]:
    """Sugerencias de testing. Retorna (raw_result, memory_id)."""
    result = await call_core("monitor/testing", target=target)
    mid = await memory.save_finding("testing_suggestions", "info", result, target)
    return result, mid


# ──────────────────────────────────────────────
# Mapa de acciones (usado en cerebro_bridge)
# ──────────────────────────────────────────────

ACTION_MAP = {
    "check":        execute_check,
    "audit":        execute_audit,
    "report":       execute_report,
    "fix":          execute_fix,
    "review":       execute_review,
    "clean-cache":  execute_clean_cache,
    "pause":        execute_monitor_pause,
    "daily-report": execute_monitor_daily_report,
    "metrics":      execute_monitor_metrics,
    "testing":      execute_monitor_testing,
}
