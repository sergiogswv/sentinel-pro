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
import logging
from datetime import datetime, timezone
from typing import Optional

from .settings import settings
from . import memory
from .tools import ACTION_MAP, call_core
from .llm_client import analyze_result

logger = logging.getLogger('sentinel_adk')


# ──────────────────────────────────────────────
# Reporte al Cerebro
# ──────────────────────────────────────────────

async def report_to_cerebro(event_type: str, severity: str, payload: dict, max_retries: int = 3) -> bool:
    """
    Envía un AgentEvent al endpoint POST /api/events del Cerebro.

    Incluye reintentos con backoff exponencial para garantizar entrega.
    Retorna True si el evento fue entregado exitosamente.
    """
    import asyncio

    event = {
        "id": str(uuid.uuid4()),
        "source": "sentinel",
        "type": event_type,
        "severity": severity,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "payload": payload,
    }

    # Logging estructurado para debugging
    logger.info(f"📤 [Cerebro] Enviando evento: {event_type} ({severity}) - ID: {event['id'][:8]}")

    for attempt in range(max_retries):
        try:
            # Timeout aumentado de 5s a 20s para dar tiempo a Cerebro de procesar
            async with httpx.AsyncClient(timeout=20.0) as client:
                resp = await client.post(f"{settings.cerebro_url}/api/events", json=event)

                if resp.status_code == 200:
                    logger.info(f"✅ [Cerebro] Evento entregado: {event_type} (attempt {attempt + 1})")
                    print(f"✅ [Cerebro] Evento reportado: {event_type} ({severity})")
                    return True
                else:
                    logger.warning(f"⚠️  [Cerebro] HTTP {resp.status_code}: {resp.text[:200]}")
                    print(f"⚠️  [Cerebro] Respuesta inesperada: {resp.status_code}")

        except httpx.TimeoutException:
            logger.warning(f"⏱️  [Cerebro] Timeout (attempt {attempt + 1}/{max_retries})")
        except Exception as exc:
            logger.error(f"❌ [Cerebro] Error: {exc} (attempt {attempt + 1}/{max_retries})")
            print(f"⚠️  [Cerebro] No disponible: {exc}")

        # Backoff exponencial: 1s, 2s, 4s
        if attempt < max_retries - 1:
            wait_time = 2 ** attempt
            logger.info(f"🔄 [Cerebro] Reintentando en {wait_time}s...")
            await asyncio.sleep(wait_time)

    logger.error(f"❌ [Cerebro] Falló entrega después de {max_retries} intentos: {event_type}")
    return False


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
    msg = f"🛡️ [Sentinel] Procesando: action='{action}' subcommand='{subcommand}' target='{target}' options={options}"
    print(msg)
    logger.info(msg)

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
        return await _handle_monitor_command(monitor_action, target, request_id, options)

    # ── 2b. Comando monitor sin subcomando ────────────────────────────
    # El modo ADK NO soporta monitoreo continuo de archivos (file watching).
    # Eso solo lo hace el Core Rust. Reportamos que no está soportado.
    if actual_action == "monitor":
        logger.warning(f"⚠️ [Sentinel] Comando 'monitor' recibido pero ADK no soporta monitoreo continuo")
        print(f"⚠️ [Sentinel] ADK no soporta monitoreo continuo. Usa Sentinel Core para file watching.")

        # Reportar al Cerebro que no está soportado
        await report_to_cerebro(
            "sentinel_monitor_not_supported",
            "warning",
            {
                "action": "monitor",
                "target": target,
                "message": "El modo ADK no soporta monitoreo continuo de archivos. Usa modo 'core' para file watching.",
                "recommendation": "Cambia SENTINEL_MODE=core en la configuración para habilitar monitoreo de archivos",
            }
        )

        return {
            "request_id": request_id,
            "status": "rejected",
            "result": None,
            "error": "El modo ADK no soporta monitoreo continuo de archivos. Usa SENTINEL_MODE=core",
        }

    # ── 3. Acciones Pro ─────────────────────────────────────────────
    executor = ACTION_MAP.get(actual_action)
    if not executor:
        # Intentar pasar directo al Core por si es una acción nueva
        print(f"⚠️  Acción '{actual_action}' no en ACTION_MAP — enviando directo al Core")
        raw = await call_core(action, target=target, subcommand=subcommand, options=options, request_id=request_id)
        # Reportar resultado a Cerebro aunque sea error
        await report_to_cerebro(
            f"sentinel_{actual_action}_completed",
            "error" if raw.get("status") == "error" else "info",
            {
                "action": actual_action,
                "target": target,
                "file": target,
                "raw_status": raw.get("status"),
                "finding": f"Análisis {actual_action}: {raw.get('error', 'Completado')}",
                "recommendation": str(raw.get("result", "Verificar resultado")),
                "issues_count": 0,
            }
        )
        return {
            "request_id": request_id,
            "status": raw.get("status", "error"),
            "result": raw.get("result"),
            "error": raw.get("error"),
        }

    # ── 4. Ejecutar en Core Rust ──────────────────────────────────────
    try:
        # Las funciones de ACTION_MAP aceptan target como arg posicional si corresponde
        if actual_action in ("fix",):
            # Solo fix recibe options para auto mode
            raw_result, memory_id = await executor(target or ".", options)
        else:
            # Todas las demás acciones reciben target
            raw_result, memory_id = await executor(target or ".")
    except Exception as exc:
        error_msg = f"Error ejecutando '{actual_action}': {exc}"
        print(f"❌ {error_msg}")
        # Reportar error a Cerebro con info del target
        await report_to_cerebro(
            f"sentinel_{actual_action}_error",
            "error",
            {
                "error": error_msg,
                "action": actual_action,
                "target": target,
                "file": target,
                "finding": f"Error en análisis {actual_action}: {str(exc)[:100]}",
                "recommendation": "Verificar conexión con Sentinel Core",
                "issues_count": 0,
            }
        )
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

    # GARANTÍA: summary nunca vacío (requerido por Cerebro Proactivo)
    if not analysis or not analysis.strip():
        res_tmp = raw_result.get("result", {})
        tmp_count = 0
        if isinstance(res_tmp, dict):
            tmp_count = len(res_tmp.get("issues", []))
            if "files" in res_tmp:
                for f in res_tmp["files"]:
                    tmp_count += len(f.get("issues", []))
        analysis = f"Análisis {actual_action} completado para {target}. {tmp_count} issues detectados."
        if tmp_count > 0:
            analysis += " Revisar el archivo para más detalles."
        print(f"⚠️  [Sentinel] summary vacío — usando fallback de contenido")

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

    # ── 9. Extraer tarea única del análisis ─────────────────────────
    # Si el LLM devolvió múltiples hallazgos, seleccionamos solo uno para Cerebro
    task_selection = _extract_single_task(analysis, issues_count)
    logger.info(f"🎯 [Sentinel] Tarea seleccionada: {task_selection['task_type']} (priority: {task_selection['priority']}) - {task_selection['selection_reason']}")
    print(f"🎯 [Sentinel] Tarea seleccionada: {task_selection['task_type']} ({task_selection['priority']})")

    # ── 10. Reportar a Cerebro ─────────────────────────────────────────
    # Construir payload estructurado con UNA SOLA TAREA accionable
    await report_to_cerebro(
        event_type=f"sentinel_{actual_action.replace('-', '_')}_completed",
        severity=severity,
        payload={
            "action": actual_action,
            "target": target,
            "summary": analysis,           # Siempre tiene contenido (garantía aplicada arriba)
            "memory_id": memory_id,
            "raw_status": raw_result.get("status"),
            # Campos estandarizados para Cerebro Proactivo
            "finding": task_selection["selected_task"],  # 🔥 Una sola tarea específica
            "recommendation": task_selection["selected_task"][:500],  # Tarea prioritaria
            "file": task_selection["file_hint"] or affected_file or target,
            "issues_count": 1,  # 🔥 Reportamos 1 tarea a la vez
            # Nuevos campos para trazabilidad de selección
            "task_type": task_selection["task_type"],
            "task_priority": task_selection["priority"],
            "original_findings_count": task_selection["original_findings_count"],
            "selection_reason": task_selection["selection_reason"],
            "full_analysis": analysis[:2000] if len(analysis) > 2000 else analysis,  # Resumen completo truncado
        },
    )

    # ── 11. Retornar CommandAck completo ───────────────────────────────
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
            "selected_task": task_selection,  # Información de la tarea seleccionada
        },
        "error": None,
    }


async def _handle_monitor_command(monitor_action: str, target: Optional[str], request_id: Optional[str], options: Optional[dict] = None) -> dict:
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
        # Pasar options al executor (las funciones ahora aceptan options como segundo parámetro opcional)
        raw_result, memory_id = await executor(target or ".", options)
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


def _extract_single_task(analysis: str, issues_count: int = 0) -> dict:
    """
    Extrae una única tarea priorizada del análisis del LLM.

    Cuando el LLM devuelve múltiples hallazgos (ej: "1. dry, 2. vulnerabilidad, 3. sql injection"),
    esta función selecciona la más prioritaria y devuelve una estructura estructurada.

    Retorna:
        {
            "selected_task": str,      # Descripción de la tarea seleccionada
            "task_type": str,          # Tipo de tarea: security|performance|maintainability|style|refactor
            "priority": str,           # critical|high|medium|low
            "file_hint": str | None,   # Archivo mencionado si se detecta
            "original_findings_count": int,  # Cuántos hallazgos había originalmente
            "selection_reason": str,   # Por qué se seleccionó esta tarea
        }
    """
    import re

    analysis_lower = analysis.lower()

    # Patrones para detectar tareas numeradas (1., 2), -, *, etc.)
    numbered_patterns = [
        r'(?:^|\n)\s*(?:\d+[.):\-]|\-|\*)\s*([^\n]+)',  # 1. texto, - texto, * texto
        r'(?:^|\n)\s*(?:hallazgo|issue|problema|mejora|sugerencia)\s*(?:\d+)?[.:\-]?\s*([^\n]+)',
    ]

    found_tasks = []

    for pattern in numbered_patterns:
        matches = re.findall(pattern, analysis, re.IGNORECASE | re.MULTILINE)
        for match in matches:
            task_text = match.strip()
            if len(task_text) > 10:  # Filtrar líneas muy cortas
                found_tasks.append(task_text)

    # Si no hay tareas numeradas, intentar dividir por párrafos que describan problemas
    if not found_tasks:
        paragraphs = [p.strip() for p in analysis.split('\n\n') if len(p.strip()) > 20]
        for para in paragraphs:
            # Detectar si el párrafo describe un problema
            problem_indicators = ['error', 'issue', 'bug', 'problema', 'vulnerabilidad', 'mejorar', 'refactor', 'optimizar', 'dead code', 'unused']
            if any(ind in para.lower() for ind in problem_indicators):
                found_tasks.append(para[:200])  # Primeros 200 chars

    # Determinar prioridad de cada tarea
    def get_task_priority(task: str) -> int:
        """Retorna score de prioridad (mayor = más prioritario)"""
        task_lower = task.lower()
        score = 0

        # Palabras clave de severidad
        critical_keywords = ['sql injection', 'injection', 'vulnerabilidad crítica', 'critical', 'seguridad crítica', 'exposición', 'password', 'credential']
        high_keywords = ['vulnerabilidad', 'security', 'seguridad', 'memory leak', 'race condition', 'dead code']
        medium_keywords = ['refactor', 'dry', 'solid', 'complejidad', 'duplicado', 'unused import']

        if any(k in task_lower for k in critical_keywords):
            score += 100
        if any(k in task_lower for k in high_keywords):
            score += 50
        if any(k in task_lower for k in medium_keywords):
            score += 20

        return score

    # Ordenar por prioridad
    found_tasks.sort(key=get_task_priority, reverse=True)

    # Seleccionar la primera (más prioritaria) o crear una genérica si no hay
    if found_tasks:
        selected = found_tasks[0]
        original_count = len(found_tasks)

        # Determinar tipo de tarea
        selected_lower = selected.lower()
        if any(k in selected_lower for k in ['sql injection', 'vulnerabilidad', 'security', 'seguridad', 'injection', 'exposición']):
            task_type = "security"
            priority = "critical" if any(k in selected_lower for k in ['injection', 'crítica', 'exposición']) else "high"
        elif any(k in selected_lower for k in ['dead code', 'unused', 'duplicado', 'dry', 'solid', 'refactor']):
            task_type = "maintainability"
            priority = "medium"
        elif any(k in selected_lower for k in ['complejidad', 'performance', 'optimizar', 'lento']):
            task_type = "performance"
            priority = "medium"
        else:
            task_type = "refactor"
            priority = "low"

        selection_reason = f"Tarea más prioritaria de {original_count} hallazgos detectados"
    else:
        # No se detectaron tareas específicas, usar el análisis completo
        selected = analysis[:300] if len(analysis) > 300 else analysis
        task_type = "refactor"
        priority = "medium"
        original_count = max(issues_count, 1)
        selection_reason = "No se detectaron tareas específicas, usando resumen general"

    # Intentar extraer referencia a archivo
    file_hint = None
    file_patterns = [
        r'(?:en|in|archivo|file)\s+[`\']?([^\s\'`]+\.(?:py|js|ts|jsx|tsx|java|rs|go|cpp|c|h))[`\']?',
        r'[`\']([^\'`]+\.(?:py|js|ts|jsx|tsx|java|rs|go|cpp|c|h))[`\']',
    ]
    for pattern in file_patterns:
        match = re.search(pattern, analysis, re.IGNORECASE)
        if match:
            file_hint = match.group(1)
            break

    return {
        "selected_task": selected,
        "task_type": task_type,
        "priority": priority,
        "file_hint": file_hint,
        "original_findings_count": original_count,
        "selection_reason": selection_reason,
    }
