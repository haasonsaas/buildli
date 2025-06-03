# buildli CLI

A Rust-native command-line assistant for understanding and navigating codebases in plain English.

## Features

- 🔍 **Natural Language Search**: Query your codebase using plain English questions
- 🚀 **Fast Indexing**: Parse and index code using tree-sitter for 500+ languages
- 🧠 **Smart Embeddings**: OpenAI or local embeddings for semantic search
- 📊 **Vector Storage**: Qdrant or local vector store for efficient retrieval
- 🔄 **Auto-reindexing**: Watch mode for automatic updates when files change
- 🛠️ **Bug Solver Mode** *(Coming Soon)*: Analyze bugs and get patch suggestions
- 🌐 **API Server**: gRPC and REST endpoints for integration
- 🔄 **Auto-update** *(Coming Soon)*: Keep buildli up to date automatically

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/haasonsaas/buildli.git
cd buildli

# Build the release binary
cargo build --release

# Install to PATH
sudo cp target/release/buildli /usr/local/bin/
```

### Prerequisites

- Rust 1.70+ (install from https://rustup.rs)
- Optional: Qdrant vector database for remote storage
- Optional: OpenAI API key for embeddings

## Quick Start

1. **Configure buildli** (optional):
```bash
# Set OpenAI API key for embeddings
buildli config --set llm.api_key=<your-openai-key>

# View configuration
buildli config --print
```

2. **Index your codebase**:
```bash
# Index current directory
buildli index .

# Index with file watching
buildli index . --watch

# Index multiple paths
buildli index /path/to/project1 /path/to/project2
```

3. **Query your code**:
```bash
# Ask questions about your codebase
buildli query "Where is the OAuth token refreshed?"

# Get more results
buildli query "How does the billing system work?" --top-k 20

# Get JSON output for scripting
buildli query "Find all database connections" --json
```

## Configuration

Configuration is stored in `~/.buildli/config.toml`:

```toml
[paths]
index_root = ["/home/dev/repos"]

[llm]
provider = "openai"
model = "gpt-4o-mini"
api_key = "env:OPENAI_API_KEY"
temperature = 0.3

[vector]
backend = "qdrant"  # or "local"
url = "http://127.0.0.1:6333"
collection_name = "buildli"

[embedding]
provider = "openai"  # or "local"
model = "text-embedding-3-small"
batch_size = 100
```

## Current Status

### ✅ Working Features
- Natural language code search with OpenAI integration
- Multi-language code parsing (Rust, Python, JavaScript, TypeScript, Go, Java, C/C++)
- Persistent local vector storage
- Configuration management
- File watching with auto-reindexing
- REST API server
- gRPC API server with streaming support

### 🚧 Coming Soon
- **Bug Solver Mode**: Automated bug analysis and patch generation
- **Auto-update**: Self-updating binary releases
- **Repository/Language Filtering**: Filter queries by specific repos or languages

## Commands

### `buildli index`
Parse and embed code from specified paths.

```bash
buildli index <paths...> [OPTIONS]

Options:
  -w, --watch          Watch for changes and auto-reindex
  -c, --commit <SHA>   Index from specific commit
  --ignore-tests       Ignore test files
```

### `buildli query`
Query the indexed codebase with natural language.

```bash
buildli query "<question>" [OPTIONS]

Options:
  -k, --top-k <N>      Number of top results (default: 10)
  --json               Output in JSON format
  -r, --repo <REPO>    Filter by repository
  -l, --lang <LANG>    Filter by language
```

### `buildli bug` *(Coming Soon)*
Analyze and solve bugs based on description.

```bash
buildli bug --desc "<description>" [OPTIONS]

Options:
  --apply              Apply the suggested patch
  --patch-file <FILE>  Save patch to file
  --no-stream          Disable streaming output
```

*Note: This feature is under development and will be available in a future release.*

### `buildli serve`
Start the API server (HTTP and gRPC).

```bash
buildli serve [OPTIONS]

Options:
  -p, --port <PORT>    Server port (default: 8080)
  -t, --token <TOKEN>  API authentication token
```

The server starts:
- HTTP API on the specified port (default: 8080)
- gRPC API on port + 1 (default: 8081)

### `buildli config`
Manage configuration.

```bash
buildli config [OPTIONS]

Options:
  --set <KEY=VALUE>    Set configuration value
  --print              Print current configuration
```

### `buildli update` *(Coming Soon)*
Update buildli to the latest version.

```bash
buildli update [OPTIONS]

Options:
  --channel <CHANNEL>  Release channel (default: stable)
```

*Note: Auto-update functionality is planned for a future release. For now, please update manually by pulling the latest source and rebuilding.*

## API Integration

### gRPC API

The gRPC API provides streaming responses for real-time interaction:

```protobuf
service BuildliService {
    rpc Query(QueryRequest) returns (stream QueryResponse);
    rpc BugSolve(BugSolveRequest) returns (stream BugSolveResponse);
    rpc IndexStatus(IndexStatusRequest) returns (IndexStatusResponse);
}
```

Connect to the gRPC server on port 8081 (or your configured port + 1).

### REST API

The REST API provides simple HTTP endpoints:
- `GET /health` - Health check
- `POST /v1/query` - Query the codebase
- `GET /v1/index/status` - Get indexing status

## Architecture

buildli uses a modular architecture:

- **Parser**: Tree-sitter based code parsing for AST extraction
- **Embeddings**: Pluggable embedding providers (OpenAI, local)
- **Vector Store**: Pluggable vector stores (Qdrant, local)
- **Query Engine**: Natural language processing with LLM integration
- **Server**: gRPC and REST API for external integrations

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- query "test query"

# Format code
cargo fmt

# Run linter
cargo clippy
```

## License

MIT