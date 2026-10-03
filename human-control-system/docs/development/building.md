# Building from Source

## Overview

This guide covers building the Human Control System from source code on all supported platforms.

## Prerequisites

### All Platforms
- **Git:** 2.30+
- **Rust:** 1.75+ (via rustup)
- **Python:** 3.11+ (for vision service)
- **CMake:** 3.16+ (for firmware)
- **Protocol Buffers:** protoc 25+ (for gRPC code generation)

### Platform-Specific

#### Windows
- Visual Studio 2022 Build Tools or VS 2022 Community
- Windows 10 SDK (10.0.19041+)
- Interception driver (for agent testing)

#### Linux
- GCC 11+ or Clang 14+
- Kernel headers (`linux-headers-$(uname -r)`)
- libudev-dev, libssl-dev, pkg-config
- X11/Wayland development libraries

#### macOS
- Xcode Command Line Tools
- Homebrew packages (see installation guide)

## Repository Structure

```
human-control-system/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── agent/                  # Agent daemon (Rust)
├── brain/                  # Decision engine (Rust)
├── adapters/               # App adapters (Rust)
├── orchestrator/           # CLI/TUI/DSL (Rust)
├── driver/                 # Input drivers (Rust)
├── vision/                 # Computer vision (Python)
├── hardware/               # Teensy firmware (C)
├── docker/                 # Dockerfiles
├── config/                 # Configuration templates
├── docs/                   # Documentation
└── tests/                  # Integration tests
```

## Quick Start

```bash
# Clone repository
git clone https://github.com/your-org/human-control-system.git
cd human-control-system

# Build all Rust components
cargo build --release --workspace

# Build vision service
cd vision
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate
pip install -r requirements.txt

# Build firmware
cd ../hardware
./scripts/build_all.sh
```

## Detailed Build Steps

### 1. Rust Workspace Build

```bash
# Install Rust toolchain
rustup default stable
rustup component add rustfmt clippy

# Build all workspace members
cargo build --release --workspace

# Build specific crate
cargo build --release -p hcs-agent
cargo build --release -p hcs-brain
cargo build --release -p hcs-adapters
cargo build --release -p hcs-orchestrator
cargo build --release -p hcs-driver

# Check without building
cargo check --workspace

# Run clippy (linting)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check --workspace
```

### 2. Vision Service (Python)

```bash
cd vision

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Upgrade pip
pip install --upgrade pip setuptools wheel

# Install dependencies
pip install -r requirements.txt

# For GPU support (NVIDIA):
pip install onnxruntime-gpu

# For TensorRT (optional):
# Follow NVIDIA TensorRT installation guide

# Verify installation
python -c "import onnxruntime; print(onnxruntime.get_available_providers())"
```

#### Generate Protobuf Python Stubs
```bash
cd vision
# Install grpcio-tools if not in requirements
pip install grpcio-tools

# Generate Python gRPC code
python -m grpc_tools.protoc \
    --proto_path=src/proto \
    --python_out=src/proto \
    --grpc_python_out=src/proto \
    src/proto/vision.proto
```

### 3. Firmware Build

```bash
cd hardware

# Using Make (simplest)
make                    # Teensy 4.0
make BOARD=TEENSY41     # Teensy 4.1
make clean              # Clean build

# Using CMake (advanced)
cmake -B build -S . \
    -DBOARD=TEENSY40 \
    -DCMAKE_TOOLCHAIN_FILE=cmake/arm-none-eabi.cmake \
    -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j$(nproc)

# Build all boards
./scripts/build_all.sh
```

#### Required Toolchain
```bash
# Ubuntu/Debian
sudo apt install gcc-arm-none-eabi cmake libusb-1.0-0-dev

# Fedora
sudo dnf install arm-none-eabi-gcc-cs arm-none-eabi-newlib cmake libusb-devel

# macOS
brew install arm-none-eabi-gcc cmake libusb

# Windows
# Download ARM GNU Toolchain from developer.arm.com
# Or: choco install arm-none-eabi-gcc
```

### 4. Protobuf Code Generation (Rust)

```bash
# Install protoc
# Ubuntu: sudo apt install protobuf-compiler
# macOS: brew install protobuf
# Windows: Download from github.com/protocolbuffers/protobuf/releases

# Install tonic-build (done automatically via build.rs)
# Verify
protoc --version  # Should be 25+
```

The Rust build automatically generates gRPC code via `build.rs` in each crate.

## Build Outputs

### Rust Binaries
```
target/release/
├── hcs-agent           # Agent daemon
├── hcs-brain           # Brain service
├── hcs-adapters        # Adapters service
├── hcs-orchestrator    # CLI/TUI
└── hcs-driver-test     # Driver test binary
```

### Vision Service
```
vision/
├── venv/               # Python virtual environment
└── src/
    └── server.py       # Entry point
```

### Firmware
```
hardware/build/
├── firmware.hex        # Teensy 4.0
├── firmware_TEENSY41.hex  # Teensy 4.1
├── firmware.bin
└── firmware.elf
```

## Configuration

### Build-Time Configuration

#### Cargo Features
```bash
# Enable specific features
cargo build --release --features "tensorrt,cuda" -p hcs-vision
cargo build --release --features "simd" -p hcs-driver
```

#### Environment Variables
```bash
# Rust
export RUSTFLAGS="-C target-cpu=native"  # Optimize for current CPU
export CARGO_NET_GIT_FETCH_WITH_CLI=true  # Use git CLI

# Python
export PYTHONPATH="${PWD}/vision/src:${PYTHONPATH}"

# Firmware
export BOARD=TEENSY41
```

### Runtime Configuration

Copy example configs:
```bash
mkdir -p config
cp config/agent.toml.example config/agent.toml
cp config/vision.toml.example config/vision.toml
cp config/brain.toml.example config/brain.toml
cp config/adapters.toml.example config/adapters.toml
cp config/orchestrator.toml.example config/orchestrator.toml
```

Edit configs for your environment.

## Cross-Compilation

### Linux → Windows
```bash
# Install target
rustup target add x86_64-pc-windows-msvc

# Install cross-compilation toolchain
# Requires Windows SDK and MSVC toolchain
cargo build --release --target x86_64-pc-windows-msvc
```

### Linux → macOS (via osxcross)
```bash
# Complex - use GitHub Actions or native macOS builder instead
```

### x86_64 → ARM64 (Linux)
```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Docker Build

```bash
cd docker

# Build all images
docker compose build

# Build specific service
docker compose build hcs-agent
docker compose build hcs-vision

# Build with no cache
docker compose build --no-cache

# Multi-platform build (requires buildx)
docker buildx build --platform linux/amd64,linux/arm64 -t hcs-agent:latest .
```

## CI/CD Build

### GitHub Actions (`.github/workflows/build.yml`)
```yaml
name: Build
on: [push, pull_request]

jobs:
  build-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build --release --workspace
      - name: Test
        run: cargo test --workspace
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
      - name: Format
        run: cargo fmt --check --workspace

  build-vision:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: '3.11' }
      - name: Install deps
        run: cd vision && pip install -r requirements.txt
      - name: Test
        run: cd vision && pytest tests/

  build-firmware:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install ARM toolchain
        run: sudo apt install -y gcc-arm-none-eabi cmake
      - name: Build
        run: cd hardware && ./scripts/build_all.sh
```

## Troubleshooting

### Rust Build Errors

**OpenSSL linking error:**
```bash
export OPENSSL_DIR=$(brew --prefix openssl)  # macOS
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu  # Linux
export OPENSSL_INCLUDE_DIR=/usr/include/openssl
```

**Protobuf generation failed:**
```bash
# Ensure protoc version matches
protoc --version  # 25+
cargo clean -p hcs-agent-proto
cargo build -p hcs-agent-proto
```

**Windows: "link.exe not found"**
```bash
# Run from Visual Studio Developer Command Prompt
# Or install Build Tools with C++ workload
```

### Python Build Errors

**onnxruntime install fails:**
```bash
# Use pre-built wheels
pip install --only-binary=onnxruntime onnxruntime

# For GPU:
pip install --only-binary=onnxruntime-gpu onnxruntime-gpu
```

**paddlepaddle not available for Python 3.12+:**
```bash
# Use Tesseract instead
# Or use older Python: pyenv install 3.11
```

### Firmware Build Errors

**arm-none-eabi-gcc not found:**
```bash
# Verify installation
arm-none-eabi-gcc --version

# Add to PATH if needed
export PATH="/opt/arm-gnu-toolchain/bin:$PATH"
```

**teensy_loader_cli not found:**
```bash
# Build from source
git clone https://github.com/PaulStoffregen/teensy_loader_cli
cd teensy_loader_cli && make && sudo cp teensy_loader_cli /usr/local/bin/
```

## Verification

### Run Tests
```bash
# Rust tests
cargo test --workspace

# Specific crate
cargo test -p hcs-agent
cargo test -p hcs-driver -- --nocapture

# Python tests
cd vision && pytest tests/ -v

# Integration tests
cargo test --test integration_test
```

### Run Services
```bash
# Agent
./target/release/hcs-agent

# Vision
cd vision && source venv/bin/activate && python -m src.server

# Brain
./target/release/hcs-brain

# Orchestrator
./target/release/hcs-orchestrator --help
```

## Performance Optimization

### Release Profile Optimization
```toml
# Cargo.toml [profile.release]
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### CPU-Specific Optimization
```bash
# Native CPU optimization
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Specific CPU (e.g., for distribution)
RUSTFLAGS="-C target-cpu=skylake" cargo build --release
```

## Next Steps

- [Testing Guide](testing.md)
- [Contributing Guide](contributing.md)
- [Docker Deployment](../installation/docker.md)
- [Configuration Reference](../development/building.md#runtime-configuration)