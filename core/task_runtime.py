"""Software task coordination: context, bounded execution, evidence and recovery.

The voice model starts jobs; this module owns their lifecycle. Cancellation is
cooperative: a running native handler finishes before any next action is allowed.
No worker is abandoned with a false timeout while it can still change the PC.
"""
from __future__ import annotations

import asyncio
import copy
import json
import math
import re
import threading
import time
import uuid
from collections import deque
from pathlib import Path
from typing import Callable


READ_ACTIONS = {
    "app_control": {"state", "elements", "windows"},
    "computer_control": {"active_window", "list_windows", "screen_analyze", "screenshot"},
    "browser_control": {"get_text", "get_url", "screenshot", "list_browsers"},
    "file_controller": {"read", "list", "info", "find", "largest", "disk_usage"},
    "excel_control": {"read_sheet"},
}
READ_TOOLS = {"recall_memory", "web_search", "weather_report", "system_status"}
SELF_VERIFYING_ACTIONS = {"excel_control": {"new_workbook", "create_table", "write_range", "clear_table"},
                          "computer_control": {"maximize_window", "minimize_window", "restore_window"}}
EXCLUDED_TOOLS = {"task_control", "shutdown_jarvis", "android_control", "printer_control",
                  "smart_home", "iot_control", "screen_process", "close_camera",
                  "agent_department", "delegate_agent_task", "dev_agent"}
TERMINAL = {"completed", "failed", "cancelled", "waiting", "suspended"}
_FAILURE = re.compile(
    r"^(?:error\b|\w+ error\b|element not found\b|not_found\b|no (?:active|accessible|selector|text provided|key specified)\b|failed\b|failure\b|could not\b|couldn't\b|cannot\b|can't\b|"
    r"not found\b|unknown (?:tool|action)\b|access denied\b|no results\b|"
    r"ui automation unavailable\b|unable\b|file controller error\b|"
    r".*? (?:failed|timed out)(?::| after|\b))", re.I)


def is_read_only(name: str, args: dict) -> bool:
    return name in READ_TOOLS or str(args.get("action", "")).lower() in READ_ACTIONS.get(name, set())


def result_status(result) -> str:
    text = str(result or "").strip()
    if "[CONFIRMATION_PENDING]" in text:
        return "waiting"
    if "[VISION_ACTIVE]" in text:
        return "pending"
    if not text or _FAILURE.match(text):
        return "failed"
    return "executed"


class WorkingMemory:
    """Bounded conversation and resource references shared by all tools."""
    def __init__(self):
        self._lock = threading.RLock()
        self.messages = deque(maxlen=12)
        self.outcomes = deque(maxlen=12)
        self.resources: dict[str, str] = {}
        self.last_search: dict = {}

    def user(self, text: str):
        with self._lock:
            text = str(text).strip()
            if text and (not self.messages or self.messages[-1] != text):
                self.messages.append(text[:2000])

    def set_last_search(self, query: str, response: str):
        with self._lock:
            self.last_search = {"query": query, "response": response[:3000]}

    def record(self, name: str, args: dict, result, status: str):
        with self._lock:
            self.outcomes.append({"tool": name, "arguments": copy.deepcopy(args),
                                  "status": status, "result": str(result)[:1800]})
            if status != "executed":
                return
            if name == "browser_control" and args.get("url"):
                self.resources["last_requested_url"] = str(args["url"])
            if name in {"file_controller", "file_processor", "word_document", "excel_control"}:
                path = args.get("file_path") or args.get("path")
                if path:
                    aliases = {key: Path.home() / folder for key, folder in {
                        "desktop": "Desktop", "downloads": "Downloads", "documents": "Documents"}.items()}
                    aliases["home"] = Path.home()
                    target = aliases.get(str(path).casefold(), Path(str(path)).expanduser())
                    if args.get("name") and name == "file_controller" and args.get("action") not in {"find", "list"}:
                        target /= str(args["name"])
                    if args.get("action") == "rename" and args.get("new_name"):
                        target = target.with_name(str(args["new_name"]))
                    elif args.get("action") in {"move", "copy"} and args.get("destination"):
                        target = Path(args["destination"]).expanduser() / target.name
                    self.resources["last_file"] = str(target.absolute())

    def snapshot(self) -> dict:
        with self._lock:
            return copy.deepcopy({"recent_user_messages": list(self.messages),
                                  "recent_tools": list(self.outcomes),
                                  "resources": self.resources, "last_search": self.last_search})

    def restore(self, data: dict):
        with self._lock:
            self.messages.extend(data.get("recent_user_messages", [])[-12:])
            self.outcomes.extend(data.get("recent_tools", [])[-12:])
            self.resources.update(data.get("resources", {}))
            self.last_search = data.get("last_search", {})


class TaskStore:
    def __init__(self, path: Path | None):
        self.path = path

    def load(self) -> dict:
        try:
            return json.loads(self.path.read_text(encoding="utf-8")) if self.path else {}
        except (OSError, ValueError):
            return {}

    def save(self, tasks: dict, context: dict):
        if not self.path:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        temporary.write_text(json.dumps({"tasks": list(tasks.values())[-30:], "context": context},
                                        ensure_ascii=False, indent=2), encoding="utf-8")
        temporary.replace(self.path)


def validate_value(value, schema: dict, path="arguments"):
    """Validate the JSON subset used by Gemini declarations, before any action."""
    kind = str(schema.get("type", "")).upper()
    valid = {"OBJECT": isinstance(value, dict), "ARRAY": isinstance(value, list),
             "STRING": isinstance(value, str), "BOOLEAN": isinstance(value, bool),
             "INTEGER": isinstance(value, int) and not isinstance(value, bool),
             "NUMBER": isinstance(value, (int, float)) and not isinstance(value, bool)}
    if kind and not valid.get(kind, True):
        raise ValueError(f"{path} must be {kind}")
    if kind == "NUMBER" and not math.isfinite(value):
        raise ValueError(f"{path} must be finite")
    if "enum" in schema and value not in schema["enum"]:
        raise ValueError(f"{path} is not an allowed value")
    if kind == "OBJECT":
        properties = schema.get("properties")
        for required in schema.get("required", []):
            if required not in value:
                raise ValueError(f"{path}.{required} is required")
        for key, item in value.items():
            if properties is None:
                continue
            if key not in properties:
                raise ValueError(f"Unknown parameter {path}.{key}")
            validate_value(item, properties[key], f"{path}.{key}")
    elif kind == "ARRAY":
        for index, item in enumerate(value):
            validate_value(item, schema.get("items", {}), f"{path}[{index}]")


class TaskRuntime:
    def __init__(self, *, planner: Callable, execute: Callable, declarations: Callable,
                 observe: Callable, verify: Callable, memory: Callable = lambda goal: "",
                 notify: Callable = lambda task: None, store: Path | None = None,
                 max_steps=16, max_seconds=600):
        self.planner, self.execute = planner, execute
        self.declarations, self.observe, self.verify = declarations, observe, verify
        self.memory, self.notify = memory, notify
        self.max_steps, self.max_seconds = max_steps, max_seconds
        self.working = WorkingMemory()
        self.store = TaskStore(store)
        saved = self.store.load()
        self.tasks = {task["id"]: task for task in saved.get("tasks", [])}
        self.working.restore(saved.get("context", {}))
        for task in self.tasks.values():
            if task["status"] not in TERMINAL:
                task["status"] = "suspended"
                task["detail"] = "Application restarted; inspect state before explicitly resuming."
        self._jobs: dict[str, asyncio.Task] = {}
        self._action_lock = asyncio.Lock()
        self._unverified_direct = None

    def _save(self, required=False):
        try:
            self.store.save(self.tasks, self.working.snapshot())
        except OSError:
            if required:
                raise

    def _update(self, task, status, detail):
        task.update(status=status, detail=str(detail)[:1500], updated=time.time())
        self._save()
        try:
            self.notify(copy.deepcopy(task))
        except Exception:
            pass

    def status(self, task_id="") -> dict:
        if task_id:
            task = self.tasks.get(task_id)
            return copy.deepcopy(task) if task else {"error": "Task not found"}
        return {"tasks": [copy.deepcopy(task) for task in list(self.tasks.values())[-8:]]}

    def start(self, goal: str, resume_id="") -> dict:
        if not str(goal).strip() and not resume_id:
            return {"error": "A goal is required"}
        if any(not job.done() for job in self._jobs.values()):
            return {"error": "A task is already running. Check its status or cancel it first."}
        self._jobs = {key: job for key, job in self._jobs.items() if not job.done()}
        while len(self.tasks) >= 30 and not resume_id:
            old_id = next((key for key, saved in self.tasks.items()
                           if saved["status"] in {"completed", "cancelled", "failed"}), None)
            if old_id is None:
                return {"error": "Task history is full; resolve or clear an older waiting task first."}
            self.tasks.pop(old_id)
        if resume_id:
            task = self.tasks.get(resume_id)
            if not task or task["status"] not in {"waiting", "failed", "cancelled", "suspended"}:
                return {"error": "This task cannot be resumed"}
            task["cancel_requested"] = False
            task["resuming"] = True
            if str(goal).strip():
                task.setdefault("clarifications", []).append(str(goal).strip()[:4000])
                task["clarifications"] = task["clarifications"][-12:]
                self.working.user(goal)
        else:
            task = {"id": uuid.uuid4().hex[:10], "goal": str(goal).strip()[:4000],
                    "status": "queued", "created": time.time(), "steps": [],
                    "plan": [], "cancel_requested": False, "resuming": False}
            self.tasks[task["id"]] = task
        self._update(task, "queued", "Preparing task context")
        self._jobs[task["id"]] = asyncio.create_task(self._run(task))
        return {"task_id": task["id"], "status": "queued",
                "instruction": "Task started. Do not duplicate its actions. Use status/cancel/resume to manage it."}

    def cancel(self, task_id="") -> dict:
        task = self.tasks.get(task_id) if task_id else next(
            (t for t in reversed(list(self.tasks.values())) if t["status"] not in TERMINAL), None)
        if not task or task["status"] in TERMINAL:
            return {"error": "No running task"}
        task["cancel_requested"] = True
        self._update(task, "cancelling", "Stopping after the current operation finishes; no further actions will start.")
        return {"task_id": task["id"], "status": "cancelling"}

    async def call(self, name: str, args: dict, should_run=None, owner="") -> dict:
        """Serialize real actions across the voice loop and background task."""
        async with self._action_lock:
            signature = json.dumps([name, args], sort_keys=True, ensure_ascii=False)
            if not owner and self._unverified_direct == signature:
                return {"tool": name, "arguments": args, "result": "Error: Previous identical action is unverified. Inspect the target using a read-only tool before retrying.",
                        "status": "failed", "verified": False}
            if should_run is not None and not should_run():
                return {"tool": name, "arguments": args, "result": "Cancelled before execution",
                        "status": "cancelled", "verified": False}
            changes_focus = name in {"app_control", "browser_control", "excel_control"} or (
                name == "computer_control" and bool(args.get("title") or args.get("app")))
            if not owner and (not is_read_only(name, args) or changes_focus) and any(
                    task["status"] not in TERMINAL for task in self.tasks.values()):
                return {"tool": name, "arguments": args, "result": "A task owns execution. Cancel it before starting conflicting actions.",
                        "status": "busy", "verified": False}
            # Shield only the worker, not the entire job. If the caller is
            # cancelled, retain the lock until its native operation ends.
            worker = asyncio.create_task(self.execute(name, args))
            try:
                result = await asyncio.shield(worker)
            except asyncio.CancelledError:
                await worker
                raise
            metadata, scheduling = {}, None
            if hasattr(result, "response"):
                metadata = {k: v for k, v in result.response.items() if k != "result"}
                scheduling = getattr(result, "scheduling", None)
                result = result.response.get("result", "")
            status = result_status(result)
            verified = status == "executed" and (is_read_only(name, args)
                or (metadata.get("native_verified") is True and bool(metadata.get("evidence"))))
            if not owner:
                if is_read_only(name, args) and status == "executed":
                    self._unverified_direct = None
                elif name in {"app_control", "computer_control", "browser_control", "excel_control"} and not verified:
                    self._unverified_direct = signature
            self.working.record(name, args, result, status)
            self._save()
            return {"tool": name, "arguments": copy.deepcopy(args), "result": result,
                    "status": status, "verified": verified,
                    "response_metadata": metadata, "scheduling": scheduling}

    async def context(self) -> dict:
        try:
            desktop = await self.observe()
        except Exception as exc:
            desktop = {"error": str(exc)[:200]}
        return {**self.working.snapshot(), "desktop": desktop}

    async def _run(self, task):
        began = time.monotonic()
        failures: dict[str, int] = {}
        # Persist the no-replay rule across restart/resume. A previous write or
        # send can have succeeded even when its verification did not finish.
        for previous in task["steps"]:
            if previous.get("verified") or previous.get("resolved"):
                continue
            name, args = previous.get("tool"), previous.get("arguments", {})
            if name and isinstance(args, dict):
                signature = json.dumps([name, args], sort_keys=True, ensure_ascii=False)
                failures[signature] = failures.get(signature, 0) + 1
        try:
            relevant_memory = await asyncio.to_thread(self.memory, task["goal"])
            catalog = {decl["name"]: decl for decl in self.declarations()
                       if decl["name"] not in EXCLUDED_TOOLS}
            for _ in range(self.max_steps):
                if task["cancel_requested"]:
                    self._update(task, "cancelled", "Stopped; completed operations remain in the task history.")
                    return
                if time.monotonic() - began >= self.max_seconds:
                    self._update(task, "failed", "Task time budget reached; inspect progress before resuming.")
                    return
                self._update(task, "planning", "Choosing the next step from current observations")
                context = await self.context()
                request = {"task": copy.deepcopy(task), "context": context,
                           "memory": relevant_memory, "tools": list(catalog.values())}
                decision = await asyncio.to_thread(self.planner, request)
                if task["cancel_requested"]:
                    self._update(task, "cancelled", "Stopped before the next operation.")
                    return
                if not isinstance(decision, dict):
                    raise ValueError("Planner did not return a structured decision")
                if decision.get("plan"):
                    task["plan"] = [str(item)[:240] for item in decision["plan"]][:12]
                kind = decision.get("type")
                if kind == "clarify":
                    self._update(task, "waiting", decision.get("question") or "Need a missing task detail")
                    return
                if kind == "finish":
                    unresolved = [s for s in task["steps"] if not s.get("verified") and not s.get("resolved")]
                    evidence = decision.get("evidence_steps", [])
                    verified_ids = {s["id"] for s in task["steps"] if s.get("verified")}
                    if unresolved or not evidence or not set(evidence).issubset(verified_ids):
                        task["planner_feedback"] = "Completion rejected: unresolved steps or missing verified evidence. Observe, repair, or clarify."
                        continue
                    self._update(task, "completed", decision.get("summary") or "Verified task completed")
                    return
                if kind != "act":
                    raise ValueError("Unknown planner decision type")
                name, args = decision.get("tool"), decision.get("arguments", {})
                if name not in catalog or not isinstance(args, dict):
                    task["planner_feedback"] = "Unknown tool or invalid arguments. Use the provided catalog."
                    continue
                check = decision.get("verify")
                try:
                    validate_value(args, catalog[name].get("parameters", {"type": "OBJECT"}))
                    if (not is_read_only(name, args) and not isinstance(check, dict)
                            and args.get("action") not in SELF_VERIFYING_ACTIONS.get(name, set())):
                        raise ValueError("A state-changing action needs an independent verification check")
                    if check:
                        self.verify.validate(check, catalog)
                except ValueError as exc:
                    task["planner_feedback"] = str(exc)
                    continue
                signature = json.dumps([name, args], sort_keys=True, ensure_ascii=False)
                retry_limit = 2 if is_read_only(name, args) else 1
                if failures.get(signature, 0) >= retry_limit:
                    task["planner_feedback"] = "This exact action already failed or remains unverified. Observe, use a different approach, or clarify; do not repeat its side effects."
                    continue
                self._update(task, "running", decision.get("reason") or f"Running {name}")
                # Persist the attempt BEFORE dispatch. After a crash, a native
                # side effect may have completed without returning its result.
                step = {"id": len(task["steps"]) + 1, "tool": name, "arguments": copy.deepcopy(args),
                        "status": "executing", "verified": False, "verification": "Dispatch pending or outcome unknown",
                        "verification_check": copy.deepcopy(check)}
                task["steps"].append(step)
                self._save(required=not is_read_only(name, args))
                outcome = await self.call(name, args, lambda: not task["cancel_requested"], owner=task["id"])
                step.update(outcome)
                step["verification"] = (outcome.get("response_metadata", {}).get("evidence") or
                                        "Read-only observation") if outcome["verified"] else "Not yet verified"
                self._save()
                if outcome["status"] == "waiting":
                    self._update(task, "waiting", "Action needs the user's confirmation. Confirm on screen, then resume to inspect the outcome.")
                    return
                if task["cancel_requested"]:
                    self._update(task, "cancelled", "Stopped after the current operation; its outcome remains in task history.")
                    return
                async def verify_call(name, args):
                    return await self.call(name, args, owner=task["id"])
                if outcome["status"] == "executed" and check:
                    self._update(task, "verifying", "Checking the actual result")
                    passed, evidence = await self.verify(check, verify_call, self.observe)
                    step.update(verified=passed, verification=str(evidence)[:2400])
                if not step["verified"]:
                    failures[signature] = failures.get(signature, 0) + 1
                    task["planner_feedback"] = "Action failed or remains unverified. Inspect evidence before retrying; do not duplicate outward effects."
                else:
                    task.pop("planner_feedback", None)
                    for old in task["steps"][:-1]:
                        if old["id"] in decision.get("resolves", []):
                            previous_check = old.get("verification_check")
                            if previous_check:
                                self.verify.validate(previous_check, catalog)
                                passed, evidence = await self.verify(previous_check, verify_call, self.observe)
                                if passed:
                                    old.update(resolved=step["id"], verification=str(evidence)[:2400])
                                    continue
                            task["planner_feedback"] = "Repair not proven: the original step's postcondition must pass; an unrelated successful observation is insufficient."
                self._save()
            self._update(task, "failed", "Reached the planning step limit without verified completion. Progress is saved.")
        except asyncio.CancelledError:
            self._update(task, "suspended", "Application stopped. Inspect current state before resuming.")
            raise
        except Exception as exc:
            self._update(task, "failed", f"Task stopped: {exc}")


TASK_CONTROL_DECLARATION = {
    "name": "task_control",
    "description": (
        "Plan, execute, verify and recover a software workflow in the background. "
        "Use start for goals needing multiple dependent steps, research plus an artifact, "
        "ambiguous desktop workflows or a failed direct action needing recovery. "
        "Pass the full user goal and constraints, not a guessed list of clicks. "
        "The task owns execution: do not duplicate its actions. Use status to read progress, "
        "cancel to stop after the current operation, resume to continue a saved task after "
        "clarification/confirmation, context to inspect recent user requests, tools and active window. "
        "Simple one-step actions should use their direct tool. This handles software only."
    ),
    "parameters": {
        "type": "OBJECT", "properties": {
            "action": {"type": "STRING", "enum": ["start", "status", "cancel", "resume", "context"]},
            "goal": {"type": "STRING", "description": "Complete goal in the user's language, including constraints"},
            "task_id": {"type": "STRING", "description": "ID returned by start or status; required for resume"},
        }, "required": ["action"],
    },
}
