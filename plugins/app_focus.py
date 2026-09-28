"""
Game & App Quick Launcher / Window Focus Plugin for ULTRON / JARVIS.

Allows launching games and apps (Steam, Epic, Valorant, GTA V, VS Code, Chrome, etc.),
focusing existing open application windows, bringing windows to front, and closing apps.
"""

from __future__ import annotations

import os
import sys
import subprocess
import platform
from pathlib import Path

_SYSTEM = platform.system()
if _SYSTEM == "Windows":
    _WIN_HIDE: dict = {"creationflags": subprocess.CREATE_NO_WINDOW}
else:
    _WIN_HIDE: dict = {}

PLUGIN = {
    "name": "app_focus",
    "description": (
        "Game & App Quick Launcher and Window Focus Manager. "
        "Allows launching games (Steam, Epic, Valorant, GTA V, Minecraft, CS2, etc.) or applications, "
        "focusing/bringing an open window to the front, and closing/terminating apps. "
        "Use this plugin when the user asks to launch a game/app, focus/switch to an open window, "
        "or close an app."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "The action to perform: 'launch' (start app/game), 'focus' (bring open window to front), or 'close' (close app/game).",
            },
            "app_name": {
                "type": "STRING",
                "description": "Name of the app or game (e.g., 'Valorant', 'Steam', 'VS Code', 'Chrome', 'GTA V', 'Discord').",
            },
        },
        "required": ["action", "app_name"],
    },
}

# Common games and launcher URI schemes / executables
_GAME_LAUNCHERS: dict[str, str] = {
    "steam": "steam://open/main",
    "cs2": "steam://rungameid/730",
    "counter strike": "steam://rungameid/730",
    "counter-strike": "steam://rungameid/730",
    "dota 2": "steam://rungameid/570",
    "dota": "steam://rungameid/570",
    "gta v": "steam://rungameid/271590",
    "gta 5": "steam://rungameid/271590",
    "gta": "steam://rungameid/271590",
    "cyberpunk": "steam://rungameid/1091500",
    "pubg": "steam://rungameid/578080",
    "apex": "steam://rungameid/1172470",
    "apex legends": "steam://rungameid/1172470",
    "valorant": "com.riotgames.valorant",
    "epic": "com.epicgames.launcher://",
    "epic games": "com.epicgames.launcher://",
    "fortnite": "com.epicgames.launcher://apps/fn%3A4ae86284f01b4e45870859371dd22637%3AFortnite?action=launch",
}

def _focus_window_windows(app_name: str) -> bool:
    """
    Focuses a window on Windows using PowerShell / Win32 API.
    """
    ps_script = f"""
$appName = "{app_name.lower()}"
$code = @"
using System;
using System.Runtime.InteropServices;
public class WinUtil {{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}}
"@
Add-Type -TypeDefinition $code

$procs = Get-Process | Where-Logic {{ $_.MainWindowHandle -ne 0 }}
foreach ($p in Get-Process) {{
    if ($p.MainWindowHandle -ne [IntPtr]::Zero -and ($p.ProcessName.ToLower().Contains($appName) -or $p.MainWindowTitle.ToLower().Contains($appName))) {{
        [WinUtil]::ShowWindow($p.MainWindowHandle, 9) # SW_RESTORE = 9
        [WinUtil]::SetForegroundWindow($p.MainWindowHandle)
        exit 0
    }}
}}
exit 1
"""
    try:
        res = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps_script],
            capture_output=True,
            text=True,
            **_WIN_HIDE
        )
        return res.returncode == 0
    except Exception:
        return False

def _close_app_windows(app_name: str) -> bool:
    """
    Closes an app by process name or window title on Windows.
    """
    clean_name = app_name.lower().replace(".exe", "").strip()
    try:
        # Try taskkill first
        res = subprocess.run(
            ["taskkill", "/IM", f"{clean_name}.exe", "/F"],
            capture_output=True,
            text=True,
            **_WIN_HIDE
        )
        if res.returncode == 0:
            return True

        # Try PowerShell stop-process by pattern
        ps_script = f"Get-Process | Where-Object {{ $_.ProcessName -like '*{clean_name}*' -or $_.MainWindowTitle -like '*{clean_name}*' }} | Stop-Process -Force"
        res2 = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps_script],
            capture_output=True,
            text=True,
            **_WIN_HIDE
        )
        return res2.returncode == 0
    except Exception:
        return False

def _launch_app_or_game(app_name: str) -> tuple[bool, str]:
    """
    Launches an app or game via URI or system open handler.
    """
    key = app_name.lower().strip()

    # 1. Check game launcher URIs
    if key in _GAME_LAUNCHERS:
        uri = _GAME_LAUNCHERS[key]
        if _SYSTEM == "Windows":
            os.startfile(uri)
            return True, f"Sir, launching game '{app_name}' via protocol."
        elif _SYSTEM == "Darwin":
            subprocess.run(["open", uri], check=False)
            return True, f"Sir, launching '{app_name}'."
        else:
            subprocess.run(["xdg-open", uri], check=False)
            return True, f"Sir, launching '{app_name}'."

    # 2. Try actions/open_app.py open_app function if available
    try:
        from actions.open_app import open_app
        msg = open_app(app_name)
        if "not found" not in msg.lower() and "failed" not in msg.lower():
            return True, msg
    except Exception:
        pass

    # 3. Direct startfile / system call
    try:
        if _SYSTEM == "Windows":
            os.startfile(app_name)
            return True, f"Sir, started '{app_name}'."
        elif _SYSTEM == "Darwin":
            subprocess.run(["open", "-a", app_name], check=False)
            return True, f"Sir, started '{app_name}'."
        else:
            subprocess.run([app_name], check=False)
            return True, f"Sir, started '{app_name}'."
    except Exception as e:
        return False, f"Sir, could not launch '{app_name}': {e}"

def run(parameters: dict, player=None, session_memory=None) -> str:
    """
    Main plugin execution entrypoint.
    """
    action = parameters.get("action", "").lower().strip()
    app_name = parameters.get("app_name", "").strip()

    if not app_name:
        return "Sir, please specify the name of the app or game."

    result_msg = ""

    if action == "launch":
        # First check if already open, and focus if requested or launch
        success, msg = _launch_app_or_game(app_name)
        result_msg = msg

    elif action == "focus":
        if _SYSTEM == "Windows":
            focused = _focus_window_windows(app_name)
            if focused:
                result_msg = f"Sir, switched window focus to '{app_name}'."
            else:
                # If window focus failed, attempt launching
                success, msg = _launch_app_or_game(app_name)
                result_msg = f"Sir, '{app_name}' window wasn't open, so I launched it: {msg}"
        else:
            result_msg = f"Sir, window focus switching is optimized for Windows systems."

    elif action == "close":
        if _SYSTEM == "Windows":
            closed = _close_app_windows(app_name)
            if closed:
                result_msg = f"Sir, closed '{app_name}' successfully."
            else:
                result_msg = f"Sir, could not find an active window or process for '{app_name}'."
        else:
            result_msg = f"Sir, closing apps via plugin is currently supported on Windows."
    else:
        result_msg = f"Sir, unknown action '{action}'. Supported actions: launch, focus, close."

    if player:
        try:
            player.write_log(f"ULTRON (AppFocus): {result_msg}")
        except Exception:
            pass

    return result_msg
