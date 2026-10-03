//! Brain Service Main Entry Point


use brain::proto::brain_proto::brain_service_server::BrainServiceServer;
use brain::service::BrainServiceImpl;
use brain::config::BrainConfig;
use tonic::transport::Server;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();
    
    info!("Starting Brain Service");
    
    // Load configuration
    let config = BrainConfig::load().unwrap_or_default();
    info!(host = %config.server.host, port = config.server.port, "Configuration loaded");
    
    // Create service
    let service = BrainServiceImpl::new(config.clone());
    let service_state = service.state().clone();
    
    // Start metrics server if enabled
    if config.metrics.enabled {
        let metrics_addr: SocketAddr = format!("{}:{}", config.server.host, config.metrics.port).parse()?;
        let metrics_service = service_state.clone();
        tokio::spawn(async move {
            if let Err(e) = run_metrics_server(metrics_addr, metrics_service).await {
                error!(error = %e, "Metrics server error");
            }
        });
    }
    
    // Start gRPC server
    let grpc_addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    info!(address = %grpc_addr, "Starting gRPC server");
    
    let grpc_server = Server::builder()
        .add_service(BrainServiceServer::new(service))
        .serve(grpc_addr);
    
    // Handle shutdown signals
    let shutdown = async {
        signal::ctrl_c().await.ok();
        info!("Shutdown signal received");
    };
    
    tokio::select! {
        result = grpc_server => {
            if let Err(e) = result {
                error!(error = %e, "gRPC server error");
            }
        }
        _ = shutdown => {
            info!("Shutting down gracefully");
        }
    }
    
    info!("Brain Service stopped");
    Ok(())
}

/// Run Prometheus metrics server
async fn run_metrics_server(addr: SocketAddr, _state: Arc<brain::service::BrainServiceState>) -> anyhow::Result<()> {
    use warp::Filter;
    
    let metrics_route = warp::path("metrics")
        .map(|| {
            // Generate metrics
            let metrics = "# HELP brain_uptime_seconds Brain service uptime in seconds\n\
                           # TYPE brain_uptime_seconds counter\n\
                           brain_uptime_seconds 0\n";
            warp::reply::with_header(metrics, "Content-Type", "text/plain; version=0.0.4")
        });
    
    warp::serve(metrics_route).run(addr).await;
    Ok(())
}