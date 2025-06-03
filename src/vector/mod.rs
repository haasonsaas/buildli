pub mod local_store;

use anyhow::Result;
use async_trait::async_trait;
use qdrant_client::{
    prelude::*,
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointStruct, SearchPoints,
        VectorParams, VectorsConfig,
    },
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::indexer::parser::CodeChunk;
pub use local_store::PersistentLocalVectorStore;

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn initialize(&self, collection_name: &str, vector_size: usize) -> Result<()>;
    async fn upsert_documents(&self, documents: Vec<Document>) -> Result<()>;
    async fn search(&self, query_vector: Vec<f32>, top_k: usize) -> Result<Vec<SearchResult>>;
    async fn delete_by_file(&self, file_path: &Path) -> Result<()>;
    fn create_document(&self, chunk: CodeChunk, embedding: Vec<f32>) -> Document;
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct QdrantStore {
    client: QdrantClient,
    collection_name: String,
}

impl QdrantStore {
    pub async fn new(url: &str, collection_name: &str) -> Result<Self> {
        let client = QdrantClient::from_url(url).build()
            .map_err(|e| anyhow::anyhow!("Failed to create Qdrant client: {}", e))?;
        
        Ok(Self {
            client,
            collection_name: collection_name.to_string(),
        })
    }
}

#[async_trait]
impl VectorStore for QdrantStore {
    async fn initialize(&self, collection_name: &str, vector_size: usize) -> Result<()> {
        let collections = self.client.list_collections().await?;
        
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == collection_name);
        
        if !exists {
            self.client
                .create_collection(&CreateCollection {
                    collection_name: collection_name.to_string(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: vector_size as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await?;
        }
        
        Ok(())
    }

    async fn upsert_documents(&self, documents: Vec<Document>) -> Result<()> {
        let points: Vec<PointStruct> = documents
            .into_iter()
            .map(|doc| {
                PointStruct::new(
                    doc.id,
                    doc.embedding,
                    doc.metadata
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect::<std::collections::HashMap<_, _>>(),
                )
            })
            .collect();
        
        self.client
            .upsert_points(&self.collection_name, None, points, None)
            .await?;
        
        Ok(())
    }

    async fn search(&self, query_vector: Vec<f32>, top_k: usize) -> Result<Vec<SearchResult>> {
        let search_result = self
            .client
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: query_vector,
                limit: top_k as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await?;
        
        let results = search_result
            .result
            .into_iter()
            .map(|point| SearchResult {
                score: point.score,
                metadata: point
                    .payload
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<std::collections::HashMap<_, _>>(),
            })
            .collect();
        
        Ok(results)
    }

    async fn delete_by_file(&self, file_path: &Path) -> Result<()> {
        let filter = qdrant_client::qdrant::Filter {
            must: vec![qdrant_client::qdrant::Condition {
                condition_one_of: Some(
                    qdrant_client::qdrant::condition::ConditionOneOf::Field(
                        qdrant_client::qdrant::FieldCondition {
                            key: "file_path".to_string(),
                            r#match: Some(qdrant_client::qdrant::Match {
                                match_value: Some(
                                    qdrant_client::qdrant::r#match::MatchValue::Text(
                                        file_path.display().to_string(),
                                    ),
                                ),
                            }),
                            ..Default::default()
                        },
                    ),
                ),
            }],
            ..Default::default()
        };
        
        self.client
            .delete_points(&self.collection_name, None, &filter.into(), None)
            .await?;
        
        Ok(())
    }

    fn create_document(&self, chunk: CodeChunk, embedding: Vec<f32>) -> Document {
        Document {
            id: Uuid::new_v4().to_string(),
            embedding,
            metadata: HashMap::from([
                ("file_path".to_string(), json!(chunk.file_path)),
                ("content".to_string(), json!(chunk.content)),
                ("line_start".to_string(), json!(chunk.line_start)),
                ("line_end".to_string(), json!(chunk.line_end)),
                ("chunk_type".to_string(), json!(format!("{:?}", chunk.chunk_type))),
                ("language".to_string(), json!(chunk.language)),
            ]),
        }
    }
}

pub struct LocalVectorStore {
    documents: std::sync::RwLock<Vec<Document>>,
}

impl LocalVectorStore {
    pub fn new() -> Self {
        Self {
            documents: std::sync::RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl VectorStore for LocalVectorStore {
    async fn initialize(&self, _collection_name: &str, _vector_size: usize) -> Result<()> {
        Ok(())
    }

    async fn upsert_documents(&self, documents: Vec<Document>) -> Result<()> {
        let mut store = self.documents.write().unwrap();
        store.extend(documents);
        Ok(())
    }

    async fn search(&self, query_vector: Vec<f32>, top_k: usize) -> Result<Vec<SearchResult>> {
        let store = self.documents.read().unwrap();
        let mut results: Vec<(f32, &Document)> = store
            .iter()
            .map(|doc| {
                let score = cosine_similarity(&query_vector, &doc.embedding);
                (score, doc)
            })
            .collect();
        
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        Ok(results
            .into_iter()
            .take(top_k)
            .map(|(score, doc)| SearchResult {
                score,
                metadata: doc.metadata.clone(),
            })
            .collect())
    }

    async fn delete_by_file(&self, file_path: &Path) -> Result<()> {
        let mut store = self.documents.write().unwrap();
        let path_str = file_path.display().to_string();
        store.retain(|doc| {
            doc.metadata
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| p != path_str)
                .unwrap_or(true)
        });
        Ok(())
    }

    fn create_document(&self, chunk: CodeChunk, embedding: Vec<f32>) -> Document {
        Document {
            id: Uuid::new_v4().to_string(),
            embedding,
            metadata: HashMap::from([
                ("file_path".to_string(), json!(chunk.file_path)),
                ("content".to_string(), json!(chunk.content)),
                ("line_start".to_string(), json!(chunk.line_start)),
                ("line_end".to_string(), json!(chunk.line_end)),
                ("chunk_type".to_string(), json!(format!("{:?}", chunk.chunk_type))),
                ("language".to_string(), json!(chunk.language)),
            ]),
        }
    }
}

pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}