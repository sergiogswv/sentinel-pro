use crate::ai::obtener_embeddings;
use crate::config::ModelConfig;
use crate::kb::VectorDB;
use qdrant_client::qdrant::value::Kind;

pub struct ContextBuilder {
    vector_db: VectorDB,
    embedding_model: ModelConfig,
}

impl ContextBuilder {
    pub fn new(vector_db: VectorDB, embedding_model: ModelConfig) -> Self {
        Self {
            vector_db,
            embedding_model,
        }
    }

    pub async fn build_context(&self, query: &str, limit: u64, rerank: bool) -> anyhow::Result<String> {
        // 1. Obtener embedding de la consulta
        let embedding = obtener_embeddings(vec![query.to_string()], &self.embedding_model)?;
        let query_vector = embedding[0].clone();

        // 2. Buscar en VectorDB (pedimos un poco más para re-ranking)
        let search_limit = if rerank { limit * 2 } else { limit };
        let mut matches = self.vector_db.search_similar(query_vector, search_limit).await?;

        // 3. Re-ranking (Refinamiento semántico simple)
        if rerank {
             // Mock re-ranking: Priorizar coincidencias exactas de palabras clave en el contenido
             matches.sort_by(|a, b| {
                let score_a = self.calculate_rerank_score(a, query);
                let score_b = self.calculate_rerank_score(b, query);
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
             });
             // Recortar al límite original
             matches.truncate(limit as usize);
        }

        // 4. Formatear resultados
        if matches.is_empty() {
            return Ok("No se encontró contexto relevante en el codebase.".to_string());
        }

        let mut context = String::from("Contexto relevante del codebase:\n\n");
        for m in matches {
            let payload = &m.payload;
            let file_path = if let Some(Kind::StringValue(v)) =
                payload.get("file_path").and_then(|v| v.kind.as_ref())
            {
                v.clone()
            } else {
                "desconocido".to_string()
            };

            let content = if let Some(Kind::StringValue(v)) =
                payload.get("content").and_then(|v| v.kind.as_ref())
            {
                v.clone()
            } else {
                "".to_string()
            };
            
            if content.trim().is_empty() {
                continue;
            }

            let lines = if let Some(Kind::StringValue(v)) =
                payload.get("lines").and_then(|v| v.kind.as_ref())
            {
                v.clone()
            } else {
                "".to_string()
            };

            context.push_str(&format!(
                "Archivo: {} (Líneas: {})\n```\n{}\n```\n\n",
                file_path, lines, content
            ));
        }

        Ok(context)
    }

    fn calculate_rerank_score(&self, point: &qdrant_client::qdrant::ScoredPoint, query: &str) -> f32 {
        let mut score = point.score; // Base vector score
        
        let content = if let Some(Kind::StringValue(v)) =
             point.payload.get("content").and_then(|v| v.kind.as_ref())
        {
            v.to_lowercase()
        } else {
            return score;
        };
        
        let file_path = if let Some(Kind::StringValue(v)) =
             point.payload.get("file_path").and_then(|v| v.kind.as_ref())
        {
            v.to_lowercase()
        } else {
            "".to_string()
        };

        let query_lower = query.to_lowercase();
        
        // Bonus por coincidencias exactas de términos en el nombre del archivo
        if file_path.contains(&query_lower) {
            score += 0.1;
        }

        // Bonus por contener funciones/clases mencionadas en la query
        // (Lógica simplificada)
        if content.contains(&query_lower) {
            score += 0.05;
        }

        score
    }
}
