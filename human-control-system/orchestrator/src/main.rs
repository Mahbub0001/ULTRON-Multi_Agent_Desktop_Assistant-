use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod cli;
mod tui;
mod dsl;
mod recorder;
mod client;
mod proto;

use cli::{CliArgs, Commands};
use client::grpc_client::GrpcClient;
use dsl::{DslParser, Interpreter, InterpreterConfig};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = CliArgs::parse();

    // Initialize logging
    init_logging(args.log_level.as_deref(), args.verbose)?;

    info!("Starting HCS Orchestrator v{}", env!("CARGO_PKG_VERSION"));

    // Load config
    let config = load_config(args.config.as_deref())?;

    // Create gRPC client
    let client = Arc::new(Mutex::new(
        GrpcClient::new(
            args.agent_endpoint.unwrap_or(config.grpc.agent_endpoint),
            args.vision_endpoint.unwrap_or(config.grpc.vision_endpoint),
            args.brain_endpoint.unwrap_or(config.grpc.brain_endpoint),
            args.adapters_endpoint.unwrap_or(config.grpc.adapters_endpoint),
            args.auth_token.or(config.grpc.auth_token),
        ).await?
    ));

    // Execute command
    let result = match args.command {
        Commands::Run(run_args) => {
            let client = client.lock().await.clone();
            cli::commands::run_command(run_args, client).await
        }
        Commands::Repl(repl_args) => {
            let client = client.lock().await.clone();
            cli::commands::repl_command(repl_args, client).await
        }
        Commands::Macro(macro_args) => {
            let client = client.lock().await.clone();
            cli::commands::macro_command(macro_args, client).await
        }
        Commands::Record(record_args) => {
            let client = client.lock().await.clone();
            cli::commands::record_command(record_args, client).await
        }
        Commands::Replay(replay_args) => {
            let client = client.lock().await.clone();
            cli::commands::replay_command(replay_args, client).await
        }
        Commands::ListMacros(list_args) => {
            let client = client.lock().await.clone();
            cli::commands::list_macros_command(list_args, client).await
        }
        Commands::Info(info_args) => {
            let client = client.lock().await.clone();
            cli::commands::info_command(info_args, client).await
        }
        Commands::Validate(validate_args) => {
            cli::commands::validate_command(validate_args).await
        }
        Commands::Tui(tui_args) => {
            cli::commands::tui_command(tui_args).await
        }
        Commands::Recording(recording_args) => {
            let client = client.lock().await.clone();
            cli::commands::recording_command(recording_args, client).await
        }
    };

    if let Err(e) = result {
        error!("Command failed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn init_logging(log_level: Option<&str>, verbose: bool) -> Result<(), anyhow::Error> {
    let level = log_level
        .or_else(|| std::env::var("RUST_LOG").ok().as_deref())
        .map(|s| s.parse::<Level>())
        .transpose()?
        .unwrap_or(if verbose { Level::DEBUG } else { Level::INFO });

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(env_filter)
        .init();

    Ok(())
}

fn load_config(config_path: Option<&PathBuf>) -> Result<OrchestratorConfig> {
    let path = config_path
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|d| d.join("hcs").join("orchestrator.toml")))
        .or_else(|| Some(PathBuf::from("config/orchestrator.toml")));

    if let Some(path) = path {
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return Ok(toml::from_str(&content)?);
        }
    }

    // Return default config
    Ok(OrchestratorConfig::default())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct OrchestratorConfig {
    grpc: GrpcConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GrpcConfig {
    agent_endpoint: String,
    vision_endpoint: String,
    brain_endpoint: String,
    adapters_endpoint: String,
    auth_token: Option<String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            grpc: GrpcConfig {
                agent_endpoint: "http://127.0.0.1:50051".to_string(),
                vision_endpoint: "http://127.0.0.1:50052".to_string(),
                brain_endpoint: "http://127.0.0.1:50053".to_string(),
                adapters_endpoint: "http://127.0.0.1:50054".to_string(),
                auth_token: None,
            },
        }
    }
}