import re
import json
from typing import Optional

class UniversalLLMParser:
    """
    Parser resiliente para respuestas de LLM con múltiples estrategias.
    Útil para manejar quirks de distintos modelos (ej. <thought> en Gemma, etc).
    """
    
    @staticmethod
    def strip_thinking_tags(text: str) -> str:
        """Elimina bloques <thought>, <think>, <reasoning>, etc."""
        patterns = [
            r'<thought>.*?</thought>',
            r'<think>.*?</think>', 
            r'<reasoning>.*?</reasoning>',
            r'```thinking.*?```',
        ]
        for p in patterns:
            text = re.sub(p, '', text, flags=re.DOTALL)
        return text.strip()
    
    @staticmethod
    def extract_json(text: str) -> Optional[dict]:
        """Extrae JSON de texto con múltiples estrategias."""
        # 1. JSON directo
        try:
            return json.loads(text)
        except: pass
        
        # 2. JSON en bloque de código
        match = re.search(r'```(?:json)?\s*(\{.*?\})\s*```', text, re.DOTALL)
        if match:
            try: return json.loads(match.group(1))
            except: pass
        
        # 3. Primer objeto JSON válido en el texto
        match = re.search(r'\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}', text, re.DOTALL)
        if match:
            try: return json.loads(match.group())
            except: pass
        
        return None
    
    @classmethod
    def parse_analysis(cls, raw: str) -> dict:
        """Parsea el output de análisis con fallback graceful."""
        cleaned = cls.strip_thinking_tags(raw)
        
        # Intentar JSON estructurado
        data = cls.extract_json(cleaned)
        if data:
            return data
        
        # Fallback: extraer información clave de texto libre
        return {
            "finding": cleaned[:500] if len(cleaned) > 20 else None,
            "format": "raw_text",
            "parse_success": False,
        }
