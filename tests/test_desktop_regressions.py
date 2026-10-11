import asyncio
import unittest
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

from actions.excel_data import matrix, write_data
from actions.excel_control import _dispatch_excel
from core import universal_operator as operator
from core.task_runtime import TaskRuntime


class TestExcelData(unittest.TestCase):
    def test_months_are_columns_and_five_rows_are_rows(self):
        result = matrix({"headers": [str(i) for i in range(12)], "row_count": 5, "random_values": True})
        self.assertEqual(len(result), 6)
        self.assertTrue(all(len(row) == 12 for row in result))
        self.assertTrue(all(isinstance(v, int) for row in result[1:] for v in row))

    def test_jagged_rows_and_formula_text(self):
        with self.assertRaises(ValueError):
            matrix({"headers": ["A", "B"], "data": [["1"]]})
        self.assertEqual(matrix({"data": [["=1+1", "003", "12"]]}), [["'=1+1", "003", 12]])

    def test_bulk_write_creates_real_table_and_reads_back(self):
        excel, sheet, target = MagicMock(), MagicMock(), MagicMock()
        sheet.ListObjects.Count = 0
        origin = MagicMock()
        origin.Row = origin.Column = 1
        sheet.Range.side_effect = [origin, target]
        target.Rows.Count, target.Columns.Count = 4, 2
        target.Address, sheet.Name = "$A$1:$B$4", "Sheet1"
        excel.WorksheetFunction.CountA.return_value = 0
        table = sheet.ListObjects.Add.return_value
        table.ListRows.Count, table.ListColumns.Count, table.Name = 3, 2, "Table1"
        result = write_data(excel, sheet, {"headers": ["Jan", "Feb"],
                            "data": [["1", "2"], ["3", "4"], ["5", "6"]]}, True)
        self.assertEqual(target.Value2, (("Jan", "Feb"), (1, 2), (3, 4), (5, 6)))
        self.assertEqual(result.evidence["data_rows"], 3)
        sheet.ListObjects.Add.assert_called_once()

    def test_missing_workbook_never_runs_keyboard_fallback(self):
        with patch("actions.excel_control._get_active_excel", return_value=(None, None, None)), \
                patch("actions.excel_control._keyboard_autofit") as fallback:
            self.assertTrue(_dispatch_excel({"action": "format_table"}).startswith("Error:"))
        fallback.assert_not_called()


class TestTargeting(unittest.TestCase):
    def test_window_maximize_requires_actual_win32_state(self):
        from core.desktop_target import window_state
        import win32con
        with patch("core.desktop_target.focus", return_value=123), \
                patch("win32gui.ShowWindow") as show, \
                patch("win32gui.GetWindowPlacement", return_value=(0, win32con.SW_SHOWMAXIMIZED)), \
                patch("win32gui.GetWindowText", return_value="Excel"):
            result = window_state("maximize", "Excel")
        show.assert_called_once_with(123, win32con.SW_MAXIMIZE)
        self.assertEqual(result.evidence["window_state"], "maximize")

    def test_old_screen_cache_is_never_used_for_click_target(self):
        from actions import computer_control as cc
        with patch.object(cc, "_LAST_SCREEN_ELEMENTS", [{"label": "Send", "x": 50, "y": 60}]), \
                patch("core.uia.available", return_value=False), \
                patch.object(cc, "_get_api_key", return_value=""), \
                patch.object(operator, "get_active_window", return_value=operator.WindowContext(title="Other app")):
            self.assertIsNone(cc._screen_find("Send"))

    def test_foreign_target_focus_failure_stops_input(self):
        from actions.computer_control import computer_control
        with patch("actions.computer_control._focus_window", return_value="Error: not focused"), \
                patch("actions.computer_control._type") as type_text:
            result = computer_control({"action": "type", "text": "x", "title": "Excel"})
        self.assertTrue(result.startswith("Error:"))
        type_text.assert_not_called()

    def test_grounded_click_rejects_changed_window(self):
        from actions import computer_control as cc
        import time
        saved = {"point": (20, 30), "hwnd": 1, "rect": (0, 0, 100, 100), "title": "A", "time": time.monotonic()}
        with patch.object(cc, "_LAST_GROUNDING", saved), patch.object(cc, "_require_pyautogui"), \
                patch.object(cc.pyautogui, "click") as click, \
                patch.object(operator, "get_active_window", return_value=operator.WindowContext(hwnd=2, title="B")):
            with self.assertRaises(RuntimeError):
                cc._click(20, 30)
        click.assert_not_called()

    def test_unknown_intent_does_not_guess_shortcut(self):
        self.assertIsNone(operator._match_intent("zoom", ""))
        self.assertIsNone(operator._match_intent("generic", "do not close this"))
        self.assertIsNone(operator._match_intent("excel", "table banao"))

    def test_native_browser_typing_requires_browser_focus(self):
        from actions.browser_control import browser_control
        with patch("actions.browser_control._registry.has", return_value=False), \
                patch.object(operator, "get_active_window", return_value=operator.WindowContext(title="Excel", app_category="excel")):
            self.assertTrue(browser_control({"action": "type", "text": "hello"}).startswith("Error:"))


class TestLiveOutcome(unittest.IsolatedAsyncioTestCase):
    async def test_dom_duplicate_button_is_not_clicked(self):
        from actions.browser_control import _BrowserSession
        session = _BrowserSession.__new__(_BrowserSession)
        locator = MagicMock()
        locator.count = AsyncMock(return_value=2)
        locator.click = AsyncMock()
        page = MagicMock()
        page.get_by_text.return_value = locator
        session._get_page = AsyncMock(return_value=page)
        result = await session.click(text="Submit")
        self.assertTrue(result.startswith("Error:"))
        locator.click.assert_not_called()

    def runtime(self, execute):
        return TaskRuntime(planner=lambda _: {}, execute=execute, declarations=lambda: [],
                           observe=AsyncMock(return_value={}), verify=MagicMock())

    async def test_native_evidence_survives_dispatch(self):
        execute = AsyncMock(return_value=SimpleNamespace(response={"result": "Written", "native_verified": True,
                                   "evidence": {"data_rows": 5, "columns": 12}}, scheduling=None))
        result = await self.runtime(execute).call("excel_control", {"action": "create_table"})
        self.assertTrue(result["verified"])

    async def test_unverified_direct_repeat_requires_observation(self):
        execute = AsyncMock(return_value="Clicked")
        runtime = self.runtime(execute)
        args = {"action": "click", "element": "Publish"}
        await runtime.call("app_control", args)
        second = await runtime.call("app_control", args)
        self.assertEqual(second["status"], "failed")
        self.assertEqual(execute.await_count, 1)
        await runtime.call("app_control", {"action": "state"})
        await runtime.call("app_control", args)
        self.assertEqual(execute.await_count, 3)


if __name__ == "__main__":
    unittest.main()
