"""Workflow-level regressions discovered during the business logic audit."""
import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

from core.task_runtime import TaskRuntime
from core.task_observer import Verifier
from tests.test_task_runtime import Decisions, APP_TOOL, FILE_TOOL, finish


class TestWorkflowBusinessLogic(unittest.IsolatedAsyncioTestCase):
    def runtime(self, planner, execute, **kwargs):
        return TaskRuntime(planner=planner, execute=execute, declarations=lambda: [APP_TOOL, FILE_TOOL],
                           observe=AsyncMock(return_value={"title": "Report - Notepad"}), verify=Verifier(), **kwargs)

    async def test_resume_keeps_new_clarification(self):
        planner = Decisions({"type": "clarify", "question": "Which folder?"}, {"type": "clarify"})
        runtime = self.runtime(planner, AsyncMock())
        job = runtime.start("Save my report")
        await runtime._jobs[job["task_id"]]
        runtime.start("Use the Documents folder", resume_id=job["task_id"])
        await runtime._jobs[job["task_id"]]
        self.assertIn("Use the Documents folder", planner.requests[-1]["task"]["clarifications"])

    async def test_action_is_journalled_before_worker_starts(self):
        began, release = asyncio.Event(), asyncio.Event()
        async def execute(name, args):
            began.set()
            await release.wait()
            return "Clicked"
        action = {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "Publish"},
                  "verify": {"kind": "window_title", "text": "Published"}}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "state.json"
            runtime = self.runtime(Decisions(action), execute, store=path)
            job = runtime.start("Publish report")
            try:
                await asyncio.wait_for(began.wait(), 3)
                stored = json.loads(path.read_text(encoding="utf-8"))["tasks"][0]
                self.assertEqual(stored["steps"][0]["status"], "executing")
                self.assertEqual(stored["steps"][0]["arguments"]["element"], "Publish")
            finally:
                release.set()
                await runtime._jobs[job["task_id"]]

    async def test_unrelated_observation_cannot_resolve_failed_write(self):
        with tempfile.TemporaryDirectory() as folder:
            path = str(Path(folder) / "not-created.txt")
            planner = Decisions(
                {"type": "act", "tool": "file_controller", "arguments": {"action": "write", "path": path},
                 "verify": {"kind": "file_contains", "path": path, "text": "report"}},
                {"type": "act", "tool": "app_control", "arguments": {"action": "state"}, "resolves": [1]},
                finish(2))
            runtime = self.runtime(planner, AsyncMock(side_effect=["Error: disk full", "Notepad is open"]))
            job = runtime.start("Write report")
            await runtime._jobs[job["task_id"]]
            self.assertNotEqual(runtime.status(job["task_id"])["status"], "completed")
            self.assertFalse(runtime.status(job["task_id"])["steps"][0].get("resolved"))

    async def test_focusing_read_cannot_interrupt_owned_desktop(self):
        execute = AsyncMock(return_value="Excel state")
        runtime = self.runtime(Decisions(), execute)
        runtime.tasks["job"] = {"id": "job", "status": "running"}
        result = await runtime.call("app_control", {"action": "state", "app": "Excel"})
        self.assertEqual(result["status"], "busy")
        execute.assert_not_called()

    async def test_no_mutation_when_checkpoint_cannot_be_saved(self):
        action = {"type": "act", "tool": "app_control", "arguments": {"action": "click", "element": "Publish"},
                  "verify": {"kind": "window_title", "text": "Published"}}
        execute = AsyncMock(return_value="Clicked")
        runtime = self.runtime(Decisions(action), execute)
        with patch.object(runtime.store, "save", side_effect=OSError("disk full")):
            job = runtime.start("Publish report")
            await runtime._jobs[job["task_id"]]
        execute.assert_not_called()

    async def test_browser_form_preflight_prevents_partial_success(self):
        from actions.browser_control import _BrowserSession
        session = _BrowserSession.__new__(_BrowserSession)
        good, bad = MagicMock(), MagicMock()
        good.count = AsyncMock(return_value=1)
        bad.count = AsyncMock(return_value=0)
        good.fill = AsyncMock()
        page = MagicMock()
        page.locator.side_effect = [good, bad]
        session._get_page = AsyncMock(return_value=page)
        result = await session.fill_form({"#name": "A", "#missing": "B"})
        self.assertTrue(result.startswith("Error:"))
        good.fill.assert_not_called()


class TestDataAndControlBusinessLogic(unittest.TestCase):
    def test_uncertain_text_set_does_not_repeat_with_keyboard(self):
        from core import uia
        control = uia.Control("Message", rect=(1, 1, 20, 20))
        element = MagicMock()
        element.iface_value.CurrentValue = "Hello"
        element.set_edit_text.side_effect = RuntimeError("Disconnected after write")
        with patch.object(uia, "connect"), patch.object(uia, "find", return_value=control), \
                patch.object(uia, "_locate", return_value=element):
            result = uia.type_text("App", "Message", " world")
        self.assertTrue(result.startswith("Error:"))
        element.set_focus.assert_not_called()

    def test_explicit_row_count_must_match_data(self):
        from actions.excel_data import matrix
        with self.assertRaises(ValueError):
            matrix({"headers": ["A"], "row_count": 5, "data": [["1"]]})

    def test_formula_header_is_written_as_literal_text(self):
        from actions.excel_data import matrix
        self.assertEqual(matrix({"headers": ["=1+1"], "data": [["1"]]})[0], ["'=1+1"])

    def test_uncertain_uia_invoke_is_not_followed_by_another_click(self):
        from core import uia
        control = uia.Control("Send", rect=(1, 1, 20, 20))
        element = MagicMock()
        element.invoke.side_effect = RuntimeError("Lost connection after invoke")
        with patch.object(uia, "connect"), patch.object(uia, "find", return_value=control), \
                patch.object(uia, "_locate", return_value=element):
            result = uia.click("App", "Send")
        self.assertTrue(result.startswith("Error:"))
        element.click_input.assert_not_called()
