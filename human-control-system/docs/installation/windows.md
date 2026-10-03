# Windows Installation Guide

## Overview

This guide covers installing and configuring the Human Control System on Windows 10/11. The primary input driver on Windows is the **Interception** kernel driver, which must be installed separately before running HCS.

## Prerequisites

- **OS:** Windows 10 version 1903+ or Windows 11
- **Architecture:** x64 (ARM64 not yet supported for Interception)
- **Runtime:** Visual C++ Redistributable 2015-2022
- **Privileges:** Administrator (for driver installation)

## 1. Install Interception Driver

The Interception driver enables low-level keyboard and mouse injection.

### Option A: Automatic (Recommended)

```powershell
# Run as Administrator
# Download and install latest Interception release
$url = "https://github.com/oblita/Interception/releases/latest/download/Interception.zip"
$output = "$env:TEMP\Interception.zip"
Invoke-WebRequest -Uri $url -OutFile $output
Expand-Archive -Path $output -DestinationPath "$env:TEMP\Interception" -Force
cd "$env:TEMP\Interception"
.\install-interception.exe /install
```

### Option B: Manual

1. Download latest release from: https://github.com/oblita/Interception/releases
2. Extract ZIP archive
3. Open **PowerShell as Administrator**
4. Run: `.\install-interception.exe /install`
5. Reboot if prompted

### Verify Installation

```powershell
# Check driver is loaded
fltmc instances | findstr interception

# Check DLL exists
ls C:\Windows\System32\interception.dll

# Test with demo (if included)
.\interception-demo.exe
```

### Uninstall (if needed)

```powershell
# Run as Administrator
.\install-interception.exe /uninstall
```

## 2. Install Rust Toolchain (for building from source)

```powershell
# Install Rust via rustup
winget install Rustlang.Rustup

# Or use the installer
# https://rustup.rs/

# Restart shell, then:
rustup default stable
rustup component add rustfmt clippy
```

## 3. Install Python (for Vision Service)

```powershell
# Via winget
winget install Python.Python.3.11

# Or download from python.org
# Ensure "Add to PATH" is checked during install

# Verify
python --version  # Should be 3.11+
pip --version
```

## 4. Install Docker (for Containerized Deployment)

```powershell
# Docker Desktop for Windows
winget install Docker.DockerDesktop

# Enable WSL2 backend (recommended)
# Restart after install
```

## 5. Install NVIDIA GPU Support (Optional, for Vision)

```powershell
# Install NVIDIA driver (latest Game Ready or Studio)
# https://www.nvidia.com/drivers/

# Install CUDA Toolkit (if building TensorRT engines)
winget install Nvidia.CUDA

# Verify
nvidia-smi
```

## 6. Build from Source

```powershell
# Clone repository
git clone https://github.com/your-org/human-control-system.git
cd human-control-system

# Build all Rust components
cargo build --release --workspace

# Build vision service dependencies
cd vision
pip install -r requirements.txt
# For GPU support:
# pip install onnxruntime-gpu
# Or for TensorRT: follow NVIDIA TensorRT installation guide

# Build firmware (optional)
cd ../hardware
# Requires ARM toolchain - see hardware/README.md
```

## 7. Run with Docker Compose (Easiest)

```powershell
# Prerequisites: Docker Desktop running, WSL2 enabled

cd docker
# Copy config template
cp ../config/agent.toml.example ../config/agent.toml
# Edit config as needed

# Start all services
docker compose up -d

# View logs
docker compose logs -f hcs-agent

# Stop
docker compose down
```

## 8. Run Natively (Development)

### Terminal 1: Vision Service
```powershell
cd vision
python -m src.server
```

### Terminal 2: Brain Service
```powershell
cd brain
cargo run --release
```

### Terminal 3: Agent Daemon (Run as Administrator!)
```powershell
# Must run as Administrator for Interception driver access
cd agent
cargo run --release
```

### Terminal 4: Orchestrator CLI
```powershell
cd orchestrator
cargo run --release -- run examples/basic_task.yaml
```

## 9. Configuration

### Agent Configuration (`config/agent.toml`)

```toml
[server]
host = "0.0.0.0"
port = 50051
workers = 4

[driver]
backend = "auto"  # auto, interception, uinput, hid, synthetic
exclusive_mode = false
injection_delay_us = 1000
max_batch_size = 64

[auth]
jwt_secret = "your-secret-key-change-in-production"
token_ttl_seconds = 900
enable_auth = true

[macros]
# Built-in macros directory
macro_dir = "macros"

[logging]
level = "info"
format = "json"
output = "stdout"
```

### Vision Configuration (`config/vision.toml`)

```toml
[service]
host = "0.0.0.0"
port = 50052
workers = 4
log_level = "info"

[capture]
default_fps = 30
default_monitor = 0
jpeg_quality = 85
use_hardware_acceleration = true
backend = "auto"  # dxgi, gdi, dxcam

[detection]
model_path = "models/yolov8n.onnx"
device = "tensorrt"  # cpu, cuda, tensorrt
batch_size = 1
default_confidence = 0.5
default_iou = 0.45
input_width = 640
input_height = 640
use_tensorrt = true
tensorrt_cache_dir = "models/tensorrt_cache"

[ocr]
engine = "paddle"
languages = ["en"]
use_gpu = true
gpu_id = 0
cpu_threads = 4
enable_mkldnn = true
```

## 10. Verify Installation

### Test Agent Connection
```powershell
# Using grpcurl (install: winget install grpcurl)
grpcurl -plaintext localhost:50051 hcs.agent.v1.SystemService/HealthCheck
```

### Test Vision Service
```powershell
grpcurl -plaintext localhost:50052 hcs.vision.v1.VisionService/HealthCheck

# Test screen capture
grpcurl -plaintext -d '{"monitor_index": 0}' localhost:50052 hcs.vision.v1.VisionService/CaptureScreen
```

### Test Input Injection
```powershell
# Get auth token first
TOKEN=$(grpcurl -plaintext -d '{"client_id": "test", "capabilities": [{"resource": "input", "actions": ["WRITE"]}]}' localhost:50051 hcs.agent.v1.AuthService/IssueToken | jq -r .token)

# Inject a key (Win+R)
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x5B, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x52, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x52, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x5B, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
```

## Troubleshooting

### Interception Driver Not Found
```
Error: "Interception DLL not found. Install from https://github.com/oblita/Interception/releases"
```
**Solution:** Ensure `interception.dll` is in `C:\Windows\System32\` or in the same directory as the agent binary.

### Permission Denied / Access Denied
```
Error: "Failed to create interception context"
```
**Solution:** Run the agent as Administrator. Interception requires elevated privileges.

### Driver Not Loaded After Install
```
fltmc shows no interception instance
```
**Solution:** Reboot after installing Interception. Check Windows Event Viewer for driver load errors.

### Antivirus Blocks Interception
Some antivirus software flags Interception as suspicious.
**Solution:** Add exclusion for `interception.dll` and the HCS agent binary.

### Screen Capture Black/Empty
- Ensure `backend = "dxgi"` in vision config for Windows 10/11
- Check if target application runs as Administrator (capture needs same or higher privilege)
- For UWP apps, try `backend = "dxcam"`

### High Input Latency
- Reduce `injection_delay_us` in agent config (default 1000µs = 1ms)
- Use `exclusive_mode = true` for dedicated input (blocks physical input)
- Ensure power plan is "High Performance"

## Next Steps

- [Linux Installation Guide](linux.md)
- [macOS Installation Guide](macos.md)
- [Hardware/Teensy Setup](hardware.md)
- [Docker Deployment](../development/building.md#docker-deployment)
- [API Reference](../api/agent.md)