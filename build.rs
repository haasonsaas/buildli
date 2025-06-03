use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the protoc path if available
    if let Ok(protoc_path) = env::var("PROTOC") {
        println!("cargo:info=Using PROTOC from environment: {}", protoc_path);
    } else {
        // Try to find protoc in common locations
        let possible_paths = vec![
            "/opt/homebrew/opt/protobuf@29/bin/protoc",
            "/opt/homebrew/opt/protobuf/bin/protoc",
            "/usr/local/bin/protoc",
            "/usr/bin/protoc",
        ];
        
        for path in possible_paths {
            if std::path::Path::new(path).exists() {
                env::set_var("PROTOC", path);
                println!("cargo:info=Found protoc at: {}", path);
                break;
            }
        }
    }
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .out_dir(out_dir)
        .compile_protos(&["proto/buildli.proto"], &["proto"])?;
        
    println!("cargo:rerun-if-changed=proto/buildli.proto");
    
    Ok(())
}