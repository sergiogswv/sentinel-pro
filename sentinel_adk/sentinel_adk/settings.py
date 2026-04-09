"""
Configuración centralizada del Sentinel ADK Agent.
Lee de ~/.cerebro/global.config.json primero, luego .env como fallback.
"""

import os
import json
from pathlib import Path
from typing import Literal, Optional
from pydantic import field_validator
from pydantic_settings import BaseSettings


def _load_global_config() -> dict:
    """Carga la configuración global de ~/.cerebro/global.config.json"""
    global_config_path = Path.home() / ".cerebro" / "global.config.json"

    if not global_config_path.exists():
        return {}

    try:
        with open(global_config_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError) as e:
        print(f"[WARN] No se pudo cargar global.config.json: {e}")
        return {}


def _get_global_llm_config() -> dict:
    """Extrae la config LLM del global config"""
    config = _load_global_config()

    # Intentar obtener de global_config.llm
    global_config = config.get("global_config", {})
    llm_config = global_config.get("llm", {})

    # También intentar de cerebro auto_fix config
    cerebro = config.get("cerebro", {})
    auto_fix_provider = cerebro.get("auto_fix_provider")
    auto_fix_model = cerebro.get("auto_fix_model")
    auto_fix_base_url = cerebro.get("auto_fix_base_url")
    auto_fix_api_key = cerebro.get("auto_fix_api_key")

    # Mapear provider: gemini -> gemini, pero detectar gemma
    provider = llm_config.get("provider", "gemini")
    model = llm_config.get("model", "gemini-2.0-flash")
    base_url = llm_config.get("base_url", "https://generativelanguage.googleapis.com")
    api_key = llm_config.get("api_key", "")

    # Si hay auto_fix config, usarla (es más específica)
    if auto_fix_provider:
        provider = auto_fix_provider
    if auto_fix_model:
        model = auto_fix_model
    if auto_fix_base_url:
        base_url = auto_fix_base_url
    if auto_fix_api_key:
        api_key = auto_fix_api_key

    # Detectar si es Gemma (open source) y cambiar provider
    if "gemma" in model.lower():
        provider = "gemini-open-source"

    return {
        "provider": provider,
        "model": model,
        "base_url": base_url,
        "api_key": api_key,
    }


class SentinelADKSettings(BaseSettings):
    # Sentinel Core (Rust server)
    sentinel_core_url: str = "http://localhost:4001"

    # Cerebro (Orquestador Central)
    cerebro_url: str = "http://localhost:4000"

    # Proveedor de LLM
    llm_provider: Literal["gemini", "gemini-open-source", "claude", "openai", "ollama"] = "gemini"

    # Google API Base URL (for custom endpoints or regions)
    google_api_base_url: str = "https://generativelanguage.googleapis.com"

    # Google Gemini / Gemma
    google_api_key: str = ""
    gemini_model: str = "gemini-2.0-flash"

    # Anthropic Claude
    anthropic_api_key: str = ""
    claude_model: str = "claude-3-5-sonnet-latest"

    # OpenAI
    openai_api_key: str = ""
    openai_model: str = "gpt-4o"

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

    def __init__(self, **kwargs):
        # Cargar config global primero
        global_llm = _get_global_llm_config()

        # Aplicar valores del global config como defaults
        if global_llm.get("provider"):
            kwargs.setdefault("llm_provider", global_llm["provider"])
        if global_llm.get("model"):
            kwargs.setdefault("gemini_model", global_llm["model"])
        if global_llm.get("base_url"):
            kwargs.setdefault("google_api_base_url", global_llm["base_url"])
        if global_llm.get("api_key"):
            kwargs.setdefault("google_api_key", global_llm["api_key"])

        super().__init__(**kwargs)

    model_config = {
        "env_file": os.path.join(os.path.dirname(__file__), ".env"),
        "env_file_encoding": "utf-8",
        "extra": "ignore",  # Ignorar campos extra que no están definidos
    }


settings = SentinelADKSettings()
