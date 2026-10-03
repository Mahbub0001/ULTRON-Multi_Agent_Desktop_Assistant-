# HCS Orchestrator

The Human Control System Orchestrator provides a CLI/TUI interface and Task DSL for creating and executing automation tasks.

## Features

- **Task DSL**: YAML/JSON-based domain-specific language for automation
- **CLI Commands**: Run tasks, REPL, macros, recording/replay
- **TUI Dashboard**: Real-time monitoring and task editing
- **Input Recording**: Capture keyboard/mouse events for replay
- **gRPC Client**: Connects to agent, vision, brain, and adapter services

## Installation

```bash
cargo build --release --bin hcs
```

## Quick Start

### 1. Create a task file (`task.yaml`)

```yaml
name: "My First Task"
version: "1.0"
description: "Presses Ctrl+C"
steps:
  - type: action
    id: "ctrl_down"
    name: "Press Ctrl"
    action:
      kind: "key_down"
      code: 17

  - type: action
    id: "press_c"
    name: "Press C"
    action:
      kind: "key_press"
      code: 67
      delay_ms: 50

  - type: action
    id: "ctrl_up"
    name: "Release Ctrl"
    action:
      kind: "key_up"
      code: 17
```

### 2. Run the task

```bash
hcs run task.yaml
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `hcs run <task.yaml>` | Execute a task file |
| `hcs repl` | Start interactive REPL |
| `hcs macro <name>` | Execute a macro |
| `hcs record` | Record input events |
| `hcs replay <file>` | Replay a recording |
| `hcs list-macros` | List available macros |
| `hcs info` | Show system info |
| `hcs validate <task.yaml>` | Validate a task file |
| `hcs tui` | Launch TUI dashboard |

## Task DSL Reference

### Step Types

#### Action Steps
```yaml
- type: action
  id: "unique_id"
  name: "Description"
  action:
    kind: "key_press"  # key_down, key_up, key_press, mouse_move, mouse_click, mouse_down, mouse_up, mouse_scroll, delay, type_text
    # ... parameters vary by action kind
```

#### Sequential Steps
```yaml
- type: sequential
  id: "seq_id"
  name: "Sequence"
  steps:
    - type: action
      # ...
    - type: action
      # ...
```

#### Parallel Steps
```yaml
- type: parallel
  id: "par_id"
  name: "Parallel"
  fail_fast: true
  steps:
    - type: action
      # ...
    - type: action
      # ...
```

#### Conditional Steps
```yaml
- type: conditional
  id: "cond_id"
  name: "If condition"
  condition: "${variable} > 10"
  then_branch:
    type: action
    # ...
  else_branch:
    type: action
    # ...
```

#### Loop Steps
```yaml
- type: loop
  id: "loop_id"
  name: "Loop"
  loop_type: "for_each"  # for_each, while, count
  iterator: "item"       # for for_each
  collection: "${items}" # for for_each
  condition: "${count} > 0"  # for while
  count: 5               # for count
  body:
    type: action
    # ...
```

#### Vision Steps
```yaml
- type: vision
  id: "vision_id"
  name: "Capture screen"
  operation:
    type: "capture_screen"
    monitor_index: 0
    format: "png"
```

#### Variable Steps
```yaml
- type: variable
  id: "var_id"
  name: "Set variable"
  operation:
    type: "set"
    name: "my_var"
    value: "hello"
```

#### Wait Steps
```yaml
- type: wait
  id: "wait_id"
  name: "Wait for condition"
  condition: "${ready} == true"
  timeout_ms: 5000
  poll_interval_ms: 100
```

#### Log Steps
```yaml
- type: log
  id: "log_id"
  name: "Log message"
  level: "info"  # trace, debug, info, warn, error
  message: "Task completed"
```

### Variables and Templates

Variables can be defined at the task level and referenced using `${variable_name}`:

```yaml
variables:
  count:
    type: "int"
    default: 5
  message:
    type: "string"
    default: "Hello"

steps:
  - type: action
    action:
      kind: "type_text"
      text: "${message} (count: ${count})"
```

### Macros

Macros are reusable step sequences:

```yaml
macros:
  my_macro:
    name: "My Macro"
    description: "Does something"
    parameters:
      param1:
        type: "string"
        required: true
    steps:
      - type: action
        action:
          kind: "type_text"
          text: "${param1}"

steps:
  - type: macro
    macro_name: "my_macro"
    parameters:
      param1: "Hello World"
```

### Imports

Tasks can import other task files:

```yaml
imports:
  - "common_macros.yaml"
  - "shared_variables.yaml"

steps:
  # ...
```

## Recording and Replay

### Record Input
```bash
hcs record --name "my_recording" --output recording.json
```
Press the stop key (default: Ctrl+Shift+Q) or wait for timeout to stop recording.

### Replay Recording
```bash
hcs replay recording.json --speed 1.0 --jitter
```

Options:
- `--speed`: Playback speed (0.1 to 10.0, default 1.0)
- `--loop`: Loop playback
- `--jitter`: Add human-like timing variations (default: true)
- `--var`: Override variables (key=value)

## TUI Dashboard

Launch the TUI with:
```bash
hcs tui
```

Tabs:
- **F1 - Dashboard**: System stats, task status, driver info
- **F2 - Task Editor**: Edit and validate task files
- **F3 - Macro Manager**: View and run macros
- **F4 - Device Monitor**: Monitor input devices
- **F5 - Log Viewer**: View structured logs

Navigation:
- `Tab` / `Shift+Tab`: Next/previous tab
- `F1-F5`: Jump to specific tab
- `q` / `Esc`: Quit

## Configuration

Configuration file: `config/orchestrator.toml`

```toml
[grpc]
agent_endpoint = "http://127.0.0.1:50051"
vision_endpoint = "http://127.0.0.1:50052"
brain_endpoint = "http://127.0.0.1:50053"
adapters_endpoint = "http://127.0.0.1:50054"

[tui]
refresh_rate_hz = 60
theme = "dark"

[dsl]
max_execution_time_ms = 300000
max_loop_iterations = 10000

[recorder]
output_directory = "recordings"
output_format = "json"

[replay]
default_speed = 1.0
enable_jitter = true
jitter_factor = 0.1
```

## gRPC Services

The orchestrator connects to these services:

| Service | Port | Description |
|---------|------|-------------|
| Agent | 50051 | Input injection, macros, tasks |
| Vision | 50052 | Screen capture, detection, OCR |
| Brain | 50053 | Behavior tree execution |
| Adapters | 50054 | App-specific automation |

## Examples

See `examples/` directory for sample task files:
- `simple_task.yaml` - Basic key combination
- `sample_task.yaml` - Comprehensive example with all features

## Development

### Running Tests
```bash
cargo test
```

### Code Quality
```bash
cargo clippy -- -D warnings
cargo fmt --check
```

## Architecture

```
orchestrator/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli/              # CLI commands and arguments
│   ├── tui/              # Terminal UI (ratatui)
│   ├── dsl/              # Task DSL parser/interpreter
│   ├── recorder/         # Input recording/replay
│   ├── client/           # gRPC clients
│   └── proto/            # Protobuf definitions
├── config/
│   └── orchestrator.toml # Configuration
├── examples/             # Example task files
└── tests/                # Integration tests
```

## License

MIT