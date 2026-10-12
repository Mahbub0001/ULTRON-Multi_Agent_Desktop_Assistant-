"""Window focus and state changes with Win32 read-back, shared by all adapters."""
import time
from core.tool_result import VerifiedResult


def focus(title=""):
    import win32gui as gui
    import win32con as con
    active = gui.GetForegroundWindow()
    if not title:
        if not active or not gui.IsWindowVisible(active):
            raise RuntimeError("No active target window")
        return active
    query = title.casefold().strip()
    if active and query in gui.GetWindowText(active).casefold():
        return active
    matches = []
    def collect(hwnd, _):
        if gui.IsWindowVisible(hwnd) and query in gui.GetWindowText(hwnd).casefold():
            matches.append(hwnd)
    gui.EnumWindows(collect, None)
    exact = [h for h in matches if gui.GetWindowText(h).casefold() == query]
    matches = exact or matches
    if len(matches) != 1:
        raise RuntimeError(f"Target '{title}' has {len(matches)} matching windows; use its exact title")
    hwnd = matches[0]
    if gui.IsIconic(hwnd):
        gui.ShowWindow(hwnd, con.SW_RESTORE)
    gui.SetForegroundWindow(hwnd)
    deadline = time.monotonic() + 0.5
    while time.monotonic() < deadline:
        if gui.GetForegroundWindow() == hwnd:
            return hwnd
        time.sleep(0.025)
    raise RuntimeError(f"Could not confirm focus on '{title}'")


def window_state(action, title=""):
    import win32gui as gui
    import win32con as con
    try:
        hwnd = focus(title)
        desired = {"maximize": con.SW_SHOWMAXIMIZED, "minimize": con.SW_SHOWMINIMIZED,
                   "restore": con.SW_SHOWNORMAL}[action]
        gui.ShowWindow(hwnd, {"maximize": con.SW_MAXIMIZE, "minimize": con.SW_MINIMIZE,
                              "restore": con.SW_RESTORE}[action])
        deadline = time.monotonic() + 0.6
        while time.monotonic() < deadline:
            actual = gui.GetWindowPlacement(hwnd)[1]
            if actual == desired:
                evidence = {"hwnd": hwnd, "title": gui.GetWindowText(hwnd), "window_state": action}
                return VerifiedResult(f"Verified window {action}: {evidence['title']}", evidence)
            time.sleep(0.025)
        return f"Error: Could not verify window {action}."
    except Exception as exc:
        return f"Error: Window {action} failed: {exc}"


def close_window(title=""):
    """Request normal closure of one identified window and read back its state."""
    import win32gui as gui
    import win32con as con
    try:
        hwnd = focus(title)
        name = gui.GetWindowText(hwnd)
        gui.PostMessage(hwnd, con.WM_CLOSE, 0, 0)
        for _ in range(16):
            if not gui.IsWindow(hwnd) or not gui.IsWindowVisible(hwnd):
                return VerifiedResult(f"Verified window closed: {name}", {"hwnd": hwnd, "closed": True})
            time.sleep(.1)
        return (f"Error: Close requested for '{name}', but the window remains open. "
                "Check for a save prompt or confirmation dialog; do not force-close or repeat automatically.")
    except Exception as exc:
        return f"Error: Window close failed: {exc}"
