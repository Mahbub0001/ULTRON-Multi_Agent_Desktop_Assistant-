# actions/game_control.py
"""
Game launcher & in-game navigation for Mark-LIV.

The common ask — "open Roblox and select Blox Fruits" — is two very different
problems, and this action handles both:

  1. LAUNCH  — deep-link straight into the experience. Roblox registers the
     ``roblox://`` protocol, so the desktop client can start directly into a
     game's place instead of crawling the website. We wait for the player
     process to actually come up; if the protocol isn't wired (or the client
     isn't installed), we fall back to the game's web page and click Play
     with visual element search.
  2. STOP    — kill the game process cleanly when asked.

Registry entries are plain dicts, so adding a game is one line.
"""
from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path

try:
    import psutil
    _PSUTIL = True
except ImportError:
    _PSUTIL = False

from core.uia import best_match

# engine: "roblox" (asset deep link) | "steam" (rungameid) | "app" (exe alias)
GAMES: dict[str, dict] = {
    # -- Roblox experiences (asset ids) --
    "blox fruits":              {"title": "Blox Fruits",             "engine": "roblox", "asset_id": 2753915549},
    "brookhaven":               {"title": "Brookhaven RP",           "engine": "roblox", "asset_id": 4924922222},
    "brookhaven rp":            {"title": "Brookhaven RP",           "engine": "roblox", "asset_id": 4924922222},
    "adopt me":                 {"title": "Adopt Me",                "engine": "roblox", "asset_id": 920587237},
    "tower of hell":            {"title": "Tower of Hell",           "engine": "roblox", "asset_id": 1962086868},
    "natural disaster survival": {"title": "Natural Disaster Survival", "engine": "roblox", "asset_id": 189707},
    "murder mystery 2":         {"title": "Murder Mystery 2",        "engine": "roblox", "asset_id": 142823291},
    "meepcity":                 {"title": "MeepCity",                "engine": "roblox", "asset_id": 5520316},
    "arsenal":                  {"title": "Arsenal",                 "engine": "roblox", "asset_id": 286090429},
    "jailbreak":                {"title": "Jailbreak",               "engine": "roblox", "asset_id": 606849621},

    # -- Launchers / apps (delegate to open_app aliases) --
    "roblox":                   {"title": "Roblox",        "engine": "app", "app": "roblox"},
    "steam":                    {"title": "Steam",         "engine": "app", "app": "steam"},
    "minecraft":                {"title": "Minecraft",     "engine": "app", "app": "minecraft"},
    "fortnite":                 {"title": "Fortnite",      "engine": "app", "app": "fortnite"},
    "epic games":               {"title": "Epic Games",    "engine": "app", "app": "epic"},
}

_ROBLOX_PROCESS = ("robloxbetapro", "robloxplayerbeta", "windows10universal")
_GAME_PROCESS: dict[str, list[str]] = {
    "minecraft": ("javaw", "minecraft"),
    "fortnite":  ("fortniteclient-win64",),
    "steam":     ("steam",),
}


# ---------------------------------------------------------------------------
# Process helpers
# ---------------------------------------------------------------------------

def _running(names: tuple[str, ...]) -> bool:
    if _PSUTIL:
        try:
            for p in psutil.process_iter(["name"]):
                n = (p.info.get("name") or "").lower()
                if any(t in n for t in names):
                    return True
        except Exception:
            return False
        return False
    # Fallback: tasklist once (Windows only).
    try:
        out = subprocess.run(["tasklist", "/FO", "CSV", "/NH"],
                             capture_output=True, text=True, timeout=8)
        low = out.stdout.lower()
        return any(t in low for t in names)
    except Exception:
        return False


def _wait_for(names: tuple[str, ...], timeout: float) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if _running(names):
            return True
        time.sleep(0.7)
    return False


def _kill(names: tuple[str, ...]) -> list[str]:
    killed = []
    if _PSUTIL:
        try:
            for p in psutil.process_iter(["name", "pid"]):
                n = (p.info.get("name") or "").lower()
                if any(t in n for t in names):
                    try:
                        p.terminate()
                        killed.append(f"{n} ({p.info['pid']})")
                    except Exception:
                        pass
        except Exception:
            pass
        return killed
    for t in names:
        try:
            subprocess.run(["taskkill", "/IM", f"{t}.exe", "/F"],
                           capture_output=True, timeout=8)
            killed.append(t)
        except Exception:
            pass
    return killed


# ---------------------------------------------------------------------------
# Launch strategies
# ---------------------------------------------------------------------------

def _open_url(url: str) -> bool:
    try:
        if os.name == "nt":
            os.startfile(url)  # noqa: S606 — registered protocol / URL
        else:
            opener = "open" if sys_is_darwin() else "xdg-open"
            subprocess.Popen([opener, url])
        return True
    except Exception:
        try:
            if os.name == "nt":
                subprocess.Popen(["cmd", "/c", "start", "", url])
                return True
        except Exception:
            pass
    return False


def sys_is_darwin() -> bool:
    import sys
    return sys.platform == "darwin"


def _vision_click(description: str) -> bool:
    """Find something on screen with Gemini and click it. Best effort."""
    try:
        from actions.computer_control import _screen_find
        coords = _screen_find(description)
        if not coords:
            return False
        import pyautogui
        pyautogui.click(coords[0], coords[1])
        return True
    except Exception:
        return False


def _play_roblox(entry: dict, game_name: str) -> str:
    asset = entry.get("asset_id")
    title = entry.get("title", game_name)

    # 1) Deep link — the client starts straight into the experience.
    if asset:
        uri = f"roblox://experiences/start?assetId={asset}"
        if _open_url(uri):
            if _wait_for(_ROBLOX_PROCESS, timeout=12):
                return (f"Roblox is launching '{title}' — the experience "
                        "client is starting.")
            # Protocol didn't produce a player (not installed / not signed in).

    # 2) Web fallback — open the game page, click Play with vision.
    if asset:
        page = f"https://www.roblox.com/games/{asset}"
        if _open_url(page):
            time.sleep(5)                       # let the page render
            if _vision_click("the green Play button on the Roblox game page"):
                return (f"Opened '{title}' on roblox.com and pressed Play — "
                        "the client should follow in a moment.")
            return (f"Opened '{title}' on roblox.com (page: {page}). "
                    "Press the green Play button to start, or say "
                    "'click play' and I will click it.")

    # 3) Just the Roblox home/app.
    from actions.open_app import open_app
    return str(open_app({"app_name": "roblox"}))


def _launch_app(entry: dict) -> str:
    from actions.open_app import open_app
    return str(open_app({"app_name": entry.get("app", entry.get("title", ""))}))


# ---------------------------------------------------------------------------
# Handler
# ---------------------------------------------------------------------------

def game_control(parameters: dict, response=None, player=None,
                 session_memory=None) -> str:
    action = str(parameters.get("action", "")).strip().lower()
    game = str(parameters.get("game", "") or "").strip()

    if not action:
        return "Error: 'action' is required."

    try:
        # ---- list -------------------------------------------------------
        if action in ("list", "games", "library"):
            lines = [f"- {v['title']} ({v['engine']})" for v in GAMES.values()]
            return "Known games:\n" + "\n".join(lines)

        if not game:
            return "Error: 'game' is required."

        key = best_match(list(GAMES.keys()), game, min_score=0.35) or \
            game.lower().strip()
        entry = GAMES.get(key)

        # ---- stop -------------------------------------------------------
        if action in ("stop", "close", "kill"):
            names = _GAME_PROCESS.get(key) or (
                _ROBLOX_PROCESS if (entry and entry.get("engine") == "roblox")
                else (key,))
            killed = _kill(tuple(names))
            if killed:
                return f"Closed: {', '.join(killed)}."
            return f"{game} is not running."

        # ---- play (default) ---------------------------------------------
        if action in ("play", "launch", "open", "start"):
            if entry is None:
                # Unknown title — try opening it as an app/alias.
                from actions.open_app import open_app
                out = str(open_app({"app_name": game}))
                return f"'{game}' is not in the game library — tried launching it as an app. {out}"

            engine = entry.get("engine")
            if engine == "roblox":
                return _play_roblox(entry, game)
            if engine == "steam" and entry.get("id"):
                uri = f"steam://rungameid/{entry['id']}"
                if _open_url(uri):
                    time.sleep(3)
                    return f"Launching '{entry['title']}' through Steam."
                return "Could not reach Steam — is it installed?"
            if engine == "app":
                names = _GAME_PROCESS.get(key)
                out = _launch_app(entry)
                if names and _wait_for(tuple(names), timeout=10):
                    return f"{out} {entry['title']} is running."
                return out

            return f"Game '{entry.get('title', game)}' has no launch method."

        return (f"Unknown action '{action}'. Valid: play, stop, list.")

    except Exception as e:
        return f"game_control failed: {e}"


TOOL = {
    "name": "game_control",
    "description": (
        "Launch and stop games. Actions: play (deep-link a Roblox experience "
        "straight into the desktop client — waits for it to start and falls "
        "back to the web page + clicking Play; launches Steam/app games "
        "directly), stop (kill the game's process), list (show the game "
        "library). Use for 'open Roblox and select Blox Fruits', 'play Tower "
        "of Hell', 'launch Minecraft', or 'close the game'. Unknown titles "
        "are tried as app names."),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "enum": ["play", "stop", "list"],
                "description": "What to do.",
            },
            "game": {
                "type": "STRING",
                "description": "Game title, e.g. 'Blox Fruits', 'Minecraft', 'Roblox'.",
            },
        },
        "required": ["action"],
    },
    "handler": game_control,
}
