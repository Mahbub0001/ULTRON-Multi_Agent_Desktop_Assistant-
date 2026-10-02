// Build script for gRPC code generation

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = &["proto/agent.proto"];
    let proto_dir = "proto";
    
    // Generate Rust code from proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(proto_files, &[proto_dir])?;
    
    // Re-run if proto files change
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={}", proto_file);
    }
    
    Ok(())
}