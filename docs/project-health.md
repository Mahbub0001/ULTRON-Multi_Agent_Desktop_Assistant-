# Project health — 2026-10-03

## Completed repairs

- Python package metadata now uses the actual `readme.md` filename.
- The setup installer resolves `requirements.txt` relative to its own file.
- The Windows launcher prefers `.venv` and resolves the project from the launcher
  location instead of a hard-coded drive path.
- OAuth token and client-secret ignore patterns work without trailing comments.
  Rust build output, pytest cache, and the downloaded Rust installer are ignored.
- Cargo development/release profiles are defined at the workspace root.
- `human-control-system/cargo-windows.ps1` initializes the Visual Studio x64
  environment and explicitly selects its MSVC linker, avoiding Git's `link.exe`.
- The orchestrator build uses tonic-build 0.10's `compile` API, correct sibling
  paths, and Cargo's output directory, matching `include_proto!`.
- Protobuf schemas no longer contain unsupported `rust_module` options, missing
  imports/types, duplicate field numbers, or invalid field terminators. Existing
  non-conflicting field numbers were preserved; conflicting fields received
  unused numbers. Windows show-command aliases are explicitly allowed.

## Local environment

The project's `.venv` contains the requirements and pytest. Visual Studio C++
Build Tools and a Windows SDK were installed successfully. Playwright's Chromium
and Firefox installation completed successfully. These installations are local
and are not committed dependencies.

Launch the Python assistant with `run_jarvis.bat` or:

```powershell
.venv\Scripts\python.exe main.py
```

## Validation

- 53 selected Python tests passed in `.venv`: setup, live-session state, agent UI,
  town canvas, memory, agent tools, browser URL handling, mocked email operations,
  action integration, and file path validation.
- Python source parsing passed for the inspected app and test files.
- All 24 discovered action declarations validated.
- `pip check` reported no broken requirements.
- Cargo manifest metadata validation passed without the former profile warnings.
- All five protobuf schemas compiled together with `protoc`.
- `git diff --check` passed.

Live Gemini calls, microphone/speaker operation, desktop input injection, and
physical hardware were not exercised. The selected tests do not establish that
every feature works end to end.

## Remaining Rust implementation work

The complete Rust workspace **does not build yet**. Toolchain and protobuf
generation now work, exposing existing source-level failures. The latest check
reported 17 driver, 34 brain, and 50 adapter compiler diagnostics; some are
cascading failures from missing module declarations.

- Driver: missing `platform/common.rs` and `platform/windows.rs`, Linux-only code
  compiled on Windows, unresolved dependencies, and invalid union derivation.
- Brain: missing module entrypoints, mismatched types, WASM API incompatibilities,
  and thread-safety errors.
- Adapters: missing module entrypoints, unresolved imports/types, and platform API
  mismatches. The orchestrator may expose further errors after these are fixed.

Reproduce the current check from the project root:

```powershell
.\human-control-system\cargo-windows.ps1 check --workspace --offline
```

The local diagnostic log is `scratch/cargo-check.log` (ignored by Git). Completing
these subsystem implementations is additional work beyond the initial config and
environment repairs; this report does not claim the entire project is fixed.
