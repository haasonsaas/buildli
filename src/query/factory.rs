use crate::{
    indexer::factory::{EmbeddingProviderType, VectorStoreType},
    query::QueryEngine,
};

pub type BuildliQueryEngine = QueryEngine<EmbeddingProviderType, VectorStoreType>;