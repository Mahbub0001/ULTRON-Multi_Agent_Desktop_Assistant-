"""Deterministic workflow evaluations: real files, mocked models and UI actions."""
import asyncio
import copy
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import AsyncMock, patch

from core.task_runtime import TaskRuntime, WorkingMemory, result_status, validate_value
from core.task_observer import Verifier


FILE_TOOL = {"name": "file_controller", "parameters": {"type": "OBJECT", "properties": {
    "action": {"type": "STRING"}, "path": {"type": "STRING"}, "content": {"type": "STRING"}},
    "required": ["action", "path"]}}
APP_TOOL = {"name": "app_control", "parameters": {"type": "OBJECT", "properties": {
    "action": {"type": "STRING"}, "app": {"type": "STRING"}, "element": {"type": "STRING"}},
    "required": ["action"]}}


class Decisions:
    def __init__(self, *items):
        self.items = list(items)
        self.requests = []

    def __call__(self, request):
        self.requests.append(copy.deepcopy(request))
        return self.items.pop(0) if self.items else {"type": "clarify", "question": "Need evidence"}


def finish(*ids):
    return {"type": "finish", "summary": "কাজ যাচাই করে শেষ হয়েছে", "evidence_steps": list(ids)}


class TestTaskWorkflows(unittest.IsolatedAsyncioTestCase):
    def runtime(self, planner, execute, **kwargs):
        return TaskRuntime(planner=planner, execute=execute,
                           declarations=lambda: [FILE_TOOL, APP_TOOL],
                           observe=AsyncMock(return_value={"title": "Report - Notepad", "hwnd": 10}),
                           verify=Verifier(), **kwargs)

    async def run_job(self, runtime, goal="রিপোর্ট লিখে save করো"):
        started = runtime.start(goal)
        await runtime._jobs[started["task_id"]]
        return runtime.status(started["task_id"])

    async def test_real_file_workflow_verified_and_restored(self):
        with tempfile.TemporaryDirectory() as folder:
            target, store = Path(folder) / "report.txt", Path(folder) / "state.json"
            planner = Decisions(
                {"type": "act", "plan": ["Write the report", "Verify saved content"],
                 "tool": "file_controller", "arguments": {"action": "write", "path": str(target), "content": "বাংলা রিপোর্ট"},
                 "verify": {"kind": "file_contains", "path": str(target), "text": "বাংলা রিপোর্ট"}}, finish(1))
            async def execute(name, args):
                target.write_text(args["content"], encoding="utf-8")
                return "Saved file"
            runtime = self.runtime(planner, execute, store=store)
            runtime.working.user("আগের রিপোর্টটার মতো বাংলায় লিখো")
            task = await self.run_job(runtime)
            self.assertEqual(task["status"], "completed")
            self.assertTrue(task["steps"][0]["verified"])
            restored = self.runtime(planner, execute, store=store)
            self.assertEqual(restored.status(task["id"])["status"], "completed")
            self.assertEqual(restored.working.snapshot()["resources"]["last_file"], str(target))
            self.assertIn("আগের রিপোর্টটার মতো বাংলায় লিখো", planner.requests[0]["context"]["recent_user_messages"])

    async def test_false_completion_rejected(self):
        execute = AsyncMock(return_value="Done")
        runtime = self.runtime(Decisions(finish(1)), execute)
        task = await self.run_job(runtime)
        self.assertEqual(task["status"], "waiting")
        execute.assert_not_called()

    async def test_failed_action_repaired_by_new_method(self):
        planner = Decisions(
            {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "Save"},
             "verify": {"kind": "window_title", "text": "Report"}},
            {"type": "act", "tool": "app_control", "arguments": {"action": "state"}, "resolves": [1]},
            finish(2))
        execute = AsyncMock(side_effect=["Error: element not found", "Report saved in Notepad"])
        task = await self.run_job(self.runtime(planner, execute))
        self.assertEqual(task["status"], "completed")
        self.assertEqual(task["steps"][0]["resolved"], 2)
        self.assertEqual(execute.await_count, 2)

    async def test_unverified_side_effect_is_not_repeated(self):
        action = {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "Publish"},
                  "verify": {"kind": "window_title", "text": "Published"}}
        execute = AsyncMock(return_value="Clicked Publish")
        task = await self.run_job(self.runtime(Decisions(action, action, finish(1)), execute))
        self.assertNotEqual(task["status"], "completed")
        self.assertEqual(execute.await_count, 1)

    async def test_unverified_side_effect_is_not_replayed_after_resume(self):
        action = {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "Publish"},
                  "verify": {"kind": "window_title", "text": "Published"}}
        execute = AsyncMock(return_value="Clicked Publish")
        with tempfile.TemporaryDirectory() as folder:
            store = Path(folder) / "state.json"
            first = self.runtime(Decisions(action, {"type": "clarify", "question": "Check publication status"}),
                                 execute, store=store)
            task = await self.run_job(first)
            self.assertEqual(task["status"], "waiting")
            resumed = self.runtime(Decisions(action, {"type": "clarify", "question": "Check publication status"}),
                                   execute, store=store)
            started = resumed.start("", resume_id=task["id"])
            await resumed._jobs[started["task_id"]]
            self.assertEqual(execute.await_count, 1)

    async def test_invalid_arguments_and_mutating_verification_never_run(self):
        planner = Decisions(
            {"type": "act", "tool": "app_control", "arguments": {"action": "state", "invented": "x"}},
            {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "x"},
             "verify": {"kind": "tool", "tool": "app_control", "arguments": {"action": "click", "element": "x"}, "contains": "Clicked"}})
        execute = AsyncMock()
        await self.run_job(self.runtime(planner, execute))
        execute.assert_not_called()

    async def test_cancel_waiting_for_action_lock_prevents_action(self):
        planner = Decisions({"type": "act", "tool": "app_control", "arguments": {"action": "state"}})
        execute = AsyncMock(return_value="Observed")
        runtime = self.runtime(planner, execute)
        await runtime._action_lock.acquire()
        started = runtime.start("inspect app")
        for _ in range(100):
            if runtime.tasks[started["task_id"]]["status"] == "running":
                break
            await asyncio.sleep(0.005)
        runtime.cancel(started["task_id"])
        runtime._action_lock.release()
        await runtime._jobs[started["task_id"]]
        self.assertEqual(runtime.status(started["task_id"])["status"], "cancelled")
        execute.assert_not_called()

    async def test_cancel_finishes_current_worker_then_stops(self):
        began, release = asyncio.Event(), asyncio.Event()
        async def execute(name, args):
            began.set()
            await release.wait()
            return "Observed"
        runtime = self.runtime(Decisions({"type": "act", "tool": "app_control", "arguments": {"action": "state"}}, finish(1)), execute)
        started = runtime.start("inspect app")
        try:
            await asyncio.wait_for(began.wait(), timeout=3)
        except asyncio.TimeoutError:
            release.set()
            self.fail(f"Action did not start: {runtime.status(started['task_id'])}")
        runtime.cancel(started["task_id"])
        self.assertFalse(runtime._jobs[started["task_id"]].done())
        release.set()
        await runtime._jobs[started["task_id"]]
        self.assertEqual(runtime.status(started["task_id"])["status"], "cancelled")

    async def test_confirmation_is_waiting_not_completed(self):
        action = {"type": "act", "tool": "file_controller", "arguments": {"action": "delete", "path": str(Path.cwd() / "unused.txt")},
                  "verify": {"kind": "file_exists", "path": str(Path.cwd() / "unused.txt"), "exists": False}}
        task = await self.run_job(self.runtime(Decisions(action, finish(1)), AsyncMock(return_value="[CONFIRMATION_PENDING] Confirm on HUD")))
        self.assertEqual(task["status"], "waiting")
        self.assertFalse(task["steps"][0]["verified"])

    async def test_conflicting_direct_action_is_blocked_during_job(self):
        runtime = self.runtime(Decisions(), AsyncMock(return_value="Clicked"))
        runtime.tasks["running"] = {"id": "running", "status": "running"}
        outcome = await runtime.call("app_control", {"action": "click", "element": "Save"})
        self.assertEqual(outcome["status"], "busy")
        runtime.execute.assert_not_called()

    async def test_restart_does_not_automatically_execute_saved_job(self):
        with tempfile.TemporaryDirectory() as folder:
            store = Path(folder) / "tasks.json"
            runtime = self.runtime(Decisions(), AsyncMock(), store=store)
            runtime.tasks["job"] = {"id": "job", "status": "running", "goal": "Save", "steps": []}
            runtime._save()
            restored = self.runtime(Decisions(), AsyncMock(), store=store)
            self.assertEqual(restored.status("job")["status"], "suspended")
            self.assertFalse(restored._jobs)

    async def test_function_response_metadata_preserved(self):
        response = SimpleNamespace(response={"result": "Saved", "silent": True}, scheduling="SILENT")
        runtime = self.runtime(Decisions(), AsyncMock(return_value=response))
        outcome = await runtime.call("save_memory", {})
        self.assertEqual(outcome["response_metadata"], {"silent": True})
        self.assertEqual(outcome["scheduling"], "SILENT")

    async def test_unknown_planner_tool_is_rejected(self):
        execute = AsyncMock()
        await self.run_job(self.runtime(Decisions({"type": "act", "tool": "run_anything", "arguments": {}}), execute))
        execute.assert_not_called()


class TestContextAndContracts(unittest.TestCase):
    def test_failed_action_does_not_replace_file_reference(self):
        memory = WorkingMemory()
        memory.record("file_controller", {"action": "write", "path": "good.txt"}, "Saved", "executed")
        before = memory.snapshot()["resources"]
        memory.record("file_controller", {"action": "write", "path": "bad.txt"}, "Error", "failed")
        self.assertEqual(before, memory.snapshot()["resources"])

    def test_schema_validates_nested_types_and_required_fields(self):
        schema = {"type": "OBJECT", "properties": {"items": {"type": "ARRAY", "items": {"type": "INTEGER"}}}, "required": ["items"]}
        validate_value({"items": [1, 2]}, schema)
        with self.assertRaises(ValueError):
            validate_value({"items": [True]}, schema)
        with self.assertRaises(ValueError):
            validate_value({}, schema)

    def test_failure_and_pending_results_are_not_success(self):
        for text in ("Error: failed", "Could not save file", "File controller error (write): x", "Unknown tool: x", "Action 'x' timed out after 25 seconds"):
            self.assertEqual(result_status(text), "failed", text)
        self.assertEqual(result_status("[CONFIRMATION_PENDING] Confirm"), "waiting")
        self.assertEqual(result_status("[VISION_ACTIVE] image pending"), "pending")

    def test_reasoning_uses_rest_tier_and_json(self):
        from core.task_planner import plan
        from core import gemini
        with patch("core.gemini.as_json", return_value={"type": "clarify"}) as generate:
            plan({"task": {"goal": "আগের ফাইলটা save করো"}})
        self.assertEqual(generate.call_args.kwargs["tier"], gemini.REASONING)
        self.assertEqual(generate.call_args.kwargs["config"]["response_mime_type"], "application/json")
        self.assertNotIn(gemini.LIVE, gemini._LADDERS[gemini.REASONING])
