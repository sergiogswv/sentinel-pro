// src/ai/providers/anthropic.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

pub struct AnthropicProvider {
    api_key: String,
    url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, url: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            url: url.to_string(),
        }
    }
}

impl super::AiProvider for AnthropicProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        let base = self.url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        };

        let response = client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": model_name,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()?;

        let status = response.status();
        let body_text = response.text()?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Error de API Anthropic (Status {}): {}",
                status,
                body_text
            ));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;
        body["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("Estructura de Anthropic inesperada. Body: {}", body_text)
            })
    }

    fn embed(&self, _client: &Client, _texts: Vec<String>, _model_name: &str) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!(
            "Anthropic no soporta embeddings vía API HTTP. Usa provider 'local'."
        ))
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let url = format!("{}/v1/models", self.url.trim_end_matches('/'));
        let response = client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()?;

        let json: serde_json::Value = response.json()?;
        let models = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta de Claude inválida"))?
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(models)
    }
}
