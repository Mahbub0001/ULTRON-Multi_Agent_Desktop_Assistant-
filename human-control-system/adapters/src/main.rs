//! Adapter Service Main Entry Point


use hcs_adapters::service::AdapterServiceImpl;
use hcs_adapters::config::AdaptersConfig;
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

    info!("Starting Adapter Service");

    // Load configuration
    let config = AdaptersConfig::load().unwrap_or_default();
    info!(host = %config.server.host, port = config.server.port, "Configuration loaded");

    // Create service
    let service = AdapterServiceImpl::new(config.clone()).await?;
    let service_state = service.state();

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

    let (ps_srv, ch_srv, gm_srv, wn_srv) = service.into_servers();
    let grpc_server = Server::builder()
        .add_service(ps_srv)
        .add_service(ch_srv)
        .add_service(gm_srv)
        .add_service(wn_srv)
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

    info!("Adapter Service stopped");
    Ok(())
}

/// Run Prometheus metrics server
async fn run_metrics_server(addr: SocketAddr, _state: Arc<hcs_adapters::service::AdapterServiceState>) -> anyhow::Result<()> {
    use warp::Filter;

    let metrics_route = warp::path("metrics")
        .map(|| {
            let metrics = "# HELP adapters_uptime_seconds Adapter service uptime in seconds\n\
                           # TYPE adapters_uptime_seconds counter\n\
                           adapters_uptime_seconds 0\n";
            warp::reply::with_header(metrics, "Content-Type", "text/plain; version=0.0.4")
        });

    warp::serve(metrics_route).run(addr).await;
    Ok(())
}