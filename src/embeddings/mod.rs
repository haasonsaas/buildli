use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

pub struct OpenAIEmbeddings {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl OpenAIEmbeddings {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self {
            client,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
        };
        
        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("OpenAI API error: {}", response.status());
        }
        
        let embedding_response: EmbeddingResponse = response.json().await?;
        let embeddings = embedding_response
            .data
            .into_iter()
            .map(|data| data.embedding)
            .collect();
        
        Ok(embeddings)
    }
}

pub struct LocalEmbeddings {
}

impl LocalEmbeddings {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EmbeddingProvider for LocalEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let hash = Sha256::digest(text.as_bytes());
        let mut embedding = vec![0.0; 384];
        
        for (i, &byte) in hash.iter().enumerate() {
            if i < embedding.len() {
                embedding[i] = (byte as f32) / 255.0;
            }
        }
        
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
}