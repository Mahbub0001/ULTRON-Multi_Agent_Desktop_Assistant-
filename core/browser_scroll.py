"""Scroll the foreground browser's content and require evidence of movement."""
from __future__ import annotations

import time

from core.tool_result import VerifiedResult


def content_rect(active):
    """Use the visible document, with a window-relative fallback for missing UIA."""
    left, top, right, bottom = active.rect
    try:
        from pywinauto import Desktop
        window = Desktop(backend="uia").window(handle=active.hwnd).wrapper_object()
        documents = []
        for doc in window.descendants(control_type="Document"):
            rect = doc.rectangle()
            if doc.is_visible() and rect.width() > 250 and rect.height() > 200:
                if left <= rect.left < rect.right <= right and top <= rect.top < rect.bottom <= bottom:
                    documents.append((rect.width() * rect.height(), (rect.left, rect.top, rect.right, rect.bottom)))
        if documents:
            return max(documents, key=lambda item: item[0])[1]
    except Exception:
        pass
    return left + 20, top + 140, right - 20, bottom - 45


def movement(before, after, direction):
    """Require coherent vertical translation; animation alone is insufficient."""
    from PIL import ImageChops, ImageStat
    if before.size != after.size:
        return None
    width = min(320, before.width)
    height = max(1, round(before.height * width / before.width))
    a = before.convert("L").resize((width, height))
    b = after.convert("L").resize((width, height))
    baseline = ImageStat.Stat(ImageChops.difference(a, b)).mean[0]
    if baseline < 2:
        return None
    best = None
    # Wheel distance and display scaling rarely land on multiples of four.
    for shift in range(1, max(1, height - max(80, height // 4))):
        if direction == "down":
            old = a.crop((0, shift, width, height))
            new = b.crop((0, 0, width, height - shift))
        else:
            old = a.crop((0, 0, width, height - shift))
            new = b.crop((0, shift, width, height))
        error = ImageStat.Stat(ImageChops.difference(old, new)).mean[0]
        if error < min(8, baseline * 0.55) and (best is None or error < best[0]):
            best = (error, shift)
    return round(best[1] * before.width / width) if best else None


def scroll_native(direction="down", amount=500):
    import pyautogui
    from core.universal_operator import get_active_window
    direction = str(direction).strip().lower()
    if direction not in ("down", "up"):
        return "Error: Scroll direction must be down or up."
    try:
        amount = max(120, min(2400, abs(int(amount))))
        active = get_active_window()
        if active.app_category != "browser":
            return "Error: Focus the intended browser before scrolling."
        left, top, right, bottom = content_rect(active)
        # Capture the central feed, excluding fixed sidebars and browser chrome.
        width, height = right - left, bottom - top
        if width < 250 or height < 200:
            return "Error: Browser content is not sufficiently visible to scroll."
        region = (left + int(width * .32), top + 20, int(width * .36), height - 40)
        # Keep enough shared content between captures to prove translation.
        # A large wheel burst can replace the entire feed and defeat read-back.
        remaining = max(1, min(20, round(amount / 120)))
        chunk = max(1, min(2, (height - 40) // 240))
        total_pixels = 0
        while remaining:
            clicks = min(chunk, remaining) * (-1 if direction == "down" else 1)
            pixels = _scroll_chunk(active, region, left, top, width, height, clicks, direction)
            if isinstance(pixels, str):
                return pixels
            total_pixels += pixels
            remaining -= abs(clicks)
        return VerifiedResult(f"Verified browser content moved {direction} (approximately {total_pixels}px).",
                              {"direction": direction, "content_movement_px": total_pixels})
    except Exception as exc:
        return f"Error: Browser scroll could not be verified: {exc}."


def _scroll_chunk(active, region, left, top, width, height, clicks, direction):
    import pyautogui
    from PIL import ImageChops
    from core.universal_operator import get_active_window

    for fraction in (.5, .6):
        current = get_active_window()
        if current.hwnd != active.hwnd or current.rect != active.rect or current.title != active.title:
            return "Error: Browser target changed; inspect before retrying."
        pyautogui.moveTo(left + int(width * fraction), top + height // 2)
        time.sleep(.08)
        before = pyautogui.screenshot(region=region)
        pyautogui.scroll(clicks)
        time.sleep(.4)
        after = pyautogui.screenshot(region=region)
        current = get_active_window()
        if current.hwnd != active.hwnd or current.rect != active.rect or current.title != active.title:
            return "Error: Browser target changed during scrolling; outcome uncertain."
        pixels = movement(before, after, direction)
        if pixels:
            return pixels
        if ImageChops.difference(before.convert("RGB"), after.convert("RGB")).getbbox():
            return ("Error: Browser content changed but scroll direction could not be verified. "
                    "Inspect the current page before another scroll; no automatic repeat was sent.")
    return ("Error: Scroll movement could not be verified. The page may be at its boundary, "
            "a popup may block it, or the content may need inspection. Do not report it scrolled.")
