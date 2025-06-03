use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "buildli",
    about = "A Rust-native command-line assistant for understanding and navigating codebases in plain English",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Parse and embed code from specified paths")]
    Index {
        #[arg(help = "Paths to index (directories or files)")]
        paths: Vec<PathBuf>,

        #[arg(short, long, help = "Watch for changes and auto-reindex")]
        watch: bool,

        #[arg(short, long, help = "Index from specific commit")]
        commit: Option<String>,

        #[arg(long, help = "Ignore test files")]
        ignore_tests: bool,
    },

    #[command(about = "Query the indexed codebase with natural language")]
    Query {
        #[arg(help = "Natural language question about the codebase")]
        question: String,

        #[arg(short = 'k', long, default_value = "10", help = "Number of top results")]
        top_k: usize,

        #[arg(long, help = "Output format (json for machine-readable)")]
        json: bool,

        #[arg(short, long, help = "Filter by repository")]
        repo: Option<Vec<String>>,

        #[arg(short, long, help = "Filter by language")]
        lang: Option<Vec<String>>,
    },

    #[command(about = "Analyze and solve bugs based on description")]
    Bug {
        #[arg(short, long, help = "Bug description")]
        desc: String,

        #[arg(long, help = "Apply the suggested patch")]
        apply: bool,

        #[arg(long, help = "Save patch to file")]
        patch_file: Option<PathBuf>,

        #[arg(long, help = "Disable streaming output")]
        no_stream: bool,
    },

    #[command(about = "Start gRPC/REST/SSE server")]
    Serve {
        #[arg(short, long, default_value = "8080", help = "Server port")]
        port: u16,

        #[arg(short, long, help = "API authentication token")]
        token: Option<String>,
    },

    #[command(about = "Manage configuration")]
    Config {
        #[arg(long, help = "Set configuration value (e.g., --set openai.api_key=...)")]
        set: Option<String>,

        #[arg(long, help = "Print current configuration")]
        print: bool,
    },

    #[command(about = "Update buildli to the latest version")]
    Update {
        #[arg(long, default_value = "stable", help = "Release channel")]
        channel: String,
    },
}