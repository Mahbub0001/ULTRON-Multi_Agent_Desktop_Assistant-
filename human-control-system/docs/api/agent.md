# Agent Service API Reference

## Overview

The Agent Service (`hcs-agent`) is the central daemon managing input drivers, macro execution, task scheduling, and authentication. It exposes gRPC services on port 50051 by default.

## Services

### InputService
Handles low-level keyboard and mouse injection.

#### InjectKey
Inject a single keyboard event.

```protobuf
rpc InjectKey(KeyEvent) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "code": 65,                    // Virtual key code (e.g., 0x41 = 'A')
  "state": "KEY_STATE_DOWN",     // KEY_STATE_DOWN or KEY_STATE_UP
  "scan_code": 30,               // Hardware scan code (optional)
  "extended": false,             // Extended key flag (E0/E1)
  "timestamp_us": 1234567890     // Optional timestamp
}
```

**Response:** Empty

**Example:**
```bash
# Press 'A'
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
  -d '{"code": 65, "state": "KEY_STATE_DOWN"}' \
  localhost:50051 hcs.agent.v1.InputService/InjectKey

# Release 'A'
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
  -d '{"code": 65, "state": "KEY_STATE_UP"}' \
  localhost:50051 hcs.agent.v1.InputService/InjectKey
```

#### InjectMouse
Inject a single mouse event.

```protobuf
rpc InjectMouse(MouseEvent) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "x": 100,                      // Absolute X or relative dx
  "y": 200,                      // Absolute Y or relative dy
  "dx": 10,                      // Relative X movement
  "dy": 5,                       // Relative Y movement
  "button": "MOUSE_BUTTON_LEFT", // Button (if click)
  "button_state": "KEY_STATE_DOWN", // Button state
  "absolute": false,             // True for absolute positioning
  "timestamp_us": 1234567890
}
```

**Response:** Empty

**Example:**
```bash
# Move mouse relatively
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
  -d '{"dx": 100, "dy": 50}' \
  localhost:50051 hcs.agent.v1.InputService/InjectMouse

# Left click
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
  -d '{"button": "MOUSE_BUTTON_LEFT", "button_state": "KEY_STATE_DOWN"}' \
  localhost:50051 hcs.agent.v1.InputService/InjectMouse
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
  -d '{"button": "MOUSE_BUTTON_LEFT", "button_state": "KEY_STATE_UP"}' \
  localhost:50051 hcs.agent.v1.InputService/InjectMouse
```

#### InjectBatch
Inject multiple events atomically.

```protobuf
rpc InjectBatch(InputBatch) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "events": [
    {"key": {"code": 17, "state": "KEY_STATE_DOWN"}},     // Ctrl
    {"key": {"code": 67, "state": "KEY_STATE_DOWN"}},     // C
    {"key": {"code": 67, "state": "KEY_STATE_UP"}},       // C
    {"key": {"code": 17, "state": "KEY_STATE_UP"}},       // Ctrl
    {"delay": {"microseconds": 50000}},                   // 50ms delay
    {"key": {"code": 17, "state": "KEY_STATE_DOWN"}},     // Ctrl
    {"key": {"code": 86, "state": "KEY_STATE_DOWN"}},     // V
    {"key": {"code": 86, "state": "KEY_STATE_UP"}},       // V
    {"key": {"code": 17, "state": "KEY_STATE_UP"}}        // Ctrl
  ]
}
```

**Response:** Empty

#### InjectStream
High-frequency streaming injection (client-streaming RPC).

```protobuf
rpc InjectStream(stream InputEvent) returns (google.protobuf.Empty);
```

**Usage:** Send continuous stream of `InputEvent` messages for lowest latency.

#### ListDevices
Get available input devices.

```protobuf
rpc ListDevices(google.protobuf.Empty) returns (DeviceList);
```

**Response:**
```json
{
  "devices": [
    {
      "id": "1",
      "name": "HID Keyboard Device",
      "device_type": "DEVICE_TYPE_KEYBOARD",
      "vendor_id": 1234,
      "product_id": 5678,
      "is_keyboard": true,
      "is_mouse": false,
      "is_touch": false
    }
  ]
}
```

#### ConfigureDriver
Configure the active input driver.

```protobuf
rpc ConfigureDriver(DriverConfig) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "backend": "BACKEND_TYPE_INTERCEPTION",
  "exclusive_mode": false,
  "injection_delay_us": 1000,
  "max_batch_size": 64,
  "device_filter": {
    "vendor_ids": [1234],
    "product_ids": [5678],
    "device_names": ["My Keyboard"]
  }
}
```

#### GetDriverInfo
Get current driver status and statistics.

```protobuf
rpc GetDriverInfo(google.protobuf.Empty) returns (DriverInfo);
```

**Response:**
```json
{
  "backend": "BACKEND_TYPE_INTERCEPTION",
  "initialized": true,
  "ready": true,
  "devices": [...],
  "stats": {
    "events_injected": 12345,
    "events_failed": 0,
    "avg_latency_us": 450,
    "last_error": ""
  }
}
```

---

### MacroService
Manages and executes predefined input macros.

#### ExecuteMacro
Execute a registered macro by name.

```protobuf
rpc ExecuteMacro(MacroRequest) returns (MacroResponse);
```

**Request:**
```json
{
  "name": "copy_paste",
  "parameters": {
    "delay_ms": 100
  },
  "variance_ms": 50  // Human-like timing variance
}
```

**Response:**
```json
{
  "success": true,
  "error": ""
}
```

#### ListMacros
Get all registered macros.

```protobuf
rpc ListMacros(google.protobuf.Empty) returns (MacroList);
```

**Response:**
```json
{
  "macros": [
    {
      "name": "copy_paste",
      "description": "Copy (Ctrl+C) then Paste (Ctrl+V)",
      "schema": {
        "parameters": {
          "delay_ms": {
            "type": "int",
            "description": "Delay between copy and paste",
            "required": false,
            "default": "100"
          }
        }
      }
    }
  ]
}
```

#### RegisterMacro
Register a new custom macro.

```protobuf
rpc RegisterMacro(MacroDefinition) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "name": "custom_shortcut",
  "description": "My custom shortcut",
  "schema": {
    "parameters": {
      "repeat": { "type": "int", "default": "1" }
    }
  },
  "events": [
    {"key": {"code": 91, "state": "KEY_STATE_DOWN"}},   // Win
    {"key": {"code": 82, "state": "KEY_STATE_DOWN"}},   // R
    {"key": {"code": 82, "state": "KEY_STATE_UP"}},
    {"key": {"code": 91, "state": "KEY_STATE_UP"}},
    {"delay": {"microseconds": 500000}},
    {"key": {"code": 78, "state": "KEY_STATE_DOWN"}},   // N (notepad)
    {"key": {"code": 78, "state": "KEY_STATE_UP"}},
    {"key": {"code": 28, "state": "KEY_STATE_DOWN"}},   // Enter
    {"key": {"code": 28, "state": "KEY_STATE_UP"}}
  ]
}
```

#### UnregisterMacro
Remove a macro.

```protobuf
rpc UnregisterMacro(MacroName) returns (google.protobuf.Empty);
```

---

### TaskService
Manages asynchronous task execution with progress streaming.

#### SubmitTask
Submit a task for execution.

```protobuf
rpc SubmitTask(TaskRequest) returns (TaskResponse);
```

**Request:**
```json
{
  "task_id": "task-123",
  "task_type": "macro_sequence",
  "parameters": {
    "macros": ["open_notepad", "type_hello", "save_file"],
    "delay_between": 500
  },
  "priority": 10,
  "timeout_ms": 30000
}
```

**Response:**
```json
{
  "task_id": "task-123",
  "status": "TASK_STATUS_PENDING",
  "result": {},
  "error": "",
  "started_at": 0,
  "completed_at": 0
}
```

#### GetTaskStatus
Get current task status.

```protobuf
rpc GetTaskStatus(TaskId) returns (TaskResponse);
```

#### CancelTask
Cancel a running task.

```protobuf
rpc CancelTask(TaskId) returns (google.protobuf.Empty);
```

#### StreamTask
Stream task progress (server-streaming RPC).

```protobuf
rpc StreamTask(TaskStreamRequest) returns (stream TaskStreamResponse);
```

**Response Stream:**
```json
{
  "task_id": "task-123",
  "status": "TASK_STATUS_RUNNING",
  "progress": { "current_macro": "type_hello", "step": 2, "total": 3 },
  "log": "Executing type_hello macro"
}
```

#### ListTasks
List tasks with filtering.

```protobuf
rpc ListTasks(TaskFilter) returns (TaskList);
```

---

### AuthService
Capability-based JWT authentication.

#### IssueToken
Issue a new capability token.

```protobuf
rpc IssueToken(TokenRequest) returns (TokenResponse);
```

**Request:**
```json
{
  "client_id": "orchestrator-1",
  "capabilities": [
    {
      "resource": "input",
      "actions": ["READ", "WRITE", "EXECUTE"]
    },
    {
      "resource": "vision",
      "actions": ["READ", "EXECUTE"]
    },
    {
      "resource": "macro",
      "actions": ["EXECUTE"]
    }
  ],
  "ttl_seconds": 3600
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1700000000,
  "capabilities": [...]
}
```

#### ValidateToken
Validate a token and check capabilities.

```protobuf
rpc ValidateToken(TokenValidationRequest) returns (TokenValidationResponse);
```

**Request:**
```json
{
  "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "required_resource": "input",
  "required_action": "WRITE"
}
```

**Response:**
```json
{
  "valid": true,
  "client_id": "orchestrator-1",
  "capabilities": [...]
}
```

#### RevokeToken
Revoke a token.

```protobuf
rpc RevokeToken(TokenId) returns (google.protobuf.Empty);
```

#### ListTokens
List active tokens (admin only).

```protobuf
rpc ListTokens(google.protobuf.Empty) returns (TokenList);
```

---

### SystemService
System information and management.

#### HealthCheck
```protobuf
rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
```

**Response:**
```json
{
  "status": "SERVING_STATUS_SERVING",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "components": {
    "driver": { "status": "COMPONENT_STATUS_HEALTHY", "message": "Interception ready" },
    "macro_registry": { "status": "COMPONENT_STATUS_HEALTHY", "message": "12 macros loaded" }
  }
}
```

#### GetSystemInfo
```protobuf
rpc GetSystemInfo(google.protobuf.Empty) returns (SystemInfo);
```

**Response:**
```json
{
  "hostname": "desktop-abc",
  "os": "Windows 11",
  "arch": "x86_64",
  "uptime_seconds": 86400,
  "cpu": { "brand": "Intel i9-13900K", "cores": 24, "threads": 32, "usage_percent": 15.5 },
  "memory": { "total_bytes": 34359738368, "available_bytes": 21474836480, "used_bytes": 12884901888 },
  "gpus": [{ "name": "RTX 4090", "memory_bytes": 25769803776, "usage_percent": 5.2, "driver_version": "545.84" }],
  "driver": { "backend": "BACKEND_TYPE_INTERCEPTION", "initialized": true, "ready": true, ... }
}
```

#### GetMetrics
Get Prometheus-formatted metrics.

```protobuf
rpc GetMetrics(google.protobuf.Empty) returns (MetricsResponse);
```

**Response:**
```json
{
  "metrics": "# HELP hcs_input_events_total Total input events injected\n# TYPE hcs_input_events_total counter\nhcs_input_events_total{type=\"keyboard\"} 12345\n..."
}
```

#### Shutdown
Gracefully shutdown the agent.

```protobuf
rpc Shutdown(google.protobuf.Empty) returns (google.protobuf.Empty);
```

#### ReloadConfig
Reload configuration without restart.

```protobuf
rpc ReloadConfig(google.protobuf.Empty) returns (google.protobuf.Empty);
```

---

## Data Types

### KeyEvent
| Field | Type | Description |
|-------|------|-------------|
| code | uint32 | Virtual key code |
| state | KeyState | DOWN or UP |
| scan_code | uint32 | Hardware scan code |
| extended | bool | Extended key (E0/E1) |
| timestamp_us | uint64 | Event timestamp |

### MouseEvent
| Field | Type | Description |
|-------|------|-------------|
| x | int32 | Absolute X or relative dx |
| y | int32 | Absolute Y or relative dy |
| dx | int32 | Relative X movement |
| dy | int32 | Relative Y movement |
| button | MouseButton | Button identifier |
| button_state | KeyState | Button state |
| absolute | bool | True for absolute coordinates |
| timestamp_us | uint64 | Event timestamp |

### KeyState
- `KEY_STATE_DOWN` (1)
- `KEY_STATE_UP` (2)

### MouseButton
- `MOUSE_BUTTON_LEFT` (1)
- `MOUSE_BUTTON_RIGHT` (2)
- `MOUSE_BUTTON_MIDDLE` (3)
- `MOUSE_BUTTON_X1` (4)
- `MOUSE_BUTTON_X2` (5)
- `MOUSE_BUTTON_WHEEL_UP` (6)
- `MOUSE_BUTTON_WHEEL_DOWN` (7)
- `MOUSE_BUTTON_WHEEL_LEFT` (8)
- `MOUSE_BUTTON_WHEEL_RIGHT` (9)

### Capability
| Field | Type | Description |
|-------|------|-------------|
| resource | string | Resource name (input, vision, brain, macro, system, adapters.*) |
| actions | Action[] | Allowed actions |
| constraints | map<string,string> | Additional constraints |

### Action
- `ACTION_READ` (1)
- `ACTION_WRITE` (2)
- `ACTION_EXECUTE` (3)
- `ACTION_ADMIN` (4)

### TaskStatus
- `TASK_STATUS_PENDING` (1)
- `TASK_STATUS_RUNNING` (2)
- `TASK_STATUS_COMPLETED` (3)
- `TASK_STATUS_FAILED` (4)
- `TASK_STATUS_CANCELLED` (5)

### BackendType
- `BACKEND_TYPE_AUTO` (1)
- `BACKEND_TYPE_INTERCEPTION` (2)
- `BACKEND_TYPE_UINPUT` (3)
- `BACKEND_TYPE_HID` (4)
- `BACKEND_TYPE_SYNTHETIC` (5)

---

## Client Examples

### Python Client
```python
import grpc
from hcs_agent_proto import agent_pb2, agent_pb2_grpc

class AgentClient:
    def __init__(self, host="localhost", port=50051):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.input = agent_pb2_grpc.InputServiceStub(self.channel)
        self.macro = agent_pb2_grpc.MacroServiceStub(self.channel)
        self.auth = agent_pb2_grpc.AuthServiceStub(self.channel)
        self.token = None
    
    def authenticate(self, client_id, capabilities):
        req = agent_pb2.TokenRequest(
            client_id=client_id,
            capabilities=[agent_pb2.Capability(resource=r, actions=a) for r,a in capabilities],
            ttl_seconds=3600
        )
        resp = self.auth.IssueToken(req)
        self.token = resp.token
        return self.token
    
    def _auth_metadata(self):
        return [("authorization", f"Bearer {self.token}")]
    
    def press_key(self, code, down=True):
        event = agent_pb2.KeyEvent(
            code=code,
            state=agent_pb2.KEY_STATE_DOWN if down else agent_pb2.KEY_STATE_UP
        )
        self.input.InjectKey(event, metadata=self._auth_metadata())
    
    def type_text(self, text):
        for ch in text:
            code = ord(ch.upper())  # Simplified
            self.press_key(code, True)
            self.press_key(code, False)
    
    def run_macro(self, name, params=None):
        req = agent_pb2.MacroRequest(name=name, parameters=params or {})
        return self.macro.ExecuteMacro(req, metadata=self._auth_metadata())

# Usage
client = AgentClient()
client.authenticate("my-client", [
    ("input", ["READ", "WRITE", "EXECUTE"]),
    ("macro", ["EXECUTE"])
])
client.type_text("Hello World!")
client.run_macro("copy_paste")
```

### Rust Client
```rust
use hcs_agent_proto::{
    agent_service_client::AgentServiceClient,
    input_service_client::InputServiceClient,
    macro_service_client::MacroServiceClient,
    auth_service_client::AuthServiceClient,
    TokenRequest, Capability, Action, KeyEvent, KeyState, MacroRequest,
};
use tonic::{Request, metadata::MetadataValue};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AgentServiceClient::connect("http://localhost:50051").await?;
    
    // Authenticate
    let auth_client = AuthServiceClient::new(client.channel().clone());
    let token_resp = auth_client.issue_token(Request::new(TokenRequest {
        client_id: "rust-client".into(),
        capabilities: vec![
            Capability { resource: "input".into(), actions: vec![Action::Write as i32, Action::Execute as i32] },
            Capability { resource: "macro".into(), actions: vec![Action::Execute as i32] },
        ],
        ttl_seconds: 3600,
    })).await?.into_inner();
    
    let token = token_resp.token;
    let auth_header = MetadataValue::from_str(&format!("Bearer {}", token))?;
    
    // Inject key
    let mut input_client = InputServiceClient::new(client.channel().clone());
    let mut req = Request::new(KeyEvent { code: 65, state: KeyState::Down as i32, ..Default::default() });
    req.metadata_mut().insert("authorization", auth_header.clone());
    input_client.inject_key(req).await?;
    
    // Run macro
    let mut macro_client = MacroServiceClient::new(client.channel().clone());
    let mut req = Request::new(MacroRequest { name: "copy_paste".into(), ..Default::default() });
    req.metadata_mut().insert("authorization", auth_header);
    macro_client.execute_macro(req).await?;
    
    Ok(())
}
```

### Go Client
```go
package main

import (
	"context"
	"log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	
	agentpb "github.com/your-org/hcs/agent/proto"
)

func main() {
	conn, err := grpc.Dial("localhost:50051", grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatal(err)
	}
	defer conn.Close()
	
	authClient := agentpb.NewAuthServiceClient(conn)
	
	// Issue token
	tokenResp, err := authClient.IssueToken(context.Background(), &agentpb.TokenRequest{
		ClientId: "go-client",
		Capabilities: []*agentpb.Capability{
			{Resource: "input", Actions: []agentpb.Action{agentpb.Action_WRITE, agentpb.Action_EXECUTE}},
		},
		TtlSeconds: 3600,
	})
	if err != nil {
		log.Fatal(err)
	}
	
	// Create auth metadata
	md := metadata.New(map[string]string{"authorization": "Bearer " + tokenResp.Token})
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	
	// Inject key
	inputClient := agentpb.NewInputServiceClient(conn)
	_, err = inputClient.InjectKey(ctx, &agentpb.KeyEvent{
		Code:  65, // 'A'
		State: agentpb.KeyState_KEY_STATE_DOWN,
	})
	if err != nil {
		log.Fatal(err)
	}
	
	// Run macro
	macroClient := agentpb.NewMacroServiceClient(conn)
	_, err = macroClient.ExecuteMacro(ctx, &agentpb.MacroRequest{
		Name: "copy_paste",
	})
	if err != nil {
		log.Fatal(err)
	}
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `UNAUTHENTICATED` | Missing or invalid token |
| `PERMISSION_DENIED` | Token lacks required capability |
| `NOT_FOUND` | Macro/device/task not found |
| `INVALID_ARGUMENT` | Invalid request parameters |
| `FAILED_PRECONDITION` | Driver not initialized |
| `INTERNAL` | Internal server error |
| `UNAVAILABLE` | Service not ready |

---

## Rate Limits

- **Input Injection:** 1000 events/second per client
- **Macro Execution:** 100 executions/second
- **Token Issuance:** 10 tokens/minute per client_id
- **Task Submission:** 50 concurrent tasks per client

---

## Changelog

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release |
| 1.1.0 | Added InjectStream, MacroParameterSchema |
| 1.2.0 | Added capability constraints, task priorities |