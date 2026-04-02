"""
cerebro_bridge.py — Puente entre Cerebro ↔ Sentinel Core ↔ LLM.

FLUJO por comando:
  1. Cerebro envía OrchestratorCommand { action, target, subcommand }
  2. El action ya está definido — no necesitamos un LLM para decidir qué ejecutar.
  3. Sentinel Core (Rust) ejecuta la acción y retorna raw JSON.
  4. La memoria SQLite persiste el resultado.
  5. El LLM (Gemini/Claude/OpenAI) recibe:
       - El resultado crudo
       - El contexto histórico (hot_files, hallazgos críticos recientes)
     Y produce una síntesis accionable en texto.
  6. El bridge reporta a Cerebro:
       - POST /api/events con el evento estructurado
       - CommandAck con { status, result: { raw, analysis, memory_id } }

CONTRATO MANTENIDO:
  Input:  OrchestratorCommand { action, target?, subcommand?, options?, request_id? }
  Output: CommandAck { request_id?, status, result?, error? }
"""

import uuid
import httpx
from datetime import datetime, timezone
from typing import Optional

from .settings import settings
from . import memory
from .tools import ACTION_MAP, call_core
from .llm_client import analyze_result


# ──────────────────────────────────────────────
# Reporte al Cerebro
# ──────────────────────────────────────────────

async def report_to_cerebro(event_type: str, severity: str, payload: dict):
    """Envía un AgentEvent al endpoint POST /api/events del Cerebro."""
    event = {
        "id": str(uuid.uuid4()),
        "source": "sentinel",
        "type": event_type,
        "severity": severity,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "payload": payload,
    }
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.post(f"{settings.cerebro_url}/api/events", json=event)
            if resp.status_code >= 400:
                print(f"⚠️  [Cerebro] Respuesta inesperada: {resp.status_code}")
            else:
                print(f"✅ [Cerebro] Evento reportado: {event_type} ({severity})")
    except Exception as exc:
        print(f"⚠️  [Cerebro] No disponible: {exc}")


# ──────────────────────────────────────────────
# Handler principal de comandos
# ──────────────────────────────────────────────

async def handle_command(
    action: str,
    target: Optional[str],
    request_id: Optional[str] = None,
    subcommand: Optional[str] = None,
    options: Optional[dict] = None,
) -> dict:
    """
    Procesa un OrchestratorCommand completo.

    Retorna un CommandAck con:
      - raw:      resultado crudo del Core Rust
      - analysis: síntesis del LLM
      - memory_id: ID del hallazgo guardado en SQLite
    """
    # Mapear el subcommand al action real
    actual_action = subcommand or action
    print(f"🛡️ [Sentinel] Procesando: action='{action}' subcommand='{subcommand}' target='{target}'")

    # ── 1. Status — no requiere Core ni LLM ──────────────────────────
    if actual_action == "status":
        ctx = await memory.get_hot_files(5)
        recent = await memory.get_recent_findings(5)
        result_payload = {
            "agent": "sentinel-adk",
            "version": "1.0.0",
            "llm_provider": settings.llm_provider,
            "core_url": settings.sentinel_core_url,
            "hot_files_tracked": len(ctx),
            "recent_findings": len(recent),
        }
        await report_to_cerebro("sentinel_status", "info", result_payload)
        return {
            "request_id": request_id,
            "status": "completed",
            "result": result_payload,
            "error": None,
        }

    # ── 2. Acciones de monitoreo (monitor/*) ───────────────────────────
    if actual_action.startswith("monitor/"):
        monitor_action = actual_action.split("/")[1]
        return await _handle_monitor_command(monitor_action, target, request_id)

    # ── 3. Acciones Pro ─────────────────────────────────────────────
    executor = ACTION_MAP.get(actual_action)
    if not executor:
        # Intentar pasar directo al Core por si es una acción nueva
        print(f"⚠️  Acción '{actual_action}' no en ACTION_MAP — enviando directo al Core")
        raw = await call_core(action, target=target, subcommand=subcommand, options=options)
        return {
            "request_id": request_id,
            "status": raw.get("status", "error"),
            "result": raw.get("result"),
            "error": raw.get("error"),
        }

    # ── 4. Ejecutar en Core Rust ──────────────────────────────────────
    try:
        # Las funciones de ACTION_MAP aceptan target como arg posicional si corresponde
        if actual_action in ("clean-cache", "metrics"):
            raw_result, memory_id = await executor()
        elif actual_action in ("fix",):
            raw_result, memory_id = await executor(target or ".", options)
        else:
            raw_result, memory_id = await executor(target or ".")
    except Exception as exc:
        error_msg = f"Error ejecutando '{actual_action}': {exc}"
        print(f"❌ {error_msg}")
        await report_to_cerebro(f"sentinel_{actual_action}_error", "error", {"error": error_msg, "action": actual_action})
        return {"request_id": request_id, "status": "error", "result": None, "error": error_msg}

    # ── 5. Recuperar contexto histórico de memoria ────────────────────
    mem_context = None
    try:
        hot_files = await memory.get_hot_files(limit=5)
        recent_critical = await memory.get_recent_findings(limit=3, severity_filter="critical")
        mem_context = {
            "hot_files": hot_files,
            "recent_critical_findings": recent_critical,
        }
    except Exception:
        pass  # La memoria falla silenciosamente, no bloquea el análisis

    # ── 6. LLM analiza el resultado crudo + contexto ──────────────────
    analysis = ""
    try:
        analysis = await analyze_result(
            action=actual_action,
            raw_result=raw_result,
            memory_context=mem_context,
        )
    except Exception as exc:
        analysis = f"[Análisis LLM no disponible: {exc}]"
        print(f"⚠️  LLM falló, se retorna raw result: {exc}")

    # ── 7. Determinar severidad final para el evento ──────────────────
    severity = _infer_severity(actual_action, raw_result, analysis)

    # ── 8. Extraer info para Dashboard ────────────────────────────────
    # Extraer finding/recomendación del LLM para el Dashboard
    res_data = raw_result.get("result", {})
    issues_count = 0
    if isinstance(res_data, dict):
        issues_count = len(res_data.get("issues", [])) if "issues" in res_data else 0
        if "files" in res_data:
            for f in res_data["files"]:
                issues_count += len(f.get("issues", []))

    # Construir descripción del problema para Cerebro
    finding_desc = f"Análisis {actual_action}: {issues_count} issues detectados"
    if raw_result.get("status") == "error":
        finding_desc = f"Error en análisis {actual_action}: {raw_result.get('error', 'Unknown error')}"
    elif issues_count == 0:
        finding_desc = f"Análisis {actual_action}: No se detectaron issues"

    # Extraer primer archivo afectado si existe
    affected_file = None
    if isinstance(res_data, dict) and "files" in res_data and len(res_data["files"]) > 0:
        affected_file = res_data["files"][0].get("path")

    # ── 9. Reportar a Cerebro ─────────��───────────────────────────────
    await report_to_cerebro(
        event_type=f"sentinel_{actual_action.replace('-', '_')}_completed",
        severity=severity,
        payload={
            "action": actual_action,
            "target": target,
            "summary": analysis,
            "memory_id": memory_id,
            "raw_status": raw_result.get("status"),
            # Campos para Dashboard (compatibles con Architect/Cerebro)
            "finding": finding_desc,
            "recommendation": analysis[:2000] if analysis else "Revisar hallazgos de calidad detectados",
            "file": affected_file or target,
            "issues_count": issues_count,
        },
    )

    # ── 10. Retornar CommandAck completo ───────────────────────────────
    return {
        "request_id": request_id,
        "status": "completed",
        "result": {
            "action": actual_action,
            "target": target,
            "raw": raw_result,          # Resultado crudo del Core Rust (para Dashboard)
            "analysis": analysis,       # Síntesis del LLM (para Telegram/Dashboard)
            "memory_id": memory_id,
            "severity": severity,
        },
        "error": None,
    }


async def _handle_monitor_command(monitor_action: str, target: Optional[str], request_id: Optional[str]) -> dict:
    """Maneja comandos de monitoreo."""
    executor = ACTION_MAP.get(monitor_action)
    if not executor:
        return {
            "request_id": request_id,
            "status": "error",
            "result": None,
            "error": f"Comando de monitor '{monitor_action}' no reconocido",
        }

    try:
        raw_result, memory_id = await executor(target or ".")
    except Exception as exc:
        error_msg = f"Error ejecutando monitor/{monitor_action}: {exc}"
        print(f"❌ {error_msg}")
        await report_to_cerebro(f"sentinel_monitor_{monitor_action}_error", "error", {"error": error_msg})
        return {"request_id": request_id, "status": "error", "result": None, "error": error_msg}

    # Reportar evento
    event_type_map = {
        "pause": "monitor_pause",
        "daily-report": "daily_report",
        "metrics": "metrics",
        "testing": "testing_suggestions",
    }
    event_type = event_type_map.get(monitor_action, f"monitor_{monitor_action}")

    await report_to_cerebro(
        event_type=f"sentinel_{event_type}",
        severity="info",
        payload={
            "action": f"monitor/{monitor_action}",
            "target": target,
            "result": raw_result,
            "memory_id": memory_id,
        },
    )

    return {
        "request_id": request_id,
        "status": "completed",
        "result": {
            "action": f"monitor/{monitor_action}",
            "target": target,
            "raw": raw_result,
            "memory_id": memory_id,
        },
        "error": None,
    }


def _infer_severity(action: str, raw_result: dict, analysis: str) -> str:
    """
    Determina la severidad del evento reportado a Cerebro.
    Prioridad: error del Core → palabras clave en análisis → action por defecto.
    """
    if raw_result.get("status") == "error":
        return "error"

    analysis_lower = analysis.lower()
    if any(w in analysis_lower for w in ("crítico", "crítica", "critical", "grave", "severo")):
        return "critical"
    if any(w in analysis_lower for w in ("alto riesgo", "vulnerabilidad", "advertencia", "warning", "dead code")):
        return "warning"
    if action in ("audit", "fix"):
        return "warning"
    return "info"
