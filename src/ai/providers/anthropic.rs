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
        let mut extracted_text = String::new();

        // Intentar formato Anthropic: { content: [{ type: "text", text: "..." }, ...] }
        if let Some(contents) = body["content"].as_array() {
            for content in contents {
                if content["type"] == "text" {
                    if let Some(t) = content["text"].as_str() {
                        extracted_text.push_str(t);
                    }
                } else if content["text"].is_string() {
                    extracted_text.push_str(content["text"].as_str().unwrap());
                }
            }
        }
        // Fallback: intentar formato OpenAI-compatible: { choices: [{ message: { content: "..." } }] }
        else if let Some(choices) = body["choices"].as_array() {
            if let Some(first_choice) = choices.get(0) {
                if let Some(content) = first_choice["message"]["content"].as_str() {
                    extracted_text = content.to_string();
                }
            }
        }
        // Fallback: intentar formato Gemini: { candidates: [{ content: { parts: [{ text: "..." }] } }] }
        else if let Some(candidates) = body["candidates"].as_array() {
            for candidate in candidates {
                if let Some(parts) = candidate["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(t) = part["text"].as_str() {
                            extracted_text.push_str(t);
                        }
                    }
                }
            }
        }

        if extracted_text.is_empty() {
            // Error más informativo con el formato real recibido
            let format_type = if body["choices"].is_array() {
                "OpenAI-compatible"
            } else if body["candidates"].is_array() {
                "Gemini"
            } else {
                "desconocido"
            };
            Err(anyhow::anyhow!(
                "Respuesta de IA no pudo ser parseada (formato {}): {}",
                format_type,
                body_text
            ))
        } else {
            Ok(extracted_text)
        }
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
