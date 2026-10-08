"""
app_control — smart observe/act control of ANY native Windows application.

This is the missing layer between "open the app" and "do the thing in it".
Where open_app only launches and computer_control only throws keystrokes,
app_control works at the element level through Windows UI Automation
(core/uia.py): it can see what's on screen, name every control, click the
right one, and type into the right box — in any app, without app-specific
hardcoding.

The model drives it as a loop:

    1. app_control {action: "launch",   app: "PowerPoint"}
    2. app_control {action: "elements", app: "PowerPoint"}   # look
    3. app_control {action: "click",    app: "PowerPoint", element: "New Slide"}
    4. app_control {action: "type",     app: "PowerPoint",
                    element: "Title placeholder", text: "Hello"}

Fallback ladder for click/type when UIA cannot find the element:
    UIA element → Gemini vision screen_find (computer_control) → coordinates.
So even canvases, games, and canvas-rendered UIs (Roblox menus, etc.) work —
vision finds them by description when no automation element exists.
"""
from __future__ import annotations

import time

from core import uia

# Actions that only make sense once an app is running; "launch" does not.
_LIVE_ACTIONS = ("elements", "click", "double_click", "type", "press",
                 "state", "wait")


def _vision_fallback(app: str, element: str) -> str:
    """
    Last resort: find the element by eye (Gemini on a screenshot) and click
    its center. Only reached when UIA has no matching control.
    """
    try:
        from actions.computer_control import _screen_find, _click
    except Exception:
        return ""
    # Bring the target window forward first — vision sees the focused app.
    try:
        uia.connect(app)
        time.sleep(0.3)
    except Exception:
        pass
    coords = _screen_find(f"the UI element '{element}'", window_title=app)
    if not coords:
        return ""
    _click(coords[0], coords[1], smooth=True)
    return f"Clicked '{element}' at {coords} using visual element search."


def _launch(app: str) -> str:
    from actions.open_app import open_app
    result = open_app({"app_name": app})
    # Give the window a moment to appear, then confirm what we got.
    if uia.available():
        try:
            info = uia.window_info(app)
            return f"Opened {info.title}. {result}"
        except Exception:
            pass
    return str(result)


def app_control(parameters: dict, response=None, player=None,
                session_memory=None) -> str:
    """
    Dispatch for app_control. Required: action.
    Optional: app, intent, element, text, key, timeout.
    """
    action = str(parameters.get("action", "")).strip().lower()
    app = str(parameters.get("app", "")).strip()
    intent = str(parameters.get("intent", "") or "").strip()
    element = str(parameters.get("element", "") or "").strip()
    text = str(parameters.get("text", "") or "")
    key = str(parameters.get("key", "") or "").strip()

    if not action:
        return "Error: 'action' is required."

    # Auto-detect active foreground window if app is omitted
    if not app and action not in ("windows",):
        try:
            from core import universal_operator
            ctx = universal_operator.get_active_window()
            if ctx.title and ctx.title not in ("No active window", "Could not determine active window."):
                app = ctx.title
        except Exception:
            pass

    try:
        # High-Speed Universal Autonomous Operation (OODA Loop with Closed-Loop Verification)
        if action in ("operate", "interact", "smart_action"):
            from core import universal_operator
            return universal_operator.execute_smart_operation(
                intent=intent or element,
                app=app,
                element=element,
                text=text,
                action_type=str(parameters.get("action_type", "click")).lower()
            )

        if action == "launch":
            if not app:
                return "Error: 'app' is required for launch (e.g. 'Zoom', 'Chrome', 'Notepad')."
            return _launch(app)

        if action == "windows":
            wins = uia.list_windows()
            if not wins:
                return "No visible windows found."
            return "Open windows: " + "; ".join(w["title"] for w in wins[:15])

        if not uia.available():
            # No UIA at all — degrade to vision for click, honest error else.
            if action in ("click", "double_click"):
                out = _vision_fallback(app, element or intent or "the button to click")
                return out or "UI automation unavailable and visual search failed."
            return ("UI automation unavailable on this system — "
                    "use computer_control screen_find/screen_click instead.")

        timeout = float(parameters.get("timeout", 0) or 0)

        if action == "elements":
            info, controls = uia.list_controls(
                app, include_hidden=bool(parameters.get("include_hidden")),
                limit=int(parameters.get("limit", 60) or 60))
            if not controls:
                return (f"'{info.title}' has no named controls — "
                        "it may be a canvas; use screen_find.")
            lines = [f"{i+1}. {c.name} ({c.control_type or '?'})"
                     for i, c in enumerate(controls)]
            return f"Elements in '{info.title}':\n" + "\n".join(lines)

        if action == "state":
            info = uia.window_info(app)
            _, controls = uia.list_controls(app, limit=15)
            names = ", ".join(c.name for c in controls[:15]) or "none"
            return f"Active app: {info.title}. Top elements: {names}"

        if action in ("click", "double_click"):
            target_el = element or intent
            if not target_el:
                return "Error: 'element' or 'intent' is required for click."
            try:
                if action == "click":
                    return uia.click(app, target_el, timeout=timeout)
                return uia.double_click(app, target_el, timeout=timeout)
            except RuntimeError as e:
                # First try universal domain operator fallback
                try:
                    from core import universal_operator
                    res = universal_operator.execute_smart_operation(
                        intent=target_el, app=app, element=target_el, action_type=action
                    )
                    if res and not res.startswith("Error"):
                        return res
                except Exception:
                    pass

                # If still not found, try visual grounding with Gemini Vision
                out = _vision_fallback(app, target_el)
                if out:
                    return out
                return str(e)

        if action == "type":
            if not text:
                return "Error: 'text' is required for type."
            if not element:
                # Type into whatever has focus — still useful mid-workflow.
                import pyautogui
                if text.isascii():
                    pyautogui.write(text, interval=0.01)
                else:
                    uia._paste(text)
                return "Typed into the focused control."
            try:
                return uia.type_text(app, element, text,
                                     clear=bool(parameters.get("clear")),
                                     timeout=timeout)
            except RuntimeError as e:
                # Focus by coordinates via vision, then type.
                try:
                    uia.connect(app)
                    time.sleep(0.2)
                except Exception:
                    pass
                try:
                    from actions.computer_control import _screen_find
                    coords = _screen_find(f"the text input '{element}'")
                except Exception:
                    coords = None
                if coords:
                    import pyautogui
                    pyautogui.click(coords[0], coords[1])
                    if parameters.get("clear"):
                        pyautogui.hotkey("ctrl", "a")
                    if text.isascii():
                        pyautogui.write(text, interval=0.01)
                    else:
                        uia._paste(text)
                    return f"Typed into '{element}' found visually."
                return str(e)

        if action == "press":
            if not key:
                return "Error: 'key' is required for press (e.g. 'enter')."
            return uia.press(app, key, element=element)

        if action == "wait":
            if not element:
                return "Error: 'element' is required for wait."
            return uia.wait_for(app, element,
                                timeout=float(parameters.get("timeout", 8) or 8))

        return (f"Unknown action '{action}'. Valid: operate, launch, windows, elements, "
                "state, click, double_click, type, press, wait.")

    except RuntimeError as e:
        return str(e)
    except Exception as e:
        return f"app_control failed: {e}"


TOOL = {
    "name": "app_control",
    "description": (
        "Universal intelligent desktop & native Windows app operator with closed-loop verification. "
        "Actions: operate (smart autonomous interaction by intent/goal with closed-loop verification, "
        "e.g. intent='create meeting' in Zoom, 'autofit' in Excel, 'new tab' in Chrome), "
        "launch (open app and verify window), windows (list open windows), "
        "elements (list visible controls in app), state (active app + top elements), "
        "click / double_click (press named element with multi-tier fallback), "
        "type (write text into field), press (send key), wait (block until element appears). "
        "If 'app' is omitted, it automatically detects and operates the active foreground window!"),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "enum": ["operate", "launch", "windows", "elements", "state", "click",
                         "double_click", "type", "press", "wait"],
                "description": "What to do.",
            },
            "app": {
                "type": "STRING",
                "description": "Window title hint, e.g. 'Zoom', 'Excel', 'Chrome', 'Notepad'. Optional: auto-detects active window if omitted.",
            },
            "intent": {
                "type": "STRING",
                "description": "Goal or intent to execute (e.g. 'create meeting', 'mute', 'save file', 'new tab').",
            },
            "element": {
                "type": "STRING",
                "description": "Element name as shown by elements/state, e.g. 'New Slide', 'File', 'New Meeting'.",
            },
            "text": {"type": "STRING", "description": "Text to type (type action)."},
            "key": {"type": "STRING", "description": "Key name, e.g. 'enter', 'esc' (press action)."},
            "clear": {"type": "BOOLEAN", "description": "Select existing text first (type action)."},
            "timeout": {"type": "NUMBER", "description": "Seconds to wait for the element (default 0)."},
        },
        "required": ["action"],
    },
    "handler": app_control,
}
