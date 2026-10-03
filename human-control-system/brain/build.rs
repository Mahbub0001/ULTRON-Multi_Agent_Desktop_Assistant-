fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_path = "src/proto/brain.proto";
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    
    let mut prost = prost_build::Config::new();
    prost.retain_enum_prefix();
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_with_config(prost, &[proto_path], &["src/proto"])?;
    
    println!("cargo:rerun-if-changed={}", proto_path);
    Ok(())
}