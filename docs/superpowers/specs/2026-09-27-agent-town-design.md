# Agent Town Design Specification for Mark-LIV

## Overview
Agent Town is an autonomous multi-agent subsystem integrated into Mark-LIV, inspired by the "Living Office" concept from Stonic AI. It introduces four specialized resident AI agents with distinct roles, individual memory, real-time visual status, and autonomous execution capabilities.

## Architecture Components

### 1. Configuration & Persistence (`config/agents.json`)
Defines the resident agents, their avatars, roles, system prompts, colors, and assigned abilities:
- **Alice (Research & Intelligence):** Cyan `#00d4ff` — Web research, information extraction, and synthesis.
- **Bob (Development & Engineering):** Emerald `#00ff88` — Software architecture, coding, debugging, and script execution.
- **Carol (System & Operations):** Amber `#ffcc00` — Local OS automation, file operations, app control.
- **Dave (Creative & Communications):** Orange `#ff6b00` — Documentation, reports, message composition, and review.

### 2. Core Engine (`core/agent_town.py`)
- **Agent:** Represents an autonomous worker maintaining state (`IDLE`, `WORKING`, `COMPLETED`, `ERROR`), task queue, conversation history, and latest result.
- **AgentManager:** Singleton manager handling agent registration, thread pool execution, background task dispatching, and event listeners.
- **LLM Execution Pipeline:** Connects to `core/gemini.py` / `core/llm_client.py` using per-agent system instructions. Agent tasks run on background worker threads without freezing the Qt GUI or interrupting Gemini Live audio streams.

### 3. Voice & Action Tool (`actions/agent_town.py`)
- Implements Mark-LIV's auto-discovery `TOOL` protocol.
- Exposes `delegate_agent_task(agent_name, task_description)` to the Gemini Live orchestrator.
- When the user asks Jarvis by voice (e.g., *"Ask Bob to write a python script for system monitoring"*), Jarvis calls `delegate_agent_task`, which dispatches the job to Bob asynchronously and reports an acknowledgment immediately.

### 4. User Interface (`ui.py`)
- **Agent Town Drawer:** A slide-out/modal HUD overlay styled with Mark-LIV's sci-fi aesthetic (`C.BG`, `C.PANEL`, `C.PRI`, etc.).
- **Desk Cards:** Four interactive cards representing Alice, Bob, Carol, and Dave:
  - Agent avatar / icon badge
  - Name and specialty label
  - Live animated status LED (pulsing when `WORKING`)
  - Current task / latest result preview
  - Click-to-dispatch: Dialog to send tasks directly or view full execution logs.
- **Trigger:** Accessible via the HUD header button ("TOWN") and quick drawer.

## Non-Breaking Guarantees
- Zero breaking changes to `main.py`, existing plugins, or Gemini Live streaming loops.
- Thread-safe UI updates using PyQt signals or QTimer polling.
- Graceful fallbacks if API keys are exhausted or network errors occur.
