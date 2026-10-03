use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hcs", version, about = "Human Control System Orchestrator")]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Config file path")]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true, help = "Log level")]
    pub log_level: Option<String>,

    #[arg(long, global = true, help = "Agent gRPC endpoint")]
    pub agent_endpoint: Option<String>,

    #[arg(long, global = true, help = "Vision gRPC endpoint")]
    pub vision_endpoint: Option<String>,

    #[arg(long, global = true, help = "Brain gRPC endpoint")]
    pub brain_endpoint: Option<String>,

    #[arg(long, global = true, help = "Adapters gRPC endpoint")]
    pub adapters_endpoint: Option<String>,

    #[arg(long, global = true, help = "Auth token")]
    pub auth_token: Option<String>,

    #[arg(short, long, global = true, help = "Dry run (don't execute)")]
    pub dry_run: bool,

    #[arg(short, long, global = true, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a task file
    Run(RunArgs),

    /// Start interactive REPL
    Repl(ReplArgs),

    /// Execute a macro
    Macro(MacroArgs),

    /// Record input events
    Record(RecordArgs),

    /// Replay a recording
    Replay(ReplayArgs),

    /// List available macros
    ListMacros(ListMacrosArgs),

    /// Show system info
    Info(InfoArgs),

    /// Validate a task file
    Validate(ValidateArgs),

    /// Start TUI
    Tui(TuiArgs),

    /// Manage recordings
    Recording(RecordingArgs),
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(help = "Task file path (YAML/JSON)")]
    pub task_file: PathBuf,

    #[arg(short, long, help = "Variables to override (key=value)")]
    pub var: Vec<String>,

    #[arg(short, long, help = "Output recording to file")]
    pub output: Option<PathBuf>,

    #[arg(long, help = "Continue on error")]
    pub continue_on_error: bool,

    #[arg(long, help = "Max execution time (ms)")]
    pub timeout: Option<u64>,
}

#[derive(Args, Debug)]
pub struct ReplArgs {
    #[arg(short, long, help = "Initial context variables (key=value)")]
    pub var: Vec<String>,

    #[arg(long, help = "Preload task file")]
    pub preload: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct MacroArgs {
    #[arg(help = "Macro name")]
    pub name: String,

    #[arg(short, long, help = "Parameters (key=value)")]
    pub param: Vec<String>,
}

#[derive(Args, Debug)]
pub struct RecordArgs {
    #[arg(short, long, help = "Recording name")]
    pub name: Option<String>,

    #[arg(short, long, help = "Output file")]
    pub output: Option<PathBuf>,

    #[arg(long, help = "Capture keyboard", default_value = "true")]
    pub keyboard: bool,

    #[arg(long, help = "Capture mouse", default_value = "true")]
    pub mouse: bool,

    #[arg(long, help = "Capture delays", default_value = "true")]
    pub delays: bool,

    #[arg(long, help = "Max duration (seconds)")]
    pub max_duration: Option<u32>,

    #[arg(long, help = "Stop on key combination (e.g., ctrl+shift+q)")]
    pub stop_key: Option<String>,
}

#[derive(Args, Debug)]
pub struct ReplayArgs {
    #[arg(help = "Recording file")]
    pub file: PathBuf,

    #[arg(short, long, help = "Playback speed", default_value = "1.0")]
    pub speed: f32,

    #[arg(long, help = "Loop playback")]
    pub loop_playback: bool,

    #[arg(short, long, help = "Variable overrides (key=value)")]
    pub var: Vec<String>,

    #[arg(long, help = "Enable jitter for human-like replay")]
    pub jitter: bool,
}

#[derive(Args, Debug)]
pub struct ListMacrosArgs {
    #[arg(short, long, help = "Show macro details")]
    pub detail: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    #[arg(short, long, help = "Show detailed info")]
    pub detail: bool,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[arg(help = "Task file path")]
    pub task_file: PathBuf,

    #[arg(short, long, help = "Show warnings")]
    pub warnings: bool,
}

#[derive(Args, Debug)]
pub struct TuiArgs {
    #[arg(long, help = "Initial tab (dashboard, editor, macros, devices, logs)")]
    pub tab: Option<String>,
}

#[derive(Args, Debug)]
pub struct RecordingArgs {
    #[command(subcommand)]
    pub action: RecordingAction,
}

#[derive(Subcommand, Debug)]
pub enum RecordingAction {
    /// List recordings
    List,
    /// Show recording details
    Show { id: String },
    /// Delete recording
    Delete { id: String },
    /// Export recording
    Export { id: String, output: PathBuf },
}