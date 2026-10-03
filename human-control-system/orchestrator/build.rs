fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile orchestrator proto
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["src/proto/orchestrator.proto"], &["src/proto"])?;

    // Also compile agent proto for client
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../agent/proto/agent.proto"],
            &["../agent/proto"],
        )?;

    // Compile vision proto
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../vision/src/proto/vision.proto"],
            &["../vision/src/proto"],
        )?;

    // Compile brain proto
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../brain/src/proto/brain.proto"],
            &["../brain/src/proto"],
        )?;

    // Compile adapters proto
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../adapters/src/proto/adapters.proto"],
            &["../adapters/src/proto"],
        )?;

    Ok(())
}
