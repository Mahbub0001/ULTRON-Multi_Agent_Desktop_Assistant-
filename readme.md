# 🤖 ULTRON
### Autonomous Multi-Agent Desktop & Mobile AI Operating System
Powered by **Gemini Live API** • Real-Time Voice, Vision, Multi-Agent Living Office & System Automation

[![Python 3.11+](https://img.shields.io/badge/python-3.11%2B-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-65%20passing-brightgreen.svg)](#-testing--verification)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey.svg)](#-quick-start)

---

## 🌟 Overview

**ULTRON** is a state-of-the-art personal AI operating system designed for true digital autonomy. Powered natively by Google's **Gemini Live API**, ULTRON hears, sees, speaks, plans, and controls your computer and mobile phone in real-time with ultra-low latency.

Unlike traditional voice wrappers, ULTRON features a **specialized resident Agent Department (Living Office)** — autonomous background agents that plan, write code, conduct deep research, execute system commands, and format executive documents independently while you converse naturally with the primary assistant.

Equipped with a **software-rendered holographic 3D avatar with authentic viseme lip-sync**, **wireless Android phone control via ADB**, **native multi-drive file control**, **AI music generation & background playback**, and **zero-subscription privacy**, ULTRON transforms your PC into an intelligent command center.

---

## ✨ Key Highlights

| Feature | Capability |
|---|---|
| 🏢 **Agent Department (Living Office)** | 4 autonomous resident agents (**Alice**, **Bob**, **Carol**, **Dave**) with independent ReAct loops, system tools, and live HUD canvas |
| 🎙️ **Real-Time Gemini Live Engine** | Streaming bi-directional audio conversation with natural interruptions and instant acknowledgment |
| 🧑‍🎤 **Holographic 3D Viseme Avatar** | ~50 mouth shapes/sec derived from acoustic formants and transcripts; software-rendered with 0% GPU overhead |
| 📱 **Wireless Android Phone Control** | Seamless Wi-Fi ADB control: flashlight, apps (WhatsApp, YouTube, Camera), volume, screenshot, and battery |
| 📄 **Executive Document & Word Engine** | Automatic Markdown to `.docx` conversion with 24pt Navy titles, 16pt/13pt headings, bold bullets, and margins |
| 📁 **Universal Multi-Drive File Controller** | Native C:, D:, and E: drive operations, fuzzy path resolution, and seamless application launching (VS Code, etc.) |
| 👁️ **Visual Awareness** | On-demand screen capture and webcam vision piped directly into the multimodal Gemini Live stream |
| 🎵 **Music Playback & AI Song Generation** | Stream original songs in the background via yt-dlp; generate new AI songs with `edge-tts`; full playback controls (pause, resume, stop, volume) |
| 🎚️ **Push-to-Talk & Wake Word** | Global **Ctrl+Space** push-to-talk chord and local offline **"Hey Ultron"** wake-word detection |
| 🔇 **Smart Self-Echo Guard** | Acoustic latency subtraction prevents ULTRON from answering its own voice without muting your microphone |
| 🧠 **Persistent Long-Term Memory** | Infinite storage with token-budgeted prompt indexing and sub-millisecond local recall |
| ↩️ **Universal Undo System** | Instant one-word rollback for file movements, creations, renames, and OS setting changes |
| 📊 **Hardware Telemetry HUD** | Live CPU, RAM, GPU, and temperature telemetry with proactive system health alerts |

---

## 🏢 The Agent Department (Agent Town)

ULTRON houses a dedicated department of specialized autonomous agents who live in the **Living Office** drawer. They run in background threads, execute real system tools, collaborate on subtasks, and persist their work history.

```
                  ┌───────────────────────────────┐
                  │       ULTRON CORE HUD         │
                  │   (Primary Voice & Vision)    │
                  └───────────────┬───────────────┘
                                  │ Intelligent Delegation
       ┌──────────────────────────┼──────────────────────────┐
       ▼                          ▼                          ▼
🔬 ALICE                  💻 BOB                     ⚙️ CAROL
Senior Research Analyst   Lead Software Architect    System Operations
• Deep Web & arXiv        • Python / JS / C++        • Windows Telemetry
• Fact-checking           • Script Automation        • Process Control
• Data Synthesis          • Debugging & Testing      • File Automation
       └──────────────────────────┬──────────────────────────┘
                                  ▼
                            📝 DAVE
                   Communications Specialist
                   • Executive .docx Reports
                   • Technical Documentation
                   • Structured Summaries
```

### Resident Agents Roster:
- 🔬 **Alice (Senior Research Analyst):** Specializes in deep web research, arXiv preprints, technical benchmarks, and evidence-backed synthesis.
- 💻 **Bob (Lead Software Architect & Developer):** Handles code authoring, automation scripts, test suites, architecture design, syntax verification via Python AST, and surgical file editing with a professional 10-iteration ReAct loop.
- ⚙️ **Carol (System Operations Specialist):** Manages Windows OS telemetry, background task monitoring, multi-drive operations, and system automation.
- 📝 **Dave (Creative & Communications Specialist):** Crafts structured Word documents, executive summaries, formatted reports, and documentation.

> 💡 **Intelligent Auto-Delegation:** You don't even need to name an agent! Say *"Research the latest advancements in quantum computing"* and ULTRON automatically routes the task to Alice. Say *"Write a python calculator script"* and Bob immediately gets to work.

---

## 🎵 Music Playback & AI Song Generation

ULTRON has a full-featured music engine with zero browser popups:

| Intent | Voice Command | What Happens |
|---|---|---|
| **Play original song** | *"Tum Hi Ho play karo"* / *"Shape of You banaow"* | yt-dlp streams the original audio in the background silently |
| **Generate AI song** | *"Amar jonno ekta gaan banao"* / *"AI diye gaan create koro"* | edge-tts synthesizes and plays a new original AI song |
| **Pause** | *"Gaan ta pause koro"* | `music_singer(action='pause')` |
| **Resume** | *"Chalao"* / *"Resume koro"* | `music_singer(action='resume')` |
| **Stop** | *"Band koro"* / *"Stop koro"* | `music_singer(action='stop')` |
| **Volume** | *"Volume 60 koro"* | `music_singer(action='volume', level=60)` |

Songs are cached locally in `data/music_cache/` so repeated plays are instant.

---

## 📱 Wireless Android Phone Control

Control your Android smartphone wirelessly over local Wi-Fi without picking it up:

### Setup Guide:
1. **Enable Developer Options:** On your phone, go to **Settings → About Phone** and tap **Build Number** 7 times.
2. **Enable Wireless Debugging:** Go to **Settings → System / Developer Options** and turn ON **Wireless Debugging**.
3. **Note IP & Port:** Tap Wireless Debugging to see your phone's IP (e.g., `192.168.1.105:5555`).
4. **Connect via ADB:**
   ```bash
   adb connect <phone_ip>:5555
   ```
   Accept the "Always allow from this computer" prompt on your phone screen.
5. **Save Phone IP in ULTRON:**
   Open the settings drawer (⚙) → **📱 ANDROID PHONE**, enter your phone IP, and click **SAVE IP**.

### Example Voice Commands:
- *"Phone er flashlight on koro"* / *"Flashlight off koro"*
- *"WhatsApp open koro phone e"* / *"YouTube open koro phone e"*
- *"Phone er volume baraw"* / *"Phone mute koro"*
- *"Phone er battery koto?"*
- *"Phone er screenshot nao"* (saves directly to `screenshots/`)
- *"Phone lock koro"*

---

## 📄 Professional Word & Document Formatting

ULTRON features a built-in document processor powered by `python-docx` and Windows COM:
- **Instant Markdown to Word:** Converts any research summary, paper, or text into a polished `.docx` document with one command.
- **Executive Typography:** Formatted with centered 24pt Navy titles, 16pt / 13pt bold headings, 1.15 line spacing, and native bullet lists with bold lead-in tags (no raw asterisks).
- **Zero Keystroke Flakiness:** Documents are generated programmatically with 100% precision and launched directly into Microsoft Word.

---

## ⚡ Quick Start

### 1. Prerequisites
- **Python:** 3.11, 3.12, 3.13, or 3.14
- **Operating System:** Windows 10/11 (64-bit)
- **Gemini API Key:** Free key from [Google AI Studio](https://aistudio.google.com/app/apikey)
- **Microphone & Speakers**

### 2. Clone and Setup
```bash
git clone https://github.com/Mahbub0001/ULTRON-Multi_Agent_Desktop_Assistant-.git
cd ULTRON-Multi_Agent_Desktop_Assistant-

# Create an isolated Python environment (Windows)
python -m venv .venv

# Run the OS-aware environment installer (auto-installs all dependencies)
.venv\Scripts\python.exe setup.py
```

### 3. Configure API Key
Copy the template and add your Gemini API key:
```bash
cp config/api_keys.example.json config/api_keys.json
```
Edit `config/api_keys.json` with your key:
```json
{
    "gemini_api_key": "YOUR_GEMINI_API_KEY_HERE",
    "os_system": "windows",
    "user_name": "Sir",
    "android_device_ip": ""
}
```

### 4. Launch ULTRON
```bash
.venv\Scripts\python.exe main.py
```

On Windows, `run_jarvis.bat` also uses the project's `.venv` when present.
On macOS/Linux, use `.venv/bin/python` for the commands above.

### Optional Rust subsystem

`human-control-system` is a separate subsystem and is not needed to launch the
Python assistant. Its Windows build needs Rust's MSVC toolchain, Visual Studio
Build Tools with **Desktop development with C++**, a Windows SDK, and `protoc`.
Use its helper to select the Visual Studio linker even when Git's `link.exe`
appears first on PATH:

```powershell
.\human-control-system\cargo-windows.ps1 check --workspace
```

The helper reports missing build prerequisites before invoking Cargo. A complete
workspace build also requires the subsystem's source modules to be present.

---

## 🗂️ Project Architecture

```
ULTRON/
├── main.py                      # Core event loop — Gemini Live session, audio I/O, viseme extraction
├── ui.py                        # PyQt6 HUD — 3D avatar canvas, waveform, log panel, drawer
├── setup.py                     # Cross-platform environment and dependency installer
├── pyproject.toml               # PEP 517 build system metadata
├── requirements.txt             # Pinned dependency list
├── actions/                     # Modular capability tools (self-describing ACTION declarations)
│   ├── agent_town.py            # Agent Department controller & Living Office UI drawer
│   ├── music_singer.py          # Music playback engine & AI song generation (yt-dlp + edge-tts)
│   ├── word_document.py         # Professional Word .docx generator & Markdown formatter
│   ├── file_controller.py       # Multi-drive file operations (C:, D:, E:) with app launcher
│   ├── web_search.py            # Grounded Google Search & DuckDuckGo research engine
│   ├── screen_processor.py      # Real-time screen capture & webcam vision stream
│   ├── system_monitor.py        # CPU, RAM, GPU, temperature telemetry engine
│   ├── email_assistant.py       # IMAP/SMTP email reader, sender, and attachment handler
│   ├── browser_control.py       # Playwright-based headless browser automation
│   ├── open_app.py              # Application launcher (VS Code, Chrome, Spotify, etc.)
│   ├── computer_control.py      # Desktop shortcuts, window management, focus control
│   ├── reminder.py              # Scheduled reminders and alarms
│   ├── youtube_video.py         # YouTube search and stream launcher
│   ├── flight_finder.py         # Flight search and price tracking
│   ├── background_monitor.py    # Proactive system event monitor
│   └── ...                      # Weather, Game Updater, Send Message, etc.
├── core/
│   ├── agent_town.py            # Autonomous ReAct loop engine, tool execution, resident agents
│   ├── prompt.txt               # System persona, dynamic instructions, and capability rules
│   ├── avatar.py                # 3D holographic avatar software renderer (QPainter)
│   ├── avatar_mesh.py           # MediaPipe facial geometry & rig calculations
│   ├── face_model.obj           # Canonical 3D face mesh (MediaPipe)
│   ├── viseme.py                # Dual-mode phonetic mouth shape synthesizer
│   ├── echo.py                  # Self-calibrating acoustic echo cancellation
│   ├── hotkey.py                # Global push-to-talk handler (Ctrl+Space)
│   ├── undo.py                  # Universal action rollback manager
│   ├── confirm.py               # Safety confirmation gate for critical actions
│   └── action_loader.py         # Auto-discovery engine for modular action tools
├── memory/
│   ├── memory_manager.py        # Long-term knowledge graph & persistent facts
│   └── config_manager.py        # Local settings and configuration accessor
├── tests/                       # Comprehensive unit and integration test suite (65 tests)
│   ├── test_agent_town.py       # Autonomous agent ReAct loop tests
│   ├── test_agent_town_developer.py # Bob developer tool and AST validation tests
│   ├── test_music_singer.py     # Music engine, AI song generation, and player tests
│   ├── test_email_assistant.py  # Email send/receive and attachment tests
│   ├── test_word_document.py    # Word formatting and Markdown-to-DOCX tests
│   ├── test_file_controller.py  # Multi-drive file tests
│   ├── test_browser_control.py  # Playwright browser automation tests
│   └── test_live_session.py     # Session resilience and audio tests
└── config/
    ├── api_keys.example.json    # Safe template for credentials
    ├── api_keys.json            # Local API keys and preferences (git-ignored)
    └── agents.json              # Agent persona and system instruction configurations
```

---

## 📦 Dependencies

Key packages (all auto-installed by `setup.py`):

| Package | Purpose |
|---|---|
| `google-genai` | Gemini Live API streaming |
| `PyQt6` | HUD, 3D avatar canvas, UI |
| `yt-dlp` | Background audio streaming (music playback) |
| `edge-tts` | AI song & speech synthesis |
| `miniaudio` | Native low-latency audio player |
| `python-docx` | Word document generation |
| `playwright` | Headless browser automation |
| `duckduckgo-search` | Privacy-friendly web search |
| `mediapipe` | Face mesh for 3D viseme avatar |
| `psutil` / `nvidia-ml-py` | System telemetry (CPU, RAM, GPU) |

---

## 🔒 Privacy & Local Security

- **Zero Remote Telemetry:** ULTRON does not send metrics, logs, or analytics to any third-party server.
- **Encrypted Local Storage:** All memory, conversation summaries, and configuration files live strictly on your local disk (`memory/` and `config/`).
- **No Secret Leaks:** Credentials and TLS keys are strictly git-ignored and guarded by automated security policies.
- **Direct Gemini Connection:** Audio and vision streams travel encrypted directly between your PC and Google's Gemini Live API.

---

## 🧪 Testing & Verification

ULTRON includes a rigorous test suite (65 tests) covering ReAct agent execution, music engine, document formatting, email, file controllers, and audio sessions:

```bash
python -m unittest discover -s tests -p "test_*.py" -v
```

```
Ran 65 tests in 2.29s
OK
```

All tests run cleanly on Windows 10/11 with Python 3.11–3.14.

---

## 👤 Author & Acknowledgments

- **Lead Developer:** [Mahbub (Nibir)](https://github.com/Mahbub0001)
- **Framework & Foundation:** Built upon the MediaPipe face model, PyQt6, and Google Gemini Live API.

⭐ **Star this repository to follow ULTRON's ongoing evolution!**
