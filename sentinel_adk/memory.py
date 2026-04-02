"""
memory.py — Base de datos SQLite para la memoria persistente de Sentinel.

Persiste:
  - Historial de análisis de calidad (dead code, complejidad, imports)
  - Decisiones del usuario (accepted / false_positive / ignored)
  - Perfil de calidad por archivo (quality_profile)
  - Historial de análisis ejecutados
"""

import aiosqlite
import json
from datetime import datetime, timezone
from typing import Optional
from .settings import settings


DB_PATH = settings.sentinel_db_path

SCHEMA = """
CREATE TABLE IF NOT EXISTS findings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    event_type  TEXT    NOT NULL,          -- 'check_completed', 'audit_completed', 'dead_code_found', etc.
    severity    TEXT    NOT NULL,          -- 'info' | 'warning' | 'error' | 'critical'
    target      TEXT,                      -- archivo o directorio analizado
    payload     TEXT    NOT NULL,          -- JSON del resultado completo
    decision    TEXT    DEFAULT NULL,      -- 'accepted' | 'false_positive' | 'ignored'
    decision_at TEXT    DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS quality_profiles (
    file_path   TEXT    PRIMARY KEY,
    last_seen   TEXT    NOT NULL,
    total_checks INTEGER NOT NULL DEFAULT 0,
    issues_found INTEGER NOT NULL DEFAULT 0,
    complexity_avg REAL NOT NULL DEFAULT 0.0,
    dead_code_count INTEGER NOT NULL DEFAULT 0,
    unused_imports_count INTEGER NOT NULL DEFAULT 0,
    notes       TEXT   DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS analysis_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    run_at      TEXT    NOT NULL,
    action      TEXT    NOT NULL,          -- 'check', 'audit', 'report', 'fix', 'review'
    target      TEXT,
    summary     TEXT    NOT NULL,          -- JSON resumido del resultado
    duration_ms INTEGER DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS file_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path   TEXT    NOT NULL,
    checked_at  TEXT    NOT NULL,
    issues      TEXT,                      -- JSON de issues encontrados
    complexity  REAL    DEFAULT NULL,
    lines_of_code INTEGER DEFAULT NULL
);
"""


async def init_db():
    """Inicializa el schema de la base de datos si no existe."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.executescript(SCHEMA)
        await db.commit()


async def save_finding(
    event_type: str,
    severity: str,
    payload: dict,
    target: Optional[str] = None,
) -> int:
    """Guarda un hallazgo de calidad y retorna su ID."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        cursor = await db.execute(
            """
            INSERT INTO findings (created_at, event_type, severity, target, payload)
            VALUES (?, ?, ?, ?, ?)
            """,
            (now, event_type, severity, target, json.dumps(payload)),
        )
        await db.commit()
        return cursor.lastrowid


async def set_finding_decision(finding_id: int, decision: str):
    """Actualiza la decisión del usuario sobre un hallazgo."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            "UPDATE findings SET decision = ?, decision_at = ? WHERE id = ?",
            (decision, now, finding_id),
        )
        await db.commit()


async def update_quality_profile(file_path: str, issues: int, complexity: float = 0.0, dead_code: int = 0, unused_imports: int = 0):
    """
    Actualiza el perfil de calidad acumulado de un archivo.
    Si no existe, lo crea. Si ya existe, acumula los contadores.
    """
    now = datetime.now(timezone.utc).isoformat()

    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            """
            INSERT INTO quality_profiles (
                file_path, last_seen, total_checks, issues_found,
                complexity_avg, dead_code_count, unused_imports_count
            )
            VALUES (?, ?, 1, ?, ?, ?, ?)
            ON CONFLICT(file_path) DO UPDATE SET
                last_seen       = excluded.last_seen,
                total_checks    = total_checks + 1,
                issues_found    = issues_found + excluded.issues_found,
                complexity_avg  = (complexity_avg * total_checks + excluded.complexity_avg) / (total_checks + 1),
                dead_code_count = dead_code_count + excluded.dead_code_count,
                unused_imports_count = unused_imports_count + excluded.unused_imports_count
            """,
            (file_path, now, issues, complexity, dead_code, unused_imports),
        )
        await db.commit()


async def get_quality_profile(file_path: str) -> Optional[dict]:
    """Devuelve el perfil de calidad histórico de un archivo, o None si no existe."""
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        async with db.execute(
            "SELECT * FROM quality_profiles WHERE file_path = ?", (file_path,)
        ) as cursor:
            row = await cursor.fetchone()
            return dict(row) if row else None


async def get_hot_files(limit: int = 10) -> list[dict]:
    """
    Devuelve los archivos con más historial de issues acumulados.
    Útil para contextualizar al agente ADK sobre el estado del proyecto.
    """
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        async with db.execute(
            """
            SELECT file_path, total_checks, issues_found, complexity_avg, dead_code_count, unused_imports_count, last_seen
            FROM quality_profiles
            ORDER BY issues_found DESC, complexity_avg DESC
            LIMIT ?
            """,
            (limit,),
        ) as cursor:
            rows = await cursor.fetchall()
            return [dict(r) for r in rows]


async def save_analysis_run(action: str, target: Optional[str], summary: dict, duration_ms: Optional[int] = None):
    """Registra cada ejecución de análisis para tracking histórico."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            """
            INSERT INTO analysis_runs (run_at, action, target, summary, duration_ms)
            VALUES (?, ?, ?, ?, ?)
            """,
            (now, action, target, json.dumps(summary), duration_ms),
        )
        await db.commit()


async def get_recent_findings(limit: int = 20, severity_filter: Optional[str] = None) -> list[dict]:
    """
    Retorna los hallazgos más recientes, opcionalmente filtrados por severity.
    Este método lo usará el agente ADK para recordar el contexto de calidad.
    """
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        query = "SELECT * FROM findings"
        params: list = []
        if severity_filter:
            query += " WHERE severity = ?"
            params.append(severity_filter)
        query += " ORDER BY created_at DESC LIMIT ?"
        params.append(limit)
        async with db.execute(query, params) as cursor:
            rows = await cursor.fetchall()
            result = []
            for row in rows:
                d = dict(row)
                d["payload"] = json.loads(d["payload"] or "{}")
                result.append(d)
            return result


async def save_file_history(file_path: str, issues: dict, complexity: float = None, lines_of_code: int = None):
    """Guarda el historial de un archivo específico."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            """
            INSERT INTO file_history (file_path, checked_at, issues, complexity, lines_of_code)
            VALUES (?, ?, ?, ?, ?)
            """,
            (file_path, now, json.dumps(issues), complexity, lines_of_code),
        )
        await db.commit()


async def get_file_history(file_path: str, limit: int = 5) -> list[dict]:
    """Obtiene el historial reciente de un archivo específico."""
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        async with db.execute(
            """
            SELECT * FROM file_history
            WHERE file_path = ?
            ORDER BY checked_at DESC
            LIMIT ?
            """,
            (file_path, limit),
        ) as cursor:
            rows = await cursor.fetchall()
            result = []
            for row in rows:
                d = dict(row)
                d["issues"] = json.loads(d["issues"] or "{}")
                return result
