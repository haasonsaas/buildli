use crate::{
    embeddings::{EmbeddingProvider, LocalEmbeddings, OpenAIEmbeddings},
    indexer::Indexer,
    vector::{PersistentLocalVectorStore, QdrantStore, VectorStore},
};
use async_trait::async_trait;

pub enum EmbeddingProviderType {
    OpenAI(OpenAIEmbeddings),
    Local(LocalEmbeddings),
}

pub enum VectorStoreType {
    Qdrant(QdrantStore),
    Local(PersistentLocalVectorStore),
}

#[async_trait]
impl EmbeddingProvider for EmbeddingProviderType {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        match self {
            EmbeddingProviderType::OpenAI(provider) => provider.embed(text).await,
            EmbeddingProviderType::Local(provider) => provider.embed(text).await,
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        match self {
            EmbeddingProviderType::OpenAI(provider) => provider.embed_batch(texts).await,
            EmbeddingProviderType::Local(provider) => provider.embed_batch(texts).await,
        }
    }
}

#[async_trait]
impl VectorStore for VectorStoreType {
    async fn initialize(&self, collection_name: &str, vector_size: usize) -> anyhow::Result<()> {
        match self {
            VectorStoreType::Qdrant(store) => store.initialize(collection_name, vector_size).await,
            VectorStoreType::Local(store) => store.initialize(collection_name, vector_size).await,
        }
    }

    async fn upsert_documents(&self, documents: Vec<crate::vector::Document>) -> anyhow::Result<()> {
        match self {
            VectorStoreType::Qdrant(store) => store.upsert_documents(documents).await,
            VectorStoreType::Local(store) => store.upsert_documents(documents).await,
        }
    }

    async fn search(&self, query_vector: Vec<f32>, top_k: usize) -> anyhow::Result<Vec<crate::vector::SearchResult>> {
        match self {
            VectorStoreType::Qdrant(store) => store.search(query_vector, top_k).await,
            VectorStoreType::Local(store) => store.search(query_vector, top_k).await,
        }
    }

    async fn delete_by_file(&self, file_path: &std::path::Path) -> anyhow::Result<()> {
        match self {
            VectorStoreType::Qdrant(store) => store.delete_by_file(file_path).await,
            VectorStoreType::Local(store) => store.delete_by_file(file_path).await,
        }
    }

    fn create_document(&self, chunk: crate::indexer::parser::CodeChunk, embedding: Vec<f32>) -> crate::vector::Document {
        match self {
            VectorStoreType::Qdrant(store) => store.create_document(chunk, embedding),
            VectorStoreType::Local(store) => store.create_document(chunk, embedding),
        }
    }
}

pub type BuildliIndexer = Indexer<EmbeddingProviderType, VectorStoreType>;