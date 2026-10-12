import json
import sys
import unittest
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

from PIL import Image
from actions import app_control as apps, browser_control as browser, computer_control as computer, open_app as opening
from core import desktop_target, uia, universal_operator


class WindowClosure(unittest.TestCase):
    def gui(self):
        gui = MagicMock()
        gui.GetWindowText.return_value = "Test app"
        gui.IsWindow.return_value = True
        gui.IsWindowVisible.return_value = True
        return gui

    def test_failed_focus_never_requests_closure(self):
        gui = self.gui()
        with patch.dict(sys.modules, {"win32gui": gui, "win32con": SimpleNamespace(WM_CLOSE=16)}), \
                patch.object(desktop_target, "focus", side_effect=RuntimeError("missing")):
            self.assertTrue(desktop_target.close_window("missing").startswith("Error:"))
        gui.PostMessage.assert_not_called()

    def test_close_is_sent_to_identified_handle_and_verified(self):
        gui = self.gui()
        gui.IsWindow.return_value = False
        with patch.dict(sys.modules, {"win32gui": gui, "win32con": SimpleNamespace(WM_CLOSE=16)}), \
                patch.object(desktop_target, "focus", return_value=42):
            result = desktop_target.close_window("Test app")
        self.assertTrue(result.evidence["closed"])
        gui.PostMessage.assert_called_once_with(42, 16, 0, 0)

    def test_save_prompt_is_not_claimed_closed_or_retried(self):
        gui = self.gui()
        with patch.dict(sys.modules, {"win32gui": gui, "win32con": SimpleNamespace(WM_CLOSE=16)}), \
                patch.object(desktop_target, "focus", return_value=42), patch.object(desktop_target.time, "sleep"):
            result = desktop_target.close_window()
        self.assertIn("save prompt", result)
        gui.PostMessage.assert_called_once()

    def test_website_name_does_not_close_browser_window(self):
        with patch.object(desktop_target, "close_window") as close:
            self.assertTrue(computer._close_window("LinkedIn").startswith("Error:"))
        close.assert_not_called()


class ApplicationOpening(unittest.TestCase):
    def test_unrelated_name_does_not_match_an_alias_substring(self):
        self.assertEqual(opening._normalize("my word processor"), "my word processor")

    def test_browser_tab_named_excel_is_not_excel(self):
        gui = MagicMock()
        gui.IsWindowVisible.return_value = True
        gui.GetWindowText.return_value = "Excel - Google Chrome"
        gui.EnumWindows.side_effect = lambda callback, arg: callback(42, arg)
        process = MagicMock()
        process.GetWindowThreadProcessId.return_value = (1, 123)
        with patch.dict(sys.modules, {"win32gui": gui, "win32process": process}), \
                patch.object(opening, "_SYSTEM", "Windows"), \
                patch.object(opening.psutil, "Process", return_value=SimpleNamespace(name=lambda: "chrome.exe")), \
                patch.object(desktop_target, "focus") as focus:
            self.assertIsNone(opening._focus_existing_app("Excel", "excel"))
        focus.assert_not_called()

    def test_launch_failure_is_not_overwritten_by_uia_title(self):
        with patch("actions.open_app.open_app", return_value="Error: launch failed"), \
                patch.object(uia, "window_info") as inspect:
            self.assertTrue(apps._launch("Excel").startswith("Error:"))
        inspect.assert_not_called()


class MouseAndTyping(unittest.TestCase):
    def tearDown(self):
        computer._LAST_GROUNDING = None

    def test_unicode_typing_does_not_use_ascii_key_injection(self):
        with patch("core.uia._paste") as paste, patch.object(computer.time, "sleep"), \
                patch.object(computer.pyautogui, "typewrite") as ascii_type:
            computer._type("বাংলা")
            computer._smart_type("হাই", clear_first=False)
        self.assertEqual(paste.call_count, 2)
        ascii_type.assert_not_called()

    def test_missing_right_click_target_never_left_clicks(self):
        with patch.object(computer, "_screen_find", return_value=None), \
                patch.object(universal_operator, "execute_smart_operation") as fallback:
            result = computer.computer_control({"action": "right_click", "description": "missing button"})
        self.assertTrue(result.startswith("Error:"))
        fallback.assert_not_called()

    def test_visual_double_click_preserves_click_count_without_uia(self):
        with patch.object(computer, "_screen_find", return_value=(20, 30)), \
                patch.object(computer, "_click") as click, patch.object(uia, "connect") as connect:
            apps._vision_fallback("Chrome", "specific control", clicks=2)
        click.assert_called_once_with(20, 30, smooth=True, clicks=2)
        connect.assert_not_called()

    def test_missing_input_never_types_into_current_app(self):
        with patch.object(computer, "_screen_find", return_value=None), \
                patch.object(computer, "_type") as typing, patch.object(computer, "_smart_type") as smart, \
                patch.object(universal_operator, "get_active_window", return_value=SimpleNamespace(app_category="browser")):
            for action in ("type", "smart_type"):
                result = computer.computer_control({"action": action, "description": "missing field", "text": "sample"})
                self.assertTrue(result.startswith("Error:"))
        typing.assert_not_called()
        smart.assert_not_called()

    def test_window_changed_during_glide_never_clicks(self):
        original = universal_operator.WindowContext(hwnd=1, title="Chrome", rect=(0, 0, 100, 100))
        changed = universal_operator.WindowContext(hwnd=2, title="Other", rect=(0, 0, 100, 100))
        computer._LAST_GROUNDING = {"point": (20, 30), "hwnd": 1, "title": "Chrome", "rect": original.rect,
                                    "time": computer.time.monotonic()}
        with patch.object(universal_operator, "get_active_window", side_effect=[original, changed]), \
                patch.object(computer.pyautogui, "moveTo"), patch.object(computer.pyautogui, "click") as click:
            with self.assertRaises(RuntimeError):
                computer._click(20, 30)
        click.assert_not_called()

    def test_changed_button_patch_never_clicks(self):
        original = universal_operator.WindowContext(hwnd=1, title="Chrome", rect=(0, 0, 100, 100))
        computer._LAST_GROUNDING = {"point": (20, 30), "hwnd": 1, "title": "Chrome", "rect": original.rect,
            "time": computer.time.monotonic(), "patch": Image.new("RGB", (32, 32), "white"), "patch_region": (4, 14, 32, 32)}
        with patch.object(universal_operator, "get_active_window", return_value=original), \
                patch.object(computer.pyautogui, "screenshot", return_value=Image.new("RGB", (32, 32), "black")), \
                patch.object(computer.pyautogui, "click") as click:
            with self.assertRaises(RuntimeError):
                computer._click(20, 30)
        click.assert_not_called()

    def test_contextual_post_target_requires_visual_grounding(self):
        with patch.object(uia, "connect") as connect:
            with self.assertRaisesRegex(RuntimeError, "visual grounding"):
                uia.find("Chrome", "Like below this post")
        connect.assert_not_called()

    def test_uncertain_uia_action_is_not_replayed(self):
        ctx = universal_operator.WindowContext(title="Chrome")
        with patch.object(universal_operator, "get_active_window", return_value=ctx), \
                patch.object(uia, "available", return_value=True), \
                patch.object(uia, "click", return_value="Error: Invoke outcome uncertain"), \
                patch.object(computer, "_screen_find") as visual:
            result = universal_operator.execute_smart_operation(intent="Like")
        self.assertTrue(result.startswith("Error:"))
        visual.assert_not_called()

    def test_repeated_button_labels_are_preserved_for_inspection(self):
        window = MagicMock()
        window.window_text.return_value = "Feed"
        controls = [uia.Control("Like", "Button", rect=(0, 0, 40, 20)),
                    uia.Control("Like", "Button", rect=(0, 100, 40, 120))]
        with patch.object(uia, "connect", return_value=window), patch.object(uia, "_iter_controls", return_value=controls):
            self.assertEqual(len(uia.list_controls("Feed")[1]), 2)

    def test_visual_verification_requires_boolean_and_observed_evidence(self):
        ctx = universal_operator.WindowContext(title="Chrome")
        with patch.object(computer, "_get_api_key", return_value="test-key"), \
                patch.object(universal_operator, "get_active_window", return_value=ctx), \
                patch.object(computer.pyautogui, "screenshot", return_value=Image.new("RGB", (10, 10))), \
                patch("core.gemini.call") as generate:
            for matched, evidence, successful in (("true", "selected", False), (True, "", False),
                                                    (False, "menu only", False), (True, "Love selected on target post", True)):
                generate.return_value = SimpleNamespace(text=json.dumps({"matched": matched, "observation": evidence}))
                result = computer._verify_screen("Love selected on the exact post")
                self.assertEqual(result.startswith("Visually verified"), successful)


class BrowserHover(unittest.IsolatedAsyncioTestCase):
    async def test_ambiguous_hover_does_not_select_first_post(self):
        page = MagicMock()
        locator = MagicMock()
        locator.count = AsyncMock(return_value=2)
        locator.hover = AsyncMock()
        page.get_by_role.return_value = locator
        session = browser._BrowserSession.__new__(browser._BrowserSession)
        session._get_page = AsyncMock(return_value=page)
        result = await session.hover("Like")
        self.assertIn("ambiguous", result)
        locator.hover.assert_not_called()

    async def test_selected_state_needs_actual_attribute_readback(self):
        page = MagicMock()
        locator = MagicMock()
        locator.count = AsyncMock(return_value=1)
        locator.is_visible = AsyncMock(return_value=True)
        locator.get_attribute = AsyncMock(return_value="false")
        page.locator.return_value = locator
        session = browser._BrowserSession.__new__(browser._BrowserSession)
        session._get_page = AsyncMock(return_value=page)
        self.assertTrue((await session.verify_state("#post .Love", "aria-pressed", "true")).startswith("Error:"))
        locator.get_attribute.return_value = "true"
        self.assertTrue((await session.verify_state("#post .Love", "aria-pressed", "true")).startswith("Verified"))
