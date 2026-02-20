use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{Repo, RepoType};
use tokenizers::Tokenizer;

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Singleton para el modelo de embeddings para evitar recargas costosas
static GLOBAL_MODEL: Lazy<Mutex<Option<Arc<EmbeddingModel>>>> = Lazy::new(|| Mutex::new(None));

/// Estructura para manejar el modelo de embeddings local
pub struct EmbeddingModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingModel {
    /// Carga el modelo "sentence-transformers/all-MiniLM-L6-v2" desde HuggingFace Hub
    /// o desde el cache local si ya existe.
    pub fn new() -> Result<Self> {
        let device = Device::Cpu; // Usamos CPU por defecto para mayor compatibilidad

        // Corrección de variables de entorno para evitar `RelativeUrlWithoutBase`
        for var in &["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "HF_ENDPOINT"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() && !val.contains("://") {
                    unsafe {
                        std::env::set_var(var, format!("http://{}", val));
                    }
                }
            }
        }

        // Configurar spinner para carga con mayor estabilidad
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.magenta} {msg}")
                .unwrap(),
        );
        pb.set_draw_target(indicatif::ProgressDrawTarget::stdout());
        pb.set_message("Cargando motor de IA local desde HF Hub...");
        pb.enable_steady_tick(std::time::Duration::from_millis(80));

        // Bypass bug de `ureq` v3 en hf-hub usando la API asíncrona de tokio y reqwest
        let fetch_models = || -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let api = hf_hub::api::tokio::Api::new()
                    .map_err(|e| anyhow::anyhow!("Error inicializando HF Api asíncrona: {}", e))?;
                let repo = api.repo(Repo::new(
                    "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                    RepoType::Model,
                ));
                let config_filename = repo.get("config.json").await
                    .map_err(|e| anyhow::anyhow!("Error descargando config.json de HF Hub: {}", e))?;
                let tokenizer_filename = repo.get("tokenizer.json").await
                    .map_err(|e| anyhow::anyhow!("Error descargando tokenizer.json de HF Hub: {}", e))?;
                let weights_filename = repo.get("model.safetensors").await
                    .map_err(|e| anyhow::anyhow!("Error descargando model.safetensors de HF Hub: {}", e))?;
                
                Ok((config_filename, tokenizer_filename, weights_filename))
            })
        };

        let (config_filename, tokenizer_filename, weights_filename) = match std::thread::spawn(fetch_models).join() {
            Ok(res) => res?,
            Err(_) => return Err(anyhow::anyhow!("Error crítico: hilo de decarga HF hizo panic.")),
        };

        pb.set_message("Inicializando arquitectura neuronal...");

        // Cargar configuración
        let config_content = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config_content)?;

        // Ajuste específico para all-MiniLM-L6-v2 si es necesario
        // En algunos casos la activación puede diferir, pero el json suele ser correcto.

        // Cargar tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(|e| anyhow::anyhow!("Error cargando tokenizer: {}", e))?;

        // Cargar pesos usando safetensors
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights_filename],
                candle_core::DType::F32,
                &device,
            )?
        };

        // Inicializar modelo
        let model = BertModel::load(vb, &config)?;

        pb.finish_and_clear();

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Genera embeddings para una lista de textos
    pub fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        for text in texts {
            // Tokenizar
            let tokens = self
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| anyhow::anyhow!("Error tokenizando texto: {}", e))?;

            let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
            let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

            // Inferencia
            let output = self.model.forward(&token_ids, &token_type_ids)?;

            // Mean Pooling (promedio de la última capa oculta)
            // output es [batch_size, seq_len, hidden_size]
            // Queremos promediar sobre seq_len, ignorando padding si fuera batch real,
            // pero aquí procesamos 1 por 1 para simplificar memoria por ahora.

            // Obtener la última capa oculta
            let hidden_states = output;

            // Realizar mean pooling simple: (N, L, H) -> (N, H)
            let (_n, l, _h) = hidden_states.dims3()?;
            let sum = hidden_states.sum(1)?;
            let pooled = (sum / (l as f64))?;

            // Normalizar (L2 norm) para similitud coseno
            let pooled_norm = pooled.sqr()?.sum_keepdim(1)?.sqrt()?;
            let embedding = (pooled.broadcast_div(&pooled_norm))?; // (1, H)

            // Convertir a vector
            let vec: Vec<f32> = embedding.squeeze(0)?.to_vec1()?;
            embeddings.push(vec);
        }

        Ok(embeddings)
    }

    /// Genera embedding para un solo texto
    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.embed(&[text.to_string()])?;
        Ok(result[0].clone())
    }

    /// Obtiene o inicializa la instancia global del modelo (Singleton)
    pub fn get_or_init() -> Result<Arc<Self>> {
        let mut lock = GLOBAL_MODEL.lock().unwrap();
        if let Some(model) = lock.as_ref() {
            return Ok(Arc::clone(model));
        }

        let model = Arc::new(Self::new()?);
        *lock = Some(Arc::clone(&model));
        Ok(model)
    }
}

// Tests unitarios
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embeddings_generation() {
        // Este test descargará el modelo, así que puede ser lento
        // Lo marcamos como ignore por defecto para CI rápido
        // cargo test -- --ignored para ejecutarlo
        if std::env::var("RUN_ML_TESTS").is_err() {
            return;
        }

        let model = EmbeddingModel::new().unwrap();
        let texts = vec!["Hola mundo".to_string(), "Hello world".to_string()];
        let embeddings = model.embed(&texts).unwrap();

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 384); // all-MiniLM-L6-v2 dimension

        // Verificar similitud (debería ser alta)
        let sim = cosine_similarity(&embeddings[0], &embeddings[1]);
        assert!(sim > 0.5);
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (norm_a * norm_b)
    }
}
