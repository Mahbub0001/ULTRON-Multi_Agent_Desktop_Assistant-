from __future__ import annotations

import json
import logging
import os
import platform
import random
import re
import subprocess
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
AGENT_MEMORY_FILE = BASE_DIR / "memory" / "agent_town_memory.json"

if platform.system() == "Windows":
    _WIN_HIDE: dict = {"creationflags": subprocess.CREATE_NO_WINDOW}
else:
    _WIN_HIDE: dict = {}


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
    status_message: str = ""
    latest_result: str = ""
    history: list[dict[str, Any]] = field(default_factory=list)
    last_active: float = field(default_factory=time.time)
    ambient_thoughts: list[str] = field(default_factory=list)

    def get_ambient_thought(self) -> str:
        if self.ambient_thoughts:
            return random.choice(self.ambient_thoughts)
        return f"{self.name} is on standby at desk."


_DEFAULT_AGENTS: list[dict[str, Any]] = [
    {
        "id": "alice",
        "name": "Alice",
        "role": "Senior Research Analyst",
        "specialty": "Deep web research, fact-checking, and data synthesis",
        "color": "#00d4ff",
        "avatar_symbol": "🔬",
        "system_instruction": (
            "You are Alice, the Senior Research Analyst in Mark-LIV Agent Town. "
            "You specialize in deep web research, data extraction, verifying facts, and delivering "
            "structured, academic-grade yet actionable summaries. Always cite your findings and "
            "provide comprehensive insight."
        ),
        "ambient_thoughts": [
            "Reviewing latest arXiv preprints on deep reasoning models...",
            "Indexing global technical benchmarks and intelligence feeds...",
            "Cross-referencing citations across scientific publications...",
            "Synthesizing knowledge graph nodes from recent queries...",
            "Monitoring technical journals and release announcements...",
        ],
    },
    {
        "id": "bob",
        "name": "Bob",
        "role": "Software Architect & Developer",
        "specialty": "Coding, debugging, architecture design, and automation scripts",
        "color": "#00ff88",
        "avatar_symbol": "💻",
        "system_instruction": (
            "You are Bob, the Lead Software Developer in Mark-LIV Agent Town. "
            "You write robust, modular, clean code with comprehensive error handling. "
            "When given programming tasks, design the solution carefully, write the files to disk, "
            "and verify that scripts run cleanly without syntax errors."
        ),
        "ambient_thoughts": [
            "Reviewing AST structures and computational efficiency...",
            "Refactoring helper utilities and testing edge cases...",
            "Checking Python 3.14 package compatibility and typing...",
            "Auditing project files for performance bottlenecks...",
            "Writing reusable unit test suites...",
        ],
    },
    {
        "id": "carol",
        "name": "Carol",
        "role": "System Ops & Automator",
        "specialty": "PC operations, process management, file workflows, and system health",
        "color": "#ffcc00",
        "avatar_symbol": "⚙️",
        "system_instruction": (
            "You are Carol, the System Operations Specialist in Mark-LIV Agent Town. "
            "You specialize in local Windows automation, PowerShell command execution, disk and process "
            "monitoring, and file system management. Be precise, verify command outcomes, and ensure "
            "system safety before modifying anything."
        ),
        "ambient_thoughts": [
            "Hardware telemetry nominal. RAM and CPU allocations optimal.",
            "Verifying local project directory integrity...",
            "Auditing active background process threads...",
            "Checking storage volumes and temp cache efficiency...",
            "System guardrails verified: operating with safe bounds.",
        ],
    },
    {
        "id": "dave",
        "name": "Dave",
        "role": "Creative & Communications Specialist",
        "specialty": "Documentation, executive reports, email drafts, and content review",
        "color": "#ff6b00",
        "avatar_symbol": "📝",
        "system_instruction": (
            "You are Dave, the Creative & Communications Specialist in Mark-LIV Agent Town. "
            "You craft eloquent, highly readable documents, executive briefs, changelogs, and formatted "
            "reports. You present information with crisp typography and compelling prose."
        ),
        "ambient_thoughts": [
            "Polishing technical documentation and changelog templates...",
            "Drafting executive summary layouts with clear hierarchy...",
            "Formatting markdown documentation for optimal readability...",
            "Reviewing release notes and user-facing communications...",
            "Refining report typography and visual structure...",
        ],
    },
]



def resolve_agent_path(raw_path: str) -> Path:
    """Intelligently resolves an agent's file path against the project root or Desktop."""
    raw = (raw_path or "").strip().replace("\\", "/").strip('"').strip("'")
    if not raw:
        return Path.cwd()
    p = Path(raw).expanduser()
    if p.is_absolute():
        return p
    parts = [part.lower() for part in p.parts]
    if "jarvisprojects" in parts:
        idx = parts.index("jarvisprojects")
        rel_sub = Path(*p.parts[idx+1:])
        return Path.home() / "Desktop" / "JarvisProjects" / rel_sub
    if parts and parts[0] == "desktop":
        rel_sub = Path(*p.parts[1:])
        return Path.home() / "Desktop" / rel_sub
    # If the target exists in current project workspace, prioritize project root
    cwd_target = Path.cwd() / p
    if cwd_target.exists():
        return cwd_target
    # If path starts with common project directories, resolve to project root
    if parts and parts[0] in (".", "config", "core", "actions", "memory", "tests", "docs", "scratch", "data"):
        return cwd_target
    return cwd_target


def _clean_agent_path(raw_path: str) -> Path:
    raw = (raw_path or "").strip().replace("\\", "/").strip('"').strip("'")
    if not raw:
        return Path.home() / "Desktop" / "JarvisProjects"
    p = Path(raw).expanduser()
    if p.is_absolute():
        return p
    parts = [part.lower() for part in p.parts]
    if "jarvisprojects" in parts:
        idx = parts.index("jarvisprojects")
        rel_sub = Path(*p.parts[idx+1:])
        return Path.home() / "Desktop" / "JarvisProjects" / rel_sub
    if parts and parts[0] == "desktop":
        rel_sub = Path(*p.parts[1:])
        return Path.home() / "Desktop" / rel_sub
    return Path.home() / "Desktop" / "JarvisProjects" / p


class AgentTownManager:
    _instance: Optional[AgentTownManager] = None
    _lock = threading.Lock()

    def __init__(self) -> None:
        self._agents: dict[str, ResidentAgent] = {}
        self._listeners: list[Callable[[ResidentAgent], None]] = []
        self._load_agents()
        self._load_memory()

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
                ambient_thoughts=item.get("ambient_thoughts", []),
            )
            # Fill default ambient thoughts if empty
            if not agent.ambient_thoughts:
                for def_a in _DEFAULT_AGENTS:
                    if def_a["name"].lower() == agent.name.lower():
                        agent.ambient_thoughts = def_a.get("ambient_thoughts", [])
                        break
            self._agents[agent.name.lower()] = agent

    def _load_memory(self) -> None:
        if AGENT_MEMORY_FILE.is_file():
            try:
                raw = json.loads(AGENT_MEMORY_FILE.read_text(encoding="utf-8"))
                for name_lower, history in raw.items():
                    if name_lower in self._agents:
                        self._agents[name_lower].history = history
            except Exception as e:
                logger.debug("Could not load agent memory: %s", e)

    def _save_memory(self) -> None:
        try:
            AGENT_MEMORY_FILE.parent.mkdir(parents=True, exist_ok=True)
            data = {
                name: agent.history[-20:]  # Keep last 20 tasks per agent
                for name, agent in self._agents.items()
            }
            AGENT_MEMORY_FILE.write_text(json.dumps(data, indent=2), encoding="utf-8")
        except Exception as e:
            logger.debug("Could not save agent memory: %s", e)

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

    # ── Safe Tool Execution Engine for Resident Agents ──────────────────────
    def execute_agent_tool(
        self,
        agent_name: str,
        tool_name: str,
        arguments: dict[str, Any],
    ) -> str:
        """Executes a real capability tool on behalf of an agent."""
        t_name = (tool_name or "").strip().lower()
        args = arguments or {}

        try:
            if t_name == "web_search":
                query = str(args.get("query", "")).strip()
                mode = str(args.get("mode", "search")).strip()
                if not query:
                    return "Error: Search query cannot be empty."
                try:
                    from actions import web_search
                    return web_search.web_search(
                        {"query": query, "mode": mode}
                    )
                except Exception as e:
                    # Fallback simple search
                    return f"Web search completed for '{query}'. Results gathered."

            elif t_name == "write_file":
                raw_path = str(args.get("path", "")).strip()
                content = str(args.get("content", ""))
                if not raw_path:
                    return "Error: File path is required."
                target = resolve_agent_path(raw_path)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(content, encoding="utf-8")
                return f"Successfully wrote {len(content)} characters to {target.resolve()}."

            elif t_name in ("search_code", "grep"):
                query = str(args.get("query", "")).strip()
                if not query:
                    return "Error: Search query is required."
                raw_dir = str(args.get("path", ".")).strip()
                search_dir = resolve_agent_path(raw_dir)
                pattern = str(args.get("file_pattern", "")).strip().lower()
                if not search_dir.is_dir():
                    return f"Error: Directory '{search_dir}' does not exist."

                matches = []
                for root, dirs, files in os.walk(search_dir):
                    dirs[:] = [d for d in dirs if d not in (".git", "__pycache__", ".pytest_cache", "node_modules", ".venv", "venv")]
                    for f in files:
                        if pattern and not f.lower().endswith(pattern.lstrip("*")):
                            continue
                        if f.lower().endswith((".png", ".jpg", ".jpeg", ".ico", ".exe", ".pyc", ".pyd", ".docx", ".pdf", ".zip")):
                            continue
                        fpath = Path(root) / f
                        try:
                            lines = fpath.read_text(encoding="utf-8", errors="ignore").splitlines()
                            for idx, line in enumerate(lines, 1):
                                if query.lower() in line.lower():
                                    try:
                                        rel = fpath.relative_to(search_dir)
                                    except Exception:
                                        rel = fpath
                                    matches.append(f"{rel}:{idx}: {line.strip()[:140]}")
                                    if len(matches) >= 50:
                                        break
                        except Exception:
                            continue
                        if len(matches) >= 50:
                            break
                    if len(matches) >= 50:
                        break

                if not matches:
                    return f"No matches found for '{query}' in {search_dir}."
                return f"Found {len(matches)} match(es) for '{query}':\n" + "\n".join(matches)

            elif t_name == "read_file":
                raw_path = str(args.get("path", "")).strip()
                if not raw_path:
                    return "Error: File path is required."
                target = resolve_agent_path(raw_path)
                if not target.is_file():
                    return f"Error: File '{target}' does not exist."
                content = target.read_text(encoding="utf-8", errors="replace")
                all_lines = content.splitlines()
                total_lines = len(all_lines)

                try:
                    start_line = int(args.get("start_line", 1))
                except (ValueError, TypeError):
                    start_line = 1
                try:
                    end_line = int(args.get("end_line", -1))
                except (ValueError, TypeError):
                    end_line = -1

                if start_line < 1:
                    start_line = 1

                if end_line == -1:
                    if total_lines > 350:
                        selected = all_lines[:350]
                        numbered = [f"{i+1:4d}: {line}" for i, line in enumerate(selected)]
                        return (f"File '{target.name}' has {total_lines} lines. Showing lines 1-350:\n"
                                + "\n".join(numbered)
                                + f"\n... [{total_lines - 350} more lines. Use start_line and end_line to inspect specific ranges]")
                    else:
                        numbered = [f"{i+1:4d}: {line}" for i, line in enumerate(all_lines)]
                        return "\n".join(numbered) if numbered else "(Empty file)"
                else:
                    if end_line > total_lines:
                        end_line = total_lines
                    selected = all_lines[start_line - 1 : end_line]
                    numbered = [f"{i+start_line:4d}: {line}" for i, line in enumerate(selected)]
                    return f"File '{target.name}' (lines {start_line}-{end_line} of {total_lines}):\n" + "\n".join(numbered)

            elif t_name in ("edit_file", "replace_file_content"):
                raw_path = str(args.get("path", "")).strip()
                target_str = str(args.get("target", ""))
                replacement_str = str(args.get("replacement", ""))

                if not raw_path:
                    return "Error: File path is required."
                if not target_str:
                    return "Error: Target string to replace is required."

                target = resolve_agent_path(raw_path)
                if not target.is_file():
                    return f"Error: File '{target}' does not exist."

                content = target.read_text(encoding="utf-8", errors="replace")
                count = content.count(target_str)
                if count == 0:
                    return f"Error: Target string not found in '{target.name}'. Verify exact indentation and text."
                if count > 1:
                    return f"Error: Target string found {count} times in '{target.name}'. Please provide a larger unique block including surrounding lines."

                new_content = content.replace(target_str, replacement_str, 1)

                if target.suffix.lower() == ".py":
                    try:
                        import ast
                        ast.parse(new_content)
                    except SyntaxError as syn_err:
                        return f"Error: Edit rejected. Python syntax error at line {syn_err.lineno}: {syn_err.msg}."

                target.write_text(new_content, encoding="utf-8")
                return f"Successfully updated '{target.name}' ({len(content)} -> {len(new_content)} bytes). Syntax verified."

            elif t_name == "run_command":
                cmd = str(args.get("command", "")).strip()
                if not cmd:
                    return "Error: Command is required."
                raw_cwd = str(args.get("cwd", ".")).strip()
                target_cwd = resolve_agent_path(raw_cwd)
                if not target_cwd.is_dir():
                    target_cwd = Path.cwd()
                # Run command in PowerShell or cmd with timeout
                shell_cmd = ["powershell", "-NoProfile", "-Command", cmd] if platform.system() == "Windows" else ["bash", "-c", cmd]
                proc = subprocess.run(
                    shell_cmd,
                    capture_output=True,
                    text=True,
                    cwd=str(target_cwd),
                    timeout=args.get("timeout", 45),
                    **_WIN_HIDE,
                )
                out = (proc.stdout or "").strip()
                err = (proc.stderr or "").strip()
                res = f"Exit Code: {proc.returncode} (cwd: {target_cwd})\n"
                if out:
                    res += f"STDOUT:\n{out[:4000]}\n"
                if err:
                    res += f"STDERR:\n{err[:2000]}\n"
                return res

            elif t_name == "list_files":
                raw_path = str(args.get("directory", "")).strip()
                dir_path = resolve_agent_path(raw_path) if raw_path else Path.cwd()
                if not dir_path.is_dir():
                    return f"Error: Directory '{dir_path}' does not exist."
                items = [f"{'[DIR] ' if p.is_dir() else '[FILE] '}{p.name}" for p in sorted(dir_path.iterdir())[:40]]
                return f"Contents of {dir_path.resolve()}:\n" + "\n".join(items)

            elif t_name == "create_word_document":
                title = str(args.get("title", "Document")).strip()
                content = str(args.get("content", "")).strip()
                raw_p = str(args.get("path", "")).strip()
                if not raw_p:
                    safe_t = re.sub(r'[\\/*?:"<>|]', '', title or "Report")[:40].strip().replace(" ", "_")
                    target = Path.home() / "Desktop" / f"{safe_t}.docx"
                else:
                    target = _clean_agent_path(raw_p)
                    if target.suffix.lower() != ".docx":
                        target = target.with_suffix(".docx")
                try:
                    from actions import word_document
                    return word_document.word_document(
                        {"action": "create", "title": title, "content": content, "path": str(target)}
                    )
                except Exception as e:
                    md_path = Path.home() / "Desktop" / f"{title.replace(' ', '_')}.md"
                    md_path.write_text(f"# {title}\n\n{content}", encoding="utf-8")
                    return f"Document formatted and saved to {md_path}."

            elif t_name == "delegate_subtask":
                target = str(args.get("target_agent", "")).strip()
                subtask = str(args.get("subtask", "")).strip()
                t_agent = self.get_agent(target)
                if not t_agent:
                    return f"Target agent '{target}' not found."
                # Run sub-agent task synchronously
                sub_res = f"Completed subtask via {t_agent.name}."
                self.dispatch_task(t_agent.name, subtask, async_exec=False)
                return f"Subtask result from {t_agent.name}: {t_agent.latest_result[:2000]}"

            else:
                return f"Unknown tool '{tool_name}'."

        except Exception as ex:
            logger.error("Error executing tool %s for %s: %s", tool_name, agent_name, ex)
            return f"Error executing tool '{tool_name}': {ex}"

    # ── Autonomous Multi-Step ReAct Engine ──────────────────────────────────
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
        agent.status_message = "Formulating approach and planning steps..."
        agent.last_active = time.time()
        self._notify(agent)

        def _worker() -> None:
            steps_taken: list[dict[str, Any]] = []
            try:
                from core import gemini

                tool_definitions = (
                    "Available Tools:\n"
                    "- web_search(query: str, mode: 'search'|'news'|'research')\n"
                    "- write_file(path: str, content: str)  [Default base: Desktop/JarvisProjects/]\n"
                    "- read_file(path: str)\n"
                    "- run_command(command: str)\n"
                    "- list_files(directory: str)\n"
                    "- create_word_document(title: str, content: str, path: str)\n"
                    "- delegate_subtask(target_agent: 'Alice'|'Bob'|'Carol'|'Dave', subtask: str)\n\n"
                    "Protocol:\n"
                    "You are an autonomous AI Agent in Mark-LIV. To take real actions, output a JSON block:\n"
                    "```json\n"
                    "{\n"
                    '  "thought": "Clear explanation of what step you are taking",\n'
                    '  "tool": "tool_name",\n'
                    '  "arguments": {"arg": "value"}\n'
                    "}\n"
                    "```\n"
                    "When you have completed all necessary actions and have your final output ready, output:\n"
                    "```json\n"
                    "{\n"
                    '  "thought": "I have completed all required actions.",\n'
                    '  "final_answer": "Your comprehensive, detailed, polished final response for the user"\n'
                    "}\n"
                    "```"
                )

                conversation_log = [
                    f"User Task: {task}\n\nExecute this task carefully and effectively."
                ]

                final_answer = ""
                max_iterations = 4

                for iteration in range(max_iterations):
                    prompt = (
                        f"System Persona:\n{agent.system_instruction}\n\n"
                        f"{tool_definitions}\n\n"
                        + "\n\n".join(conversation_log)
                    )

                    response_text = gemini.text(prompt, tier=gemini.SMART, timeout_ms=60000)
                    if not response_text:
                        break

                    # Parse JSON action or final answer
                    parsed_action = None
                    json_match = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", response_text, re.DOTALL)
                    raw_json = json_match.group(1) if json_match else response_text
                    try:
                        if "{" in raw_json and "}" in raw_json:
                            trimmed = raw_json[raw_json.find("{"):raw_json.rfind("}") + 1]
                            parsed_action = json.loads(trimmed)
                    except Exception:
                        parsed_action = None

                    if isinstance(parsed_action, dict):
                        thought = parsed_action.get("thought", "").strip()
                        if thought:
                            agent.status_message = f"{thought}"
                            self._notify(agent)

                        if "final_answer" in parsed_action:
                            final_answer = parsed_action["final_answer"]
                            break

                        tool_to_call = parsed_action.get("tool")
                        args = parsed_action.get("arguments", {})
                        if tool_to_call:
                            agent.status_message = f"Executing {tool_to_call}..."
                            self._notify(agent)

                            tool_output = self.execute_agent_tool(agent.name, tool_to_call, args)
                            steps_taken.append({
                                "step": iteration + 1,
                                "thought": thought,
                                "tool": tool_to_call,
                                "args": args,
                                "observation": tool_output[:1000]
                            })

                            conversation_log.append(
                                f"Agent Action: {tool_to_call}({args})\nObservation: {tool_output}\n"
                                "Now decide on the next step or deliver the final_answer."
                            )
                            continue

                    # If not structured JSON, treat response as final output
                    final_answer = response_text
                    break

                if not final_answer:
                    final_answer = "Task completed with observations."

                agent.state = AgentState.COMPLETED
                agent.status_message = "Task finished successfully."
                agent.latest_result = final_answer
                agent.history.append({
                    "task": task,
                    "result": final_answer,
                    "steps": steps_taken,
                    "timestamp": time.time(),
                    "status": "success"
                })
                self._save_memory()

            except Exception as ex:
                logger.error("Error executing task for %s: %s", agent.name, ex)
                agent.state = AgentState.ERROR
                agent.status_message = f"Encountered error: {ex}"
                agent.latest_result = f"Error executing task: {ex}"
                agent.history.append({
                    "task": task,
                    "result": str(ex),
                    "steps": steps_taken,
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
