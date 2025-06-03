use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;

use super::{cosine_similarity, Document, SearchResult, VectorStore};
use crate::indexer::parser::CodeChunk;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredDocument {
    id: String,
    embedding: Vec<f32>,
    metadata: HashMap<String, serde_json::Value>,
}

pub struct PersistentLocalVectorStore {
    documents: RwLock<Vec<StoredDocument>>,
    store_path: PathBuf,
}

impl PersistentLocalVectorStore {
    pub async fn new() -> Result<Self> {
        let project_dirs = directories::ProjectDirs::from("", "", "buildli")
            .ok_or_else(|| anyhow::anyhow!("Failed to determine project directories"))?;
        
        let data_dir = project_dirs.data_dir();
        let store_path = data_dir.join("local_vector_store.json");
        
        // Create directory if it doesn't exist
        if let Some(parent) = store_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let mut store = Self {
            documents: RwLock::new(Vec::new()),
            store_path,
        };
        
        // Load existing data if available
        store.load().await?;
        
        Ok(store)
    }
    
    async fn load(&mut self) -> Result<()> {
        if self.store_path.exists() {
            let data = fs::read_to_string(&self.store_path).await?;
            let documents: Vec<StoredDocument> = serde_json::from_str(&data)?;
            *self.documents.write().await = documents;
        }
        Ok(())
    }
    
    async fn save(&self) -> Result<()> {
        let documents = self.documents.read().await;
        let data = serde_json::to_string_pretty(&*documents)?;
        fs::write(&self.store_path, data).await?;
        Ok(())
    }
}

#[async_trait]
impl VectorStore for PersistentLocalVectorStore {
    async fn initialize(&self, _collection_name: &str, _vector_size: usize) -> Result<()> {
        Ok(())
    }

    async fn upsert_documents(&self, documents: Vec<Document>) -> Result<()> {
        let mut store = self.documents.write().await;
        
        for doc in documents {
            // Remove existing document with same ID if any
            store.retain(|d| d.id != doc.id);
            
            // Add new document
            store.push(StoredDocument {
                id: doc.id,
                embedding: doc.embedding,
                metadata: doc.metadata,
            });
        }
        
        drop(store); // Release the lock before saving
        self.save().await?;
        Ok(())
    }

    async fn search(&self, query_vector: Vec<f32>, top_k: usize) -> Result<Vec<SearchResult>> {
        let store = self.documents.read().await;
        let mut results: Vec<(f32, &StoredDocument)> = store
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
        let mut store = self.documents.write().await;
        let path_str = file_path.display().to_string();
        store.retain(|doc| {
            doc.metadata
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| p != path_str)
                .unwrap_or(true)
        });
        
        drop(store); // Release the lock before saving
        self.save().await?;
        Ok(())
    }

    fn create_document(&self, chunk: CodeChunk, embedding: Vec<f32>) -> Document {
        Document {
            id: uuid::Uuid::new_v4().to_string(),
            embedding,
            metadata: HashMap::from([
                ("file_path".to_string(), serde_json::json!(chunk.file_path)),
                ("content".to_string(), serde_json::json!(chunk.content)),
                ("line_start".to_string(), serde_json::json!(chunk.line_start)),
                ("line_end".to_string(), serde_json::json!(chunk.line_end)),
                ("chunk_type".to_string(), serde_json::json!(format!("{:?}", chunk.chunk_type))),
                ("language".to_string(), serde_json::json!(chunk.language)),
            ]),
        }
    }
}