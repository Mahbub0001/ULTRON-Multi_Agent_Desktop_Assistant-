from __future__ import annotations

from typing import Any, Callable, Optional
from core.agent_town import AgentTownManager, AgentState


def get_agent_status(agent_name: str = "") -> str:
    """Checks the status, availability, and thoughts of an agent or all agents."""
    mgr = AgentTownManager.get_instance()
    agent_key = (agent_name or "").strip().lower()

    if not agent_key or agent_key in ("all", "department", "town", "everyone", "everybody", "agents"):
        agents = mgr.get_all_agents()
        lines = ["🏢 Agent Department (Agent Town) Roster:"]
        for a in agents:
            state_desc = "FREE (IDLE)" if a.state == AgentState.IDLE else f"BUSY ({a.state.value})"
            if a.state == AgentState.WORKING and a.current_task:
                state_desc += f" — Working on: {a.current_task[:45]}..."
            lines.append(f"• {a.avatar_symbol} {a.name} ({a.role}): {state_desc} — {a.specialty}")
        lines.append("All resident agents are active in Living Office and available for tasks, sir.")
        return "\n".join(lines)

    agent = mgr.get_agent(agent_key)
    if not agent:
        valid_names = ", ".join(f"{a.name} ({a.role})" for a in mgr.get_all_agents())
        return f"Agent '{agent_name}' not found in Agent Department. Available resident agents are: {valid_names}."

    if agent.state == AgentState.IDLE:
        thought = agent.get_ambient_thought()
        return (
            f"{agent.avatar_symbol} {agent.name} ({agent.role}) is currently FREE (IDLE) at her desk in Agent Town.\n"
            f"Specialty: {agent.specialty}\n"
            f"Status: Standby and ready for your instructions, sir.\n"
            f"Current thought: '{thought}'"
        )
    elif agent.state == AgentState.WORKING:
        return (
            f"{agent.avatar_symbol} {agent.name} ({agent.role}) is currently BUSY (WORKING).\n"
            f"Current task: '{agent.current_task}'\n"
            f"Progress: {agent.status_message or 'Executing...'}\n"
            f"Specialty: {agent.specialty}"
        )
    elif agent.state == AgentState.COMPLETED:
        res_summary = (agent.latest_result[:250] + "…") if len(agent.latest_result) > 250 else agent.latest_result
        return (
            f"{agent.avatar_symbol} {agent.name} ({agent.role}) is currently FREE (Finished previous task).\n"
            f"Previous task: '{agent.current_task}'\n"
            f"Latest result: {res_summary or 'Completed successfully.'}\n"
            f"She is ready for new instructions, sir."
        )
    else:
        return f"{agent.avatar_symbol} {agent.name} is in state: {agent.state.value}."


def list_agent_department() -> str:
    return get_agent_status("all")


def get_agent_report(agent_name: str) -> str:
    """Returns the latest result / report of an agent."""
    mgr = AgentTownManager.get_instance()
    agent = mgr.get_agent(agent_name)
    if not agent:
        return f"Agent '{agent_name}' not found in Agent Department."
    if not agent.latest_result and not agent.history:
        return f"{agent.name} has not completed any tasks yet in this session."
    report = agent.latest_result
    if not report and agent.history:
        report = agent.history[-1].get("result", "")
    return f"📄 Latest Report from {agent.name} ({agent.role}):\n{report}"


def open_agent_town(player: Optional[Any] = None) -> str:
    """Opens the Agent Town Living Office UI drawer on screen."""
    if player and hasattr(player, "_toggle_agent_town"):
        try:
            from PyQt6.QtCore import QMetaObject, Qt
            QMetaObject.invokeMethod(player, "_toggle_agent_town", Qt.ConnectionType.QueuedConnection)
            return "Opened Agent Town Living Office on your screen, sir."
        except Exception:
            try:
                player._toggle_agent_town()
                return "Opened Agent Town Living Office on your screen, sir."
            except Exception as e:
                return f"Could not toggle Agent Town drawer: {e}"
    return "Agent Town Living Office is active in background, sir."


def delegate_agent_task(
    parameters: dict[str, Any],
    player: Optional[Any] = None,
    speak: Optional[Callable[[str], None]] = None,
    session_memory: Optional[Any] = None,
    **kwargs: Any,
) -> str:
    params = parameters or {}
    agent_name = str(params.get("agent", "")).strip()
    task = str(params.get("task", "")).strip()

    # If user/model called delegate with empty task or asking for status
    if agent_name and (not task or task.lower() in ("status", "free", "is free", "check", "availability", "current status", "state")):
        return get_agent_status(agent_name)

    if not agent_name or not task:
        msg = "Sir, please specify both the agent name and the task description."
        _log(msg, player)
        return msg

    mgr = AgentTownManager.get_instance()
    agent = mgr.get_agent(agent_name)
    if not agent:
        valid_names = ", ".join(a.name for a in mgr.get_all_agents())
        msg = f"Unknown agent '{agent_name}'. Available agents in Agent Town are: {valid_names}."
        _log(msg, player)
        return msg

    mgr.dispatch_task(agent.name, task, async_exec=True)
    msg = f"Task delegated to {agent.name} ({agent.role}): {task}"
    _log(msg, player)

    if speak:
        try:
            speak(f"Delegating to {agent.name}, sir.")
        except Exception:
            pass

    return msg


def agent_department(
    parameters: dict[str, Any],
    player: Optional[Any] = None,
    speak: Optional[Callable[[str], None]] = None,
    session_memory: Optional[Any] = None,
    **kwargs: Any,
) -> str:
    params = parameters or {}
    action = str(params.get("action", "status")).strip().lower()
    agent = str(params.get("agent", "")).strip()
    task = str(params.get("task", "")).strip()

    if action in ("status", "get_status", "check", "availability", "free", "state"):
        return get_agent_status(agent)

    elif action in ("list", "list_agents", "roster", "department", "agents"):
        return list_agent_department()

    elif action in ("report", "get_report", "result", "output"):
        return get_agent_report(agent or "Alice")

    elif action in ("open", "open_town", "open_office", "open_drawer", "show", "show_office"):
        return open_agent_town(player)

    elif action in ("delegate", "assign", "dispatch", "task", "work"):
        return delegate_agent_task({"agent": agent, "task": task}, player=player, speak=speak, **kwargs)

    else:
        # Default fallback: check status or list
        if agent:
            return get_agent_status(agent)
        return list_agent_department()


def _log(message: str, player: Optional[Any] = None) -> None:
    try:
        print(f"[AgentTown] {message}")
    except Exception:
        try:
            print(f"[AgentTown] {message.encode('ascii', errors='replace').decode('ascii')}")
        except Exception:
            pass
    if player:
        try:
            player.write_log(f"JARVIS: {message}")
        except Exception:
            pass


# ── Tool declarations (auto-discovered by core/action_loader.py) ─────────────
TOOL_DELEGATE = {
    "name": "delegate_agent_task",
    "description": (
        "Delegates an autonomous background task to a specialized resident agent in Agent Town "
        "(Alice for Deep Research, Bob for Software Development and Coding, "
        "Carol for System Ops and Automation, Dave for Writing and Reports)."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "agent": {
                "type": "STRING",
                "description": "Name of the resident agent (Alice, Bob, Carol, or Dave).",
            },
            "task": {
                "type": "STRING",
                "description": "The specific instruction or task to perform.",
            },
        },
        "required": ["agent", "task"],
    },
    "handler": delegate_agent_task,
}

TOOL_DEPARTMENT = {
    "name": "agent_department",
    "description": (
        "Manages and queries Nibir sir's Agent Department / Agent Town resident agents: "
        "Alice (Deep Research), Bob (Coding & Software Dev), Carol (System Ops & Automation), Dave (Reports & Writing). "
        "Use this tool whenever the user asks about an agent's availability or status (e.g. 'Alice কি এখন ফ্রি আছে?'), "
        "asks what an agent is doing, wants to list all agents, view their latest reports, or open the Living Office screen."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": (
                    "Action to perform: 'status' (check if an agent is free/busy or get all status), "
                    "'list' (list all department agents), 'report' (get latest report from agent), "
                    "'open_town' (open Living Office UI drawer), 'delegate' (assign a task)."
                ),
            },
            "agent": {
                "type": "STRING",
                "description": "Name of the agent: 'Alice', 'Bob', 'Carol', 'Dave', or 'all'.",
            },
            "task": {
                "type": "STRING",
                "description": "Task description when action is 'delegate'.",
            },
        },
        "required": ["action"],
    },
    "handler": agent_department,
}

# Exposed tools
TOOL = TOOL_DELEGATE
TOOLS = [TOOL_DELEGATE, TOOL_DEPARTMENT]
