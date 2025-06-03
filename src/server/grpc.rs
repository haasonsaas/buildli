use crate::{
    config::ConfigManager,
    embeddings::{LocalEmbeddings, OpenAIEmbeddings},
    indexer::factory::{EmbeddingProviderType, VectorStoreType},
    query::{factory::BuildliQueryEngine, LlmClient, QueryEngine},
    vector::{PersistentLocalVectorStore, QdrantStore, VectorStore},
    BuildliError,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

// Import the generated proto types
pub mod proto {
    tonic::include_proto!("buildli");
}

use proto::{
    buildli_service_server::{BuildliService, BuildliServiceServer},
    BugSolveRequest, BugSolveResponse, CodeReference, IndexStatusRequest, IndexStatusResponse,
    QueryRequest, QueryResponse,
};

pub struct BuildliGrpcService {
    config_manager: ConfigManager,
    stats: Arc<RwLock<IndexStats>>,
}

#[derive(Default)]
struct IndexStats {
    total_files: i64,
    indexed_files: i64,
    total_chunks: i64,
    last_updated: String,
}

impl BuildliGrpcService {
    pub fn new(config_manager: ConfigManager) -> Self {
        Self {
            config_manager,
            stats: Arc::new(RwLock::new(IndexStats::default())),
        }
    }
}

#[tonic::async_trait]
impl BuildliService for BuildliGrpcService {
    type QueryStream = Pin<Box<dyn Stream<Item = Result<QueryResponse, Status>> + Send>>;
    type BugSolveStream = Pin<Box<dyn Stream<Item = Result<BugSolveResponse, Status>> + Send>>;

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::QueryStream>, Status> {
        let query_request = request.into_inner();
        let config = self.config_manager.load().await.map_err(|e| {
            Status::internal(format!("Failed to load configuration: {}", e))
        })?;

        if config.llm.api_key.is_none() {
            return Err(Status::failed_precondition(
                "OpenAI API key not configured. Set llm.api_key in config.",
            ));
        }

        // Create embedder
        let embedder = match config.embedding.provider.as_str() {
            "openai" => EmbeddingProviderType::OpenAI(OpenAIEmbeddings::new(
                config.llm.api_key.clone().unwrap(),
                config.embedding.model.clone(),
            )),
            _ => EmbeddingProviderType::Local(LocalEmbeddings::new()),
        };

        // Create vector store
        let vector_store = match config.vector.backend.as_str() {
            "qdrant" => {
                let store = QdrantStore::new(&config.vector.url, &config.vector.collection_name)
                    .await
                    .map_err(|e| Status::internal(format!("Failed to create vector store: {}", e)))?;
                VectorStoreType::Qdrant(store)
            }
            _ => {
                let store = PersistentLocalVectorStore::new()
                    .await
                    .map_err(|e| Status::internal(format!("Failed to create vector store: {}", e)))?;
                VectorStoreType::Local(store)
            }
        };

        // Create LLM client
        let llm_client = LlmClient::new(
            config.llm.api_key.unwrap(),
            config.llm.model.clone(),
            config.llm.temperature,
        );

        // Create query engine
        let query_engine: BuildliQueryEngine = QueryEngine::new(embedder, vector_store, llm_client);
        
        let top_k = query_request.top_k.max(1) as usize;
        let question = query_request.question.clone();

        // Create a channel for streaming responses
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        
        tokio::spawn(async move {
            match query_engine.query(&question, top_k, true).await {
                Ok(response) => {
                    // Send the main response
                    let proto_references: Vec<CodeReference> = response
                        .references
                        .into_iter()
                        .map(|r| CodeReference {
                            file_path: r.file_path,
                            line_start: r.line_start as i32,
                            line_end: r.line_end as i32,
                            snippet: r.snippet,
                            relevance_score: r.relevance_score,
                        })
                        .collect();

                    let _ = tx
                        .send(Ok(QueryResponse {
                            chunk: response.answer,
                            references: proto_references,
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!("Query failed: {}", e))))
                        .await;
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }

    async fn bug_solve(
        &self,
        request: Request<BugSolveRequest>,
    ) -> Result<Response<Self::BugSolveStream>, Status> {
        let bug_request = request.into_inner();
        
        // For now, return a simple response indicating the feature is coming soon
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        
        tokio::spawn(async move {
            let _ = tx
                .send(Ok(BugSolveResponse {
                    chunk: format!(
                        "Bug solver mode for '{}' is coming soon. This feature will analyze your bug description and suggest patches.",
                        bug_request.description
                    ),
                    patch: String::new(),
                    affected_files: vec![],
                }))
                .await;
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }

    async fn index_status(
        &self,
        _request: Request<IndexStatusRequest>,
    ) -> Result<Response<IndexStatusResponse>, Status> {
        let stats = self.stats.read().await;
        
        Ok(Response::new(IndexStatusResponse {
            total_files: stats.total_files,
            indexed_files: stats.indexed_files,
            total_chunks: stats.total_chunks,
            last_updated: stats.last_updated.clone(),
        }))
    }
}

pub fn create_grpc_service(config_manager: ConfigManager) -> BuildliServiceServer<BuildliGrpcService> {
    let service = BuildliGrpcService::new(config_manager);
    BuildliServiceServer::new(service)
}