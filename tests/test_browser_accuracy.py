"""Regression coverage for false website-open and scroll success reports."""
import unittest
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

from PIL import Image

from actions import browser_control as browser
from actions.open_app import open_app
from core.browser_scroll import movement, scroll_native


class WebsiteOpening(unittest.TestCase):
    def test_full_website_urls_are_preserved(self):
        url = "https://www.linkedin.com/feed/update/urn:li:activity:123?source=notifications"
        self.assertEqual(browser._normalize_url(url), url)

    def test_youtube_search_uses_encoded_youtube_url(self):
        with patch.object(browser, "_native_browser_window", return_value=True), \
                patch.object(browser._registry, "has", return_value=False), \
                patch.object(browser, "_navigate_native_url", return_value="verified") as navigate:
            browser.browser_control({"action": "search", "engine": "youtube", "query": "C++ & python"})
        navigate.assert_called_once_with("https://www.youtube.com/results?search_query=C%2B%2B+%26+python")

    def test_native_launch_without_a_window_is_not_success(self):
        from actions import open_app as apps
        with patch.object(apps, "_SYSTEM", "Windows"), \
                patch("core.desktop_target.focus", side_effect=RuntimeError("missing")), \
                patch.object(apps.time, "sleep"):
            self.assertTrue(apps._verify_launch("Unknown App", "Unknown App").startswith("Error:"))

    def test_youtube_navigates_even_when_chrome_is_open(self):
        for name in ("youtube", "yt", "youtube.com"):
            with self.subTest(name=name), \
                    patch("actions.computer_control._focus_window", return_value="Focused window: Chrome") as focus, \
                    patch.object(browser, "browser_control", return_value="verified youtube") as navigate:
                self.assertEqual(open_app({"app_name": name}), "verified youtube")
                navigate.assert_called_once_with({"action": "go_to", "url": "https://www.youtube.com"}, player=None)
                focus.assert_not_called()

    def test_browser_launch_is_not_proof_of_navigation(self):
        with patch.object(browser._registry, "has", return_value=False), \
                patch.object(browser, "_native_browser_window", return_value=None), \
                patch.object(browser, "_open_native", return_value="Opened in your default browser: url"), \
                patch.object(browser, "_verify_native_url", return_value="Error: Navigation not verified."):
            result = browser.browser_control({"action": "go_to", "url": "https://www.youtube.com"})
        self.assertTrue(result.startswith("Error:"))

    def test_query_readback_must_match(self):
        with patch.object(browser, "_native_browser_window", return_value=True), \
                patch.object(browser, "_read_native_url", return_value="https://www.youtube.com/results?search_query=wrong"), \
                patch.object(browser.time, "sleep"):
            self.assertTrue(browser._verify_native_url("https://www.youtube.com/results?search_query=python").startswith("Error:"))

    def test_delayed_navigation_is_verified(self):
        with patch.object(browser, "_native_browser_window", return_value=True), \
                patch.object(browser, "_read_native_url", side_effect=["https://www.linkedin.com/feed", "https://www.youtube.com/"]), \
                patch.object(browser.time, "sleep"):
            self.assertIn("verified", browser._verify_native_url("https://www.youtube.com"))


class ScrollEvidence(unittest.TestCase):
    def images(self):
        import random
        randomizer = random.Random(7)
        image = Image.frombytes("L", (320, 600), randomizer.randbytes(320 * 600))
        before = image.crop((0, 0, 320, 400))
        after = image.crop((0, 80, 320, 480))
        return before, after

    def test_translation_must_match_requested_direction(self):
        before, after = self.images()
        self.assertEqual(movement(before, after, "down"), 80)
        self.assertEqual(movement(after, before, "up"), 80)
        self.assertIsNone(movement(before, after, "up"))

    def test_static_page_and_animation_are_not_scroll_proof(self):
        before, _ = self.images()
        self.assertIsNone(movement(before, before, "down"))
        after = before.copy()
        after.paste(255, (10, 10, 150, 80))
        self.assertIsNone(movement(before, after, "down"))

    def test_large_scroll_retains_enough_overlap_for_proof(self):
        import random
        image = Image.frombytes("L", (320, 1000), random.Random(8).randbytes(320 * 1000))
        self.assertEqual(movement(image.crop((0, 0, 320, 600)),
                                  image.crop((0, 352, 320, 952)), "down"), 352)

    def test_scroll_distance_need_not_be_a_multiple_of_four(self):
        import random
        image = Image.frombytes("L", (320, 600), random.Random(9).randbytes(320 * 600))
        self.assertEqual(movement(image.crop((0, 0, 320, 400)),
                                  image.crop((0, 81, 320, 481)), "down"), 81)

    def test_mouse_moves_into_target_window_content(self):
        active = SimpleNamespace(hwnd=42, app_category="browser", title="LinkedIn", rect=(900, 100, 1700, 900))
        before, after = self.images()
        with patch("core.universal_operator.get_active_window", return_value=active), \
                patch("core.browser_scroll.content_rect", return_value=(920, 240, 1680, 855)), \
                patch("core.browser_scroll.time.sleep"), \
                patch("pyautogui.moveTo") as move, patch("pyautogui.scroll") as wheel, \
                patch("pyautogui.screenshot", side_effect=[before, after, before, after]):
            result = scroll_native()
        self.assertIn("Verified", result)
        self.assertEqual(move.call_count, 2)
        move.assert_called_with(1300, 547)
        self.assertEqual([call.args for call in wheel.call_args_list], [(-2,), (-2,)])
        self.assertEqual(result.evidence["content_movement_px"], 160)

    def test_changed_but_unverified_content_is_not_scrolled_again(self):
        active = SimpleNamespace(hwnd=42, app_category="browser", title="LinkedIn", rect=(0, 0, 1000, 800))
        before, after = self.images()
        with patch("core.universal_operator.get_active_window", return_value=active), \
                patch("core.browser_scroll.content_rect", return_value=(20, 140, 980, 755)), \
                patch("core.browser_scroll.time.sleep"), patch("pyautogui.moveTo"), \
                patch("core.browser_scroll.movement", return_value=None), \
                patch("pyautogui.scroll") as wheel, \
                patch("pyautogui.screenshot", side_effect=[before, after]):
            result = scroll_native()
        self.assertIn("no automatic repeat", result)
        wheel.assert_called_once()

    def test_no_movement_returns_error_after_bounded_retry(self):
        active = SimpleNamespace(hwnd=42, app_category="browser", title="LinkedIn", rect=(0, 0, 1000, 800))
        before, _ = self.images()
        with patch("core.universal_operator.get_active_window", return_value=active), \
                patch("core.browser_scroll.content_rect", return_value=(20, 140, 980, 755)), \
                patch("core.browser_scroll.time.sleep"), patch("pyautogui.moveTo"), \
                patch("pyautogui.scroll") as wheel, patch("pyautogui.screenshot", return_value=before):
            self.assertTrue(scroll_native().startswith("Error:"))
        self.assertEqual(wheel.call_count, 2)

    def test_native_tab_wins_over_existing_automation_session(self):
        with patch.object(browser, "_native_browser_window", return_value=True), \
                patch.object(browser._registry, "get") as session, \
                patch("core.browser_scroll.scroll_native", return_value="Verified movement") as scroll:
            result = browser.browser_control({"action": "scroll"})
        self.assertIn("Verified", result)
        scroll.assert_called_once()
        session.assert_not_called()

    def test_wrong_foreground_app_is_not_scrolled(self):
        with patch("core.universal_operator.get_active_window", return_value=SimpleNamespace(app_category="generic")), \
                patch("pyautogui.scroll") as wheel:
            self.assertTrue(scroll_native().startswith("Error:"))
        wheel.assert_not_called()


class ConnectedScroll(unittest.IsolatedAsyncioTestCase):
    async def test_scroll_requires_actual_offset_change(self):
        for changed in (False, True):
            page = MagicMock()
            page.url = "https://example.com"
            page.evaluate = AsyncMock(side_effect=[
                [{"top": 0, "tag": "HTML"}], {"width": 1000, "height": 800},
                [{"top": 500 if changed else 0, "tag": "HTML"}]])
            page.mouse.move = AsyncMock()
            page.mouse.wheel = AsyncMock()
            session = browser._BrowserSession.__new__(browser._BrowserSession)
            session._get_page = AsyncMock(return_value=page)
            with patch.object(browser.asyncio, "sleep", new=AsyncMock()):
                result = await session.scroll()
            self.assertEqual(result.startswith("Verified"), changed)
