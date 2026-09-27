from __future__ import annotations

from typing import Any, Callable, Optional
from core.agent_town import AgentTownManager


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


def _log(message: str, player: Optional[Any] = None) -> None:
    print(f"[AgentTown] {message}")
    if player:
        try:
            player.write_log(f"JARVIS: {message}")
        except Exception:
            pass


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
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
