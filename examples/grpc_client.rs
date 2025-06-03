use tonic::Request;

// Include the generated proto code
pub mod buildli {
    tonic::include_proto!("buildli");
}

use buildli::{
    buildli_service_client::BuildliServiceClient,
    QueryRequest, IndexStatusRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the gRPC server
    let mut client = BuildliServiceClient::connect("http://localhost:9091").await?;
    println!("✓ Connected to buildli gRPC server");

    // Test index status
    println!("\n1. Testing index status...");
    let request = Request::new(IndexStatusRequest {
        paths: vec![],
    });
    
    let response = client.index_status(request).await?;
    let status = response.into_inner();
    
    println!("Index Status:");
    println!("  Total files: {}", status.total_files);
    println!("  Indexed files: {}", status.indexed_files);
    println!("  Total chunks: {}", status.total_chunks);
    println!("  Last updated: {}", status.last_updated);

    // Test query (requires API key to be configured)
    println!("\n2. Testing query...");
    let request = Request::new(QueryRequest {
        question: "How does the CLI parsing work?".to_string(),
        top_k: 5,
        repos: vec![],
        languages: vec![],
    });
    
    match client.query(request).await {
        Ok(response) => {
            let mut stream = response.into_inner();
            
            println!("Query results:");
            while let Some(result) = stream.message().await? {
                if !result.chunk.is_empty() {
                    println!("Answer: {}", result.chunk);
                }
                if !result.references.is_empty() {
                    println!("\nReferences:");
                    for reference in result.references {
                        println!("  - {}:{}", reference.file_path, reference.line_start);
                    }
                }
            }
        }
        Err(e) => {
            println!("Query error (expected if no API key configured): {}", e);
        }
    }

    Ok(())
}

// To run this example:
// 1. Add to Cargo.toml:
//    [[example]]
//    name = "grpc_client"
//    required-features = []
//
// 2. Add these dependencies to Cargo.toml:
//    [dependencies]
//    tonic = "0.12"
//
// 3. Run: cargo run --example grpc_client