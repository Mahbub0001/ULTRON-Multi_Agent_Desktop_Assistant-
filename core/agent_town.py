from __future__ import annotations

import json
import logging
import sys
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Optional

logger = logging.getLogger(__name__)


def _base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent


BASE_DIR = _base_dir()
AGENTS_CONFIG_FILE = BASE_DIR / "config" / "agents.json"


class AgentState(str, Enum):
    IDLE = "IDLE"
    WORKING = "WORKING"
    COMPLETED = "COMPLETED"
    ERROR = "ERROR"


@dataclass
class ResidentAgent:
    id: str
    name: str
    role: str
    specialty: str
    color: str
    avatar_symbol: str
    system_instruction: str
    state: AgentState = AgentState.IDLE
    current_task: str = ""
    latest_result: str = ""
    history: list[dict[str, Any]] = field(default_factory=list)
    last_active: float = field(default_factory=time.time)


_DEFAULT_AGENTS: list[dict[str, Any]] = [
    {
        "id": "alice",
        "name": "Alice",
        "role": "Senior Research Analyst",
        "specialty": "Deep web research, fact-checking, and data synthesis",
        "color": "#00d4ff",
        "avatar_symbol": "🔬",
        "system_instruction": "You are Alice, the Senior Research Analyst in Mark-LIV Agent Town. You specialize in deep research, data synthesis, fact-checking, and structured summaries.",
    },
    {
        "id": "bob",
        "name": "Bob",
        "role": "Software Architect & Developer",
        "specialty": "Coding, debugging, architecture design, and automation scripts",
        "color": "#00ff88",
        "avatar_symbol": "💻",
        "system_instruction": "You are Bob, the Lead Software Developer in Mark-LIV Agent Town. You specialize in writing clean code, reviewing architectures, debugging errors, and creating automation scripts.",
    },
    {
        "id": "carol",
        "name": "Carol",
        "role": "System Ops & Automator",
        "specialty": "PC operations, process management, file workflows, and system health",
        "color": "#ffcc00",
        "avatar_symbol": "⚙️",
        "system_instruction": "You are Carol, the System Operations Specialist in Mark-LIV Agent Town. You specialize in local PC workflows, file management, system health, and desktop operations.",
    },
    {
        "id": "dave",
        "name": "Dave",
        "role": "Creative & Communications Specialist",
        "specialty": "Documentation, executive reports, email drafts, and content review",
        "color": "#ff6b00",
        "avatar_symbol": "📝",
        "system_instruction": "You are Dave, the Creative & Communications Specialist in Mark-LIV Agent Town. You specialize in crafting concise reports, documentation, message drafts, and content reviews.",
    },
]


class AgentTownManager:
    _instance: Optional[AgentTownManager] = None
    _lock = threading.Lock()

    def __init__(self) -> None:
        self._agents: dict[str, ResidentAgent] = {}
        self._listeners: list[Callable[[ResidentAgent], None]] = []
        self._load_agents()

    @classmethod
    def get_instance(cls) -> AgentTownManager:
        with cls._lock:
            if cls._instance is None:
                cls._instance = cls()
            return cls._instance

    def _load_agents(self) -> None:
        data: list[dict[str, Any]] = []
        if AGENTS_CONFIG_FILE.is_file():
            try:
                raw = json.loads(AGENTS_CONFIG_FILE.read_text(encoding="utf-8"))
                data = raw.get("agents", [])
            except Exception as e:
                logger.warning("Failed to parse %s: %s", AGENTS_CONFIG_FILE, e)

        if not data:
            data = _DEFAULT_AGENTS

        self._agents.clear()
        for item in data:
            agent = ResidentAgent(
                id=item.get("id", item.get("name", "").lower()),
                name=item.get("name", "Agent"),
                role=item.get("role", "Specialist"),
                specialty=item.get("specialty", ""),
                color=item.get("color", "#00d4ff"),
                avatar_symbol=item.get("avatar_symbol", "🤖"),
                system_instruction=item.get("system_instruction", ""),
            )
            self._agents[agent.name.lower()] = agent

    def get_all_agents(self) -> list[ResidentAgent]:
        return list(self._agents.values())

    def get_agent(self, name_or_id: str) -> Optional[ResidentAgent]:
        key = (name_or_id or "").strip().lower()
        if key in self._agents:
            return self._agents[key]
        for a in self._agents.values():
            if a.id.lower() == key:
                return a
        return None

    def register_listener(self, callback: Callable[[ResidentAgent], None]) -> None:
        if callback not in self._listeners:
            self._listeners.append(callback)

    def unregister_listener(self, callback: Callable[[ResidentAgent], None]) -> None:
        if callback in self._listeners:
            self._listeners.remove(callback)

    def _notify(self, agent: ResidentAgent) -> None:
        for cb in list(self._listeners):
            try:
                cb(agent)
            except Exception as e:
                logger.error("Error in AgentTown listener: %s", e)

    def dispatch_task(
        self,
        agent_name: str,
        task: str,
        on_complete: Optional[Callable[[ResidentAgent, str], None]] = None,
        async_exec: bool = True,
    ) -> bool:
        agent = self.get_agent(agent_name)
        if not agent:
            logger.warning("Agent '%s' not found in AgentTown", agent_name)
            return False

        agent.state = AgentState.WORKING
        agent.current_task = task
        agent.last_active = time.time()
        self._notify(agent)

        def _worker() -> None:
            try:
                from core import gemini

                prompt = (
                    f"System: {agent.system_instruction}\n\n"
                    f"User Request: {task}\n\n"
                    "Respond with a high quality, thorough and structured output suited for your role."
                )
                result = gemini.call(prompt, tier=gemini.SMART, timeout_ms=60000)
                if not result:
                    result = "Task processed, but no response content was generated."

                agent.state = AgentState.COMPLETED
                agent.latest_result = result
                agent.history.append({
                    "task": task,
                    "result": result,
                    "timestamp": time.time(),
                    "status": "success"
                })
            except Exception as ex:
                agent.state = AgentState.ERROR
                agent.latest_result = f"Error executing task: {ex}"
                agent.history.append({
                    "task": task,
                    "result": str(ex),
                    "timestamp": time.time(),
                    "status": "error"
                })
            finally:
                agent.last_active = time.time()
                self._notify(agent)
                if on_complete:
                    try:
                        on_complete(agent, agent.latest_result)
                    except Exception as e:
                        logger.error("Error in on_complete callback for %s: %s", agent.name, e)

        if async_exec:
            t = threading.Thread(target=_worker, daemon=True, name=f"AgentTownWorker-{agent.name}")
            t.start()
        else:
            _worker()

        return True
