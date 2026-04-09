"""
Configuración centralizada del Sentinel ADK Agent.
Lee variables de entorno desde .env o el sistema, y hereda de global.config.json
"""

import json
import os
from pathlib import Path
from typing import Literal, Optional
from pydantic import field_validator
from pydantic_settings import BaseSettings


def _load_global_config() -> dict:
    """Carga la configuración global de Cerebro si existe."""
    global_config_path = Path.home() / ".cerebro" / "global.config.json"
    if global_config_path.exists():
        try:
            with open(global_config_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    return {}


def _get_global_llm_config() -> dict:
    """Extrae configuración LLM de global.config.json."""
    global_cfg = _load_global_config()
    return global_cfg.get("global_config", {}).get("llm", {})


# Cargar configuración global para valores por defecto
_GLOBAL_LLM = _get_global_llm_config()


class SentinelADKSettings(BaseSettings):
    # Sentinel Core (Rust server)
    sentinel_core_url: str = "http://localhost:4001"

    # Cerebro (Orquestador Central)
    cerebro_url: str = "http://localhost:4000"

    # Proveedor de LLM (hereda de global.config.json si existe)
    llm_provider: Literal["gemini", "gemini-open-source", "claude", "openai", "ollama"] = (
        _GLOBAL_LLM.get("provider", "gemini") if _GLOBAL_LLM.get("provider") else "gemini"
    )

    # Google API Base URL (for custom endpoints or regions)
    # Para gemini-open-source, usa el base_url de la config global
    google_api_base_url: str = (
        _GLOBAL_LLM.get("base_url", "https://generativelanguage.googleapis.com")
        if _GLOBAL_LLM.get("base_url")
        else "https://generativelanguage.googleapis.com"
    )

    # Google Gemini (hereda API key y modelo de global.config.json)
    google_api_key: str = _GLOBAL_LLM.get("api_key", "")
    gemini_model: str = (
        _GLOBAL_LLM.get("model", "gemini-2.0-flash")
        if _GLOBAL_LLM.get("model")
        else "gemini-2.0-flash"
    )

    # Anthropic Claude
    anthropic_api_key: str = ""
    claude_model: str = "claude-3-5-sonnet-latest"

    # OpenAI (usa API key de Google para endpoint compatible)
    # Cuando provider es gemini-open-source, usamos la API key de Google
    # y el modelo Gemma para el endpoint OpenAI-compatible
    openai_api_key: str = _GLOBAL_LLM.get("api_key", "")
    # Si el provider global es gemini-open-source, usar el modelo Gemma
    # de lo contrario usar gpt-4o por defecto
    openai_model: str = (
        _GLOBAL_LLM.get("model", "gemma-4-31b-it")
        if _GLOBAL_LLM.get("provider") == "gemini-open-source" and _GLOBAL_LLM.get("model")
        else "gpt-4o"
    )

    # Ollama (local)
    ollama_base_url: str = "http://localhost:11434"
    ollama_model: str = "llama3.2"

    # ADK Server
    sentinel_adk_port: int = 4011

    # Memoria persistente
    sentinel_db_path: str = "./sentinel_memory.db"

    @field_validator("llm_provider", mode="before")
    @classmethod
    def validate_llm_provider(cls, v):
        """Si el valor está vacío, retorna el default 'gemini'."""
        if not v or v == "":
            return "gemini"
        return v

    model_config = {
        "env_file": os.path.join(os.path.dirname(__file__), ".env"),
        "env_file_encoding": "utf-8"
    }


settings = SentinelADKSettings()
