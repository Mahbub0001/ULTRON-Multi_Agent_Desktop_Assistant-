# Testing Guide

## Overview

This guide covers the testing strategy, tools, and procedures for the Human Control System.

## Test Organization

```
tests/
├── unit/                    # Unit tests (co-located with source)
├── integration/             # Cross-crate integration tests
├── e2e/                     # End-to-end tests
├── fixtures/                # Test data files
└── mocks/                   # Mock implementations

Each Rust crate:
├── src/
│   ├── module.rs
│   └── module_test.rs       # Or #[cfg(test)] mod tests in module.rs
└── tests/                   # Integration tests for this crate

Vision (Python):
├── tests/
│   ├── test_capture.py
│   ├── test_detection.py
│   ├── test_ocr.py
│   ├── test_service.py
│   └── conftest.py
```

## Running Tests

### All Tests
```bash
# Rust workspace tests
cargo test --workspace

# With output
cargo test --workspace -- --nocapture

# Specific crate
cargo test -p hcs-agent
cargo test -p hcs-driver

# Vision tests
cd vision
source venv/bin/activate
pytest tests/ -v

# All tests including integration
cargo test --workspace --all-targets
```

### Test Categories

#### Unit Tests
```bash
# Run only unit tests (fast)
cargo test --workspace --lib

# Specific module
cargo test -p hcs-driver interception::tests
```

#### Integration Tests
```bash
# Run integration tests
cargo test --workspace --test integration_test

# With specific features
cargo test --workspace --test integration_test --features "test_utils"
```

#### End-to-End Tests
```bash
# Requires running services
cd tests/e2e
./run_e2e_tests.sh

# Or with Docker
docker compose -f docker-compose.test.yml up --abort-on-container-exit
```

### Filter Tests
```bash
# By name pattern
cargo test --workspace keyboard
cargo test --workspace "test_inject"

# By test type
cargo test --workspace --lib unit_
cargo test --workspace --test integration_

# Exclude patterns
cargo test --workspace -- --skip slow
cargo test --workspace -- --skip "test_*_gpu"
```

## Test Infrastructure

### Mock Drivers

```rust
// driver/src/synthetic.rs
pub struct SyntheticDriver {
    events: Arc<Mutex<Vec<InjectedEvent>>>,
}

impl SyntheticDriver {
    pub fn new() -> Self {
        Self { events: Arc::new(Mutex::new(Vec::new())) }
    }
    
    pub fn get_events(&self) -> Vec<InjectedEvent> {
        self.events.lock().clone()
    }
    
    pub fn clear(&self) {
        self.events.lock().clear();
    }
}

#[async_trait]
impl InputDriver for SyntheticDriver {
    async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        self.events.lock().push(InjectedEvent::Keyboard(event));
        Ok(())
    }
    // ...
}
```

### Test Fixtures

```rust
// tests/fixtures/mod.rs
pub fn sample_key_events() -> Vec<InputEvent> {
    vec![
        InputEvent::Keyboard(KeyboardEvent { code: 0x41, state: KeyState::Down, ..Default::default() }),
        InputEvent::Keyboard(KeyboardEvent { code: 0x41, state: KeyState::Up, ..Default::default() }),
    ]
}

pub fn sample_mouse_events() -> Vec<InputEvent> {
    vec![
        InputEvent::Mouse(MouseEvent { x: 100, y: 200, button: Some(MouseButton::Left), button_state: Some(KeyState::Down), ..Default::default() }),
    ]
}
```

### Test Utilities

```rust
// tests/utils/mod.rs
use hcs_agent_proto::agent_service_client::AgentServiceClient;
use tonic::transport::Channel;

pub async fn create_test_client() -> AgentServiceClient<Channel> {
    AgentServiceClient::connect("http://localhost:50051").await.unwrap()
}

pub async fn get_test_token(client: &mut AgentServiceClient<Channel>) -> String {
    let resp = client.issue_token(TokenRequest {
        client_id: "test".into(),
        capabilities: vec![Capability { resource: "input".into(), actions: vec![Action::Write as i32] }],
        ttl_seconds: 3600,
    }).await.unwrap();
    resp.into_inner().token
}
```

## Writing Tests

### Rust Unit Test Example

```rust
// driver/src/interception.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyEvent, KeyState, MouseEvent, MouseButton};
    
    #[test]
    fn test_virtual_key_to_linux() {
        let driver = UInputDriver::new(Default::default()).await.unwrap();
        
        // Test common keys
        assert_eq!(driver.virtual_key_to_linux(0x41), KEY_A);  // 'A'
        assert_eq!(driver.virtual_key_to_linux(0x20), KEY_SPACE);
        assert_eq!(driver.virtual_key_to_linux(0x0D), KEY_ENTER);
        assert_eq!(driver.virtual_key_to_linux(0x1B), KEY_ESC);
        
        // Test modifiers
        assert_eq!(driver.virtual_key_to_linux(0x10), KEY_LEFTSHIFT);
        assert_eq!(driver.virtual_key_to_linux(0x11), KEY_LEFTCTRL);
        assert_eq!(driver.virtual_key_to_linux(0x12), KEY_LEFTALT);
        assert_eq!(driver.virtual_key_to_linux(0x5B), KEY_LEFTMETA);
    }
    
    #[test]
    fn test_mouse_button_mapping() {
        let driver = UInputDriver::new(Default::default()).await.unwrap();
        
        // These would be tested via inject_mouse mock
        // Verify BTN_LEFT = 0x110, BTN_RIGHT = 0x111, etc.
    }
}
```

### Rust Integration Test Example

```rust
// agent/tests/integration_test.rs
use hcs_agent_proto::{
    agent_service_client::AgentServiceClient,
    input_service_client::InputServiceClient,
    auth_service_client::AuthServiceClient,
    TokenRequest, Capability, Action, KeyEvent, KeyState,
};
use tonic::{Request, metadata::MetadataValue};
use std::str::FromStr;

#[tokio::test]
async fn test_full_input_flow() {
    // Start agent (or connect to running)
    let mut client = AgentServiceClient::connect("http://localhost:50051").await.unwrap();
    
    // Get auth token
    let mut auth_client = AuthServiceClient::new(client.channel().clone());
    let token_resp = auth_client.issue_token(Request::new(TokenRequest {
        client_id: "integration_test".into(),
        capabilities: vec![Capability { 
            resource: "input".into(), 
            actions: vec![Action::Write as i32, Action::Execute as i32] 
        }],
        ttl_seconds: 3600,
    })).await.unwrap();
    let token = token_resp.into_inner().token;
    
    // Inject key
    let mut input_client = InputServiceClient::new(client.channel().clone());
    let mut req = Request::new(KeyEvent {
        code: 0x41,  // 'A'
        state: KeyState::Down as i32,
        ..Default::default()
    });
    req.metadata_mut().insert("authorization", MetadataValue::from_str(&format!("Bearer {}", token)).unwrap());
    
    let resp = input_client.inject_key(req).await;
    assert!(resp.is_ok());
    
    // Verify via driver info
    let driver_info = client.get_driver_info(Request::new(Empty)).await.unwrap();
    assert!(driver_info.get_ref().ready);
}
```

### Python Unit Test Example

```python
# vision/tests/test_detection.py
import pytest
import numpy as np
from unittest.mock import Mock, patch
from src.detection.yolo import YOLODetector
from src.config import DetectionConfig

@pytest.fixture
def detector():
    config = DetectionConfig(
        model_path="models/yolov8n.onnx",
        device="cpu",
        confidence=0.5,
        iou=0.45,
        input_width=640,
        input_height=640
    )
    with patch('onnxruntime.InferenceSession') as mock_session:
        mock_session.return_value.run.return_value = [
            np.array([[[100, 100, 200, 200, 0.9, 0]]])  # x, y, w, h, conf, class
        ]
        yield YOLODetector(config)

def test_detect_objects(detector):
    # Create test image
    image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)
    
    detections = detector.detect(image, confidence_threshold=0.5)
    
    assert len(detections) == 1
    assert detections[0].class_id == 0
    assert detections[0].confidence == 0.9
    assert detections[0].bbox.x == 100
    assert detections[0].bbox.y == 100

def test_detect_empty(detector):
    with patch('onnxruntime.InferenceSession') as mock_session:
        mock_session.return_value.run.return_value = [np.array([[]])]
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)
        detections = detector.detect(image)
        assert len(detections) == 0
```

### Python Service Test Example

```python
# vision/tests/test_service.py
import pytest
import grpc
from unittest.mock import Mock, AsyncMock
from src.service.vision_service import VisionService
from src.proto import vision_pb2, vision_pb2_grpc

@pytest.fixture
def service():
    with patch('src.service.vision_service.ScreenCapture') as mock_capture, \
         patch('src.service.vision_service.YOLODetector') as mock_detector, \
         patch('src.service.vision_service.OCREngine') as mock_ocr:
        service = VisionService(config={})
        service.capture = mock_capture
        service.detector = mock_detector
        service.ocr = mock_ocr
        yield service

@pytest.mark.asyncio
async def test_capture_screen(service):
    # Mock capture
    mock_frame = Mock()
    mock_frame.image_data = b"fake_image_data"
    mock_frame.width = 1920
    mock_frame.height = 1080
    mock_frame.format = "BGR"
    service.capture.capture.return_value = mock_frame
    
    request = vision_pb2.CaptureRequest(monitor_index=0)
    context = Mock()
    
    response = await service.CaptureScreen(request, context)
    
    assert response.width == 1920
    assert response.height == 1080
    assert response.image_data == b"fake_image_data"
    service.capture.capture.assert_called_once_with(0, None, True, "BGR")
```

## Test Data Management

### Golden Files
```bash
# Store expected outputs
tests/fixtures/
├── detection/
│   ├── image1.jpg
│   └── image1_detections.json
├── ocr/
│   ├── text_image.png
│   └── text_image_ocr.json
└── capture/
    └── test_monitor_config.json
```

### Test Models
```bash
# Small test models for CI
models/test/
├── yolov8n_test.onnx      # Tiny model for testing
├── paddleocr_det_test/    # Minimal detection model
└── paddleocr_rec_test/    # Minimal recognition model
```

## Continuous Integration

### GitHub Actions Test Workflow

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]

jobs:
  rust-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Format check
        run: cargo fmt --check --workspace
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
      - name: Unit tests
        run: cargo test --workspace --lib
      - name: Integration tests
        run: cargo test --workspace --test integration_test
  
  python-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: '3.11' }
      - name: Install deps
        run: |
          cd vision
          pip install -r requirements.txt
          pip install pytest pytest-asyncio pytest-mock
      - name: Test
        run: cd vision && pytest tests/ -v --cov=src --cov-report=xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./vision/coverage.xml
  
  firmware-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install ARM toolchain
        run: sudo apt install -y gcc-arm-none-eabi cmake
      - name: Build firmware
        run: cd hardware && ./scripts/build_all.sh
      - name: Check formatting
        run: cd hardware && clang-format --dry-run --Werror firmware/**/*.c firmware/**/*.h
      - name: Static analysis
        run: cd hardware && cppcheck --enable=all --std=c99 firmware/
```

### Pre-commit Hooks

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/doublify/pre-commit-rust
    rev: v1.0
    hooks:
      - id: fmt
      - id: clippy
      - id: cargo-check
  
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-toml
  
  - repo: https://github.com/psf/black
    rev: 23.12.0
    hooks:
      - id: black
        args: [--check]
  
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.1.0
    hooks:
      - id: ruff
        args: [--check]
```

## Coverage Requirements

| Component | Minimum Coverage |
|-----------|-----------------|
| Driver crate | 85% |
| Agent crate | 80% |
| Brain crate | 80% |
| Adapters crate | 75% |
| Orchestrator crate | 80% |
| Vision service | 80% |

### Generate Coverage Reports

```bash
# Rust (requires llvm-tools)
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Generate HTML report
cargo llvm-cov --workspace --html --output-dir coverage/html

# Generate lcov for CI
cargo llvm-cov --workspace --lcov --output-path coverage/lcov.info

# Python
cd vision
pytest --cov=src --cov-report=html --cov-report=xml
```

## Performance Testing

### Benchmark Tests
```rust
// benches/input_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hcs_driver::{InputController, DriverConfig};

fn benchmark_key_injection(c: &mut Criterion) {
    let mut controller = InputController::new(DriverConfig::default()).await.unwrap();
    controller.initialize().await.unwrap();
    
    c.bench_function("inject_key", |b| {
        b.iter(|| {
            controller.inject_key(black_box(KeyEvent {
                code: 0x41,
                state: KeyState::Down,
                ..Default::default()
            })).await.unwrap();
        })
    });
}

criterion_group!(benches, benchmark_key_injection);
criterion_main!(benches);
```

```bash
# Run benchmarks
cargo bench --bench input_benchmark
```

### Load Testing
```bash
# Test with multiple concurrent clients
cd tests/load
./run_load_test.sh --clients 10 --duration 60 --rate 100
```

## Hardware-in-the-Loop Testing

### Teensy Test Rig
```bash
# Connect two Teensys: one as DUT, one as test controller
# DUT runs HCS firmware
# Test controller sends commands, verifies responses

# Automated test
cd hardware/tests
./run_hil_test.sh --dut /dev/ttyACM0 --controller /dev/ttyACM1
```

### Test Scenarios
1. **Latency test:** Measure round-trip time for key tap
2. **Throughput test:** Send 1000 key events, measure drops
3. **Reliability test:** Run for 24 hours, check for crashes
4. **Protocol test:** Verify all commands, ACK/NACK handling

## Debugging Failed Tests

### Common Issues

**Flaky tests:**
```bash
# Run multiple times
for i in {1..10}; do cargo test test_name; done

# Use --test-threads=1 for ordering issues
cargo test --test-threads=1
```

**Integration test fails - service not running:**
```bash
# Start services in background
cargo run --release -p hcs-agent &
AGENT_PID=$!
cargo test --test integration_test
kill $AGENT_PID
```

**Python test import errors:**
```bash
# Ensure PYTHONPATH
export PYTHONPATH="${PWD}/vision/src:${PYTHONPATH}"
cd vision && pytest tests/
```

### Test Logs
```bash
# Enable debug logging for tests
RUST_LOG=debug cargo test test_name -- --nocapture

# Python
pytest tests/test_detection.py -v -s --log-cli-level=DEBUG
```

## Test Maintenance

### Adding New Tests
1. Place unit tests in same file as code (`#[cfg(test)] mod tests`)
2. Place integration tests in `tests/` directory at crate root
3. Use descriptive names: `test_<function>_<scenario>_<expected>`
4. Follow AAA pattern: Arrange, Act, Assert
5. Clean up resources in `Drop` or `finally` blocks

### Updating Golden Files
```bash
# Regenerate expected outputs
cargo test --test golden_test -- --update-snapshots

# Or manually
python -m tests.update_golden_files
```

## Next Steps

- [Building Guide](building.md)
- [Contributing Guide](contributing.md)
- [Architecture Overview](../architecture.md)