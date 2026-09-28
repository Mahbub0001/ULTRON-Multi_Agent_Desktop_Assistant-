#computer_control.py
import io
import json
import platform
import re
import string
import subprocess
import sys

if platform.system() == "Windows":
    _WIN_HIDE: dict = {"creationflags": subprocess.CREATE_NO_WINDOW}
else:
    _WIN_HIDE: dict = {}
import time
import random
from pathlib import Path

try:
    import pyautogui
    pyautogui.FAILSAFE = True
    pyautogui.PAUSE    = 0.05
    _PYAUTOGUI = True
except ImportError:
    _PYAUTOGUI = False

try:
    import pyperclip
    _PYPERCLIP = True
except ImportError:
    _PYPERCLIP = False

def _base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent


_BASE         = _base_dir()
_CONFIG_PATH  = _BASE / "config" / "api_keys.json"
_MEMORY_PATH  = _BASE / "memory" / "long_term.json"

def _load_config() -> dict:
    try:
        return json.loads(_CONFIG_PATH.read_text(encoding="utf-8"))
    except Exception:
        return {}

def _platform_os() -> str:
    return {"Windows": "windows", "Darwin": "mac", "Linux": "linux"}.get(
        platform.system(), "linux"
    )

def _get_os() -> str:
    return _load_config().get("os_system", _platform_os()).lower()


def _get_api_key() -> str:
    return _load_config().get("gemini_api_key", "")

_SAFE_SCREENSHOT_ROOTS = (
    Path.home(),
)

def _safe_screenshot_path(requested: str | None) -> Path:
    fallback = Path.home() / "Desktop" / "jarvis_screenshot.png"
    if not requested:
        return fallback
    try:
        p = Path(requested).expanduser().resolve()
        for root in _SAFE_SCREENSHOT_ROOTS:
            if p.is_relative_to(root.resolve()):
                p.parent.mkdir(parents=True, exist_ok=True)
                return p
    except Exception:
        pass
    return fallback

def _require_pyautogui():
    if not _PYAUTOGUI:
        raise RuntimeError("PyAutoGUI not installed. Run: pip install pyautogui")

_FIRST_NAMES = [
    "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Drew", "Quinn",
    "Avery", "Blake", "Cameron", "Dakota", "Emerson", "Finley", "Harper",
]
_LAST_NAMES = [
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller",
    "Davis", "Wilson", "Moore", "Taylor", "Anderson", "Thomas", "Jackson",
]
_DOMAINS = ["gmail.com", "yahoo.com", "outlook.com", "proton.me", "mail.com"]


def _random_data(data_type: str) -> str:
    dt = data_type.lower().strip()

    if dt == "first_name":
        return random.choice(_FIRST_NAMES)

    if dt == "last_name":
        return random.choice(_LAST_NAMES)

    if dt == "name":
        return f"{random.choice(_FIRST_NAMES)} {random.choice(_LAST_NAMES)}"

    if dt == "email":
        first = random.choice(_FIRST_NAMES).lower()
        last  = random.choice(_LAST_NAMES).lower()
        num   = random.randint(10, 999)
        return f"{first}.{last}{num}@{random.choice(_DOMAINS)}"

    if dt == "username":
        return f"{random.choice(_FIRST_NAMES).lower()}{random.randint(100, 9999)}"

    if dt == "password":
        chars = string.ascii_letters + string.digits + "!@#$%"
        raw   = (
            random.choice(string.ascii_uppercase)
            + random.choice(string.digits)
            + random.choice("!@#$%")
            + "".join(random.choices(chars, k=9))
        )
        return "".join(random.sample(raw, len(raw)))

    if dt == "phone":
        return f"+1{random.randint(200,999)}{random.randint(1_000_000, 9_999_999)}"

    if dt == "birthday":
        y = random.randint(1980, 2000)
        m = random.randint(1, 12)
        d = random.randint(1, 28)
        return f"{m:02d}/{d:02d}/{y}"

    if dt == "address":
        num    = random.randint(100, 9999)
        street = random.choice(["Main St", "Oak Ave", "Park Blvd", "Elm St", "Cedar Ln"])
        return f"{num} {street}"

    if dt == "zip_code":
        return str(random.randint(10000, 99999))

    if dt == "city":
        return random.choice(["New York", "Los Angeles", "Chicago", "Houston", "Phoenix"])

    return f"random_{data_type}_{random.randint(1000, 9999)}"

def _user_profile() -> dict:
    """Read identity fields from long-term memory."""
    try:
        if _MEMORY_PATH.exists():
            data     = json.loads(_MEMORY_PATH.read_text(encoding="utf-8"))
            identity = data.get("identity", {})
            return {k: v.get("value", "") for k, v in identity.items()}
    except Exception:
        pass
    return {}

def _type(text: str, interval: float = 0.03) -> str:
    _require_pyautogui()
    time.sleep(0.3)
    pyautogui.typewrite(text, interval=interval)
    return f"Typed: {text[:60]}{'…' if len(text) > 60 else ''}"


def _smart_type(text: str, clear_first: bool = True) -> str:
    _require_pyautogui()
    if clear_first:
        _clear_field()
        time.sleep(0.1)

    if len(text) > 20 and _PYPERCLIP:
        pyperclip.copy(text)
        time.sleep(0.1)
        paste_key = "command" if _get_os() == "mac" else "ctrl"
        pyautogui.hotkey(paste_key, "v")
        return f"Smart-typed (clipboard): {text[:60]}{'…' if len(text) > 60 else ''}"

    pyautogui.typewrite(text, interval=0.04)
    return f"Smart-typed: {text[:60]}{'…' if len(text) > 60 else ''}"


def _click(x=None, y=None, button: str = "left", clicks: int = 1) -> str:
    _require_pyautogui()
    if x is not None and y is not None:
        pyautogui.click(x, y, button=button, clicks=clicks)
        return f"{'Double-c' if clicks == 2 else 'C'}licked ({x}, {y}) [{button}]"
    pyautogui.click(button=button, clicks=clicks)
    return f"Clicked at current position [{button}]"


def _hotkey(*keys) -> str:
    _require_pyautogui()
    pyautogui.hotkey(*keys)
    return f"Hotkey: {'+'.join(keys)}"


def _press(key: str) -> str:
    _require_pyautogui()
    pyautogui.press(key)
    return f"Pressed: {key}"


def _scroll(direction: str = "down", amount: int = 3) -> str:
    _require_pyautogui()
    vertical   = direction in ("up", "down")
    clicks     = amount if direction in ("up", "right") else -amount
    pyautogui.scroll(clicks) if vertical else pyautogui.hscroll(clicks)
    return f"Scrolled {direction} ×{amount}"


def _move(x: int, y: int, duration: float = 0.3) -> str:
    _require_pyautogui()
    pyautogui.moveTo(x, y, duration=duration)
    return f"Mouse → ({x}, {y})"


def _drag(x1: int, y1: int, x2: int, y2: int, duration: float = 0.6) -> str:
    _require_pyautogui()
    pyautogui.moveTo(x1, y1, duration=0.2)
    time.sleep(0.15)
    pyautogui.mouseDown(button="left")
    time.sleep(0.15)
    pyautogui.moveTo(x2, y2, duration=duration)
    time.sleep(0.15)
    pyautogui.mouseUp(button="left")
    time.sleep(0.1)
    return f"Dragged ({x1},{y1}) → ({x2},{y2})"


def _clipboard_get() -> str:
    if _PYPERCLIP:
        return pyperclip.paste()
    _hotkey("ctrl", "c")
    time.sleep(0.2)
    return "(copied — pyperclip unavailable for read)"


def _clipboard_paste(text: str) -> str:
    if _PYPERCLIP:
        pyperclip.copy(text)
        time.sleep(0.1)
        _require_pyautogui()
        paste_key = "command" if _get_os() == "mac" else "ctrl"
        pyautogui.hotkey(paste_key, "v")
        return f"Pasted: {text[:60]}{'…' if len(text) > 60 else ''}"
    return "pyperclip not available"


def _screenshot(save_path: str | None = None) -> str:
    _require_pyautogui()
    path = _safe_screenshot_path(save_path)
    img  = pyautogui.screenshot()
    img.save(str(path))
    return f"Screenshot saved: {path}"


def _clear_field() -> str:
    _require_pyautogui()
    select_key = "command" if _get_os() == "mac" else "ctrl"
    pyautogui.hotkey(select_key, "a")
    time.sleep(0.1)
    pyautogui.press("delete")
    return "Field cleared"

def _focus_window(title: str) -> str:
    os_name = _get_os()

    if os_name == "windows":
        # 1. Direct win32gui check (fastest, brings to front, restores minimized)
        try:
            import win32gui
            import win32con
            matched_hwnd = None
            def _enum_cb(hwnd, _):
                nonlocal matched_hwnd
                if win32gui.IsWindowVisible(hwnd):
                    text = win32gui.GetWindowText(hwnd).strip()
                    if text and title.lower() in text.lower():
                        matched_hwnd = hwnd
            win32gui.EnumWindows(_enum_cb, None)
            if matched_hwnd:
                try:
                    if win32gui.IsIconic(matched_hwnd):
                        # Only restore if actually minimized; SW_RESTORE on a maximized window un-maximizes it!
                        win32gui.ShowWindow(matched_hwnd, win32con.SW_RESTORE)
                    else:
                        win32gui.ShowWindow(matched_hwnd, win32con.SW_SHOW)

                    try:
                        import win32process
                        import win32api
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
                    except Exception:
                        win32gui.SetForegroundWindow(matched_hwnd)
                except Exception:
                    pass
                time.sleep(0.15)
                return f"Focused window: {title}"
        except Exception:
            pass

        # 2. PowerShell AppActivate fallback
        try:
            script = f'(New-Object -ComObject WScript.Shell).AppActivate("{title}")'
            proc = subprocess.run(
                ["powershell", "-NoProfile", "-NonInteractive", "-Command", script],
                capture_output=True, timeout=5, text=True, **_WIN_HIDE,
            )
            if proc.stdout and proc.stdout.strip().lower() == "true":
                time.sleep(0.3)
                return f"Focused window: {title}"
        except Exception:
            pass

        return f"Window not found: {title}"

    if os_name == "mac":
        script = (
            f'tell application "System Events" to '
            f'set frontmost of (first process whose name contains "{title}") to true'
        )
        try:
            res = subprocess.run(
                ["osascript", "-e", script],
                capture_output=True, timeout=5,
            )
            if res.returncode == 0:
                time.sleep(0.3)
                return f"Focused window: {title}"
        except Exception:
            pass
        return f"Window not found: {title}"

    if os_name == "linux":
        try:
            result = subprocess.run(
                ["wmctrl", "-a", title],
                capture_output=True, timeout=5,
            )
            if result.returncode == 0:
                time.sleep(0.3)
                return f"Focused window: {title}"
        except FileNotFoundError:
            pass
        try:
            result = subprocess.run(
                ["xdotool", "search", "--name", title, "windowactivate"],
                capture_output=True, timeout=5,
            )
            if result.returncode == 0:
                time.sleep(0.3)
                return f"Focused window: {title}"
        except FileNotFoundError:
            return "focus_window (Linux) requires wmctrl or xdotool"
        except Exception as e:
            return f"focus_window (Linux) failed: {e}"

        return f"Window not found: {title}"

    return f"focus_window: unknown OS '{os_name}'"

def _list_windows() -> str:
    os_name = _get_os()
    if os_name == "windows":
        try:
            import win32gui
            windows = []
            def _enum_cb(hwnd, _):
                if win32gui.IsWindowVisible(hwnd):
                    text = win32gui.GetWindowText(hwnd).strip()
                    if text and len(text) > 1 and text not in ("Program Manager", "Settings"):
                        windows.append(text)
            win32gui.EnumWindows(_enum_cb, None)
            if windows:
                seen = set()
                uniq = [w for w in windows if not (w in seen or seen.add(w))]
                return "Open Windows:\n• " + "\n• ".join(uniq[:15])
        except Exception:
            pass
        try:
            res = subprocess.run(
                ["powershell", "-NoProfile", "-Command", "Get-Process | Where-Object {$_.MainWindowTitle} | Select-Object -ExpandProperty MainWindowTitle"],
                capture_output=True, text=True, timeout=5, **_WIN_HIDE
            )
            titles = [l.strip() for l in res.stdout.splitlines() if l.strip()]
            if titles:
                seen = set()
                uniq = [t for t in titles if not (t in seen or seen.add(t))]
                return "Open Windows:\n• " + "\n• ".join(uniq[:15])
        except Exception:
            pass
    return "Could not list windows."

def _active_window() -> str:
    os_name = _get_os()
    if os_name == "windows":
        try:
            import win32gui
            hwnd = win32gui.GetForegroundWindow()
            if hwnd:
                title = win32gui.GetWindowText(hwnd).strip()
                if title:
                    return f"Active Window: {title}"
        except Exception:
            pass
    return "Could not determine active window."

def _close_window(title: str = "") -> str:
    _require_pyautogui()
    if title:
        _focus_window(title)
        time.sleep(0.2)
    pyautogui.hotkey("alt", "f4")
    return f"Closed window{' (' + title + ')' if title else ''}."

def _maximize_window() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "up")
    return "Maximized active window."

def _minimize_window() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "down")
    return "Minimized active window."

def _restore_window() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "up")
    return "Restored active window."

def _snap_left() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "left")
    return "Snapped window to left."

def _snap_right() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "right")
    return "Snapped window to right."

def _move_window_monitor(direction: str = "left") -> str:
    _require_pyautogui()
    if direction == "left":
        pyautogui.hotkey("win", "shift", "left")
    elif direction == "right":
        pyautogui.hotkey("win", "shift", "right")
    return f"Moved window to {direction} monitor."

def _close_tab() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "w")
    return "Closed tab."

def _new_tab() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "t")
    return "Opened new tab."

def _reopen_tab() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "shift", "t")
    return "Reopened last closed tab."

def _next_tab() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "tab")
    return "Next tab."

def _prev_tab() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "shift", "tab")
    return "Previous tab."

def _copy() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "c")
    return "Copied."

def _cut() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "x")
    return "Cut."

def _paste() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "v")
    return "Pasted."

def _select_all() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "a")
    return "Selected all."

def _find() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "f")
    return "Opened find."

def _print() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "p")
    return "Opened print dialog."

def _save_as() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "shift", "s")
    return "Opened save as dialog."

def _zoom_in() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "+")
    return "Zoomed in."

def _zoom_out() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "-")
    return "Zoomed out."

def _zoom_reset() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "0")
    return "Zoom reset."

def _fullscreen() -> str:
    _require_pyautogui()
    pyautogui.press("f11")
    return "Toggled fullscreen."

def _task_manager() -> str:
    _require_pyautogui()
    pyautogui.hotkey("ctrl", "shift", "esc")
    return "Opened Task Manager."

def _run_dialog() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "r")
    return "Opened Run dialog."

def _settings() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "i")
    return "Opened Windows Settings."

def _file_explorer() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "e")
    return "Opened File Explorer."

def _action_center() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "a")
    return "Opened Action Center."

def _quick_settings() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "shift", "s")
    return "Opened Quick Settings / Snip tool."

def _emoji_picker() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", ".")
    return "Opened emoji picker."

def _clipboard_history() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "v")
    return "Opened clipboard history."

def _virtual_desktop_new() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "ctrl", "d")
    return "Created new virtual desktop."

def _virtual_desktop_close() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "ctrl", "f4")
    return "Closed current virtual desktop."

def _virtual_desktop_switch(direction: str = "left") -> str:
    _require_pyautogui()
    if direction == "left":
        pyautogui.hotkey("win", "ctrl", "left")
    elif direction == "right":
        pyautogui.hotkey("win", "ctrl", "right")
    return f"Switched virtual desktop {direction}."

def _lock_screen() -> str:
    _require_pyautogui()
    pyautogui.hotkey("win", "l")
    return "Locked screen."

# App-specific shortcuts
def _app_shortcut(app: str, action: str) -> str:
    """Execute app-specific shortcuts."""
    _require_pyautogui()
    app = app.lower().strip()
    
    shortcuts = {
        # VS Code
        "vscode": {
            "command_palette": ["ctrl", "shift", "p"],
            "terminal": ["ctrl", "`"],
            "new_file": ["ctrl", "n"],
            "save_all": ["ctrl", "k", "s"],
            "format": ["shift", "alt", "f"],
            "toggle_sidebar": ["ctrl", "b"],
            "toggle_panel": ["ctrl", "j"],
            "go_to_file": ["ctrl", "p"],
            "go_to_line": ["ctrl", "g"],
            "search": ["ctrl", "shift", "f"],
            "replace": ["ctrl", "h"],
            "zen_mode": ["ctrl", "k", "z"],
            "split_editor": ["ctrl", "\\"],
            "close_editor": ["ctrl", "w"],
            "reopen_closed": ["ctrl", "shift", "t"],
            "debug_start": ["f5"],
            "debug_stop": ["shift", "f5"],
        },
        # Chrome/Edge/Brave
        "chrome": {
            "devtools": ["f12"],
            "devtools_console": ["ctrl", "shift", "j"],
            "devtools_elements": ["ctrl", "shift", "c"],
            "new_incognito": ["ctrl", "shift", "n"],
            "history": ["ctrl", "h"],
            "downloads": ["ctrl", "j"],
            "bookmarks": ["ctrl", "shift", "o"],
            "clear_browsing": ["ctrl", "shift", "delete"],
            "focus_address": ["ctrl", "l"],
            "focus_search": ["ctrl", "e"],
        },
        "edge": {
            "devtools": ["f12"],
            "devtools_console": ["ctrl", "shift", "j"],
            "devtools_elements": ["ctrl", "shift", "c"],
            "new_inprivate": ["ctrl", "shift", "n"],
            "history": ["ctrl", "h"],
            "downloads": ["ctrl", "j"],
            "collections": ["ctrl", "shift", "y"],
            "focus_address": ["ctrl", "l"],
        },
        "firefox": {
            "devtools": ["f12"],
            "devtools_console": ["ctrl", "shift", "k"],
            "devtools_elements": ["ctrl", "shift", "c"],
            "new_private": ["ctrl", "shift", "p"],
            "history": ["ctrl", "shift", "h"],
            "downloads": ["ctrl", "j"],
            "bookmarks": ["ctrl", "shift", "o"],
            "focus_address": ["ctrl", "l"],
            "focus_search": ["ctrl", "k"],
        },
        # YouTube (in browser)
        "youtube": {
            "play_pause": ["k"],
            "next": ["shift", "n"],
            "prev": ["shift", "p"],
            "fullscreen": ["f"],
            "theater": ["t"],
            "mini_player": ["i"],
            "captions": ["c"],
            "mute": ["m"],
            "volume_up": ["up"],
            "volume_down": ["down"],
            "seek_forward_10": ["l"],
            "seek_back_10": ["j"],
            "seek_forward_frame": [">"],
            "seek_back_frame": ["<"],
            "restart": ["0"],
            "speed_up": ["shift", "."],
            "speed_down": ["shift", ","],
            "speed_normal": ["shift", "/"],
        },
        # Generic text editors
        "notepad": {
            "new_window": ["ctrl", "shift", "n"],
            "save_as": ["ctrl", "shift", "s"],
            "word_wrap": ["alt", "o", "w"],
            "font": ["alt", "o", "f"],
            "status_bar": ["alt", "v", "s"],
        },
        "word": {
            "bold": ["ctrl", "b"],
            "italic": ["ctrl", "i"],
            "underline": ["ctrl", "u"],
            "center": ["ctrl", "e"],
            "left_align": ["ctrl", "l"],
            "right_align": ["ctrl", "r"],
            "justify": ["ctrl", "j"],
            "heading1": ["ctrl", "alt", "1"],
            "heading2": ["ctrl", "alt", "2"],
            "heading3": ["ctrl", "alt", "3"],
            "bullet_list": ["ctrl", "shift", "l"],
        },
        "excel": {
            "new_sheet": ["shift", "f11"],
            "insert_row": ["ctrl", "shift", "+"],
            "delete_row": ["ctrl", "-"],
            "filter": ["ctrl", "shift", "l"],
            "format_cells": ["ctrl", "1"],
            "auto_sum": ["alt", "="],
        },
    }
    
    if app not in shortcuts:
        return f"Unknown app: {app}. Available: {', '.join(shortcuts.keys())}"
    
    if action not in shortcuts[app]:
        available = ", ".join(shortcuts[app].keys())
        return f"Unknown action '{action}' for {app}. Available: {available}"
    
    keys = shortcuts[app][action]
    pyautogui.hotkey(*keys)
    return f"{app}: {action} ({'+'.join(keys)})"

def _switch_app() -> str:
    _require_pyautogui()
    pyautogui.hotkey("alt", "tab")
    return "Switched to previous application."

def _screen_find(description: str) -> tuple[int, int] | None:
    api_key = _get_api_key()
    if not api_key:
        print("[ComputerControl] ⚠️ No API key for screen_find")
        return None

    try:
        from google import genai
        from google.genai import types as gtypes

        _require_pyautogui()
        w, h  = pyautogui.size()
        img   = pyautogui.screenshot()
        buf   = io.BytesIO()
        img.save(buf, format="PNG")
        image_bytes = buf.getvalue()

        prompt = (
            f"This is a screenshot of a {w}×{h} pixel screen. "
            f"Locate the UI element described as: '{description}'. "
            f"Reply with ONLY the center coordinates as: x,y "
            f"If the element is not visible, reply: NOT_FOUND"
        )

        from core import gemini
        response = gemini.call(
            [gtypes.Part.from_bytes(data=image_bytes, mime_type="image/png"), prompt],
            tier=gemini.FAST, timeout_ms=20_000,
        )
        if response is None:
            return None

        text = (response.text or "").strip()
        if "NOT_FOUND" in text.upper():
            return None

        match = re.search(r"(\d+)\s*,\s*(\d+)", text)
        if match:
            return int(match.group(1)), int(match.group(2))

    except Exception as e:
        print(f"[ComputerControl] ⚠️ screen_find failed: {e}")

    return None

def computer_control(
    parameters: dict,
    response=None,
    player=None,
    session_memory=None,
) -> str:
    """
    Dispatch table for all computer control actions.

    parameters keys (all optional unless noted):
      action        : (required) one of the actions listed below
      text          : text to type or paste
      x, y          : screen coordinates
      button        : 'left' | 'right' (default: left)
      keys          : hotkey string, e.g. 'ctrl+c'
      key           : single key name, e.g. 'enter'
      direction     : 'up' | 'down' | 'left' | 'right'
      amount        : scroll amount (default: 3)
      seconds       : wait duration
      title         : window title fragment for focus_window
      description   : natural-language element description for screen_find/click
      type          : data type for random_data
      field         : memory field name for user_data
      clear_first   : bool, clear field before typing (default: true)
      path          : save path for screenshot (must be inside home dir)

    Actions:
      type          — type text at cursor
      smart_type    — clear field + type (clipboard-backed)
      click         — left click
      double_click  — double left click
      right_click   — right click
      move          — move mouse
      drag          — click-drag between two points
      hotkey        — key combination
      press         — single key
      scroll        — scroll the wheel
      copy          — read clipboard
      paste         — write + paste clipboard
      screenshot    — capture screen (safe path only)
      wait          — sleep N seconds
      clear_field   — select-all + delete
      focus_window  — bring window to foreground
      screen_find   — AI element finder (returns x,y)
      screen_click  — AI element finder + click
      random_data   — generate fake form data
      user_data     — pull real data from memory
    """
    params = parameters or {}
    action = params.get("action", "").lower().strip()

    if not action:
        return "No action specified for computer_control."

    if player:
        player.write_log(f"[Computer] {action}")

    print(f"[ComputerControl] ▶ {action}  {params}")

    try:

        if action == "type":
            return _type(params.get("text", ""))

        if action == "smart_type":
            return _smart_type(
                params.get("text", ""),
                clear_first=params.get("clear_first", True),
            )

        if action in ("click", "left_click"):
            return _click(params.get("x"), params.get("y"), "left", 1)

        if action == "double_click":
            return _click(params.get("x"), params.get("y"), "left", 2)

        if action == "right_click":
            return _click(params.get("x"), params.get("y"), "right", 1)

        if action == "move":
            return _move(int(params.get("x", 0)), int(params.get("y", 0)))

        if action == "drag":
            return _drag(
                int(params.get("x1", 0)), int(params.get("y1", 0)),
                int(params.get("x2", 0)), int(params.get("y2", 0)),
            )

        if action == "hotkey":
            raw  = params.get("keys", "")
            keys = [k.strip() for k in raw.split("+")] if isinstance(raw, str) else raw
            return _hotkey(*keys)

        if action == "press":
            return _press(params.get("key", "enter"))

        if action == "scroll":
            return _scroll(
                direction=params.get("direction", "down"),
                amount=int(params.get("amount", 3)),
            )

        if action == "copy":
            return _clipboard_get()

        if action == "paste":
            return _clipboard_paste(params.get("text", ""))

        if action == "screenshot":
            return _screenshot(params.get("path"))

        if action == "screen_find":
            coords = _screen_find(params.get("description", ""))
            return f"{coords[0]},{coords[1]}" if coords else "NOT_FOUND"

        if action == "screen_click":
            desc = params.get("description", "")
            d_lower = desc.lower()
            cur_title = _active_window().lower()

            # Smart web-app direct navigation interception:
            # If user or model is trying to click navigation buttons/bell on LinkedIn or GitHub,
            # navigate directly to the URL in the active tab instead of risking missed clicks.
            if "linkedin" in cur_title or "linkedin" in d_lower:
                if "notification" in d_lower:
                    try:
                        from actions.browser_control import browser_control
                        return browser_control({"action": "go_to", "url": "https://www.linkedin.com/notifications"}, player=player)
                    except Exception:
                        pass
                if any(k in d_lower for k in ["message", "messaging", "inbox"]):
                    try:
                        from actions.browser_control import browser_control
                        return browser_control({"action": "go_to", "url": "https://www.linkedin.com/messaging"}, player=player)
                    except Exception:
                        pass
                if any(k in d_lower for k in ["network", "connection", "mynetwork"]):
                    try:
                        from actions.browser_control import browser_control
                        return browser_control({"action": "go_to", "url": "https://www.linkedin.com/mynetwork"}, player=player)
                    except Exception:
                        pass
                if any(k in d_lower for k in ["feed", "home"]):
                    try:
                        from actions.browser_control import browser_control
                        return browser_control({"action": "go_to", "url": "https://www.linkedin.com/feed"}, player=player)
                    except Exception:
                        pass

            if "github" in cur_title or "github" in d_lower:
                if "notification" in d_lower:
                    try:
                        from actions.browser_control import browser_control
                        return browser_control({"action": "go_to", "url": "https://github.com/notifications"}, player=player)
                    except Exception:
                        pass

            coords = _screen_find(desc)
            if coords:
                time.sleep(0.2)
                _click(x=coords[0], y=coords[1])
                return f"Clicked '{desc}' at {coords}"
            return f"Element not found on screen: '{desc}'"

        if action == "wait":
            secs = float(params.get("seconds", 1.0))
            secs = min(secs, 30.0)
            time.sleep(secs)
            return f"Waited {secs}s"

        if action == "clear_field":
            return _clear_field()

        if action in ("focus_window", "switch_to"):
            return _focus_window(params.get("title", ""))

        if action in ("list_windows", "windows", "get_windows"):
            return _list_windows()

        if action in ("active_window", "current_window"):
            return _active_window()

        if action in ("close_window", "quit_app"):
            return _close_window(params.get("title", ""))

        if action in ("maximize_window", "maximize"):
            return _maximize_window()

        if action in ("minimize_window", "minimize"):
            return _minimize_window()

        if action in ("restore_window", "restore"):
            return _restore_window()

        if action in ("snap_left", "snap_left"):
            return _snap_left()

        if action in ("snap_right", "snap_right"):
            return _snap_right()

        if action in ("move_window_monitor", "move_monitor"):
            return _move_window_monitor(params.get("direction", "left"))

        if action in ("close_tab",):
            return _close_tab()

        if action in ("new_tab",):
            return _new_tab()

        if action in ("reopen_tab",):
            return _reopen_tab()

        if action in ("next_tab",):
            return _next_tab()

        if action in ("prev_tab", "previous_tab"):
            return _prev_tab()

        if action in ("copy",):
            return _copy()

        if action in ("cut",):
            return _cut()

        if action in ("paste",):
            return _paste()

        if action in ("select_all",):
            return _select_all()

        if action in ("find",):
            return _find()

        if action in ("print",):
            return _print()

        if action in ("save_as",):
            return _save_as()

        if action in ("zoom_in",):
            return _zoom_in()

        if action in ("zoom_out",):
            return _zoom_out()

        if action in ("zoom_reset",):
            return _zoom_reset()

        if action in ("fullscreen",):
            return _fullscreen()

        if action in ("task_manager",):
            return _task_manager()

        if action in ("run_dialog",):
            return _run_dialog()

        if action in ("settings", "windows_settings"):
            return _settings()

        if action in ("file_explorer", "explorer"):
            return _file_explorer()

        if action in ("action_center",):
            return _action_center()

        if action in ("quick_settings", "snip_tool"):
            return _quick_settings()

        if action in ("emoji_picker",):
            return _emoji_picker()

        if action in ("clipboard_history",):
            return _clipboard_history()

        if action in ("virtual_desktop_new", "new_desktop"):
            return _virtual_desktop_new()

        if action in ("virtual_desktop_close", "close_desktop"):
            return _virtual_desktop_close()

        if action in ("virtual_desktop_switch", "switch_desktop"):
            return _virtual_desktop_switch(params.get("direction", "left"))

        if action in ("lock_screen", "lock"):
            return _lock_screen()

        if action in ("app_shortcut", "app_hotkey"):
            return _app_shortcut(params.get("app", ""), params.get("app_action", ""))

        if action in ("switch_app", "alt_tab"):
            return _switch_app()

        if action in ("save", "save_file"):
            return _hotkey("ctrl", "s")

        if action in ("select_all",):
            return _hotkey("ctrl", "a")

        if action in ("undo",):
            return _hotkey("ctrl", "z")

        if action in ("redo",):
            return _hotkey("ctrl", "y")

        if action == "random_data":
            dt     = params.get("type", "name")
            result = _random_data(dt)
            print(f"[ComputerControl] 🎲 random {dt} → {result}")
            return result

        if action == "user_data":
            field   = params.get("field", "name")
            profile = _user_profile()
            value   = profile.get(field, "")
            if not value:
                value = _random_data(field)
                print(f"[ComputerControl] ⚠️ No '{field}' in memory, using random: {value}")
            return value

        return f"Unknown action: '{action}'"

    except Exception as e:
        print(f"[ComputerControl] ❌ {action}: {e}")
        return f"computer_control '{action}' failed: {e}"


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "computer_control",
    "description": "Complete computer & desktop app control: type, click, hotkeys, scroll, window management (snap, restore, move monitor), tab management, text editing (copy, cut, paste, select all, find, zoom), system shortcuts (task manager, settings, file explorer, clipboard history, virtual desktops, lock), and app-specific hotkeys (VS Code, Chrome, Firefox, YouTube, Word, Excel, Notepad).",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "type | smart_type | click | double_click | right_click | hotkey | press | scroll | move | drag | copy | cut | paste | screenshot | wait | clear_field | focus_window | list_windows | active_window | close_window | maximize_window | minimize_window | restore_window | snap_left | snap_right | move_window_monitor | close_tab | new_tab | reopen_tab | next_tab | prev_tab | select_all | find | print | save_as | zoom_in | zoom_out | zoom_reset | fullscreen | task_manager | run_dialog | settings | file_explorer | action_center | quick_settings | emoji_picker | clipboard_history | virtual_desktop_new | virtual_desktop_close | virtual_desktop_switch | lock_screen | app_shortcut | switch_app | save | select_all | undo | redo | screen_find | screen_click | random_data | user_data"
            },
            "text": {
                "type": "STRING",
                "description": "Text to type or paste"
            },
            "x": {
                "type": "INTEGER",
                "description": "X coordinate"
            },
            "y": {
                "type": "INTEGER",
                "description": "Y coordinate"
            },
            "x1": {
                "type": "INTEGER",
                "description": "Start X for drag"
            },
            "y1": {
                "type": "INTEGER",
                "description": "Start Y for drag"
            },
            "x2": {
                "type": "INTEGER",
                "description": "End X for drag"
            },
            "y2": {
                "type": "INTEGER",
                "description": "End Y for drag"
            },
            "keys": {
                "type": "STRING",
                "description": "Key combination e.g. 'ctrl+c'"
            },
            "key": {
                "type": "STRING",
                "description": "Single key e.g. 'enter'"
            },
            "direction": {
                "type": "STRING",
                "description": "up | down | left | right (for scroll, move_window_monitor, virtual_desktop_switch, snap)"
            },
            "amount": {
                "type": "INTEGER",
                "description": "Scroll amount (default: 3)"
            },
            "seconds": {
                "type": "NUMBER",
                "description": "Seconds to wait"
            },
            "title": {
                "type": "STRING",
                "description": "Window title for focus_window"
            },
            "description": {
                "type": "STRING",
                "description": "Element description for screen_find/screen_click"
            },
            "type": {
                "type": "STRING",
                "description": "Data type for random_data"
            },
            "field": {
                "type": "STRING",
                "description": "Field for user_data: name|email|city"
            },
            "clear_first": {
                "type": "BOOLEAN",
                "description": "Clear field before typing (default: true)"
            },
            "path": {
                "type": "STRING",
                "description": "Save path for screenshot"
            },
            "app": {
                "type": "STRING",
                "description": "App name for app_shortcut: vscode | chrome | edge | firefox | youtube | notepad | word | excel"
            },
            "app_action": {
                "type": "STRING",
                "description": "App-specific action for app_shortcut (e.g. command_palette, devtools, play_pause, bold, etc.)"
            }
        },
        "required": [
            "action"
        ]
    },
    "handler": computer_control,
}
