# Agent Town Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an autonomous multi-agent subsystem ("Agent Town") in Mark-LIV featuring 4 resident AI agents (Alice, Bob, Carol, Dave) with background execution, voice tool delegation, and a sci-fi HUD drawer interface.

**Architecture:** A configuration file (`config/agents.json`) defines agent personas and capabilities; `core/agent_town.py` provides thread-safe agent state tracking and background task execution via Gemini; `actions/agent_town.py` exposes a Gemini tool for voice delegation; and `ui.py` renders an interactive Agent Town drawer with sci-fi desk cards and live status indicators.

**Tech Stack:** Python 3.10+, PyQt6, threading, Gemini API (`core.gemini` / `core.llm_client`), pytest.

## Global Constraints
- Must not break existing Gemini Live audio streaming loops or main window functionality.
- Background tasks must run asynchronously without blocking the Qt GUI thread.
- Follow Mark-LIV's sci-fi color palette (`C.BG`, `C.PANEL`, `C.PRI`, `C.BORDER`, etc.).
- Auto-discovery compatibility: `actions/agent_town.py` must export a valid `TOOL` dict.

---

### Task 1: Agent Configuration & Agent Town Core Engine

**Files:**
- Create: `config/agents.json`
- Create: `core/agent_town.py`
- Test: `tests/test_agent_town.py`

**Interfaces:**
- Consumes: `core.gemini` (for LLM calling), `config/agents.json` (for resident agent settings)
- Produces: `AgentState`, `ResidentAgent`, `AgentTownManager` (singleton with `dispatch_task`, `get_agent`, `get_all_agents`, `register_listener`)

- [ ] **Step 1: Write the failing unit tests for `AgentTownManager`**

```python
# tests/test_agent_town.py
import pytest
from core.agent_town import AgentTownManager, AgentState

def test_load_default_agents():
    mgr = AgentTownManager.get_instance()
    agents = mgr.get_all_agents()
    assert len(agents) == 4
    names = [a.name for a in agents]
    assert "Alice" in names
    assert "Bob" in names
    assert "Carol" in names
    assert "Dave" in names
    assert mgr.get_agent("Alice").state == AgentState.IDLE

def test_dispatch_task_sync_mock(monkeypatch):
    mgr = AgentTownManager.get_instance()
    
    # Mock LLM execution to avoid live API calls during unit test
    def mock_call(contents, tier="fast", timeout_ms=30000):
        return "Mock research report from agent."
        
    import core.gemini as gemini
    monkeypatch.setattr(gemini, "call", mock_call)
    
    finished = []
    def on_complete(agent, result):
        finished.append((agent.name, result))
        
    mgr.dispatch_task("Alice", "Find recent AI agent news", on_complete=on_complete, async_exec=False)
    
    alice = mgr.get_agent("Alice")
    assert alice.state == AgentState.COMPLETED
    assert alice.latest_result == "Mock research report from agent."
    assert len(finished) == 1
    assert finished[0][0] == "Alice"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest tests/test_agent_town.py -v`
Expected: FAIL (ModuleNotFoundError: No module named 'core.agent_town')

- [ ] **Step 3: Implement `config/agents.json` and `core/agent_town.py`**

Create `config/agents.json` with the 4 resident agents:
```json
{
  "agents": [
    {
      "id": "alice",
      "name": "Alice",
      "role": "Senior Research Analyst",
      "specialty": "Deep web research, fact-checking, and data synthesis",
      "color": "#00d4ff",
      "avatar_symbol": "🔬",
      "system_instruction": "You are Alice, the Senior Research Analyst in Mark-LIV Agent Town. You specialize in deep research, data synthesis, fact-checking, and structured summaries."
    },
    {
      "id": "bob",
      "name": "Bob",
      "role": "Software Architect & Developer",
      "specialty": "Coding, debugging, architecture design, and automation scripts",
      "color": "#00ff88",
      "avatar_symbol": "💻",
      "system_instruction": "You are Bob, the Lead Software Developer in Mark-LIV Agent Town. You specialize in writing clean code, reviewing architectures, debugging errors, and creating automation scripts."
    },
    {
      "id": "carol",
      "name": "Carol",
      "role": "System Ops & Automator",
      "specialty": "PC operations, process management, file workflows, and system health",
      "color": "#ffcc00",
      "avatar_symbol": "⚙️",
      "system_instruction": "You are Carol, the System Operations Specialist in Mark-LIV Agent Town. You specialize in local PC workflows, file management, system health, and desktop operations."
    },
    {
      "id": "dave",
      "name": "Dave",
      "role": "Creative & Communications Specialist",
      "specialty": "Documentation, executive reports, email drafts, and content review",
      "color": "#ff6b00",
      "avatar_symbol": "📝",
      "system_instruction": "You are Dave, the Creative & Communications Specialist in Mark-LIV Agent Town. You specialize in crafting concise reports, documentation, message drafts, and content reviews."
    }
  ]
}
```

Implement `core/agent_town.py`:
- `AgentState(Enum)`: `IDLE`, `WORKING`, `COMPLETED`, `ERROR`.
- `ResidentAgent` dataclass: `id`, `name`, `role`, `specialty`, `color`, `avatar_symbol`, `system_instruction`, `state`, `current_task`, `latest_result`, `history`.
- `AgentTownManager` class:
  - Singleton pattern `get_instance()`.
  - Reads `config/agents.json` on init (with hardcoded fallback defaults if missing).
  - `dispatch_task(agent_name, task, on_complete=None, async_exec=True)`:
    - Sets state to `WORKING` and notifies listeners.
    - If `async_exec`: runs target function in a `threading.Thread(daemon=True)`.
    - If not `async_exec`: runs synchronously for testing.
    - Calls `gemini.call()` with agent's system prompt and user's task.
    - Updates `latest_result`, appends to `history`, sets state to `COMPLETED` (or `ERROR`), and fires callbacks.

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest tests/test_agent_town.py -v`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add config/agents.json core/agent_town.py tests/test_agent_town.py
git commit -m "feat(agent-town): add agent configuration and core engine"
```

---

### Task 2: Voice & Action Tool Integration (`actions/agent_town.py`)

**Files:**
- Create: `actions/agent_town.py`
- Test: `tests/test_agent_action.py`

**Interfaces:**
- Consumes: `core.agent_town.AgentTownManager`
- Produces: `TOOL` dict with `name="delegate_agent_task"` and handler function `delegate_agent_task(parameters, speak=None, ...)`

- [ ] **Step 1: Write failing unit test for `delegate_agent_task` action**

```python
# tests/test_agent_action.py
import pytest
from actions.agent_town import delegate_agent_task, TOOL

def test_tool_declaration():
    assert TOOL["name"] == "delegate_agent_task"
    assert "parameters" in TOOL
    assert "agent" in TOOL["parameters"]["properties"]
    assert "task" in TOOL["parameters"]["properties"]

def test_delegate_agent_task_execution(monkeypatch):
    dispatched = []
    import core.agent_town as at
    mgr = at.AgentTownManager.get_instance()
    
    def mock_dispatch(agent_name, task, on_complete=None, async_exec=True):
        dispatched.append((agent_name, task))
        return True
        
    monkeypatch.setattr(mgr, "dispatch_task", mock_dispatch)
    
    spoken = []
    def mock_speak(text):
        spoken.append(text)
        
    res = delegate_agent_task(
        {"agent": "Bob", "task": "Write a python script"},
        speak=mock_speak
    )
    assert "Bob" in res
    assert len(dispatched) == 1
    assert dispatched[0][0] == "Bob"
    assert dispatched[0][1] == "Write a python script"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest tests/test_agent_action.py -v`
Expected: FAIL (ModuleNotFoundError: No module named 'actions.agent_town')

- [ ] **Step 3: Implement `actions/agent_town.py`**

Implement `delegate_agent_task` and `TOOL` declaration:
```python
from __future__ import annotations
from typing import Any, Callable, Optional
from core.agent_town import AgentTownManager

def delegate_agent_task(
    parameters: dict[str, Any],
    speak: Optional[Callable[[str], None]] = None,
    player: Optional[Any] = None,
    **kwargs: Any,
) -> str:
    params = parameters or {}
    agent_name = str(params.get("agent", "")).strip()
    task = str(params.get("task", "")).strip()
    
    if not agent_name or not task:
        return "Please specify both the agent name and the task to delegate, sir."
        
    mgr = AgentTownManager.get_instance()
    agent = mgr.get_agent(agent_name)
    if not agent:
        valid_names = ", ".join(a.name for a in mgr.get_all_agents())
        return f"Unknown agent '{agent_name}'. Available agents in Agent Town are: {valid_names}."
        
    mgr.dispatch_task(agent.name, task, async_exec=True)
    msg = f"Task delegated to {agent.name} ({agent.role}): {task}"
    if speak:
        speak(f"Delegating to {agent.name}, sir.")
    return msg

TOOL = {
    "name": "delegate_agent_task",
    "description": "Delegates a specialized task to a resident Agent Town worker (Alice for Research, Bob for Development, Carol for System Ops, Dave for Writing).",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "agent": {
                "type": "STRING",
                "description": "Name of the agent to delegate to (Alice, Bob, Carol, or Dave)."
            },
            "task": {
                "type": "STRING",
                "description": "Detailed description of the task to be performed."
            }
        },
        "required": ["agent", "task"]
    },
    "handler": delegate_agent_task,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest tests/test_agent_action.py -v`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add actions/agent_town.py tests/test_agent_action.py
git commit -m "feat(agent-town): add delegate_agent_task action tool"
```

---

### Task 3: Interactive Agent Town HUD Drawer & Desk Cards in `ui.py`

**Files:**
- Modify: `ui.py`
- Test: `tests/test_agent_town_ui.py`

**Interfaces:**
- Consumes: `core.agent_town.AgentTownManager`, `core.agent_town.AgentState`, `core.agent_town.ResidentAgent`
- Produces: `AgentCardWidget`, `AgentTownDrawer` classes in `ui.py`, header button "◈ TOWN", and drawer toggle logic.

- [ ] **Step 1: Write headless test for AgentCardWidget & AgentTownDrawer**

```python
# tests/test_agent_town_ui.py
import pytest
from PyQt6.QtWidgets import QApplication
from ui import AgentCardWidget, AgentTownDrawer
from core.agent_town import AgentTownManager, AgentState

@pytest.fixture(scope="session")
def qapp():
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    return app

def test_agent_card_widget(qapp):
    mgr = AgentTownManager.get_instance()
    alice = mgr.get_agent("Alice")
    card = AgentCardWidget(alice)
    assert card._agent.name == "Alice"
    assert "Senior Research" in card._role_lbl.text()
    
    # Test state update
    alice.state = AgentState.WORKING
    card.refresh_ui()
    assert "WORKING" in card._status_lbl.text()

def test_agent_town_drawer_creation(qapp):
    mgr = AgentTownManager.get_instance()
    drawer = AgentTownDrawer(None)
    assert len(drawer._cards) == 4
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest tests/test_agent_town_ui.py -v`
Expected: FAIL (ImportError: cannot import name 'AgentCardWidget' from 'ui')

- [ ] **Step 3: Implement `AgentCardWidget` and `AgentTownDrawer` in `ui.py` and wire into `MainWindow`**

- Create `AgentCardWidget(QFrame)`:
  - Header with icon badge, agent name, and live LED indicator (`IDLE` = green glow, `WORKING` = cyan pulse, `COMPLETED` = yellow glow, `ERROR` = red).
  - Specialty & active task description label.
  - Latest result preview area (with scrolling or preview tooltip).
  - "DISPATCH" / "VIEW LOG" button opening a direct task dialog.
- Create `AgentTownDrawer(QWidget)`:
  - Header: `"◈  AGENT TOWN // LIVING OFFICE"` with Close button.
  - Sub-header showing active agent statistics (`X/4 AGENTS ACTIVE`).
  - Grid layout holding the 4 `AgentCardWidget` instances.
  - Polling timer (1000ms) or listener callback to refresh cards automatically.
- In `MainWindow`:
  - Add `self._agent_town_drawer = AgentTownDrawer(self.centralWidget())` in `__init__`.
  - Position drawer in `resizeEvent` (similar to `_quick_drawer`).
  - Add `"TOWN"` button in `_build_header()` right side action group.
  - Clicking `"TOWN"` toggles `self._agent_town_drawer.toggle()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest tests/test_agent_town_ui.py -v`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `pytest -v`
Expected: All tests pass.

- [ ] **Step 6: Commit changes**

```bash
git add ui.py tests/test_agent_town_ui.py
git commit -m "feat(ui): add Agent Town drawer and desk cards to Mark-LIV HUD"
```

---

### Task 4: Integration Verification & Polish

**Files:**
- Test: `tests/test_integration.py`
- Modify: `main.py` (if any registration is needed)

- [ ] **Step 1: Write integration test verifying action discovery and agent workflow**
- [ ] **Step 2: Run verification command and inspect output**
- [ ] **Step 3: Commit and document**
