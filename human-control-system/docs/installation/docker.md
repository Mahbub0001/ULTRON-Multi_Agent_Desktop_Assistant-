# Docker Deployment Guide

## Overview

This guide covers deploying the Human Control System using Docker and Docker Compose. The deployment includes all core services with proper GPU acceleration, device access, and configuration management.

## Prerequisites

- **Docker Engine:** 24.0+
- **Docker Compose:** 2.20+ (plugin or standalone)
- **NVIDIA Container Toolkit:** For GPU acceleration (vision service)
- **Linux Host:** Required for uinput/Interception driver access

> **Note:** Windows/macOS hosts can run the vision/brain/orchestrator containers, but the agent requires Linux for uinput or Windows for Interception. Use native installation on those platforms.

## 1. Install Docker & NVIDIA Toolkit

### Ubuntu/Debian
```bash
# Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# NVIDIA Container Toolkit
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
    sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
    sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt update
sudo apt install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify
docker run --rm --gpus all nvidia/cuda:12.4-base nvidia-smi
```

### RHEL/Fedora
```bash
# Docker
sudo dnf install -y docker docker-compose-plugin
sudo systemctl enable --now docker
sudo usermod -aG docker $USER

# NVIDIA
sudo dnf config-manager --add-repo https://nvidia.github.io/libnvidia-container/stable/rpm/nvidia-container-toolkit.repo
sudo dnf install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

## 2. Prepare Configuration

```bash
cd human-control-system

# Create config directory
mkdir -p config data logs models wasm-plugins tasks

# Copy example configurations
cp config/agent.toml.example config/agent.toml 2>/dev/null || true
cp config/vision.toml.example config/vision.toml 2>/dev/null || true
cp config/brain.toml.example config/brain.toml 2>/dev/null || true
cp config/adapters.toml.example config/adapters.toml 2>/dev/null || true
cp config/orchestrator.toml.example config/orchestrator.toml 2>/dev/null || true

# Edit configurations as needed
# Important: Set JWT secrets in production!
```

### Required Config Changes for Production

```toml
# config/agent.toml
[auth]
jwt_secret = "your-256-bit-secret-key-here-change-this"  # CHANGE THIS!
token_ttl_seconds = 900
enable_auth = true

[server]
host = "0.0.0.0"
port = 50051
```

```toml
# config/vision.toml
[detection]
device = "tensorrt"  # Use TensorRT for best performance
use_tensorrt = true
```

## 3. Download Models

```bash
# Create models directory
mkdir -p models

# Download YOLOv8n ONNX model
cd models
wget https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx

# For TensorRT: engines will be built on first run
mkdir -p tensorrt_cache

# Download PaddleOCR models (auto-downloaded on first run if configured)
# Or manually:
# wget https://paddleocr.bj.bcebos.com/PaddleOCR/det/ch_ppocr_mobile_v2.0_det_infer.tar
# wget https://paddleocr.bj.bcebos.com/PaddleOCR/rec/ch_ppocr_mobile_v2.0_rec_infer.tar
```

## 4. Docker Compose Services

### docker-compose.yml Overview

```yaml
services:
  hcs-agent:        # Main agent (privileged, host devices)
  hcs-vision:       # Vision inference (GPU, NVIDIA runtime)
  hcs-brain:        # Decision engine (WASM plugins)
  hcs-orchestrator: # CLI/TUI (interactive)
  hcs-dev:          # Development environment
```

### Service Details

#### hcs-agent
- **Privileged:** Yes (needs /dev/uinput, /dev/input, /dev/hidraw)
- **Network:** Host (for low-latency gRPC)
- **PID/IPC:** Host (for process access)
- **Devices:** uinput, input, hidraw
- **Volumes:** config, data, logs, X11 socket

#### hcs-vision
- **Runtime:** nvidia
- **GPUs:** All
- **Network:** Host (for screen capture)
- **Volumes:** config, models, X11 socket

#### hcs-brain
- **Network:** Host
- **Volumes:** config, wasm-plugins, data

#### hcs-orchestrator
- **Profile:** cli (run with `docker compose --profile cli up`)
- **Interactive:** stdin_open, tty
- **Volumes:** config, tasks, history

## 5. Deployment Commands

### Start All Services (Production)
```bash
cd docker
docker compose up -d

# View status
docker compose ps

# View logs
docker compose logs -f hcs-agent
```

### Start Specific Services
```bash
# Agent only
docker compose up -d hcs-agent

# Agent + Vision (common combo)
docker compose up -d hcs-agent hcs-vision

# With Brain
docker compose up -d hcs-agent hcs-vision hcs-brain
```

### Start Interactive Orchestrator
```bash
# Run CLI command
docker compose --profile cli run --rm hcs-orchestrator run tasks/my_task.yaml

# Start REPL
docker compose --profile cli run --rm -it hcs-orchestrator repl

# Start TUI
docker compose --profile cli run --rm -it hcs-orchestrator tui
```

### Development Environment
```bash
# Full dev environment with all tools
docker compose --profile dev up -d hcs-dev

# Enter container
docker exec -it hcs-dev bash

# Inside container: build, test, run
cargo test --workspace
cargo run --release --bin hcs-agent
```

## 6. Health Checks

```bash
# Check all services
docker compose ps

# Individual health checks
grpcurl -plaintext localhost:50051 hcs.agent.v1.SystemService/HealthCheck
grpcurl -plaintext localhost:50052 hcs.vision.v1.VisionService/HealthCheck
grpcurl -plaintext localhost:50053 brain.BrainService/HealthCheck
grpcurl -plaintext localhost:50054 hcs.adapters.v1.AdapterService/HealthCheck

# Or use curl for HTTP health (if enabled)
curl http://localhost:9090/health  # Agent metrics
curl http://localhost:9091/health  # Vision metrics
```

## 7. Monitoring & Logs

### View Logs
```bash
# Follow all logs
docker compose logs -f

# Specific service
docker compose logs -f hcs-vision

# Last 100 lines
docker compose logs --tail=100 hcs-agent

# With timestamps
docker compose logs -t hcs-brain
```

### Access Metrics (Prometheus)
```bash
# Agent metrics
curl http://localhost:9090/metrics

# Vision metrics
curl http://localhost:9091/metrics

# Brain metrics
curl http://localhost:9092/metrics
```

### Structured Logs (JSON)
```bash
# Parse with jq
docker compose logs hcs-agent | jq -r 'select(.level=="ERROR") | .message'
```

## 8. Configuration Management

### Environment Variables
```yaml
# In docker-compose.yml
environment:
  - RUST_LOG=info,hcs_agent=debug
  - HCS_CONFIG=/etc/hcs/agent.toml
```

### Override Config at Runtime
```bash
# Override specific settings
docker compose run --rm -e HCS_AGENT_DRIVER__BACKEND=uinput hcs-agent

# Or mount custom config
docker compose run --rm -v ./my-agent.toml:/etc/hcs/agent.toml:ro hcs-agent
```

### Secrets Management
```bash
# Use Docker secrets (Swarm mode)
echo "super-secret-jwt-key" | docker secret create hcs_jwt_secret -

# In compose (Swarm):
secrets:
  jwt_secret:
    external: true

services:
  hcs-agent:
    secrets:
      - jwt_secret
    environment:
      - HCS_AUTH_JWT_SECRET_FILE=/run/secrets/jwt_secret
```

## 9. Backup & Restore

### Backup Data
```bash
# Backup configuration and data
tar -czf hcs-backup-$(date +%Y%m%d).tar.gz config/ data/ logs/ models/

# Or use Docker volumes
docker run --rm -v hcs-data:/data -v $(pwd):/backup alpine tar czf /backup/data-backup.tar.gz /data
```

### Restore
```bash
tar -xzf hcs-backup-20260101.tar.gz
docker compose down
docker compose up -d
```

## 10. Updates & Maintenance

### Pull Latest Images
```bash
docker compose pull
docker compose up -d
```

### Rebuild from Source
```bash
# Build images locally
docker compose build --no-cache

# Or build specific service
docker compose build --no-cache hcs-vision
```

### Clean Up
```bash
# Stop and remove containers
docker compose down

# Remove volumes (DATA LOSS!)
docker compose down -v

# Remove images
docker compose down --rmi all

# Full cleanup
docker system prune -a --volumes
```

## 11. Troubleshooting

### Agent: uinput Permission Denied
```bash
# Ensure uinput module loaded on host
sudo modprobe uinput
ls -la /dev/uinput

# Check container has device access
docker exec hcs-agent ls -la /dev/uinput
```

### Agent: Interception Driver (Windows Host)
- Run agent natively on Windows host, not in container
- Docker on Windows cannot access Interception driver

### Vision: GPU Not Detected
```bash
# Verify NVIDIA runtime
docker run --rm --gpus all nvidia/cuda:12.4-base nvidia-smi

# Check container GPU access
docker exec hcs-vision nvidia-smi

# Ensure nvidia-container-toolkit installed
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

### Vision: Screen Capture Black
```bash
# Ensure X11 socket mounted
docker exec hcs-vision ls -la /tmp/.X11-unix/

# Check DISPLAY variable
docker exec hcs-vision env | grep DISPLAY

# For headless: use Xvfb
# Add to vision service:
# command: ["xvfb-run", "-a", "python", "-m", "src.server"]
```

### Brain: WASM Plugins Not Loading
```bash
# Check plugin directory mounted
docker exec hcs-brain ls -la /opt/hcs/wasm-plugins/

# Check plugin format (must be .wasm Component Model)
file /opt/hcs/wasm-plugins/*.wasm
```

### Network: Services Can't Communicate
```bash
# Verify host network mode
docker inspect hcs-agent | grep NetworkMode

# Test connectivity between containers
docker exec hcs-agent grpcurl -plaintext hcs-vision:50052 hcs.vision.v1.VisionService/HealthCheck
```

### Out of Memory
```bash
# Check memory limits
docker stats

# Increase limits in docker-compose.yml
deploy:
  resources:
    limits:
      memory: 8G
```

## 12. Production Hardening

### Security Checklist
- [ ] Change default JWT secret
- [ ] Enable mTLS for gRPC (certificates)
- [ ] Restrict network access (firewall)
- [ ] Run containers as non-root where possible
- [ ] Drop unnecessary capabilities
- [ ] Enable read-only root filesystem
- [ ] Scan images for vulnerabilities

### Example Hardened Agent Service
```yaml
hcs-agent:
  # ... existing config ...
  user: "1000:1000"  # Non-root user
  read_only: true
  tmpfs:
    - /tmp
    - /var/run
  cap_drop:
    - ALL
  cap_add:
    - SYS_ADMIN      # Only if needed
    - SYS_RESOURCE
    - DAC_OVERRIDE
  security_opt:
    - no-new-privileges:true
  seccomp: ./seccomp-profile.json
```

### Resource Limits
```yaml
deploy:
  resources:
    limits:
      cpus: '4'
      memory: 4G
    reservations:
      cpus: '2'
      memory: 2G
```

## 13. Kubernetes Deployment (Advanced)

### Helm Chart Structure
```
hcs/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── agent-deployment.yaml
│   ├── vision-deployment.yaml
│   ├── brain-deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   └── pvc.yaml
```

### Key Kubernetes Considerations
- **Agent:** Needs privileged pod, hostNetwork, hostDevices
- **Vision:** Needs NVIDIA GPU resource, nodeSelector for GPU nodes
- **Storage:** PersistentVolumes for config, models, data
- **Networking:** Headless services for gRPC, ingress for external access

## Next Steps

- [Architecture Overview](../architecture.md#deployment-architecture)
- [Configuration Reference](../development/building.md#configuration)
- [Monitoring & Observability](../architecture.md#observability)
- [Development Guide](../development/building.md)