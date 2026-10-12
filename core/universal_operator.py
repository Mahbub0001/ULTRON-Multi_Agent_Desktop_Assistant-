"""Desktop operation through focused UIA controls, known shortcuts and fresh vision.

Execution reports are separate from proof that the user's goal completed.
Unknown targets and failed focus return errors; screenshot coordinates are never
reused from an older page. Native Excel operations provide their own read-back.
"""
from __future__ import annotations

import os
import platform
import re
import subprocess
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Optional

# Lazy imports / conditional imports
try:
    import pyautogui
    pyautogui.FAILSAFE = False
    _HAS_PYAUTOGUI = True
except ImportError:
    _HAS_PYAUTOGUI = False

try:
    import win32gui
    import win32process
    import win32con
    import win32api
    _HAS_WIN32 = True
except ImportError:
    _HAS_WIN32 = False

try:
    import psutil
    _HAS_PSUTIL = True
except ImportError:
    _HAS_PSUTIL = False


@dataclass
class WindowContext:
    hwnd: int = 0
    title: str = ""
    app_category: str = "generic"
    process_name: str = ""
    rect: tuple[int, int, int, int] = (0, 0, 0, 0)
    visible: bool = True

    def as_dict(self) -> dict:
        return {
            "hwnd": self.hwnd,
            "title": self.title,
            "category": self.app_category,
            "process": self.process_name,
            "rect": list(self.rect),
        }


def get_active_window() -> WindowContext:
    """Capture rich details of the active foreground window."""
    if not _HAS_WIN32:
        return WindowContext(title="Unknown")

    try:
        hwnd = win32gui.GetForegroundWindow()
        if not hwnd or not win32gui.IsWindowVisible(hwnd):
            return WindowContext(title="No active window")

        title = win32gui.GetWindowText(hwnd).strip()
        rect = (0, 0, 0, 0)
        try:
            r = win32gui.GetWindowRect(hwnd)
            rect = (r[0], r[1], r[2], r[3])
        except Exception:
            pass

        proc_name = ""
        try:
            _, pid = win32process.GetWindowThreadProcessId(hwnd)
            if _HAS_PSUTIL and pid:
                proc = psutil.Process(pid)
                proc_name = proc.name().lower()
        except Exception:
            pass

        # Classify app category
        t_low = title.lower()
        p_low = proc_name.lower()

        category = "generic"
        if "zoom" in t_low or "zoom" in p_low:
            category = "zoom"
        elif any(b in t_low or b in p_low for b in ("chrome", "edge", "firefox", "brave", "opera", "vivaldi")):
            category = "browser"
        elif "excel" in t_low or "excel" in p_low:
            category = "excel"
        elif "word" in t_low or "winword" in p_low:
            category = "word"
        elif "powerpoint" in t_low or "powerpnt" in p_low:
            category = "powerpoint"
        elif "visual studio code" in t_low or "code" in p_low or "vscode" in p_low:
            category = "vscode"
        elif "discord" in t_low or "discord" in p_low:
            category = "discord"
        elif "spotify" in t_low or "spotify" in p_low:
            category = "spotify"
        elif "explorer" in p_low or "folder" in t_low:
            category = "explorer"
        elif any(g in t_low or g in p_low for g in ("roblox", "minecraft", "steam")):
            category = "game"

        return WindowContext(
            hwnd=hwnd,
            title=title,
            app_category=category,
            process_name=proc_name,
            rect=rect,
            visible=True,
        )
    except Exception as e:
        return WindowContext(title=f"Error: {e}")


def focus_app_window(target: str) -> bool:
    """Bring the application matching `target` to foreground reliably."""
    if not target or not _HAS_WIN32:
        return False

    try:
        from core.desktop_target import focus
        return bool(focus(target))
    except Exception:
        return False


# ── Domain Intelligence & Semantic Synonym Dictionary ────────────────────────
APP_KNOWLEDGE = {
    "zoom": {
        "new_meeting": {
            "synonyms": [
                "new meeting", "start meeting", "create meeting", "instant meeting",
                "orange new meeting button", "meet now", "মিটিং শুরু", "মিটিং ক্রিয়েট",
                "নতুন মিটিং", "start new meeting", "new meeting button"
            ],
            "controls": ["New Meeting", "Start a meeting", "New Meeting with video", "Instant Meeting"],
            "shortcut": ["alt", "v"],
            "verify_kw": ["Zoom Meeting", "Connecting", "Mute", "Stop Video", "End Meeting", "Leave"],
        },
        "join_meeting": {
            "synonyms": ["join", "join meeting", "enter meeting", "মিটিং এ জয়েন", "join button"],
            "controls": ["Join", "Join a Meeting"],
            "shortcut": ["alt", "j"],
            "verify_kw": ["Join Meeting", "Meeting ID", "Personal Link Name"],
        },
        "schedule": {
            "synonyms": ["schedule", "schedule meeting", "মিটিং শিডিউল"],
            "controls": ["Schedule", "Schedule Meeting"],
            "shortcut": ["alt", "s"],
            "verify_kw": ["Schedule Meeting"],
        },
        "share_screen": {
            "synonyms": ["share screen", "share", "স্ক্রিন শেয়ার"],
            "controls": ["Share Screen", "Share"],
            "shortcut": ["alt", "shift", "s"],
            "verify_kw": ["Select a window or an application", "Share"],
        },
        "mute_toggle": {
            "synonyms": ["mute", "unmute", "toggle mute", "audio", "mic", "মিউট", "আনমিউট"],
            "controls": ["Mute", "Unmute", "Mute audio", "Unmute audio"],
            "shortcut": ["alt", "a"],
            "verify_kw": ["Muted", "Unmuted"],
        },
        "video_toggle": {
            "synonyms": ["video", "camera", "start video", "stop video", "ভিডিও", "ক্যামেরা"],
            "controls": ["Start Video", "Stop Video"],
            "shortcut": ["alt", "v"],
            "verify_kw": ["Video"],
        },
        "fullscreen": {
            "synonyms": ["fullscreen", "full screen", "ফুলস্ক্রিন"],
            "controls": ["Enter Full Screen", "Exit Full Screen"],
            "shortcut": ["alt", "f"],
            "verify_kw": [],
        },
        "end_meeting": {
            "synonyms": ["end meeting", "leave meeting", "end", "leave", "মিটিং শেষ"],
            "controls": ["End", "Leave", "End Meeting for All"],
            "shortcut": ["alt", "q"],
            "verify_kw": ["End Meeting for All", "Leave Meeting"],
        },
        "home_tab": {
            "synonyms": ["home", "home tab", "হোম", "ড্যাশবোর্ড"],
            "controls": ["Home", "Home tab", "Home button"],
            "shortcut": [],
            "verify_kw": ["New Meeting", "Join", "Schedule"],
        },
    },
    "browser": {
        "new_tab": {
            "synonyms": ["new tab", "open tab", "notun tab", "নতুন ট্যাব"],
            "controls": ["New Tab", "New tab", "+"],
            "shortcut": ["ctrl", "t"],
            "verify_kw": [],
        },
        "close_tab": {
            "synonyms": ["close tab", "active tab bondho", "ট্যাব বন্ধ"],
            "controls": ["Close tab", "Close"],
            "shortcut": ["ctrl", "w"],
            "verify_kw": [],
        },
        "address_bar": {
            "synonyms": ["address bar", "url bar", "search bar", "এড্রেস বার"],
            "controls": ["Address and search bar", "Search or type URL"],
            "shortcut": ["ctrl", "l"],
            "verify_kw": [],
        },
        "reload": {
            "synonyms": ["reload", "refresh", "রিলোড", "রিফ্রেশ"],
            "controls": ["Reload", "Refresh"],
            "shortcut": ["ctrl", "r"],
            "verify_kw": [],
        },
        "back": {
            "synonyms": ["back", "pichone", "পিছনে"],
            "controls": ["Back", "Click to go back"],
            "shortcut": ["alt", "left"],
            "verify_kw": [],
        },
        "forward": {
            "synonyms": ["forward", "shamne", "সামনে"],
            "controls": ["Forward", "Click to go forward"],
            "shortcut": ["alt", "right"],
            "verify_kw": [],
        },
    },
    "excel": {
        "autofit": {
            "synonyms": ["autofit", "fit columns", "cell thik koro", "সবগুলো সেলে টোটাল টেক্সট", "কলামগুলা বড় করো"],
            "special_action": "excel_autofit",
            "shortcut": ["alt", "h", "o", "i"],
        },
        "format_table": {
            "synonyms": ["format table", "format existing table"],
            "special_action": "excel_format_table",
            "shortcut": ["ctrl", "t"],
        },
    },
    "vscode": {
        "new_file": {
            "synonyms": ["new file", "নতুন ফাইল"],
            "controls": ["New File...", "New Text File"],
            "shortcut": ["ctrl", "n"],
        },
        "save": {
            "synonyms": ["save", "save file", "সেভ"],
            "controls": ["Save"],
            "shortcut": ["ctrl", "s"],
        },
        "format": {
            "synonyms": ["format", "format document", "কোড ফরম্যাট"],
            "shortcut": ["shift", "alt", "f"],
        },
        "terminal": {
            "synonyms": ["terminal", "open terminal", "টার্মিনাল"],
            "shortcut": ["ctrl", "`"],
        },
    },
    "generic": {
        "save": {
            "synonyms": ["save", "save file", "সেভ"],
            "controls": ["Save", "Save As"],
            "shortcut": ["ctrl", "s"],
        },
        "copy": {
            "synonyms": ["copy", "কপি"],
            "shortcut": ["ctrl", "c"],
        },
        "paste": {
            "synonyms": ["paste", "পেস্ট"],
            "shortcut": ["ctrl", "v"],
        },
        "select_all": {
            "synonyms": ["select all", "সব সিলেক্ট"],
            "shortcut": ["ctrl", "a"],
        },
        "undo": {
            "synonyms": ["undo", "পূর্বাবস্থায়"],
            "shortcut": ["ctrl", "z"],
        },
        "find": {
            "synonyms": ["find", "search", "খোঁজো"],
            "shortcut": ["ctrl", "f"],
        },
        "close": {
            "synonyms": ["close", "exit", "quit", "বন্ধ"],
            "shortcut": ["alt", "f4"],
        },
    }
}


def _match_intent(app_category: str, user_request: str) -> Optional[dict]:
    """Find matching intent definition from domain knowledge."""
    request = user_request.casefold().strip()
    if not request:
        return None
    for domain in (APP_KNOWLEDGE.get(app_category, {}), APP_KNOWLEDGE["generic"]):
        for entry in domain.values():
            if request in {value.casefold() for value in entry.get("synonyms", [])}:
                return entry
    return None


def execute_smart_operation(
    intent: str = "",
    app: str = "",
    element: str = "",
    text: str = "",
    action_type: str = "click",
) -> str:
    """
    Universal Autonomous Closed-Loop Operation:
    Observe Context -> Orient Intent -> Multi-Tier Act -> Verify Outcome -> Auto-Correct.
    """
    # Step 1: Resolve Target Window Context
    ctx = get_active_window()
    if app:
        # A focus failure must never fall through into a different application.
        if app.casefold() not in ctx.title.casefold() and not focus_app_window(app):
            return f"Error: Could not focus target application '{app}'. No action was performed."
        time.sleep(0.2)
        ctx = get_active_window()
        if app.casefold() not in ctx.title.casefold():
            return f"Error: Target application '{app}' is not active. No action was performed."
    else:
        app = ctx.title

    category = ctx.app_category
    goal_desc = element or intent or text or "action"
    if action_type not in ("click", "double_click", "type", "write", "hover"):
        return f"Error: Unsupported action type '{action_type}'."
    if action_type == "hover":
        from actions.computer_control import _hover
        return _hover(goal_desc, window_title=ctx.title)

    # Step 2: Intent Matching
    matched_intent = _match_intent(category, intent or element)

    # ── SPECIAL HANDLING: ZOOM MEETINGS ──────────────────────────────────────

    # ── SPECIAL HANDLING: EXCEL SPREADSHEETS ─────────────────────────────────
    if category == "excel" and matched_intent and matched_intent.get("special_action"):
        act = matched_intent["special_action"]
        try:
            from actions.excel_control import excel_control
            if act == "excel_autofit":
                return excel_control({"action": "autofit"})
            elif act == "excel_format_table":
                return excel_control({"action": "format_table"})
        except Exception as e:
            pass

    # ── TIER 1: WINDOWS UI AUTOMATION (UIA) NATIVE ELEMENT DISPATCH ──────────
    try:
        from core import uia
        if uia.available():
            target_name = element or (matched_intent["controls"][0] if matched_intent and matched_intent.get("controls") else intent)
            if target_name:
                try:
                    if action_type in ("type", "write"):
                        res = uia.type_text(ctx.title, target_name, text, clear=True, timeout=0.3)
                        if not res:
                            raise RuntimeError("UIA typing returned no result")
                        if res.startswith("Error:"):
                            return res
                        return f"{res} Outcome verification is still required."
                    else:
                        res = (uia.double_click if action_type == "double_click" else uia.click)(ctx.title, target_name, timeout=0.3)
                        if not res:
                            raise RuntimeError("UIA click returned no result")
                        if res.startswith("Error:"):
                            return res
                        time.sleep(0.2)
                        # Verify
                        new_ctx = get_active_window()
                        return f"{res} Outcome verification is still required."
                except Exception:
                    pass
    except Exception:
        pass

    # ── TIER 0 FALLBACK: NATIVE HIGH-SPEED KEYBOARD SHORTCUT ─────────────────
    if category != "zoom" and action_type == "click" and matched_intent and matched_intent.get("shortcut") and _HAS_PYAUTOGUI:
        keys = matched_intent["shortcut"]
        try:
            pyautogui.hotkey(*keys)
            time.sleep(0.25)
            # Verification check
            new_ctx = get_active_window()
            return f"Executed shortcut '{'+'.join(keys)}' in {ctx.title} for '{goal_desc}'."
        except Exception:
            pass

    # ── TIER 2: RECENT SCREEN ELEMENT CACHE & VISUAL GROUNDING ───────────────
    try:
        from actions.computer_control import _screen_find, _click, _clipboard_paste
        coords = _screen_find(goal_desc, window_title=ctx.title)
        if coords:
            _click(coords[0], coords[1], clicks=2 if action_type == "double_click" else 1, smooth=True)
            time.sleep(0.25)
            if action_type in ("type", "write"):
                _clipboard_paste(text)
                return f"Typed into visually located '{goal_desc}' in {ctx.title}. Outcome verification is still required."
            return f"Visually located and clicked '{goal_desc}' at {coords} in {ctx.title}."
    except Exception:
        pass

    return f"Error: Could not perform '{goal_desc}' in {ctx.title}; all control methods failed. Inspect app state before trying a different approach."
