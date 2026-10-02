import unittest

from actions.app_control import TOOL, app_control
from core.action_loader import _validate


class TestToolShape(unittest.TestCase):
    def test_tool_validates(self):
        rec = _validate(TOOL, "app_control.py")
        self.assertTrue(rec.valid, rec.error)
        self.assertEqual(rec.name, "app_control")

    def test_handler_is_the_dispatch(self):
        self.assertIs(TOOL["handler"], app_control)


class TestDispatch(unittest.TestCase):
    def test_missing_action(self):
        self.assertIn("required", app_control({"app": "x"}))

    def test_missing_app(self):
        self.assertIn("'app' is required", app_control({"action": "click"}))

    def test_unknown_action_lists_valid_ones(self):
        out = app_control({"action": "teleport", "app": "Notepad"})
        self.assertIn("Unknown action", out)
        self.assertIn("elements", out)

    def test_type_without_text(self):
        out = app_control({"action": "type", "app": "Notepad"})
        self.assertIn("'text' is required", out)

    def test_click_without_element(self):
        out = app_control({"action": "click", "app": "Notepad"})
        self.assertIn("'element' is required", out)


class TestLiveReadonly(unittest.TestCase):
    """Read-only calls that touch the real desktop — safe on any Windows box."""

    def test_windows_listing(self):
        out = app_control({"action": "windows"})
        self.assertIsInstance(out, str)
        self.assertTrue(out.startswith("Open windows:") or
                        out.startswith("No visible windows"))

    def test_launch_reports_result(self):
        # Launching Notepad is harmless; we only assert we get a string back.
        out = app_control({"action": "launch", "app": "notepad"})
        self.assertIsInstance(out, str)
        self.assertTrue(len(out) > 0)


if __name__ == "__main__":
    unittest.main()
