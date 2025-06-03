use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub paths: PathsConfig,
    
    #[serde(default)]
    pub llm: LlmConfig,
    
    #[serde(default)]
    pub vector: VectorConfig,
    
    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_index_root")]
    pub index_root: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    
    #[serde(default = "default_llm_model")]
    pub model: String,
    
    pub api_key: Option<String>,
    
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    #[serde(default = "default_vector_backend")]
    pub backend: String,
    
    #[serde(default = "default_vector_url")]
    pub url: String,
    
    #[serde(default = "default_collection_name")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embedding_provider")]
    pub provider: String,
    
    #[serde(default = "default_embedding_model")]
    pub model: String,
    
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig::default(),
            llm: LlmConfig::default(),
            vector: VectorConfig::default(),
            embedding: EmbeddingConfig::default(),
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            index_root: default_index_root(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: default_llm_model(),
            api_key: None,
            temperature: default_temperature(),
        }
    }
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            backend: default_vector_backend(),
            url: default_vector_url(),
            collection_name: default_collection_name(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_embedding_provider(),
            model: default_embedding_model(),
            batch_size: default_batch_size(),
        }
    }
}

fn default_index_root() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_llm_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_temperature() -> f32 {
    0.3
}

fn default_vector_backend() -> String {
    "qdrant".to_string()
}

fn default_vector_url() -> String {
    "http://127.0.0.1:6333".to_string()
}

fn default_collection_name() -> String {
    "buildli".to_string()
}

fn default_embedding_provider() -> String {
    "openai".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_batch_size() -> usize {
    100
}

#[derive(Clone)]
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "buildli")
            .context("Failed to determine project directories")?;
        
        let config_dir = project_dirs.config_dir();
        let config_path = config_dir.join("config.toml");
        
        Ok(Self { config_path })
    }

    pub async fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&self.config_path)
            .await
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(self.resolve_env_vars(config))
    }

    pub async fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&self.config_path, content)
            .await
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub async fn set_value(&self, key: &str, value: &str) -> Result<()> {
        let mut config = self.load().await?;
        
        match key {
            "llm.provider" => config.llm.provider = value.to_string(),
            "llm.model" => config.llm.model = value.to_string(),
            "llm.api_key" => config.llm.api_key = Some(value.to_string()),
            "llm.temperature" => config.llm.temperature = value.parse()?,
            "vector.backend" => config.vector.backend = value.to_string(),
            "vector.url" => config.vector.url = value.to_string(),
            "vector.collection_name" => config.vector.collection_name = value.to_string(),
            "embedding.provider" => config.embedding.provider = value.to_string(),
            "embedding.model" => config.embedding.model = value.to_string(),
            "embedding.batch_size" => config.embedding.batch_size = value.parse()?,
            _ => anyhow::bail!("Unknown configuration key: {}", key),
        }
        
        self.save(&config).await?;
        Ok(())
    }

    fn resolve_env_vars(&self, mut config: Config) -> Config {
        if let Some(api_key) = &config.llm.api_key {
            if api_key.starts_with("env:") {
                let env_var = api_key.strip_prefix("env:").unwrap();
                config.llm.api_key = std::env::var(env_var).ok();
            }
        }
        
        if let Ok(url) = std::env::var("BUILDLI_VECTOR_URL") {
            config.vector.url = url;
        }
        
        config
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}