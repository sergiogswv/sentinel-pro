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

    pub async fn build_context(&self, query: &str, limit: u64) -> anyhow::Result<String> {
        // 1. Obtener embedding de la consulta
        let embedding = obtener_embeddings(vec![query.to_string()], &self.embedding_model)?;
        let query_vector = embedding[0].clone();

        // 2. Buscar en VectorDB
        let matches = self.vector_db.search_similar(query_vector, limit).await?;

        // 3. Formatear resultados
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
}
