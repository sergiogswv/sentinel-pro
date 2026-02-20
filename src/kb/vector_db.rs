use crate::kb::indexer::CodeSymbol;
use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, ScoredPoint, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParamsBuilder,
};
use std::sync::Arc;

pub struct VectorDB {
    client: Arc<Qdrant>,
    collection_name: String,
    dimension: u64,
}

impl VectorDB {
    pub fn new(url: &str, dimension: u64) -> anyhow::Result<Self> {
        if url.is_empty() || !url.contains("://") {
             return Err(anyhow::anyhow!("URL de Qdrant inválida o vacía: '{}'. Debe incluir el esquema (ej: http://)", url));
        }
        let mut config = Qdrant::from_url(url);
        config.check_compatibility = false;
        let client = Qdrant::new(config)?;
        Ok(Self {
            client: Arc::new(client),
            collection_name: format!("sentinel_code_{}", dimension),
            dimension,
        })
    }

    pub async fn initialize_collection(&self) -> anyhow::Result<()> {
        if !self.client.collection_exists(&self.collection_name).await? {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(self.collection_name.clone())
                        .vectors_config(VectorParamsBuilder::new(self.dimension, Distance::Cosine)),
                )
                .await?;
            println!(
                "   ✅ Colección vectorial '{}' creada en Qdrant.",
                self.collection_name
            );
        }
        Ok(())
    }

    pub async fn upsert_symbols(
        &self,
        symbols: &[CodeSymbol],
        embeddings: Vec<Vec<f32>>,
    ) -> anyhow::Result<()> {
        let mut points = Vec::new();

        for (i, symbol) in symbols.iter().enumerate() {
            let mut payload = Payload::new();
            payload.insert("name", symbol.name.clone());
            payload.insert("kind", format!("{:?}", symbol.kind));
            payload.insert("file_path", symbol.file_path.clone());
            payload.insert("content", symbol.content.clone());
            payload.insert(
                "lines",
                format!("{}-{}", symbol.start_line, symbol.end_line),
            );

            points.push(PointStruct::new(
                uuid::Uuid::new_v4().to_string(),
                embeddings[i].clone(),
                payload,
            ));
        }

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                self.collection_name.clone(),
                points,
            ))
            .await?;
        Ok(())
    }

    pub async fn search_similar(
        &self,
        vector: Vec<f32>,
        limit: u64,
    ) -> anyhow::Result<Vec<ScoredPoint>> {
        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(self.collection_name.clone(), vector, limit)
                    .with_payload(true),
            )
            .await?;

        Ok(response.result)
    }
}
