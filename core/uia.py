"""
Native Windows UI element engine — the observe/act layer over any desktop app.

Talks to real controls (buttons, edits, list items, menu entries) through
Windows UI Automation via pywinauto, instead of blind keystrokes. Every call
follows the same shape: connect to a window by a loose title hint, then find
an element inside it by a loose name.

Matching is deliberately forgiving: exact match wins, then case-insensitive
equality, then substring, then token overlap — so "Sign In Button" finds
"Sign In", and the model never needs the pixel-perfect automation id.

pywinauto is imported lazily and every Windows-only call degrades to a clear
error string, so this module imports (and its pure helpers test) on any OS.
"""
from __future__ import annotations

import re
import time
from dataclasses import dataclass, field
from typing import Optional

try:
    import pywinauto  # noqa: F401
    _HAS_PYWINAUTO = True
except Exception:
    _HAS_PYWINAUTO = False


@dataclass
class Control:
    """One element as the rest of the system sees it — plain, serializable."""
    name: str
    control_type: str = ""
    visible: bool = True
    enabled: bool = True
    rect: tuple[int, int, int, int] = (0, 0, 0, 0)  # left, top, right, bottom

    @property
    def center(self) -> tuple[int, int]:
        l, t, r, b = self.rect
        return (l + r) // 2, (t + b) // 2

    def as_dict(self) -> dict:
        cx, cy = self.center
        return {
            "name": self.name,
            "type": self.control_type,
            "visible": self.visible,
            "enabled": self.enabled,
            "center": [cx, cy],
        }


@dataclass
class WindowInfo:
    title: str
    rect: tuple[int, int, int, int] = (0, 0, 0, 0)
    visible: bool = True

    def as_dict(self) -> dict:
        return {"title": self.title, "rect": list(self.rect), "visible": self.visible}


def available() -> bool:
    """True when the UIA backend can be used on this machine."""
    return _HAS_PYWINAUTO


# ---------------------------------------------------------------------------
# Pure matching helpers — no Windows dependency, unit-testable anywhere.
# ---------------------------------------------------------------------------

def _norm(s: str) -> str:
    return re.sub(r"\s+", " ", (s or "").strip()).lower()


def _tokens(s: str) -> set[str]:
    return {t for t in re.split(r"[^a-z0-9]+", _norm(s)) if t}


def score(candidate: str, wanted: str) -> float:
    """How well `candidate` matches `wanted`, 0.0 … 1.0. Higher is better."""
    c, w = _norm(candidate), _norm(wanted)
    if not c or not w:
        return 0.0
    if c == w:
        return 1.0
    if w in c:                       # substring: "Sign In" in "Sign In Button"
        return 0.8 + 0.15 * (len(w) / len(c))
    if c in w:
        return 0.7 + 0.1 * (len(c) / len(w))
    ct, wt = _tokens(candidate), _tokens(wanted)
    if ct and wt:
        overlap = len(ct & wt) / len(wt)
        if overlap == 1.0:           # all wanted tokens present, any order
            return 0.65
        if overlap > 0:
            return 0.4 * overlap
    return 0.0


def best_match(candidates: list[str], wanted: str,
               min_score: float = 0.4) -> Optional[str]:
    """The candidate that best matches `wanted`, or None."""
    scored = [(score(c, wanted), c) for c in candidates if c]
    scored = [(s, c) for s, c in scored if s >= min_score]
    if not scored:
        return None
    # Highest score wins; ties break on the shorter (more specific) candidate.
    scored.sort(key=lambda p: (-p[0], len(p[1])))
    return scored[0][1]


# ---------------------------------------------------------------------------
# Window discovery
# ---------------------------------------------------------------------------

def _require() -> Optional[str]:
    """Returns an error string when UIA is unusable, else None."""
    if not _HAS_PYWINAUTO:
        return ("UI automation unavailable (pywinauto not installed) — "
                "use screen_find/screen_click instead.")
    return None


def _window_visible(w) -> bool:
    try:
        return bool(w.is_visible())
    except Exception:
        return True


def list_windows() -> list[dict]:
    """Titles of every visible top-level window."""
    err = _require()
    if err:
        return []
    from pywinauto import Desktop
    out = []
    try:
        # pywinauto 0.6.9: the uia backend's find_elements() no longer accepts
        # a visible= kwarg — ask for everything, filter ourselves.
        for w in Desktop(backend="uia").windows():
            try:
                if not w.is_visible():
                    continue
            except Exception:
                pass
            title = (w.window_text() or "").strip()
            if title:
                out.append({"title": title})
    except Exception:
        return []
    return out


def connect(app: str, timeout: float = 0.0):
    """
    Connect to the window whose title best matches `app`. Returns a pywinauto
    WindowSpecification, or raises RuntimeError with a helpful message.
    """
    err = _require()
    if err:
        raise RuntimeError(err)
    from pywinauto import Desktop

    deadline = time.time() + max(0.0, timeout)
    last_titles: list[str] = []
    while True:
        try:
            # Visible-only filtering happens below (see list_windows for why).
            windows = list(Desktop(backend="uia").windows())
            windows = [w for w in windows if _window_visible(w)]
        except Exception as e:
            raise RuntimeError(f"UIA desktop scan failed: {e}")

        titles = [(w.window_text() or "").strip() for w in windows]
        titles = [t for t in titles if t]
        last_titles = titles
        chosen = best_match(titles, app)
        if chosen:
            for w in windows:
                if (w.window_text() or "").strip() == chosen:
                    try:
                        w.set_focus()
                    except Exception:
                        pass
                    return w
        if time.time() >= deadline:
            break
        time.sleep(0.4)

    hint = ", ".join(last_titles[:6]) if last_titles else "no windows found"
    raise RuntimeError(f"No window matches '{app}'. Open windows: {hint}")


def window_info(app: str) -> WindowInfo:
    w = connect(app)
    try:
        rect = w.rectangle()
        r = (rect.left, rect.top, rect.right, rect.bottom)
    except Exception:
        r = (0, 0, 0, 0)
    return WindowInfo(title=w.window_text() or app, rect=r)


# ---------------------------------------------------------------------------
# Element discovery & interaction
# ---------------------------------------------------------------------------

def _iter_controls(window) -> list[Control]:
    """Flatten the window's control tree into plain Control records."""
    out: list[Control] = []
    try:
        descendants = window.descendants()
    except Exception:
        descendants = []
    for el in descendants:
        try:
            name = (el.window_text() or "").strip()
            ctype = ""
            try:
                ctype = el.element_info.control_type or ""
            except Exception:
                pass
            visible = bool(el.is_visible())
            enabled = bool(el.is_enabled())
            r = el.rectangle()
            rect = (r.left, r.top, r.right, r.bottom)
        except Exception:
            continue
        if not name:
            continue
        out.append(Control(name=name, control_type=ctype,
                           visible=visible, enabled=enabled, rect=rect))
    return out


def list_controls(app: str, include_hidden: bool = False,
                  limit: int = 60) -> tuple[WindowInfo, list[Control]]:
    """Visible, named controls inside the app's window — the model's eyes."""
    w = connect(app)
    controls = _iter_controls(w)
    if not include_hidden:
        controls = [c for c in controls if c.visible]

    # De-duplicate by name, keep the first (outermost) of each.
    seen: set[str] = set()
    unique: list[Control] = []
    for c in controls:
        key = _norm(c.name)
        if key in seen:
            continue
        seen.add(key)
        unique.append(c)

    title = w.window_text() or app
    rect = (0, 0, 0, 0)
    try:
        r = w.rectangle()
        rect = (r.left, r.top, r.right, r.bottom)
    except Exception:
        pass
    return WindowInfo(title=title, rect=rect), unique[:limit]


def find(app: str, element: str, timeout: float = 0.0) -> Control:
    """
    Find a named element inside the app's window. Raises RuntimeError when
    nothing matches — callers turn that into guidance for the next step.
    """
    w = connect(app)
    deadline = time.time() + max(0.0, timeout)
    last: list[Control] = []
    while True:
        controls = [c for c in _iter_controls(w) if c.visible and c.enabled]
        last = controls
        chosen = best_match([c.name for c in controls], element)
        if chosen:
            for c in controls:
                if c.name == chosen:
                    return c
        if time.time() >= deadline:
            break
        time.sleep(0.4)

    available = ", ".join(sorted({c.name for c in last})[:12]) or "none"
    raise RuntimeError(
        f"'{element}' not found in this window. Visible elements: {available}")


def _locate(window, target: Control):
    """Re-find the live pywinauto element for a Control record."""
    for el in window.descendants():
        try:
            if ((el.window_text() or "").strip() == target.name
                    and el.is_visible()):
                return el
        except Exception:
            continue
    return None


def click(app: str, element: str, timeout: float = 0.0) -> str:
    """Click a named element. Returns a human-readable success line."""
    w = connect(app)
    target = find(app, element, timeout=timeout)
    el = _locate(w, target)
    if el is None:
        # Coordinates still work even when the live handle moved on.
        cx, cy = target.center
        import pyautogui
        pyautogui.click(cx, cy)
        return f"Clicked '{target.name}' at ({cx},{cy}) by coordinates."
    el.click_input()
    return f"Clicked '{target.name}' ({target.control_type or 'control'})."


def double_click(app: str, element: str, timeout: float = 0.0) -> str:
    w = connect(app)
    target = find(app, element, timeout=timeout)
    el = _locate(w, target)
    if el is None:
        cx, cy = target.center
        import pyautogui
        pyautogui.doubleClick(cx, cy)
        return f"Double-clicked '{target.name}' at ({cx},{cy})."
    el.double_click_input()
    return f"Double-clicked '{target.name}'."


def type_text(app: str, element: str, text: str, clear: bool = False,
              timeout: float = 0.0) -> str:
    """
    Type into a named element. Prefers setting the edit value directly
    (fast, reliable); falls back to focusing + keyboard typing for
    controls that reject direct sets.
    """
    w = connect(app)
    target = find(app, element, timeout=timeout)
    el = _locate(w, target)

    if el is None:
        # Focus by coordinates, then let the keyboard do the work.
        cx, cy = target.center
        import pyautogui
        pyautogui.click(cx, cy)
        if clear:
            pyautogui.hotkey("ctrl", "a")
        pyautogui.write(text, interval=0.01) if text.isascii() else _paste(text)
        return f"Typed into '{target.name}' via keyboard."

    try:
        if clear:
            el.set_edit_text(text)
        else:
            # Append: set to current value + new text when we can read it.
            try:
                current = el.window_text() or ""
            except Exception:
                current = ""
            el.set_edit_text(current + text)
        return f"Set text of '{target.name}'."
    except Exception:
        try:
            el.set_focus()
        except Exception:
            pass
        import pyautogui
        if clear:
            pyautogui.hotkey("ctrl", "a")
        if text.isascii():
            pyautogui.write(text, interval=0.01)
        else:
            _paste(text)
        return f"Typed into '{target.name}' via keyboard."


def _paste(text: str) -> None:
    """Non-ASCII text goes through the clipboard — pyautogui.write is ASCII-only."""
    import pyperclip
    import pyautogui
    old = None
    try:
        old = pyperclip.paste()
    except Exception:
        pass
    pyperclip.copy(text)
    pyautogui.hotkey("ctrl", "v")
    time.sleep(0.1)
    if old is not None:
        try:
            pyperclip.copy(old)
        except Exception:
            pass


def press(app: str, key: str, element: str = "") -> str:
    """Press a key, optionally after focusing a named element."""
    import pyautogui
    if element:
        target = find(app, element)
        w = connect(app)
        el = _locate(w, target)
        try:
            if el is not None:
                el.set_focus()
            else:
                cx, cy = target.center
                pyautogui.click(cx, cy)
        except Exception:
            cx, cy = target.center
            pyautogui.click(cx, cy)
    pyautogui.press(key)
    return f"Pressed '{key}'" + (f" in '{element}'." if element else ".")


def wait_for(app: str, element: str, timeout: float = 8.0) -> str:
    """Block until an element appears — the glue between chained steps."""
    target = find(app, element, timeout=timeout)
    return f"'{target.name}' is present ({target.control_type or 'control'})."
