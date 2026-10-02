# Human Control System - Implementation Plan

## Project Overview
Build a production-ready human-level input control system with:
- Kernel-level input drivers (Windows Interception, Linux uinput, HID hardware)
- gRPC agent daemon with capability-based auth
- Computer vision pipeline (ONNX YOLO + OCR)
- Decision engine (Behavior Tree + WASM plugins)
- App adapters (Photoshop, Chrome, Games)
- Orchestrator CLI/TUI with Task DSL
- Hardware firmware (Teensy/QMK)

---

## Task 1: Project Structure & Core Configuration ✅ COMPLETED
**Status:** Done
- Cargo.toml workspace with all dependencies
- Docker build files (Dockerfile.build, Dockerfile.vision, Dockerfile.dev, docker-compose.yml)
- Entry point scripts
- Directory structure

---

## Task 2: Kernel Input Driver ✅ COMPLETED
**Status:** Done
- Driver trait abstraction (`InputDriver`)
- Windows Interception driver implementation
- Linux uinput driver implementation
- HID driver for hardware injection (Arduino/Teensy)
- Synthetic driver for testing
- Input macros (shortcuts, gaming, text, mouse)
- Driver factory with auto-detection
- High-level `InputController` with human-like timing

---

## Task 3: Agent Daemon (gRPC + Auth) ✅ COMPLETED
**Status:** Done
- Protobuf definitions for all services
- Configuration system (TOML + env)
- JWT-based authentication with capabilities
- Driver manager with macro registry
- Task manager with async execution
- gRPC services: Input, Macro, Task, Auth, System

---

## Task 4: Computer Vision Pipeline
**Status:** In Progress
**Priority:** High

### Requirements
1. **Screen Capture**
   - Cross-platform: Windows (DXGI/Duplication API), Linux (X11/Wayland), macOS (CoreGraphics)
   - High-performance: <5ms capture latency
   - Multiple monitor support
   - Region/Window capture

2. **Object Detection (YOLOv8)**
   - ONNX Runtime with TensorRT acceleration
   - Configurable model paths
   - Batch inference support
   - Class filtering and confidence thresholds

3. **OCR (PaddleOCR / Tesseract)**
   - Text detection + recognition
   - Multi-language support
   - Region-of-interest OCR
   - Confidence scoring

4. **gRPC Vision Service**
   - `CaptureScreen` - single frame capture
   - `DetectObjects` - YOLO inference
   - `RecognizeText` - OCR on region
   - `StreamFrames` - continuous capture stream
   - Health check endpoint

5. **Configuration**
   - Model paths, device (CPU/CUDA/TensorRT)
   - Capture settings (fps, resolution, monitors)
   - Inference settings (batch size, thresholds)

### Files to Create
```
vision/
├── Cargo.toml (if Rust wrapper) / requirements.txt (Python)
├── src/
│   ├── main.rs / server.py
│   ├── capture/
│   │   ├── mod.rs
│   │   ├── windows.rs
│   │   ├── linux.rs
│   │   └── macos.rs
│   ├── detection/
│   │   ├── mod.rs
│   │   ├── yolo.rs
│   │   └── onnx_runtime.rs
│   ├── ocr/
│   │   ├── mod.rs
│   │   ├── paddle.rs
│   │   └── tesseract.rs
│   ├── proto/
│   │   └── vision.proto
│   └── service/
│       └── vision_service.rs
├── models/ (directory for ONNX models)
└── config/
    └── vision.toml
```

### Acceptance Criteria
- [ ] Screen capture works on Windows/Linux
- [ ] YOLOv8 detection returns bounding boxes with confidence
- [ ] OCR extracts text from image regions
- [ ] gRPC service responds to all endpoints
- [ ] TensorRT acceleration works on NVIDIA GPU
- [ ] Configurable via TOML
- [ ] Unit tests for each component
- [ ] Integration test with mock frames

---

## Task 5: Decision Engine (Behavior Tree + WASM)
**Status:** Pending
**Priority:** High

### Requirements
1. **Behavior Tree Engine**
   - Node types: Sequence, Selector, Parallel, Decorator, Action, Condition
   - Blackboard for shared state
   - Tree serialization (JSON/YAML)
   - Visual debugger support

2. **WASM Plugin System**
   - Wasmtime runtime with component model
   - Plugin interface: `init`, `tick`, `shutdown`
   - Sandboxed execution (fuel limits, memory limits)
   - Hot-reload support

3. **Decision Service**
   - gRPC service for tree execution
   - State persistence
   - Metrics collection

### Files to Create
```
brain/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── bt/
│   │   ├── mod.rs
│   │   ├── node.rs
│   │   ├── composite.rs
│   │   ├── decorator.rs
│   │   ├── action.rs
│   │   ├── condition.rs
│   │   ├── blackboard.rs
│   │   └── executor.rs
│   ├── wasm/
│   │   ├── mod.rs
│   │   ├── runtime.rs
│   │   ├── plugin_trait.rs
│   │   └── host_functions.rs
│   ├── proto/
│   │   └── brain.proto
│   └── service/
│       └── brain_service.rs
├── plugins/ (WASM plugins directory)
└── config/
    └── brain.toml
```

### Acceptance Criteria
- [ ] Behavior tree executes correctly (sequence, selector, parallel)
- [ ] Blackboard reads/writes work
- [ ] WASM plugins load and execute
- [ ] Fuel/memory limits enforced
- [ ] gRPC service runs trees
- [ ] Tree visualization exports DOT/JSON

---

## Task 6: App Adapters
**Status:** Pending
**Priority:** High

### Requirements
1. **Photoshop Adapter (UXP/CEP)**
   - Document manipulation (layers, filters, adjustments)
   - Batch processing
   - Action playback
   - Export/Import

2. **Chrome Adapter (CDP)**
   - Tab management
   - DOM interaction
   - Network interception
   - Console evaluation

3. **Game Memory Adapter**
   - Process attachment
   - Memory reading (pointer chains, offsets)
   - Pattern scanning
   - Code injection (detours)

4. **Window Manager Adapter**
   - Enum windows
   - Focus/Activate
   - Position/Size
   - Screenshot

### Files to Create
```
adapters/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── photoshop/
│   │   ├── mod.rs
│   │   ├── uxp.rs
│   │   ├── cep.rs
│   │   └── api.rs
│   ├── chrome/
│   │   ├── mod.rs
│   │   ├── cdp.rs
│   │   └── session.rs
│   ├── game/
│   │   ├── mod.rs
│   │   ├── memory.rs
│   │   ├── scanner.rs
│   │   └── injector.rs
│   ├── window/
│   │   ├── mod.rs
│   │   ├── windows.rs
│   │   └── linux.rs
│   ├── proto/
│   │   └── adapters.proto
│   └── service/
│       └── adapter_service.rs
└── config/
    └── adapters.toml
```

### Acceptance Criteria
- [ ] Photoshop: execute action, modify layer, export
- [ ] Chrome: navigate, click, extract text, screenshot
- [ ] Game: read memory address, write value, scan pattern
- [ ] Window: list, focus, move, resize, capture
- [ ] All adapters expose gRPC service

---

## Task 7: Orchestrator CLI/TUI + Task DSL
**Status:** Pending
**Priority:** High

### Requirements
1. **Task DSL (YAML/Starlark)**
   - Sequential/Parallel steps
   - Conditionals, loops
   - Variables and templates
   - Import/include

2. **CLI Commands**
   - `hcs run <task.yaml>` - execute task
   - `hcs repl` - interactive REPL
   - `hcs macro <name>` - run macro
   - `hcs record` - record input sequence
   - `hcs replay <file>` - replay recording

3. **TUI (Ratatui)**
   - Dashboard with live stats
   - Task builder/editor
   - Macro manager
   - Device monitor
   - Log viewer

4. **Recording/Replay**
   - Input event recording with timestamps
   - Variable-speed replay
   - Edit recordings

### Files to Create
```
orchestrator/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── commands.rs
│   │   └── args.rs
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs
│   │   ├── dashboard.rs
│   │   ├── task_editor.rs
│   │   ├── macro_manager.rs
│   │   └── device_monitor.rs
│   ├── dsl/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   ├── interpreter.rs
│   │   ├── stdlib.rs
│   │   └── types.rs
│   ├── recorder/
│   │   ├── mod.rs
│   │   ├── capture.rs
│   │   └── replay.rs
│   ├── proto/
│   │   └── orchestrator.proto
│   └── client/
│       └── grpc_client.rs
└── config/
    └── orchestrator.toml
```

### Acceptance Criteria
- [ ] CLI runs task files
- [ ] TUI launches and shows dashboard
- [ ] DSL parses and executes tasks
- [ ] Recording captures input events
- [ ] Replay reproduces recorded actions
- [ ] gRPC client connects to agent

---

## Task 8: Hardware Firmware (Teensy/QMK)
**Status:** Pending
**Priority:** Medium

### Requirements
1. **QMK-based Firmware**
   - HID keyboard + mouse + consumer + system
   - NKRO support
   - Raw HID for custom reports
   - Bootloader support

2. **Custom Protocol**
   - Binary protocol over USB Serial/Raw HID
   - Commands: key_down, key_up, mouse_move, mouse_click, etc.
   - Acknowledgment protocol
   - Heartbeat/keepalive

3. **Build System**
   - Makefile/CMake
   - CI/CD for firmware builds
   - DFU flashing script

### Files to Create
```
hardware/
├── Makefile
├── CMakeLists.txt
├── firmware/
│   ├── config.h
│   ├── hid_reports.h
│   ├── protocol.h
│   ├── main.c
│   ├── usb/
│   │   ├── descriptors.c
│   │   └── callbacks.c
│   ├── protocol/
│   │   ├── parser.c
│   │   ├── handler.c
│   │   └── ack.c
│   └── input/
│       ├── keyboard.c
│       ├── mouse.c
│       └── consumer.c
├── scripts/
│   ├── flash.sh
│   └── build_all.sh
└── rules.mk
```

### Acceptance Criteria
- [ ] Firmware compiles for Teensy 4.0/4.1
- [ ] HID reports work (keyboard, mouse, consumer)
- [ ] Custom protocol handles commands
- [ ] Acknowledgment protocol works
- [ ] Flash script works
- [ ] Latency < 1ms

---

## Task 9: Documentation & Build Guides
**Status:** Pending
**Priority:** High

### Requirements
1. **Architecture Documentation**
   - System overview diagram
   - Component interactions
   - Data flow

2. **Installation Guides**
   - Windows: Interception driver install
   - Linux: uinput permissions, groups
   - Hardware: Teensy flashing
   - Docker deployment

3. **API Documentation**
   - gRPC service references
   - Protobuf messages
   - Client examples (Python, Rust, Go)

4. **Task DSL Reference**
   - Syntax guide
   - Built-in functions
   - Examples

5. **Development Guide**
   - Building from source
   - Running tests
   - Contributing

### Files to Create
```
docs/
├── architecture.md
├── installation/
│   ├── windows.md
│   ├── linux.md
│   ├── macos.md
│   └── hardware.md
├── api/
│   ├── agent.md
│   ├── vision.md
│   ├── brain.md
│   └── adapters.md
├── dsl/
│   ├── reference.md
│   └── examples.md
├── development/
│   ├── building.md
│   ├── testing.md
│   └── contributing.md
└── images/
    └── architecture.svg
```

### Acceptance Criteria
- [ ] All installation paths documented
- [ ] API references complete
- [ ] DSL reference with examples
- [ ] Architecture diagram rendered
- [ ] Build passes in CI

---

## Global Constraints

1. **Code Quality**
   - Rust: `cargo clippy -- -D warnings`, `cargo fmt --check`
   - Python: `ruff check`, `black --check`, `mypy`
   - Tests: >80% coverage

2. **Security**
   - Capability-based auth on all gRPC services
   - No plaintext secrets
   - Input validation on all boundaries

3. **Performance**
   - Input latency < 1ms (driver)
   - Screen capture < 5ms
   - Inference < 30ms (GPU)

4. **Portability**
   - Windows 10+, Linux kernel 5.10+, macOS 12+
   - x86_64 and ARM64

5. **Observability**
   - Structured logging (JSON)
   - Prometheus metrics
   - Health endpoints