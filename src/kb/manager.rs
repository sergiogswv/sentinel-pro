use crate::ai::obtener_embeddings;
use crate::config::{ModelConfig, SentinelConfig};
use crate::kb::{CodeIndex, VectorDB};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct KBUpdate {
    pub file_path: String,
}

pub struct KBManager {
    vector_db: Arc<VectorDB>,
    embedding_model: ModelConfig,
    project_root: String,
}

impl KBManager {
    pub fn new(vector_db: Arc<VectorDB>, config: &SentinelConfig, project_root: &Path) -> Self {
        Self {
            vector_db,
            embedding_model: config.primary_model.clone(), // Usar modelo primario para embeddings si mql no est├í
            project_root: project_root.to_string_lossy().to_string(),
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
                if let Ok(embeddings) = obtener_embeddings(texts, &self.embedding_model) {
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
        for chunk in symbols.chunks(20) {
            let texts: Vec<String> = chunk.iter().map(|s| s.content.clone()).collect();
            if let Ok(embeddings) = obtener_embeddings(texts, &self.embedding_model) {
                self.vector_db.upsert_symbols(chunk, embeddings).await?;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }
}
