use crate::ai::obtener_embeddings;
use crate::config::{ModelConfig, SentinelConfig};
use crate::kb::{CodeIndex, VectorDB};
use crate::ml::embeddings::EmbeddingModel;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task;

pub struct KBUpdate {
    pub file_path: String,
}

pub struct KBManager {
    vector_db: Arc<VectorDB>,
    embedding_model: ModelConfig,
    project_root: String,
    local_model: Option<Arc<EmbeddingModel>>,
}

impl KBManager {
    pub fn new(vector_db: Arc<VectorDB>, config: &SentinelConfig, project_root: &Path) -> Self {
        // Cargar modelo local si el proveedor es 'local' o 'anthropic' (Claude)
        let local_model = if config.primary_model.provider == "local" || config.primary_model.provider == "anthropic" {
            match EmbeddingModel::get_or_init() {
                Ok(model) => Some(model),
                Err(e) => {
                    eprintln!("   ❌ Error cargando motor de IA local: {}. Asegúrate de tener conexión a internet para la primera carga o revisa HF_ENDPOINT.", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            vector_db,
            embedding_model: config.primary_model.clone(),
            project_root: project_root.to_string_lossy().to_string(),
            local_model,
        }
    }

    pub async fn start_background_task(self: Arc<Self>, mut rx: mpsc::Receiver<KBUpdate>) {
        let mut indexer = CodeIndex::new_typescript();

        while let Some(update) = rx.recv().await {
            let files = vec![update.file_path.clone()];
            if let Ok(symbols) = indexer.update_files(&files) {
                if symbols.is_empty() {
                    continue;
                }

                let texts: Vec<String> = symbols.iter().map(|s| s.content.clone()).collect();

                let embeddings_result = if let Some(local) = &self.local_model {
                    let local_arc = Arc::clone(local);
                    let texts_clone = texts.clone();
                    task::spawn_blocking(move || local_arc.embed(&texts_clone))
                        .await
                        .unwrap()
                } else {
                    obtener_embeddings(texts, &self.embedding_model)
                };

                if let Ok(embeddings) = embeddings_result {
                    let _ = self.vector_db.upsert_symbols(&symbols, embeddings).await;
                    println!("   🧠 KB: Índice actualizado para {}", update.file_path);
                }
            }
        }
    }

    pub async fn initial_index(&self) -> anyhow::Result<()> {
        let mut indexer = CodeIndex::new_typescript();
        let symbols = indexer.index_all_project(Path::new(&self.project_root))?;

        if symbols.is_empty() {
            return Ok(());
        }

        println!(
            "   🧠 KB: Generando embeddings para {} símbolos...",
            symbols.len()
        );

        // Procesar en batches de 20 para evitar límites de API
        // Si es local, podemos aumentar el batch size ya que no hay rate limit de red,
        // pero la memoria sigue siendo un factor. 32 es un buen número para CPU inference.
        let batch_size = if self.local_model.is_some() { 32 } else { 20 };

        for chunk in symbols.chunks(batch_size) {
            let texts: Vec<String> = chunk.iter().map(|s| s.content.clone()).collect();

            let embeddings_result = if let Some(local) = &self.local_model {
                let local_arc = Arc::clone(local);
                let texts_clone = texts.clone();
                // Bloqueante en CPU, usar spawn_blocking
                task::spawn_blocking(move || local_arc.embed(&texts_clone)).await?
            } else {
                obtener_embeddings(texts, &self.embedding_model)
            };

            if let Ok(embeddings) = embeddings_result {
                self.vector_db.upsert_symbols(chunk, embeddings).await?;
            }

            // Sleep solo si es API remota para respetar rate limits
            if self.local_model.is_none() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }
}
