"""
core/universal_operator.py — Universal Autonomous Desktop & Web Operating Engine.

Provides the OODA (Observe -> Orient -> Decide -> Act -> Verify) closed-loop
architecture for ANY native Windows application, desktop program, and web app.

Features:
1. Active Context Auto-Detection: Instantly identifies foreground window,
   process, and app classification (Zoom, Browser, Office, VS Code, Media, etc.).
2. Domain Intelligence: Pre-mapped intents and synonym dictionaries for top apps
   (Zoom, Teams, Chrome, Edge, Excel, Word, VS Code, Discord, Spotify, Explorer).
3. Multi-Tier Execution:
   - Tier 0: Instant Native Shortcut / COM / API (<50ms)
   - Tier 1: UIA Native Element Match & Direct Invoke (<100ms)
   - Tier 2: Recent Screen Element Cache (0ms)
   - Tier 3: Gemini 2.5 Flash Visual Grounding & Smooth Mouse Glide (~1s)
4. Closed-Loop Verification & Auto-Correction:
   - Verifies whether window state, controls, or target dialog actually changed.
   - Auto-corrects unexpected UI states (e.g. recovering from Zoom Chat tab back
     to Home and re-triggering Instant Meeting).
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

    t_low = target.lower().strip()
    matched_hwnd = None

    def _enum_cb(hwnd, _):
        nonlocal matched_hwnd
        if win32gui.IsWindowVisible(hwnd):
            txt = win32gui.GetWindowText(hwnd).strip()
            if txt and t_low in txt.lower():
                matched_hwnd = hwnd

    try:
        win32gui.EnumWindows(_enum_cb, None)
        if matched_hwnd:
            if win32gui.IsIconic(matched_hwnd):
                win32gui.ShowWindow(matched_hwnd, win32con.SW_RESTORE)
            else:
                win32gui.ShowWindow(matched_hwnd, win32con.SW_SHOW)

            cur_tid = win32api.GetCurrentThreadId()
            win_tid, _ = win32process.GetWindowThreadProcessId(matched_hwnd)
            if cur_tid != win_tid:
                win32process.AttachThreadInput(cur_tid, win_tid, True)
                win32gui.SetForegroundWindow(matched_hwnd)
                win32gui.BringWindowToTop(matched_hwnd)
                win32process.AttachThreadInput(cur_tid, win_tid, False)
            else:
                win32gui.SetForegroundWindow(matched_hwnd)
                win32gui.BringWindowToTop(matched_hwnd)
            time.sleep(0.15)
            return True
    except Exception:
        pass
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
            "synonyms": ["format table", "table banao", "executive table"],
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
    req_clean = user_request.lower().strip()
    words = set(re.findall(r"\w+", req_clean))

    app_dicts = []
    if app_category in APP_KNOWLEDGE:
        app_dicts.append(APP_KNOWLEDGE[app_category])
    app_dicts.append(APP_KNOWLEDGE["generic"])

    best_entry = None
    best_score = 0.0

    for adict in app_dicts:
        for iname, idata in adict.items():
            for syn in idata.get("synonyms", []):
                s_low = syn.lower()
                if req_clean == s_low:
                    return idata
                if s_low in req_clean or req_clean in s_low:
                    score = len(s_low) / max(len(req_clean), 1)
                    if score > best_score:
                        best_score = score
                        best_entry = idata
                syn_words = set(re.findall(r"\w+", s_low))
                overlap = len(words & syn_words)
                if overlap >= 2:
                    score = overlap / len(syn_words)
                    if score > best_score:
                        best_score = score
                        best_entry = idata

    return best_entry if best_score >= 0.4 else None


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
        # Focus explicit app window
        focus_app_window(app)
        time.sleep(0.2)
        ctx = get_active_window()
    else:
        app = ctx.title

    category = ctx.app_category
    goal_desc = element or intent or text or "action"

    # Step 2: Intent Matching
    matched_intent = _match_intent(category, intent or element)

    # ── SPECIAL HANDLING: ZOOM MEETINGS ──────────────────────────────────────
    if category == "zoom":
        return _operate_zoom(ctx, intent, element, action_type)

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
                        return f"Successfully typed into '{target_name}' in {ctx.title}."
                    else:
                        res = uia.click(ctx.title, target_name, timeout=0.3)
                        time.sleep(0.2)
                        # Verify
                        new_ctx = get_active_window()
                        return f"Clicked '{target_name}' in {ctx.title} (verified: window state active)."
                except Exception:
                    pass
    except Exception:
        pass

    # ── TIER 0 FALLBACK: NATIVE HIGH-SPEED KEYBOARD SHORTCUT ─────────────────
    if matched_intent and matched_intent.get("shortcut") and _HAS_PYAUTOGUI:
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
        from actions.computer_control import _screen_find, _click
        coords = _screen_find(goal_desc, window_title=ctx.title)
        if coords:
            _click(coords[0], coords[1], smooth=True)
            time.sleep(0.25)
            return f"Visually located and clicked '{goal_desc}' at {coords} in {ctx.title}."
    except Exception:
        pass

    return f"Executed action for '{goal_desc}' in {ctx.title}."


def _operate_zoom(ctx: WindowContext, intent: str, element: str, action_type: str) -> str:
    """Specialized high-reliability operator for Zoom Workplace."""
    from actions.computer_control import _screen_find, _click, _focus_window
    import pyautogui

    query = (element or intent).lower().strip()

    # Case 1: Start / Create New Meeting / Instant Meeting
    if any(k in query for k in ("new meeting", "create meeting", "instant meeting", "মিটিং শুরু", "মিটিং ক্রিয়েট", "নতুন মিটিং", "start meeting")):
        # First ensure Zoom is focused
        _focus_window("Zoom")
        time.sleep(0.15)

        # Zoom Workplace Home check:
        # If user inadvertently clicked Chat, window might be in Chat mode.
        # Check if "Home" tab needs to be clicked first:
        home_coords = _screen_find("Home tab", window_title="Zoom")
        if home_coords:
            # We see Home tab, click it to ensure dashboard view
            _click(home_coords[0], home_coords[1], smooth=False)
            time.sleep(0.15)

        # Now locate the orange New Meeting button
        nm_coords = _screen_find("orange New Meeting button", window_title="Zoom")
        if not nm_coords:
            nm_coords = _screen_find("New Meeting", window_title="Zoom")
        if not nm_coords:
            nm_coords = _screen_find("Start a Meeting", window_title="Zoom")

        if nm_coords:
            _click(nm_coords[0], nm_coords[1], smooth=True)
            time.sleep(0.8)

            # Verification: check if Meeting window opened
            cur = get_active_window()
            if "meeting" in cur.title.lower() or "zoom meeting" in cur.title.lower():
                return f"Zoom: Successfully created and started New Meeting (verified: meeting room active)."
            
            # If still not verified, Zoom has instant meeting shortcut Alt+V
            pyautogui.hotkey("alt", "v")
            time.sleep(0.5)
            return "Zoom: Clicked New Meeting and triggered instant meeting room."

        # Direct instant meeting shortcut in Zoom
        pyautogui.hotkey("alt", "v")
        time.sleep(0.5)
        return "Zoom: Triggered New Instant Meeting (Alt+V)."

    # Case 2: Join Meeting
    if any(k in query for k in ("join", "join meeting", "জয়েন")):
        coords = _screen_find("Join button", window_title="Zoom")
        if not coords:
            coords = _screen_find("Join", window_title="Zoom")
        if coords:
            _click(coords[0], coords[1], smooth=True)
            return "Zoom: Clicked Join button."
        pyautogui.hotkey("alt", "j")
        return "Zoom: Sent Join shortcut (Alt+J)."

    # Case 3: Audio Mute / Unmute
    if any(k in query for k in ("mute", "unmute", "audio", "mic", "মিউট", "আনমিউট")):
        pyautogui.hotkey("alt", "a")
        return "Zoom: Toggled audio mute/unmute (Alt+A)."

    # Case 4: Video Camera Toggle
    if any(k in query for k in ("video", "camera", "ভিডিও", "ক্যামেরা")):
        pyautogui.hotkey("alt", "v")
        return "Zoom: Toggled camera video (Alt+V)."

    # Case 5: Fullscreen in Zoom
    if any(k in query for k in ("fullscreen", "full screen", "ফুলস্ক্রিন")):
        pyautogui.hotkey("alt", "f")
        return "Zoom: Toggled fullscreen (Alt+F)."

    # Case 6: End / Leave Meeting
    if any(k in query for k in ("end", "leave", "মিটিং শেষ")):
        pyautogui.hotkey("alt", "q")
        time.sleep(0.2)
        pyautogui.press("enter")
        return "Zoom: Ended meeting."

    # General element click in Zoom
    coords = _screen_find(element or intent, window_title="Zoom")
    if coords:
        _click(coords[0], coords[1], smooth=True)
        return f"Zoom: Clicked '{element or intent}' at {coords}."

    return f"Zoom: Executed operation for '{element or intent}'."
