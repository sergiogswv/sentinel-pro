//! Trait AiProvider y factory build_provider
//!
//! Providers soportados (campo `provider` en ModelConfig):
//! - `"anthropic"` — Claude (Anthropic API)
//! - `"gemini"` — Google Gemini Content API
//! - `"interactions"` — Google Gemini Interactions API (endpoint distinto)
//! - `"ollama"` — Ollama local
//! - `"openai"` / `"lm-studio"` / `"groq"` / `"kimi"` / `"deepseek"` — OpenAI-compatible
//!
//! Para agregar un nuevo proveedor:
//! 1. Crear `src/ai/providers/mi_proveedor.rs` implementando `AiProvider`
//! 2. Agregar `pub mod mi_proveedor;` y re-export aquí
//! 3. Agregar un arm al match en `build_provider`

pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai_compat::OpenAiCompatProvider;

use crate::config::ModelConfig;
use reqwest::blocking::Client;

pub trait AiProvider: Send + Sync {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> anyhow::Result<String>;

    fn embed(
        &self,
        client: &Client,
        texts: Vec<String>,
        model_name: &str,
    ) -> anyhow::Result<Vec<Vec<f32>>>;

    fn list_models(&self) -> anyhow::Result<Vec<String>>;
}

/// Único punto de despacho de providers.
/// El campo `provider` en ModelConfig determina cuál se usa.
/// Si está vacío, se intenta detectar por URL.
pub fn build_provider(config: &ModelConfig) -> Box<dyn AiProvider> {
    let provider = if config.provider.is_empty() {
        let url = config.url.to_lowercase();
        if url.contains("interactions") {
            "interactions"
        } else if url.contains("googleapis") {
            "gemini"
        } else if url.contains("deepseek")
            || url.contains("groq")
            || url.contains("kimi")
            || url.contains("moonshot")
        {
            "openai"
        } else {
            "anthropic"
        }
    } else {
        config.provider.as_str()
    };

    match provider {
        "gemini" => Box::new(GeminiProvider::new(&config.api_key, &config.url, false)),
        // "interactions" es el alias para la Gemini Interactions API (distinta de Content API)
        "interactions" => Box::new(GeminiProvider::new(&config.api_key, &config.url, true)),
        "ollama" => Box::new(OllamaProvider::new(&config.url)),
        "openai" | "lm-studio" | "groq" | "kimi" | "deepseek" => {
            Box::new(OpenAiCompatProvider::new(&config.api_key, &config.url))
        }
        _ => Box::new(AnthropicProvider::new(&config.api_key, &config.url)),
    }
}
