pub mod factory;
pub mod parser;
pub mod walker;

use crate::{embeddings::EmbeddingProvider, vector::VectorStore, BuildliError, Result};
use anyhow::Context;
use parser::{CodeChunk, LanguageParser};
use std::path::Path;
use tracing::{debug, info};
use walker::FileWalker;

pub struct Indexer<E: EmbeddingProvider, V: VectorStore> {
    parser: LanguageParser,
    embedder: E,
    vector_store: V,
    file_walker: FileWalker,
}

impl<E: EmbeddingProvider, V: VectorStore> Indexer<E, V> {
    pub fn new(embedder: E, vector_store: V) -> Self {
        Self {
            parser: LanguageParser::new(),
            embedder,
            vector_store,
            file_walker: FileWalker::new(),
        }
    }

    pub async fn index_path(&mut self, path: &Path, watch: bool) -> Result<IndexStats> {
        info!("Starting indexing of path: {}", path.display());
        
        let mut stats = IndexStats::default();
        
        if watch {
            self.index_with_watch(path, &mut stats).await?;
        } else {
            self.index_once(path, &mut stats).await?;
        }
        
        Ok(stats)
    }

    async fn index_once(&mut self, path: &Path, stats: &mut IndexStats) -> Result<()> {
        let files = self.file_walker.walk(path)?;
        
        for file_path in files {
            if let Err(e) = self.index_file(&file_path, stats).await {
                debug!("Failed to index {}: {}", file_path.display(), e);
                stats.failed_files += 1;
            }
        }
        
        Ok(())
    }

    async fn index_with_watch(&mut self, path: &Path, stats: &mut IndexStats) -> Result<()> {
        self.index_once(path, stats).await?;
        
        let watcher = self.file_walker.watch(path)?;
        
        while let Ok(event) = watcher.recv() {
            match event {
                walker::WatchEvent::Created(path) | walker::WatchEvent::Modified(path) => {
                    if let Err(e) = self.index_file(&path, stats).await {
                        debug!("Failed to index changed file {}: {}", path.display(), e);
                    }
                }
                walker::WatchEvent::Deleted(path) => {
                    if let Err(e) = self.delete_file_chunks(&path).await {
                        debug!("Failed to delete chunks for {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn index_file(&mut self, path: &Path, stats: &mut IndexStats) -> Result<()> {
        debug!("Indexing file: {}", path.display());
        
        let chunks = self.parser.parse_file(path).await?;
        stats.total_files += 1;
        
        if chunks.is_empty() {
            return Ok(());
        }
        
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed_batch(&chunk_texts).await
            .map_err(|e| BuildliError::Embedding(e.to_string()))?;
        
        let documents: Vec<_> = chunks
            .into_iter()
            .zip(embeddings)
            .map(|(chunk, embedding)| {
                self.vector_store.create_document(chunk, embedding)
            })
            .collect();
        
        self.vector_store.upsert_documents(documents).await
            .map_err(|e| BuildliError::VectorStore(e.to_string()))?;
        
        stats.indexed_files += 1;
        stats.total_chunks += chunk_texts.len();
        
        Ok(())
    }

    async fn delete_file_chunks(&mut self, path: &Path) -> Result<()> {
        self.vector_store.delete_by_file(path).await
            .map_err(|e| BuildliError::VectorStore(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub total_files: usize,
    pub indexed_files: usize,
    pub failed_files: usize,
    pub total_chunks: usize,
}