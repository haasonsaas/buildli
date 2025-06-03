use anyhow::Result;
use buildli::{
    cli::{Cli, Commands},
    config::ConfigManager,
    embeddings::{LocalEmbeddings, OpenAIEmbeddings},
    indexer::{
        factory::{BuildliIndexer, EmbeddingProviderType, VectorStoreType},
        Indexer,
    },
    query::{factory::BuildliQueryEngine, LlmClient, QueryEngine},
    utils::{print_error, print_info, print_success, print_warning},
    vector::{PersistentLocalVectorStore, QdrantStore, VectorStore},
};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
    
    let config_manager = ConfigManager::new()?;
    let _config = config_manager.load().await?;
    
    match cli.command {
        Commands::Index { paths, watch, commit, ignore_tests } => {
            handle_index(config_manager, paths, watch, commit, ignore_tests).await?;
        }
        Commands::Query { question, top_k, json, repo, lang } => {
            handle_query(config_manager, question, top_k, json, repo, lang).await?;
        }
        Commands::Bug { desc, apply, patch_file, no_stream } => {
            handle_bug(config_manager, desc, apply, patch_file, no_stream).await?;
        }
        Commands::Serve { port, token } => {
            handle_serve(config_manager, port, token).await?;
        }
        Commands::Config { set, print } => {
            handle_config(config_manager, set, print).await?;
        }
        Commands::Update { channel } => {
            handle_update(channel).await?;
        }
    }
    
    Ok(())
}

async fn handle_index(
    config_manager: ConfigManager,
    paths: Vec<PathBuf>,
    watch: bool,
    _commit: Option<String>,
    _ignore_tests: bool,
) -> Result<()> {
    let config = config_manager.load().await?;
    
    if config.llm.api_key.is_none() && config.embedding.provider == "openai" {
        print_error("OpenAI API key not set. Please run: buildli config --set llm.api_key=<your-key>");
        return Ok(());
    }
    
    let embedder = match config.embedding.provider.as_str() {
        "openai" => EmbeddingProviderType::OpenAI(OpenAIEmbeddings::new(
            config.llm.api_key.clone().unwrap(),
            config.embedding.model.clone(),
        )),
        _ => EmbeddingProviderType::Local(LocalEmbeddings::new()),
    };
    
    let vector_store = match config.vector.backend.as_str() {
        "qdrant" => {
            let store = QdrantStore::new(&config.vector.url, &config.vector.collection_name).await?;
            store.initialize(&config.vector.collection_name, 384).await?;
            VectorStoreType::Qdrant(store)
        }
        _ => VectorStoreType::Local(PersistentLocalVectorStore::new().await?),
    };
    
    let mut indexer: BuildliIndexer = Indexer::new(embedder, vector_store);
    
    let paths_to_index = if paths.is_empty() {
        config.paths.index_root.clone()
    } else {
        paths
    };
    
    print_info(&format!("Starting indexing of {} paths", paths_to_index.len()));
    
    for path in paths_to_index {
        let stats = indexer.index_path(&path, watch).await?;
        print_success(&format!(
            "Indexed {} files ({} chunks) from {}",
            stats.indexed_files,
            stats.total_chunks,
            path.display()
        ));
        
        if stats.failed_files > 0 {
            print_warning(&format!("{} files failed to index", stats.failed_files));
        }
    }
    
    Ok(())
}

async fn handle_query(
    config_manager: ConfigManager,
    question: String,
    top_k: usize,
    json: bool,
    _repo: Option<Vec<String>>,
    _lang: Option<Vec<String>>,
) -> Result<()> {
    let config = config_manager.load().await?;
    
    if config.llm.api_key.is_none() {
        print_error("OpenAI API key not set. Please run: buildli config --set llm.api_key=<your-key>");
        return Ok(());
    }
    
    let embedder = match config.embedding.provider.as_str() {
        "openai" => EmbeddingProviderType::OpenAI(OpenAIEmbeddings::new(
            config.llm.api_key.clone().unwrap(),
            config.embedding.model.clone(),
        )),
        _ => EmbeddingProviderType::Local(LocalEmbeddings::new()),
    };
    
    let vector_store = match config.vector.backend.as_str() {
        "qdrant" => VectorStoreType::Qdrant(
            QdrantStore::new(&config.vector.url, &config.vector.collection_name).await?,
        ),
        _ => VectorStoreType::Local(PersistentLocalVectorStore::new().await?),
    };
    
    let llm_client = LlmClient::new(
        config.llm.api_key.unwrap(),
        config.llm.model.clone(),
        config.llm.temperature,
    );
    
    let query_engine: BuildliQueryEngine = QueryEngine::new(embedder, vector_store, llm_client);
    
    let response = query_engine.query(&question, top_k, !json).await?;
    
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("\n{}", response.answer);
        
        if !response.references.is_empty() {
            println!("\n{}", "References:".bold());
            for reference in response.references {
                println!(
                    "  {} {}:{}",
                    "→".cyan(),
                    reference.file_path,
                    reference.line_start
                );
            }
        }
    }
    
    Ok(())
}

async fn handle_bug(
    _config_manager: ConfigManager,
    desc: String,
    _apply: bool,
    _patch_file: Option<PathBuf>,
    _no_stream: bool,
) -> Result<()> {
    print_info(&format!("Bug solver mode for '{}' is not yet implemented", desc));
    Ok(())
}

async fn handle_serve(
    config_manager: ConfigManager,
    port: u16,
    token: Option<String>,
) -> Result<()> {
    print_info(&format!("Starting server on port {}", port));
    buildli::server::run_server(port, token, config_manager).await?;
    Ok(())
}

async fn handle_config(
    config_manager: ConfigManager,
    set: Option<String>,
    print: bool,
) -> Result<()> {
    if let Some(key_value) = set {
        let parts: Vec<&str> = key_value.splitn(2, '=').collect();
        if parts.len() != 2 {
            print_error("Invalid format. Use: --set key=value");
            return Ok(());
        }
        
        config_manager.set_value(parts[0], parts[1]).await?;
        print_success(&format!("Set {} = {}", parts[0], parts[1]));
    }
    
    if print {
        let config = config_manager.load().await?;
        let config_str = toml::to_string_pretty(&config)?;
        println!("{}", config_str);
        println!("\nConfig file location: {}", config_manager.config_path().display());
    }
    
    Ok(())
}

async fn handle_update(channel: String) -> Result<()> {
    print_info(&format!("Checking for updates on {} channel...", channel));
    print_warning("Auto-update feature not yet implemented. Please update manually.");
    Ok(())
}
