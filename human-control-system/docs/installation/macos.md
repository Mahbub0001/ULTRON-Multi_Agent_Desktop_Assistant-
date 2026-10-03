# macOS Installation Guide

## Overview

This guide covers installing and configuring the Human Control System on macOS 12+ (Monterey, Ventura, Sonoma). Input injection on macOS uses the **IOHID** framework via a synthetic driver (for testing) or requires a **kernel extension** for production use.

> **⚠️ Important:** macOS has strict security policies (SIP, TCC, DriverKit). Full kernel-level input injection requires:
> - Disabling SIP (System Integrity Protection) for kexts, OR
> - Using Apple's DriverKit with entitlements (Apple Developer Program), OR
> - Using the HID hardware adapter (Teensy) for undetectable injection

For development/testing, the **synthetic driver** works without special permissions.

## Prerequisites

- **OS:** macOS 12.0+ (Monterey, Ventura, Sonoma)
- **Architecture:** Apple Silicon (M1/M2/M3) or Intel x86_64
- **Xcode Command Line Tools:** Required for building
- **Homebrew:** Recommended for package management

## 1. Install Xcode Command Line Tools

```bash
# Install command line tools
xcode-select --install

# Accept license
sudo xcodebuild -license accept

# Verify
xcode-select -p
# Should show: /Library/Developer/CommandLineTools
```

## 2. Install Homebrew

```bash
# Install Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Add to PATH (Apple Silicon)
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv)"

# Add to PATH (Intel)
echo 'eval "$(/usr/local/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/usr/local/bin/brew shellenv)"

# Verify
brew --version
```

## 3. Install System Dependencies

```bash
# Core build tools
brew install \
    rust \
    python@3.11 \
    cmake \
    pkg-config \
    openssl \
    protobuf \
    grpcurl \
    jq \
    git \
    wget \
    llvm \
    clang-format

# For vision service
brew install \
    opencv \
    onnxruntime \
    tesseract \
    tesseract-lang \
    vips \
    ffmpeg

# For firmware (ARM toolchain)
brew install arm-none-eabi-gcc

# Optional: NVIDIA eGPU support (Intel Macs only)
# brew install nvidia-cuda  # Not available on Apple Silicon
```

## 4. Install Rust Toolchain

```bash
# Via rustup (preferred over brew for component management)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

source "$HOME/.cargo/env"

rustup default stable
rustup component add rustfmt clippy

# For Apple Silicon: add x86_64 target for cross-compilation if needed
rustup target add x86_64-apple-darwin

# Verify
cargo --version
rustc --version
```

## 5. Install Python & Vision Dependencies

```bash
# Use Homebrew Python
brew install python@3.11

# Create virtual environment
python3.11 -m venv ~/hcs-venv
source ~/hcs-venv/bin/activate

# Upgrade pip
pip install --upgrade pip setuptools wheel

# Install vision requirements
cd human-control-system/vision
pip install -r requirements.txt

# macOS-specific: install ONNX Runtime with CoreML support
pip install onnxruntime-coreml

# For PaddleOCR (may have issues on Apple Silicon)
# pip install paddlepaddle paddleocr
# Alternative: use Tesseract only
```

## 6. Configure Input Permissions (TCC)

macOS requires explicit permission for input monitoring and injection.

### For Synthetic Driver (Development)
No special permissions needed - runs in userspace.

### For HID Hardware (Teensy)
No special permissions - appears as standard USB HID device.

### For Kernel Extension (Production - Advanced)

> **⚠️ Requires Apple Developer Program ($99/year) and SIP disabled**

1. **Disable SIP:**
   - Reboot into Recovery Mode (Cmd+R on Intel, Power button on Apple Silicon)
   - Terminal: `csrutil disable`
   - Reboot

2. **Create DriverKit Extension:**
   - Requires Apple Developer ID
   - Sign with `com.apple.developer.driverkit` entitlement
   - Install to `/Library/Extensions`

3. **Alternative: Use IOHIDManager (Userspace)**
   - Limited to apps with Accessibility permission
   - System Settings → Privacy & Security → Accessibility → Add terminal/IDE

### Grant Accessibility Permission (For Userspace IOHID)

```bash
# Add Terminal.app (or your IDE) to Accessibility
# System Settings → Privacy & Security → Accessibility → + → Terminal

# Or via command line (requires SIP disabled)
sudo tccutil reset Accessibility
# Then manually add in System Settings
```

## 7. Install Docker (Optional)

```bash
# Docker Desktop for Mac
brew install --cask docker

# Start Docker Desktop from Applications
# Enable "Use Rosetta for x86/amd64 emulation on Apple Silicon" in Settings

# Verify
docker run --rm hello-world
```

## 8. Build from Source

```bash
# Clone repository
git clone https://github.com/your-org/human-control-system.git
cd human-control-system

# Build all Rust components
cargo build --release --workspace

# Build vision service
cd vision
source ~/hcs-venv/bin/activate
pip install -r requirements.txt

# Build firmware
cd ../hardware
./scripts/build_all.sh
```

## 9. Run with Docker Compose

```bash
cd human-control-system/docker

# On Apple Silicon, use native arm64 images
# Docker Compose automatically uses arm64

docker compose up -d

# View logs
docker compose logs -f hcs-agent
```

## 10. Run Natively

### Terminal 1: Vision Service
```bash
cd human-control-system/vision
source ~/hcs-venv/bin/activate
python -m src.server
```

### Terminal 2: Brain Service
```bash
cd human-control-system/brain
cargo run --release
```

### Terminal 3: Agent Daemon
```bash
cd human-control-system/agent
# Uses synthetic driver by default on macOS
cargo run --release
```

### Terminal 4: Orchestrator
```bash
cd human-control-system/orchestrator
cargo run --release -- run examples/basic_task.yaml
```

## 11. Configuration

### Agent Configuration (`config/agent.toml`)

```toml
[server]
host = "0.0.0.0"
port = 50051
workers = 4

[driver]
backend = "synthetic"  # synthetic, hid (for Teensy)
exclusive_mode = false
injection_delay_us = 1000
max_batch_size = 64

[auth]
jwt_secret = "your-secret-key-change-in-production"
token_ttl_seconds = 900
enable_auth = true

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
backend = "coregraphics"  # coregraphics only on macOS

[detection]
model_path = "models/yolov8n.onnx"
device = "coreml"  # cpu, coreml
batch_size = 1
default_confidence = 0.5
default_iou = 0.45
input_width = 640
input_height = 640

[ocr]
engine = "tesseract"  # paddle has limited macOS support
languages = ["eng"]
use_gpu = false
cpu_threads = 4
tesseract_data_path = "/opt/homebrew/share/tessdata"  # Apple Silicon
# tesseract_data_path = "/usr/local/share/tessdata"  # Intel
```

## 12. Verify Installation

### Test Agent
```bash
# Health check
grpcurl -plaintext localhost:50051 hcs.agent.v1.SystemService/HealthCheck
```

### Test Vision
```bash
grpcurl -plaintext localhost:50052 hcs.vision.v1.VisionService/HealthCheck

# List monitors
grpcurl -plaintext -d '{}' localhost:50052 hcs.vision.v1.VisionService/ListMonitors
```

### Test Synthetic Input Injection
```bash
# Get token
TOKEN=$(grpcurl -plaintext -d '{"client_id": "test", "capabilities": [{"resource": "input", "actions": ["WRITE"]}]}' localhost:50051 hcs.agent.v1.AuthService/IssueToken | jq -r .token)

# Inject key (Cmd+Space for Spotlight)
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x37, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey  # Cmd
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x31, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey  # Space
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x31, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 0x37, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
```

> **Note:** Synthetic driver only works within the same process context. For system-wide injection, use HID hardware.

## 13. Hardware Setup (Teensy) on macOS

### Flash Firmware
```bash
cd human-control-system/hardware

# Build for Teensy 4.0
make BOARD=TEENSY40

# Flash using teensy_loader_cli
brew install teensy_loader_cli
teensy_loader_cli -mmcu=IMXRT1062 -w -v build/firmware.hex
```

### Configure HID Driver
```toml
# In config/agent.toml
[driver]
backend = "hid"
# Device will be auto-detected by VID/PID
```

### Test HID Connection
```bash
# List USB devices
system_profiler SPUSBDataType | grep -A5 -B5 "Teensy"

# Or using Rust
cargo run --example list_hid_devices
```

## Troubleshooting

### Build Errors: OpenSSL
```bash
# Set OpenSSL path for cargo
export OPENSSL_DIR=$(brew --prefix openssl)
export OPENSSL_LIB_DIR=$(brew --prefix openssl)/lib
export OPENSSL_INCLUDE_DIR=$(brew --prefix openssl)/include
```

### Build Errors: Protobuf/GRPC
```bash
# Ensure protobuf compiler is installed
brew install protobuf

# For tonic-build, set PROTOC
export PROTOC=$(which protoc)
```

### Vision: ONNX Runtime CoreML Issues
```bash
# Install coremltools for model conversion
pip install coremltools

# Convert model to CoreML format
python -m onnxruntime.tools.convert_onnx_models_to_ort --help
```

### Vision: Screen Capture Black
- Ensure app has Screen Recording permission: System Settings → Privacy & Security → Screen Recording
- Add Terminal/IDE to the list
- For CoreGraphics backend, this is required

### Vision: Tesseract Not Found
```bash
# Set TESSDATA_PREFIX
export TESSDATA_PREFIX="/opt/homebrew/share/tessdata"  # Apple Silicon
# export TESSDATA_PREFIX="/usr/local/share/tessdata"  # Intel

# Verify
tesseract --list-langs
```

### Docker: x86_64 Images on Apple Silicon
```bash
# Enable Rosetta emulation in Docker Desktop Settings
# Or use platform flag
docker run --platform linux/amd64 ...

# Prefer arm64 native images when available
```

### Permission Denied on /dev/tty* (Teensy)
```bash
# Add user to dialout group (Linux) - on macOS, usually not needed
# But if using serial terminal:
sudo chmod 666 /dev/tty.usbmodem*
```

### High CPU Usage / Thermal Throttling
- Reduce capture FPS in vision config
- Use `device = "cpu"` for detection if thermal throttling
- Monitor with `sudo powermetrics --samplers smc`

## Limitations on macOS

| Feature | Status | Notes |
|---------|--------|-------|
| Kernel Input Injection | Limited | Requires DriverKit + Apple Dev Program |
| Synthetic Driver | ✅ Full | Process-local only |
| HID Hardware (Teensy) | ✅ Full | Recommended for production |
| Screen Capture | ✅ Full | CoreGraphics, needs Screen Recording permission |
| YOLOv8 (CoreML) | ✅ Full | Convert ONNX → CoreML for best perf |
| PaddleOCR | ⚠️ Limited | Use Tesseract instead |
| TensorRT | ❌ No | NVIDIA only |
| CUDA | ❌ No | NVIDIA only |

## Next Steps

- [Windows Installation Guide](windows.md)
- [Linux Installation Guide](linux.md)
- [Hardware/Teensy Setup](hardware.md)
- [Docker Deployment](../development/building.md#docker-deployment)
- [API Reference](../api/agent.md)