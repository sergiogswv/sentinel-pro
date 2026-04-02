"""
Configuración centralizada del Sentinel ADK Agent.
Lee variables de entorno desde .env o el sistema.
"""

import os
from typing import Literal
from pydantic_settings import BaseSettings


class SentinelADKSettings(BaseSettings):
    # Sentinel Core (Rust server)
    sentinel_core_url: str = "http://localhost:4001"

    # Cerebro (Orquestador Central)
    cerebro_url: str = "http://localhost:4000"

    # Proveedor de LLM
    llm_provider: Literal["gemini", "claude", "openai", "ollama"] = "gemini"

    # Google Gemini
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

    model_config = {
        "env_file": os.path.join(os.path.dirname(__file__), ".env"),
        "env_file_encoding": "utf-8"
    }


settings = SentinelADKSettings()
