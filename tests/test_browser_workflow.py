"""Browser actions must stay in the user's current window and preserve other tabs."""
import unittest
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

from actions import browser_control as browser


class TestNativeBrowserWorkflow(unittest.TestCase):
    def test_new_tab_uses_foreground_browser_without_launching_chrome(self):
        active = SimpleNamespace(app_category="browser", title="LinkedIn - Google Chrome")
        with patch.object(browser, "_native_browser_window", return_value=active), \
                patch.object(browser, "_open_native") as launch, \
                patch("pyautogui.hotkey") as hotkey:
            result = browser.browser_control({"action": "new_tab"})
        self.assertIn("active browser", result)
        hotkey.assert_called_once_with("ctrl", "t")
        launch.assert_not_called()

    def test_new_tab_without_browser_does_not_launch_one(self):
        with patch.object(browser, "_native_browser_window", return_value=None), \
                patch.object(browser._registry, "has", return_value=False), \
                patch.object(browser, "_open_native") as launch:
            result = browser.browser_control({"action": "new_tab"})
        self.assertTrue(result.startswith("Error:"))
        launch.assert_not_called()

    def test_short_linkedin_path_is_rejected(self):
        with patch.object(browser, "_open_native") as launch:
            result = browser.browser_control({"action": "go_to", "url": "https://www.linkedin.com/fee"})
        self.assertTrue(result.startswith("Error:"))
        launch.assert_not_called()

    def test_foreground_navigation_does_not_launch_new_window(self):
        active = SimpleNamespace(app_category="browser", title="Chrome")
        with patch.object(browser, "_native_browser_window", return_value=active), \
                patch.object(browser, "_navigate_native_url", return_value="Active browser tab address verified: https://www.linkedin.com/feed"), \
                patch.object(browser, "_open_native") as launch:
            result = browser.browser_control({"action": "go_to", "url": "https://www.linkedin.com/feed"})
        self.assertIn("verified", result)
        launch.assert_not_called()

    def test_navigation_rejects_wrong_section_on_same_site(self):
        active = SimpleNamespace(app_category="browser", title="Chrome")
        with patch.object(browser, "_native_browser_window", return_value=active), \
                patch.object(browser, "_read_native_url", return_value="https://www.linkedin.com/messaging"), \
                patch("pyautogui.hotkey"), patch("pyautogui.press"), \
                patch("pyperclip.paste", return_value="old"), patch("pyperclip.copy"), \
                patch.object(browser.time, "sleep"):
            result = browser._navigate_native_url("https://www.linkedin.com/feed")
        self.assertTrue(result.startswith("Error:"))

    def test_native_domain_close_refuses_to_close_window(self):
        with patch.object(browser._registry, "has", return_value=False), \
                patch("actions.computer_control._close_window") as close_window:
            result = browser.browser_control({"action": "close_tabs", "domain": "linkedin.com"})
        self.assertTrue(result.startswith("Error:"))
        close_window.assert_not_called()

    def test_named_site_cannot_close_unrelated_foreground_tab(self):
        with patch.object(browser._registry, "has", return_value=False), \
                patch("pyautogui.hotkey") as hotkey:
            result = browser.browser_control({"action": "close_tab", "domain": "linkedin.com"})
        self.assertTrue(result.startswith("Error:"))
        hotkey.assert_not_called()

    def test_window_close_requires_explicit_scope(self):
        with patch("actions.computer_control._close_window") as close_window:
            result = browser.browser_control({"action": "close_all"})
        self.assertTrue(result.startswith("Error:"))
        close_window.assert_not_called()

    def test_continuous_scroll_can_be_stopped(self):
        active = SimpleNamespace(app_category="browser", title="Chrome")
        with patch.object(browser, "_native_browser_window", return_value=active), \
                patch.object(browser, "_continuous_scroll", side_effect=lambda: browser._scroll_stop.wait(1)):
            self.assertIn("started", browser.browser_control({"action": "start_scroll"}))
            self.assertIn("stopped", browser.browser_control({"action": "stop_scroll"}))
            browser._scroll_thread.join(timeout=1)
            self.assertFalse(browser._scroll_thread.is_alive())


class TestConnectedBrowserWorkflow(unittest.IsolatedAsyncioTestCase):
    async def test_all_matching_tabs_leave_a_blank_tab(self):
        session = browser._BrowserSession.__new__(browser._BrowserSession)
        pages = []
        context = MagicMock()
        context.pages = pages
        linkedin = MagicMock()
        linkedin.url = "https://www.linkedin.com/feed"
        linkedin.context = context
        linkedin.is_closed.return_value = False
        async def close():
            linkedin.is_closed.return_value = True
            pages.remove(linkedin)
        linkedin.close = AsyncMock(side_effect=close)
        pages.append(linkedin)
        blank = MagicMock()
        blank.url = "about:blank"
        blank.is_closed.return_value = False
        async def new_page():
            pages.append(blank)
            return blank
        context.new_page = AsyncMock(side_effect=new_page)
        session._page = linkedin
        session._get_page = AsyncMock(return_value=linkedin)
        result = await session.close_tabs_matching("linkedin.com")
        self.assertIn("Closed 1", result)
        self.assertEqual(pages, [blank])

    async def test_close_only_matching_tabs_and_keep_other_tab(self):
        session = browser._BrowserSession.__new__(browser._BrowserSession)
        pages = []
        context = MagicMock()
        context.pages = pages
        for url in ("https://www.linkedin.com/feed", "https://example.com", "https://www.linkedin.com/messaging"):
            page = MagicMock()
            page.url = url
            page.context = context
            page.is_closed.return_value = False
            async def close(tab=page):
                tab.is_closed.return_value = True
                pages.remove(tab)
            page.close = AsyncMock(side_effect=close)
            pages.append(page)
        session._page = pages[0]
        session._get_page = AsyncMock(return_value=pages[0])
        result = await session.close_tabs_matching("linkedin.com")
        self.assertIn("Closed 2", result)
        self.assertEqual(len(pages), 1)
        self.assertEqual(pages[0].url, "https://example.com")


class TestVolumeWorkflow(unittest.TestCase):
    def test_settings_cannot_close_unrelated_tab(self):
        from actions import computer_settings as settings
        with patch.object(settings, "close_tab") as close:
            result = settings.computer_settings({"action": "close_tab"})
        self.assertTrue(result.startswith("Error:"))
        close.assert_not_called()

    def test_current_volume_is_available_as_tool_action(self):
        from actions import computer_settings as settings
        with patch.object(settings, "_PYAUTOGUI", True), patch.object(settings, "volume_get", return_value=47):
            result = settings.computer_settings({"action": "volume_get"})
        self.assertIn("47%", result)

    def test_volume_set_requires_readback(self):
        from actions import computer_settings as settings
        with patch.object(settings, "_PYAUTOGUI", True), \
                patch.object(settings, "volume_get", side_effect=[20, 20]), \
                patch.object(settings, "volume_set"):
            result = settings.computer_settings({"action": "volume_set", "value": "50"})
        self.assertTrue(result.startswith("Error:"))
