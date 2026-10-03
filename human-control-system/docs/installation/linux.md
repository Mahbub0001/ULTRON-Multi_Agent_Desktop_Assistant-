# Linux Installation Guide

## Overview

This guide covers installing and configuring the Human Control System on Linux. The primary input driver on Linux is **uinput**, a kernel module that creates virtual input devices.

## Prerequisites

- **OS:** Linux kernel 5.10+ (Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch)
- **Architecture:** x86_64, ARM64 (aarch64)
- **Kernel Headers:** Required for uinput module
- **Privileges:** Root/sudo for uinput setup, user group for runtime

## 1. Install System Dependencies

### Ubuntu / Debian
```bash
# Update package list
sudo apt update

# Install build dependencies
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    lld \
    cmake \
    git \
    curl \
    wget \
    libudev-dev \
    libx11-dev \
    libxcb1-dev \
    libxrandr-dev \
    libxi-dev \
    libglib2.0-dev \
    python3 \
    python3-pip \
    python3-venv \
    docker.io \
    docker-compose-plugin

# Install NVIDIA drivers (optional, for GPU acceleration)
# ubuntu-drivers autoinstall
```

### Fedora / RHEL / CentOS Stream
```bash
sudo dnf install -y \
    gcc gcc-c++ make pkg-config openssl-devel \
    clang lld cmake git curl wget \
    libudev-devel libX11-devel libxcb-devel \
    libXrandr-devel libXi-devel glib2-devel \
    python3 python3-pip python3-virtualenv \
    docker docker-compose

# NVIDIA
# sudo dnf install akmod-nvidia
```

### Arch Linux
```bash
sudo pacman -S --needed \
    base-devel pkg-config openssl clang lld cmake git curl wget \
    libudev libx11 libxcb libxrandr libxi glib2 \
    python python-pip python-virtualenv \
    docker docker-compose

# NVIDIA
# sudo pacman -S nvidia nvidia-utils
```

## 2. Configure uinput Permissions

The uinput kernel module creates `/dev/uinput`. Your user needs read/write access.

### Option A: udev Rule (Recommended)

```bash
# Create udev rule
sudo tee /etc/udev/rules.d/99-hcs-uinput.rules << 'EOF'
KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"
EOF

# Create input group if not exists
sudo groupadd -f input

# Add your user to input group
sudo usermod -aG input $USER

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Load uinput module
sudo modprobe uinput

# Verify
ls -la /dev/uinput
# Should show: crw-rw---- 1 root input 10, 223 ... /dev/uinput
```

### Option B: Temporary (Until Reboot)
```bash
sudo modprobe uinput
sudo chmod 666 /dev/uinput
```

### Option C: Systemd Service (Persistent)
```bash
sudo tee /etc/modules-load.d/hcs-uinput.conf << 'EOF'
uinput
EOF

sudo tee /etc/udev/rules.d/99-hcs-uinput.rules << 'EOF'
KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"
EOF

sudo systemctl restart systemd-udevd
```

### Verify uinput Access
```bash
# After logout/login or new shell
groups $USER  # Should include 'input'

# Test access
python3 -c "
import os
fd = os.open('/dev/uinput', os.O_WRONLY | os.O_NONBLOCK)
print('uinput accessible:', fd > 0)
os.close(fd)
"
```

## 3. Install Rust Toolchain

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Source environment
source "$HOME/.cargo/env"

# Install components
rustup default stable
rustup component add rustfmt clippy

# Verify
cargo --version
rustc --version
```

## 4. Install Python & Vision Dependencies

```bash
# Create virtual environment (recommended)
python3 -m venv ~/hcs-venv
source ~/hcs-venv/bin/activate

# Upgrade pip
pip install --upgrade pip setuptools wheel

# Install vision requirements
cd human-control-system/vision
pip install -r requirements.txt

# For GPU support (NVIDIA):
# pip install onnxruntime-gpu
# For TensorRT: see NVIDIA TensorRT installation guide
```

### System-wide Python (Alternative)
```bash
# Install system packages for OCR
sudo apt install -y tesseract-ocr tesseract-ocr-eng libtesseract-dev

# For PaddleOCR (may need specific Python version)
pip install paddlepaddle paddleocr
```

## 5. Install Docker & NVIDIA Container Toolkit

```bash
# Start Docker
sudo systemctl enable --now docker

# Add user to docker group
sudo usermod -aG docker $USER

# Install NVIDIA Container Toolkit (for GPU in containers)
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
    sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
    sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list

sudo apt update
sudo apt install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify GPU access in container
docker run --rm --gpus all nvidia/cuda:12.4-base nvidia-smi
```

## 6. Configure X11/Wayland for Screen Capture

### X11 (Traditional)
```bash
# Ensure X11 is running
echo $DISPLAY  # Should show :0 or similar

# For headless servers, install virtual display
sudo apt install -y xvfb
# Start: Xvfb :99 -screen 0 1920x1080x24 &
# Export: export DISPLAY=:99
```

### Wayland (Modern)
```bash
# For Wayland screen capture, need PipeWire
# Most modern distros have this by default

# Verify PipeWire
pactl info | grep "Server Name"  # Should show PipeWire

# Install PipeWire if missing
sudo apt install -y pipewire libpipewire-0.3-dev
```

### Hybrid (Both)
```bash
# For maximum compatibility, ensure both work
# XWayland provides X11 compatibility on Wayland
```

## 7. Build from Source

```bash
# Clone repository
git clone https://github.com/your-org/human-control-system.git
cd human-control-system

# Build all Rust components
cargo build --release --workspace

# Build vision service (in venv)
cd vision
source ~/hcs-venv/bin/activate
pip install -r requirements.txt

# Build firmware (optional)
cd ../hardware
# Requires ARM toolchain
# sudo apt install gcc-arm-none-eabi
./scripts/build_all.sh
```

## 8. Run with Docker Compose

```bash
cd human-control-system/docker

# Copy config templates
mkdir -p ../config
cp ../config/agent.toml.example ../config/agent.toml 2>/dev/null || true

# Start services
docker compose up -d

# View logs
docker compose logs -f hcs-agent

# Stop
docker compose down
```

## 9. Run Natively

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
# No sudo needed if uinput permissions configured correctly
cargo run --release
```

### Terminal 4: Orchestrator
```bash
cd human-control-system/orchestrator
cargo run --release -- run examples/basic_task.yaml
```

## 10. Configuration

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
backend = "auto"  # x11, wayland, pipewire

[detection]
model_path = "models/yolov8n.onnx"
device = "cuda"  # cpu, cuda, tensorrt
batch_size = 1
default_confidence = 0.5
default_iou = 0.45
input_width = 640
input_height = 640

[ocr]
engine = "paddle"
languages = ["en"]
use_gpu = true
gpu_id = 0
cpu_threads = 4
enable_mkldnn = true
```

## 11. Verify Installation

### Test Agent
```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
# Or download binary from GitHub releases

# Health check
grpcurl -plaintext localhost:50051 hcs.agent.v1.SystemService/HealthCheck
```

### Test Vision
```bash
grpcurl -plaintext localhost:50052 hcs.vision.v1.VisionService/HealthCheck

# List monitors
grpcurl -plaintext -d '{}' localhost:50052 hcs.vision.v1.VisionService/ListMonitors
```

### Test Input Injection
```bash
# Get token
TOKEN=$(grpcurl -plaintext -d '{"client_id": "test", "capabilities": [{"resource": "input", "actions": ["WRITE"]}]}' localhost:50051 hcs.agent.v1.AuthService/IssueToken | jq -r .token)

# Inject key (Super+R)
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 125, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 21, "state": "KEY_STATE_DOWN"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 21, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
grpcurl -plaintext -H "Authorization: Bearer $TOKEN" -d '{"code": 125, "state": "KEY_STATE_UP"}' localhost:50051 hcs.agent.v1.InputService/InjectKey
```

## Troubleshooting

### uinput: Permission Denied
```
Error: "Cannot open uinput device. Ensure /dev/uinput exists and you have permissions"
```
**Solution:** 
1. Check `ls -la /dev/uinput` - should be group `input` with `rw-` permissions
2. Ensure user is in `input` group: `groups $USER`
3. Logout and login (or `newgrp input`)
4. Verify module loaded: `lsmod | grep uinput`

### uinput Module Not Found
```
Error: "No such device" when opening /dev/uinput
```
**Solution:**
```bash
sudo modprobe uinput
# Make persistent:
echo uinput | sudo tee /etc/modules-load.d/uinput.conf
```

### Screen Capture Not Working (X11)
- Ensure `DISPLAY` is set: `echo $DISPLAY`
- Test with `xrandr` - should show monitors
- For headless: use Xvfb

### Screen Capture Not Working (Wayland)
- Ensure PipeWire is running: `pactl info`
- Check `XDG_SESSION_TYPE=wayland`
- Some apps need `GDK_BACKEND=wayland` or `QT_QPA_PLATFORM=wayland`

### High Input Latency
- Reduce `injection_delay_us` in config
- Check CPU governor: `cpupower frequency-set -g performance`
- Disable mitigations: add `mitigations=off` to kernel cmdline (security trade-off)

### Docker: GPU Not Available
```bash
# Test GPU in container
docker run --rm --gpus all nvidia/cuda:12.4-base nvidia-smi

# If fails:
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

### SELinux/AppArmor Issues
```bash
# Check audit log
sudo ausearch -m avc -ts recent

# For AppArmor (Ubuntu):
sudo aa-status | grep hcs
# May need to create profile or run in complain mode
```

## Next Steps

- [Windows Installation Guide](windows.md)
- [macOS Installation Guide](macos.md)
- [Hardware/Teensy Setup](hardware.md)
- [Docker Deployment](../development/building.md#docker-deployment)
- [API Reference](../api/agent.md)