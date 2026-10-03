use crate::cli::args::*;
use crate::dsl::{DslParser, Interpreter, InterpreterConfig};
use crate::recorder::{Recorder, Recording, RecordedEvent};
use crate::client::grpc_client::GrpcClient;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub async fn run_command(args: RunArgs, client: Arc<GrpcClient>) -> Result<()> {
    info!("Running task: {}", args.task_file.display());

    let parser = DslParser::new();
    let mut task = parser.parse_file(&args.task_file)
        .context("Failed to parse task file")?;

    // Validate
    let warnings = parser.validate(&task)?;
    for warning in &warnings {
        warn!("Validation warning: {}", warning);
    }

    // Parse variable overrides
    let mut variables = HashMap::new();
    for var in &args.var {
        if let Some((key, value)) = var.split_once('=') {
            variables.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        }
    }

    // Create interpreter
    let config = InterpreterConfig {
        max_execution_time_ms: args.timeout.unwrap_or(300000),
        ..Default::default()
    };
    let mut interpreter = Interpreter::new(client.clone(), config).with_variables(variables);

    // Execute
    let result = interpreter.execute(task).await?;

    if result.success {
        info!("Task completed successfully in {}ms", result.execution_time_ms);
        if let Some(output) = args.output {
            // Save recording if requested
            // TODO: Implement recording output
        }
    } else {
        error!("Task failed: {}", result.error.unwrap_or("Unknown error".to_string()));
        if !args.continue_on_error {
            std::process::exit(1);
        }
    }

    Ok(())
}

pub async fn repl_command(args: ReplArgs, client: Arc<GrpcClient>) -> Result<()> {
    info!("Starting REPL...");

    let mut context = HashMap::new();
    for var in &args.var {
        if let Some((key, value)) = var.split_once('=') {
            context.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        }
    }

    // Preload task if specified
    let mut task_def = None;
    if let Some(preload) = args.preload {
        let parser = DslParser::new();
        task_def = Some(parser.parse_file(&preload)?);
    }

    println!("HCS REPL - Type 'help' for commands, 'exit' to quit");
    println!("Available commands: run, macro, var, help, exit");

    let mut interpreter = Interpreter::new(client.clone(), InterpreterConfig::default());
    
    loop {
        print!("hcs> ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "exit" | "quit" => break,
            "help" => {
                println!("Commands:");
                println!("  run <task.yaml>     - Execute task file");
                println!("  macro <name>        - Execute macro");
                println!("  var <name> [value]  - Get/set variable");
                println!("  vars                - List all variables");
                println!("  help                - Show this help");
                println!("  exit                - Exit REPL");
            }
            "vars" => {
                for (k, v) in &context {
                    println!("  {} = {}", k, v);
                }
            }
            cmd if cmd.starts_with("var ") => {
                let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
                if parts.len() == 2 {
                    if let Some(val) = context.get(parts[1]) {
                        println!("{} = {}", parts[1], val);
                    } else {
                        println!("Variable '{}' not set", parts[1]);
                    }
                } else if parts.len() == 3 {
                    context.insert(parts[1].to_string(), serde_json::Value::String(parts[2].to_string()));
                    println!("Set {} = {}", parts[1], parts[2]);
                }
            }
            cmd if cmd.starts_with("run ") => {
                let path = &cmd[4..];
                let parser = DslParser::new();
                let task = parser.parse_file(path)?;
                interpreter = interpreter.with_variables(context.clone());
                let result = interpreter.execute(task).await?;
                if result.success {
                    println!("Task completed in {}ms", result.execution_time_ms);
                } else {
                    println!("Task failed: {}", result.error.unwrap_or("Unknown".to_string()));
                }
                context = interpreter.context.variables.clone();
            }
            cmd if cmd.starts_with("macro ") => {
                let name = &cmd[6..];
                match client.execute_macro(name, HashMap::new()).await {
                    Ok(result) => println!("Macro result: {}", result),
                    Err(e) => println!("Macro failed: {}", e),
                }
            }
            _ => {
                // Try to evaluate as Starlark expression
                println!("Unknown command. Type 'help' for available commands.");
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}

pub async fn macro_command(args: MacroArgs, client: Arc<GrpcClient>) -> Result<()> {
    info!("Executing macro: {}", args.name);

    let mut params = HashMap::new();
    for param in &args.param {
        if let Some((key, value)) = param.split_once('=') {
            params.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        }
    }

    let result = client.execute_macro(&args.name, params).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn record_command(args: RecordArgs, client: Arc<GrpcClient>) -> Result<()> {
    let name = args.name.unwrap_or_else(|| {
        format!("recording_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
    });

    info!("Starting recording: {}", name);

    let mut recorder = Recorder::new(client.clone());
    recorder.set_config(crate::recorder::RecorderConfig {
        capture_keyboard: args.keyboard,
        capture_mouse: args.mouse,
        capture_delays: args.delays,
        max_duration: args.max_duration.map(Duration::from_secs),
        stop_key: args.stop_key,
    });

    let recording = recorder.start().await?;
    info!("Recording started. Press stop key or wait for timeout.");

    // Wait for recording to complete
    let recording = recorder.stop().await?;
    
    info!("Recording stopped. Duration: {}ms, Events: {}", 
        recording.duration_us / 1000, recording.events.len());

    // Save recording
    let output = args.output.unwrap_or_else(|| PathBuf::from(format!("recordings/{}.json", name)));
    std::fs::create_dir_all(output.parent().unwrap())?;
    let json = serde_json::to_string_pretty(&recording)?;
    std::fs::write(&output, json)?;

    info!("Recording saved to: {}", output.display());
    Ok(())
}

pub async fn replay_command(args: ReplayArgs, client: Arc<GrpcClient>) -> Result<()> {
    info!("Replaying recording: {}", args.file.display());

    let content = std::fs::read_to_string(&args.file)?;
    let recording: Recording = serde_json::from_str(&content)?;

    let mut recorder = Recorder::new(client.clone());
    recorder.set_speed(args.speed);
    recorder.set_jitter(args.jitter);
    recorder.set_loop(args.loop_playback);

    // Apply variable overrides
    let mut overrides = HashMap::new();
    for var in &args.var {
        if let Some((key, value)) = var.split_once('=') {
            overrides.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        }
    }
    recorder.set_variable_overrides(overrides);

    let result = recorder.replay(&recording).await?;

    if result.success {
        info!("Replay completed successfully in {}ms", result.execution_time_ms);
    } else {
        error!("Replay failed: {}", result.error.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }

    Ok(())
}

pub async fn list_macros_command(args: ListMacrosArgs, client: Arc<GrpcClient>) -> Result<()> {
    let macros = client.list_macros().await?;
    
    if args.detail {
        for macro_info in macros {
            println!("{}", serde_json::to_string_pretty(&macro_info)?);
        }
    } else {
        for macro_info in macros {
            println!("  {}", macro_info.name);
        }
    }
    Ok(())
}

pub async fn info_command(args: InfoArgs, client: Arc<GrpcClient>) -> Result<()> {
    let sys_info = client.get_system_info().await?;
    
    if args.detail {
        println!("{}", serde_json::to_string_pretty(&sys_info)?);
    } else {
        println!("Hostname: {}", sys_info.hostname);
        println!("OS: {} {}", sys_info.os, sys_info.arch);
        println!("Uptime: {}s", sys_info.uptime_seconds);
        println!("CPU: {} ({} cores, {} threads)", sys_info.cpu.brand, sys_info.cpu.cores, sys_info.cpu.threads);
        println!("Memory: {} / {} MB", sys_info.memory.used_bytes / 1024 / 1024, sys_info.memory.total_bytes / 1024 / 1024);
        if let Some(driver) = sys_info.driver {
            println!("Driver: {:?} (initialized: {}, ready: {})", driver.backend, driver.initialized, driver.ready);
        }
    }
    Ok(())
}

pub async fn validate_command(args: ValidateArgs) -> Result<()> {
    info!("Validating task: {}", args.task_file.display());

    let parser = DslParser::new();
    let task = parser.parse_file(&args.task_file)?;

    let warnings = parser.validate(&task)?;
    
    println!("Task: {} v{}", task.name, task.version);
    println!("Steps: {}", task.steps.len());
    println!("Macros: {}", task.macros.len());
    println!("Variables: {}", task.variables.len());
    
    if args.warnings {
        if warnings.is_empty() {
            println!("No warnings.");
        } else {
            println!("Warnings:");
            for warning in warnings {
                println!("  - {}", warning);
            }
        }
    }

    println!("Validation passed!");
    Ok(())
}

pub async fn tui_command(args: TuiArgs) -> Result<()> {
    info!("Starting TUI...");
    crate::tui::run_tui(args.tab).await
}

pub async fn recording_command(args: RecordingArgs, client: Arc<GrpcClient>) -> Result<()> {
    match args.action {
        RecordingAction::List => {
            let recordings = client.list_recordings().await?;
            for rec in recordings {
                println!("{} - {} ({} events, {}ms)", rec.id, rec.name, rec.event_count, rec.duration_us / 1000);
            }
        }
        RecordingAction::Show { id } => {
            let recording = client.get_recording(&id).await?;
            println!("{}", serde_json::to_string_pretty(&recording)?);
        }
        RecordingAction::Delete { id } => {
            client.delete_recording(&id).await?;
            println!("Recording {} deleted", id);
        }
        RecordingAction::Export { id, output } => {
            let recording = client.get_recording(&id).await?;
            let json = serde_json::to_string_pretty(&recording)?;
            std::fs::write(&output, json)?;
            println!("Recording exported to {}", output.display());
        }
    }
    Ok(())
}