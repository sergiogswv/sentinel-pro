// src/ai/providers/openai_compat.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

/// Strip <thought>...</thought> blocks from Gemma model responses
/// Gemma returns internal reasoning wrapped in XML tags that should be filtered out
fn strip_thought_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut in_thought = false;
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        let remaining = &chars[i..];
        if !in_thought && remaining.starts_with(&['<', 't', 'h', 'o', 'u', 'g', 'h', 't', '>']) {
            in_thought = true;
            i += 9;
        } else if in_thought && remaining.starts_with(&['<', '/', 't', 'h', 'o', 'u', 'g', 'h', 't', '>']) {
            in_thought = false;
            i += 10;
        } else if !in_thought {
            result.push(chars[i]);
            i += 1;
        } else {
            i += 1;
        }
    }

    result.trim().to_string()
}

pub struct OpenAiCompatProvider {
    api_key: String,
    url: String,
}

impl OpenAiCompatProvider {
    pub fn new(api_key: &str, url: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            url: url.to_string(),
        }
    }
}

impl super::AiProvider for OpenAiCompatProvider {
    fn chat(&self, client: &Client, prompt: &str, model_name: &str) -> Result<String> {
        let base = self.url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": model_name,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()?;

        let status = response.status();
        let body_text = response.text()?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Error de API OpenAI-Compat (Status {}): {}",
                status,
                body_text
            ));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)?;
        
        // 1. Intentar formato estándar OpenAI
        let text = body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                // 2. Fallback: ¿Es formato Google Gemini/Gemma?
                let empty_vec = vec![];
                let parts = body["candidates"][0]["content"]["parts"].as_array().unwrap_or(&empty_vec);
                
                let mut combined_text = String::new();
                for part in parts {
                    // Ignorar bloques de pensamiento
                    if part["thought"].as_bool().unwrap_or(false) {
                        continue;
                    }
                    if let Some(part_text) = part["text"].as_str() {
                        combined_text.push_str(part_text);
                    }
                }
                
                if combined_text.is_empty() {
                    None
                } else {
                    Some(combined_text)
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Estructura de respuesta inesperada (no se encontró 'choices' ni 'candidates'). Body: {}",
                    body_text
                )
            })?;

        // Strip <thought> blocks from Gemma models (OpenAI-compatible endpoint)
        let text_clean = strip_thought_blocks(&text);
        Ok(text_clean)
    }

    fn embed(&self, client: &Client, texts: Vec<String>, model_name: &str) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/v1/embeddings", self.url.trim_end_matches('/'));

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({ "model": model_name, "input": texts }))
            .send()?;

        let body: serde_json::Value = response.json()?;
        let embeddings = body["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta de OpenAI Embeddings inesperada"))?
            .iter()
            .map(|d| -> anyhow::Result<Vec<f32>> {
                let values = d["embedding"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("OpenAI embedding: 'embedding' faltante o no es array"))?;
                values
                    .iter()
                    .map(|v| {
                        v.as_f64()
                            .ok_or_else(|| anyhow::anyhow!("OpenAI embedding: valor no numérico"))
                            .map(|f| f as f32)
                    })
                    .collect()
            })
            .collect::<anyhow::Result<Vec<Vec<f32>>>>()?;
        Ok(embeddings)
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let client = Client::new();
        let url_str = self.url.trim_end_matches('/');
        let target_url = if url_str.ends_with("/v1") {
            format!("{}/models", url_str)
        } else {
            format!("{}/v1/models", url_str)
        };

        let mut request = client.get(&target_url);
        if !self.api_key.is_empty() {
            request = request.header("authorization", format!("Bearer {}", self.api_key));
        }

        let response = request.send()?;
        let json: serde_json::Value = response.json()?;
        let models = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Respuesta API compatible inválida"))?
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(models)
    }
}
