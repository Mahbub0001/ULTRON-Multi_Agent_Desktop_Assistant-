# Brain Service API Reference

## Overview

The Brain Service (`hcs-brain`) provides a decision engine combining Behavior Trees with a WASM plugin system. It enables complex autonomous decision-making with sandboxed, hot-reloadable plugins. Exposes gRPC services on port 50053 by default.

## Services

### BrainService

#### ExecuteTree
Execute a behavior tree to completion.

```protobuf
rpc ExecuteTree(ExecuteTreeRequest) returns (ExecuteTreeResponse);
```

**Request:**
```json
{
  "tree_id": "combat_bot",
  "tree_definition": "{...}",  // JSON/YAML tree definition
  "initial_blackboard": {
    "target_hp": "100",
    "ammo": "30",
    "state": "idle"
  },
  "persist_state": true
}
```

**Response:**
```json
{
  "success": true,
  "tree_id": "combat_bot",
  "error": "",
  "final_status": "TREE_STATUS_SUCCESS",
  "final_blackboard": {
    "target_hp": "0",
    "ammo": "15",
    "state": "victory"
  },
  "ticks_executed": 45,
  "execution_time_ms": 230.5
}
```

#### TickTree
Execute a single tick of a loaded tree.

```protobuf
rpc TickTree(TickTreeRequest) returns (TickTreeResponse);
```

**Request:**
```json
{
  "tree_id": "combat_bot",
  "blackboard_updates": {
    "target_visible": "true",
    "distance": "15.5"
  }
}
```

**Response:**
```json
{
  "success": true,
  "status": "TREE_STATUS_RUNNING",
  "blackboard": {
    "target_hp": "100",
    "ammo": "30",
    "state": "engaging"
  },
  "error": "",
  "tick_time_ms": 1.2
}
```

#### GetTreeStatus
Get current status of a loaded tree.

```protobuf
rpc GetTreeStatus(GetTreeStatusRequest) returns (GetTreeStatusResponse);
```

**Request:**
```json
{ "tree_id": "combat_bot" }
```

**Response:**
```json
{
  "success": true,
  "status": "TREE_STATUS_RUNNING",
  "blackboard": { "target_hp": "85", "ammo": "28" },
  "ticks_executed": 23,
  "error": ""
}
```

#### StopTree
Stop a running tree.

```protobuf
rpc StopTree(StopTreeRequest) returns (StopTreeResponse);
```

**Request:**
```json
{ "tree_id": "combat_bot", "force": false }
```

**Response:**
```json
{ "success": true, "error": "" }
```

#### LoadTree
Load a tree definition without executing.

```protobuf
rpc LoadTree(LoadTreeRequest) returns (LoadTreeResponse);
```

**Request:**
```json
{
  "tree_id": "patrol_route",
  "tree_definition": "{...}",
  "initial_blackboard": { "waypoint_index": "0" }
}
```

**Response:**
```json
{ "success": true, "tree_id": "patrol_route", "error": "" }
```

#### UnloadTree
Unload a tree and free resources.

```protobuf
rpc UnloadTree(UnloadTreeRequest) returns (UnloadTreeResponse);
```

#### ListTrees
List all loaded trees.

```protobuf
rpc ListTrees(ListTreesRequest) returns (ListTreesResponse);
```

**Response:**
```json
{
  "trees": [
    {
      "tree_id": "combat_bot",
      "status": "TREE_STATUS_RUNNING",
      "ticks_executed": 45,
      "root_node_type": "Selector",
      "node_count": 23
    },
    {
      "tree_id": "patrol_route",
      "status": "TREE_STATUS_NOT_LOADED",
      "ticks_executed": 0,
      "root_node_type": "Sequence",
      "node_count": 12
    }
  ]
}
```

#### GetTreeVisualization
Get tree visualization for debugging.

```protobuf
rpc GetTreeVisualization(GetTreeVisualizationRequest) returns (GetTreeVisualizationResponse);
```

**Request:**
```json
{ "tree_id": "combat_bot", "format": "VISUALIZATION_FORMAT_MERMAID" }
```

**Response:**
```json
{
  "success": true,
  "visualization": "graph TD\n    A[Selector] --> B[Sequence: Attack]\n    A --> C[Sequence: Retreat]\n    B --> D[Condition: Target Visible]\n    B --> E[Action: Shoot]\n    C --> F[Condition: Low Health]\n    C --> G[Action: Flee]",
  "error": ""
}
```

**Formats:**
- `VISUALIZATION_FORMAT_DOT` (1): GraphViz DOT
- `VISUALIZATION_FORMAT_JSON` (2): Structured JSON
- `VISUALIZATION_FORMAT_MERMAID` (3): Mermaid diagram

#### ReloadPlugins
Hot-reload WASM plugins.

```protobuf
rpc ReloadPlugins(ReloadPluginsRequest) returns (ReloadPluginsResponse);
```

**Request:**
```json
{ "plugin_paths": ["/plugins/combat.wasm", "/plugins/movement.wasm"] }
```

**Response:**
```json
{
  "success": true,
  "results": [
    { "plugin_path": "/plugins/combat.wasm", "success": true, "plugin_name": "combat_ai", "error": "" },
    { "plugin_path": "/plugins/movement.wasm", "success": true, "plugin_name": "navigation", "error": "" }
  ],
  "error": ""
}
```

#### HealthCheck
```protobuf
rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
```

**Response:**
```json
{
  "healthy": true,
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "components": {
    "wasm_runtime": "healthy",
    "plugin_combat": "healthy",
    "plugin_navigation": "healthy",
    "blackboard": "healthy"
  }
}
```

#### GetMetrics
Get Prometheus metrics.

```protobuf
rpc GetMetrics(GetMetricsRequest) returns (GetMetricsResponse);
```

---

## Behavior Tree Definition

### JSON Format

```json
{
  "root": {
    "type": "Selector",
    "name": "Root",
    "children": [
      {
        "type": "Sequence",
        "name": "Attack Sequence",
        "children": [
          { "type": "Condition", "name": "Target Visible", "plugin": "vision", "function": "check_target" },
          { "type": "Action", "name": "Aim", "plugin": "combat", "function": "aim_at_target" },
          { "type": "Action", "name": "Shoot", "plugin": "combat", "function": "fire" }
        ]
      },
      {
        "type": "Sequence",
        "name": "Retreat Sequence",
        "children": [
          { "type": "Condition", "name": "Low Health", "plugin": "status", "function": "check_health", "args": {"threshold": 30} },
          { "type": "Action", "name": "Find Cover", "plugin": "navigation", "function": "find_cover" },
          { "type": "Action", "name": "Move to Cover", "plugin": "navigation", "function": "move_to" }
        ]
      },
      {
        "type": "Action",
        "name": "Idle",
        "plugin": "behavior",
        "function": "idle"
      }
    ]
  },
  "blackboard": {
    "target_hp": 100,
    "ammo": 30,
    "health": 100
  }
}
```

### YAML Format

```yaml
root:
  type: Selector
  name: Root
  children:
    - type: Sequence
      name: Attack Sequence
      children:
        - type: Condition
          name: Target Visible
          plugin: vision
          function: check_target
        - type: Action
          name: Aim
          plugin: combat
          function: aim_at_target
        - type: Action
          name: Shoot
          plugin: combat
          function: fire
    - type: Sequence
      name: Retreat Sequence
      children:
        - type: Condition
          name: Low Health
          plugin: status
          function: check_health
          args:
            threshold: 30
        - type: Action
          name: Find Cover
          plugin: navigation
          function: find_cover
        - type: Action
          name: Move to Cover
          plugin: navigation
          function: move_to
    - type: Action
      name: Idle
      plugin: behavior
      function: idle

blackboard:
  target_hp: 100
  ammo: 30
  health: 100
```

### Node Types

| Type | Description | Children | Execution |
|------|-------------|----------|-----------|
| `Sequence` | Run children in order, fail on first failure | 1+ | Continue until failure |
| `Selector` | Run children in order, succeed on first success | 1+ | Continue until success |
| `Parallel` | Run all children simultaneously | 1+ | Configurable success policy |
| `Decorator` | Wrap single child, modify result | 1 | Various (Inverter, Repeater, etc.) |
| `Action` | Execute plugin function | 0 | Run plugin, return result |
| `Condition` | Check condition via plugin | 0 | Run plugin, return bool |

### Decorator Types

```json
{
  "type": "Decorator",
  "decorator_type": "Inverter",
  "child": { "type": "Condition", "name": "Has Ammo", ... }
}
```

| Decorator | Behavior |
|-----------|----------|
| `Inverter` | Flip Success ↔ Failure |
| `Repeater` | Repeat child N times or until failure |
| `UntilFailure` | Repeat until child fails |
| `UntilSuccess` | Repeat until child succeeds |
| `Timeout` | Fail if child exceeds time limit |
| `Cooldown` | Prevent re-execution for duration |

### Blackboard

Shared key-value store accessible to all nodes:

```json
{
  "blackboard": {
    "string_key": "value",
    "int_key": 42,
    "float_key": 3.14,
    "bool_key": true,
    "nested": { "key": "value" }
  }
}
```

Access in plugins:
```rust
// Read
let health: i32 = blackboard.get("health")?;
// Write
blackboard.set("health", health - 10)?;
```

---

## WASM Plugin System

### Plugin Interface

Plugins must export the Component Model interface:

```wit
package hcs:brain/plugin;

interface plugin {
    // Initialize plugin with config
    init: func(config: string) -> result<void, string>;
    
    // Execute action/condition
    tick: func(name: string, args: string, blackboard: blackboard) -> result<tick-result, string>;
    
    // Cleanup
    shutdown: func() -> result<void, string>;
}

type blackboard = map<string, string>;
type tick-result = record { status: status, output: string };
enum status { success, failure, running, error }
```

### Rust Plugin Example

```rust
// Cargo.toml
[package]
name = "combat_plugin"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-component = "0.261"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

```rust
// src/lib.rs
use wit_component::ComponentEncoder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct InitConfig {
    difficulty: String,
}

#[derive(Deserialize)]
struct TickArgs {
    target_id: Option<String>,
}

#[derive(Serialize)]
struct TickResult {
    status: String,  // "success", "failure", "running", "error"
    output: String,
}

type Blackboard = HashMap<String, String>;

// Plugin state
static mut PLUGIN_STATE: Option<PluginState> = None;

struct PluginState {
    difficulty: String,
}

#[no_mangle]
extern "C" fn init(config_ptr: *const u8, config_len: usize) -> i32 {
    let config = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };
    let config_str = std::str::from_utf8(config).unwrap_or("{}");
    
    let config: InitConfig = serde_json::from_str(config_str).unwrap_or(InitConfig {
        difficulty: "normal".into(),
    });
    
    unsafe {
        PLUGIN_STATE = Some(PluginState { difficulty: config.difficulty });
    }
    0  // Success
}

#[no_mangle]
extern "C" fn tick(name_ptr: *const u8, name_len: usize, 
                   args_ptr: *const u8, args_len: usize,
                   bb_ptr: *const u8, bb_len: usize,
                   out_ptr: *mut u8, out_len: *mut usize) -> i32 {
    let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let args = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(args_ptr, args_len)) };
    let bb = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(bb_ptr, bb_len)) };
    let blackboard: Blackboard = serde_json::from_str(bb).unwrap_or_default();
    
    let result = match name {
        "aim_at_target" => aim_at_target(args, &blackboard),
        "fire" => fire(args, &blackboard),
        "check_target" => check_target(args, &blackboard),
        _ => TickResult { status: "error".into(), output: format!("Unknown function: {}", name) },
    };
    
    let output = serde_json::to_string(&result).unwrap();
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_ptr, *out_len) };
    let copy_len = output.len().min(out_slice.len());
    out_slice[..copy_len].copy_from_slice(output.as_bytes());
    *out_len = copy_len;
    
    0
}

#[no_mangle]
extern "C" fn shutdown() -> i32 {
    unsafe { PLUGIN_STATE = None; }
    0
}

fn aim_at_target(args: &str, bb: &Blackboard) -> TickResult {
    // Simulate aiming logic
    TickResult { status: "success".into(), output: "aimed".into() }
}

fn fire(args: &str, bb: &Blackboard) -> TickResult {
    let ammo: i32 = bb.get("ammo").and_then(|s| s.parse().ok()).unwrap_or(0);
    if ammo > 0 {
        TickResult { status: "success".into(), output: "fired".into() }
    } else {
        TickResult { status: "failure".into(), output: "no ammo".into() }
    }
}

fn check_target(args: &str, bb: &Blackboard) -> TickResult {
    let visible: bool = bb.get("target_visible").map(|s| s == "true").unwrap_or(false);
    TickResult { 
        status: if visible { "success" } else { "failure" }.into(), 
        output: "checked".into() 
    }
}
```

### Build Plugin

```bash
# Build for WASM Component Model
cargo build --target wasm32-wasip1 --release

# Convert to component
wit-component target/wasm32-wasip1/release/combat_plugin.wasm -o combat_plugin.wasm

# Copy to plugins directory
cp combat_plugin.wasm /opt/hcs/wasm-plugins/
```

### Load Plugin

```bash
# Via gRPC
grpcurl -plaintext -d '{"plugin_paths": ["/opt/hcs/wasm-plugins/combat_plugin.wasm"]}' \
  localhost:50053 brain.BrainService/ReloadPlugins
```

---

## Host Functions

Plugins can call host functions provided by the Brain service:

| Function | Description | Signature |
|----------|-------------|-----------|
| `log` | Write to brain log | `func(level: string, message: string)` |
| `get_time` | Get current timestamp | `func() -> u64` |
| `sleep` | Async sleep | `func(ms: u32)` |
| `http_get` | HTTP GET request | `func(url: string) -> result<string, string>` |
| `http_post` | HTTP POST request | `func(url: string, body: string) -> result<string, string>` |
| `emit_event` | Emit custom event | `func(name: string, data: string)` |

---

## Client Examples

### Python Client
```python
import grpc
import json
from hcs_brain_proto import brain_pb2, brain_pb2_grpc

class BrainClient:
    def __init__(self, host="localhost", port=50053):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.stub = brain_pb2_grpc.BrainServiceStub(self.channel)
    
    def load_tree(self, tree_id, definition, blackboard=None):
        req = brain_pb2.LoadTreeRequest(
            tree_id=tree_id,
            tree_definition=json.dumps(definition),
            initial_blackboard=blackboard or {}
        )
        return self.stub.LoadTree(req)
    
    def execute(self, tree_id, blackboard=None):
        req = brain_pb2.ExecuteTreeRequest(
            tree_id=tree_id,
            initial_blackboard=blackboard or {},
            persist_state=True
        )
        return self.stub.ExecuteTree(req)
    
    def tick(self, tree_id, updates=None):
        req = brain_pb2.TickTreeRequest(
            tree_id=tree_id,
            blackboard_updates=updates or {}
        )
        return self.stub.TickTree(req)
    
    def get_visualization(self, tree_id, format="MERMAID"):
        fmt = getattr(brain_pb2, f"VISUALIZATION_FORMAT_{format}")
        req = brain_pb2.GetTreeVisualizationRequest(tree_id=tree_id, format=fmt)
        return self.stub.GetTreeVisualization(req)
    
    def reload_plugins(self, paths):
        req = brain_pb2.ReloadPluginsRequest(plugin_paths=paths)
        return self.stub.ReloadPlugins(req)

# Usage
client = BrainClient()

# Define tree
tree_def = {
    "root": {
        "type": "Selector",
        "children": [
            {"type": "Action", "name": "Attack", "plugin": "combat", "function": "attack"},
            {"type": "Action", "name": "Patrol", "plugin": "movement", "function": "patrol"}
        ]
    }
}

# Load and execute
client.load_tree("bot_1", tree_def, {"health": "100", "target": "enemy_1"})
result = client.execute("bot_1")
print(f"Result: {result.final_status}, Ticks: {result.ticks_executed}")

# Get visualization
viz = client.get_visualization("bot_1", "MERMAID")
print(viz.visualization)
```

### Rust Client
```rust
use hcs_brain_proto::{
    brain_service_client::BrainServiceClient,
    ExecuteTreeRequest, LoadTreeRequest, TickTreeRequest,
    GetTreeVisualizationRequest, VisualizationFormat,
    ReloadPluginsRequest,
};
use tonic::transport::Channel;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BrainServiceClient::connect("http://localhost:50053").await?;
    
    // Tree definition as JSON
    let tree_def = serde_json::json!({
        "root": {
            "type": "Selector",
            "children": [
                { "type": "Action", "name": "Attack", "plugin": "combat", "function": "attack" },
                { "type": "Action", "name": "Patrol", "plugin": "movement", "function": "patrol" }
            ]
        }
    }).to_string();
    
    // Load tree
    let mut bb = HashMap::new();
    bb.insert("health".into(), "100".into());
    bb.insert("target".into(), "enemy_1".into());
    
    client.load_tree(LoadTreeRequest {
        tree_id: "bot_1".into(),
        tree_definition: tree_def,
        initial_blackboard: bb,
    }).await?;
    
    // Execute
    let result = client.execute_tree(ExecuteTreeRequest {
        tree_id: "bot_1".into(),
        persist_state: true,
        ..Default::default()
    }).await?;
    
    println!("Status: {:?}, Ticks: {}, Time: {:.1}ms",
        result.get_ref().final_status,
        result.get_ref().ticks_executed,
        result.get_ref().execution_time_ms);
    
    // Get Mermaid visualization
    let viz = client.get_tree_visualization(GetTreeVisualizationRequest {
        tree_id: "bot_1".into(),
        format: VisualizationFormat::VisualizationFormatMermaid as i32,
    }).await?;
    
    println!("Tree:\n{}", viz.get_ref().visualization);
    
    Ok(())
}
```

### Go Client
```go
package main

import (
	"context"
	"encoding/json"
	"log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	
	brainpb "github.com/your-org/hcs/brain/proto"
)

func main() {
	conn, err := grpc.Dial("localhost:50053", grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatal(err)
	}
	defer conn.Close()
	
	client := brainpb.NewBrainServiceClient(conn)
	
	// Tree definition
	treeDef := map[string]interface{}{
		"root": map[string]interface{}{
			"type": "Selector",
			"children": []interface{}{
				map[string]interface{}{"type": "Action", "name": "Attack", "plugin": "combat", "function": "attack"},
				map[string]interface{}{"type": "Action", "name": "Patrol", "plugin": "movement", "function": "patrol"},
			},
		},
	}
	treeDefJSON, _ := json.Marshal(treeDef)
	
	// Load tree
	_, err = client.LoadTree(context.Background(), &brainpb.LoadTreeRequest{
		TreeId:           "bot_1",
		TreeDefinition:   string(treeDefJSON),
		InitialBlackboard: map[string]string{"health": "100"},
	})
	if err != nil {
		log.Fatal(err)
	}
	
	// Execute
	resp, err := client.ExecuteTree(context.Background(), &brainpb.ExecuteTreeRequest{
		TreeId:        "bot_1",
		PersistState:  true,
	})
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("Status: %v, Ticks: %d, Time: %.1fms", 
		resp.FinalStatus, resp.TicksExecuted, resp.ExecutionTimeMs)
	
	// Visualization
	viz, err := client.GetTreeVisualization(context.Background(), &brainpb.GetTreeVisualizationRequest{
		TreeId: "bot_1",
		Format: brainpb.VisualizationFormat_VISUALIZATION_FORMAT_MERMAID,
	})
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("Tree:\n%s", viz.Visualization)
}
```

---

## Performance Considerations

- **Tick Budget:** Keep tick time < 1ms for 1000Hz loops
- **Blackboard Size:** Limit to < 100 keys for performance
- **Plugin Fuel:** Default 1M instructions per tick (configurable)
- **Memory Limit:** Default 16MB per plugin
- **Tree Depth:** Max 100 nodes deep (configurable)

---

## Error Codes

| Code | Description |
|------|-------------|
| `NOT_FOUND` | Tree/plugin not found |
| `FAILED_PRECONDITION` | Tree not loaded, plugin not initialized |
| `RESOURCE_EXHAUSTED` | Fuel/memory limit exceeded |
| `INVALID_ARGUMENT` | Invalid tree definition |
| `INTERNAL` | WASM runtime error |
| `UNAVAILABLE` | Plugin not loaded |

---

## Changelog

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release |
| 1.1.0 | Added hot-reload, Mermaid visualization |
| 1.2.0 | Added host functions, fuel limiting |