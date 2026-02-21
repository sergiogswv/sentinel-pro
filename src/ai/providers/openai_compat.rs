// src/ai/providers/openai_compat.rs
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::json;

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
        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Estructura de OpenAI-Compat inesperada. Body: {}",
                    body_text
                )
            })
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
