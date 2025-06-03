pub mod factory;

use crate::{embeddings::EmbeddingProvider, vector::{VectorStore, SearchResult}, BuildliError, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::debug;

pub struct QueryEngine<E: EmbeddingProvider, V: VectorStore> {
    embedder: E,
    vector_store: V,
    llm_client: LlmClient,
}

impl<E: EmbeddingProvider, V: VectorStore> QueryEngine<E, V> {
    pub fn new(embedder: E, vector_store: V, llm_client: LlmClient) -> Self {
        Self {
            embedder,
            vector_store,
            llm_client,
        }
    }

    pub async fn query(
        &self,
        question: &str,
        top_k: usize,
        stream_output: bool,
    ) -> Result<QueryResponse> {
        debug!("Processing query: {}", question);
        
        let query_embedding = self.embedder.embed(question).await
            .map_err(|e| BuildliError::Embedding(e.to_string()))?;
        
        let search_results = self.vector_store.search(query_embedding, top_k).await
            .map_err(|e| BuildliError::VectorStore(e.to_string()))?;
        
        if search_results.is_empty() {
            return Ok(QueryResponse {
                answer: "No relevant code found for your query.".to_string(),
                references: vec![],
            });
        }
        
        let context = self.build_context(&search_results);
        let references = self.extract_references(&search_results);
        
        let answer = if stream_output {
            self.llm_client.stream_completion(question, &context).await?
        } else {
            self.llm_client.completion(question, &context).await?
        };
        
        Ok(QueryResponse {
            answer,
            references,
        })
    }

    fn build_context(&self, results: &[SearchResult]) -> String {
        let mut context = String::new();
        
        for (i, result) in results.iter().enumerate() {
            if let Some(content) = result.metadata.get("content").and_then(|v| v.as_str()) {
                if let Some(file_path) = result.metadata.get("file_path").and_then(|v| v.as_str()) {
                    context.push_str(&format!("\n--- Result {} (score: {:.3}) ---\n", i + 1, result.score));
                    context.push_str(&format!("File: {}\n", file_path));
                    if let Some(line_start) = result.metadata.get("line_start").and_then(|v| v.as_u64()) {
                        context.push_str(&format!("Lines: {}", line_start));
                        if let Some(line_end) = result.metadata.get("line_end").and_then(|v| v.as_u64()) {
                            context.push_str(&format!("-{}", line_end));
                        }
                        context.push('\n');
                    }
                    context.push_str("```\n");
                    context.push_str(content);
                    context.push_str("\n```\n");
                }
            }
        }
        
        context
    }

    fn extract_references(&self, results: &[SearchResult]) -> Vec<CodeReference> {
        results
            .iter()
            .filter_map(|result| {
                let file_path = result.metadata.get("file_path")?.as_str()?.to_string();
                let line_start = result.metadata.get("line_start")?.as_u64()? as usize;
                let line_end = result.metadata.get("line_end")?.as_u64()? as usize;
                let snippet = result.metadata.get("content")?.as_str()?.to_string();
                
                Some(CodeReference {
                    file_path,
                    line_start,
                    line_end,
                    snippet,
                    relevance_score: result.score,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub references: Vec<CodeReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReference {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub snippet: String,
    pub relevance_score: f32,
}

pub struct LlmClient {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
}

impl LlmClient {
    pub fn new(api_key: String, model: String, temperature: f32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        
        Self {
            client,
            api_key,
            model,
            temperature,
        }
    }

    pub async fn completion(&self, question: &str, context: &str) -> Result<String> {
        let prompt = format!(
            "You are a helpful code assistant. Based on the following code context, answer the user's question.\n\
            \n\
            Context:\n{}\n\
            \n\
            Question: {}\n\
            \n\
            Please provide a clear and concise answer, referencing specific files and line numbers when relevant.",
            context, question
        );
        
        let request = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You are a helpful code assistant."},
                {"role": "user", "content": prompt}
            ],
            "temperature": self.temperature,
        });
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BuildliError::Network(format!("Failed to send request to OpenAI: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(BuildliError::Network(format!("OpenAI API error: {}", response.status())));
        }
        
        let response_body: serde_json::Value = response.json().await
            .map_err(|e| BuildliError::Network(e.to_string()))?;
        let answer = response_body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response")
            .to_string();
        
        Ok(answer)
    }

    pub async fn stream_completion(&self, question: &str, context: &str) -> Result<String> {
        let prompt = format!(
            "You are a helpful code assistant. Based on the following code context, answer the user's question.\n\
            \n\
            Context:\n{}\n\
            \n\
            Question: {}\n\
            \n\
            Please provide a clear and concise answer, referencing specific files and line numbers when relevant.",
            context, question
        );
        
        let request = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You are a helpful code assistant."},
                {"role": "user", "content": prompt}
            ],
            "temperature": self.temperature,
            "stream": true,
        });
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| BuildliError::Network(format!("Failed to send request to OpenAI: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(BuildliError::Network(format!("OpenAI API error: {}", response.status())));
        }
        
        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        let mut stdout = tokio::io::stdout();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| BuildliError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);
            
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }
                    
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            stdout.write_all(content.as_bytes()).await
                                .map_err(|e| BuildliError::Io(e))?;
                            stdout.flush().await
                                .map_err(|e| BuildliError::Io(e))?;
                        }
                    }
                }
            }
        }
        
        stdout.write_all(b"\n").await
            .map_err(|e| BuildliError::Io(e))?;
        Ok(full_response)
    }
}