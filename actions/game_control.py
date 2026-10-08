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


def _vision_click(description: str, window_title: str = "") -> bool:
    """Find something on screen with Gemini and click it. Best effort."""
    try:
        from actions.computer_control import _screen_find, _focus_window
        if window_title:
            _focus_window(window_title)
            time.sleep(0.2)
        coords = _screen_find(description, window_title=window_title)
        if not coords:
            return False
        import pyautogui
        pyautogui.click(coords[0], coords[1])
        return True
    except Exception:
        return False


def _look_game(window_title: str = "") -> str:
    """Capture game screen and summarize current in-game state and HUD."""
    try:
        from actions.computer_control import _focus_window
        import pyautogui
        from PIL import ImageGrab
        import io
        from core import gemini
        from google.genai import types as gtypes

        if window_title:
            _focus_window(window_title)
            time.sleep(0.3)

        img = None
        try:
            img = pyautogui.screenshot()
        except Exception:
            try:
                img = ImageGrab.grab()
            except Exception:
                pass
        if img is None:
            return "Could not capture game screen."

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        image_bytes = buf.getvalue()

        prompt = (
            "Analyze this game screenshot. Describe in 2-3 concise sentences: "
            "1. Current game state (main menu, in-game match, loading, settings, lobby). "
            "2. Visible buttons or options the player can interact with. "
            "3. Any important alerts or status info."
        )
        response = gemini.call(
            [gtypes.Part.from_bytes(data=image_bytes, mime_type="image/png"), prompt],
            tier="gemini-2.5-flash", timeout_ms=15_000,
        )
        return (response.text or "").strip() if response else "No response from vision."
    except Exception as e:
        return f"Could not inspect game screen: {e}"


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

        key = best_match(list(GAMES.keys()), game, min_score=0.35) if game else None
        entry = GAMES.get(key) if key else None

        # ---- stop -------------------------------------------------------
        if action in ("stop", "close", "kill"):
            if not game:
                return "Error: 'game' is required to stop."
            names = _GAME_PROCESS.get(key) if key else None
            if not names:
                names = (_ROBLOX_PROCESS if (entry and entry.get("engine") == "roblox") else (game,))
            killed = _kill(tuple(names))
            if killed:
                return f"Closed: {', '.join(killed)}."
            return f"{game} is not running."

        # ---- play (default) ---------------------------------------------
        if action in ("play", "launch", "open", "start"):
            if not game:
                return "Error: 'game' is required to launch."
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

        # ---- click / button in-game -------------------------------------
        if action in ("click", "button", "menu"):
            button = str(parameters.get("button", "") or parameters.get("element", "") or parameters.get("description", "")).strip()
            if not button:
                return "Error: 'button' or 'element' description is required for in-game click."
            target_game = entry.get("title") if entry else game
            from actions.computer_control import _screen_find, _focus_window
            if target_game:
                _focus_window(target_game)
                time.sleep(0.25)
            coords = _screen_find(button, window_title=target_game or "")
            if coords:
                import pyautogui
                time.sleep(0.15)
                pyautogui.click(coords[0], coords[1])
                return f"Clicked in-game '{button}' at {coords}."
            return f"In-game element '{button}' not found on screen."

        # ---- press in-game key ------------------------------------------
        if action in ("press", "key", "hotkey"):
            key_name = str(parameters.get("key", "") or parameters.get("keys", "") or "esc").strip().lower()
            target_game = entry.get("title") if entry else game
            from actions.computer_control import _focus_window
            if target_game:
                _focus_window(target_game)
                time.sleep(0.2)
            import pyautogui
            if "+" in key_name:
                keys = [k.strip() for k in key_name.split("+")]
                pyautogui.hotkey(*keys)
                return f"Sent in-game hotkey '{key_name}'."
            pyautogui.press(key_name)
            return f"Pressed in-game key '{key_name}'."

        # ---- type in-game -----------------------------------------------
        if action in ("type", "chat"):
            text = str(parameters.get("text", "")).strip()
            if not text:
                return "Error: 'text' is required for type."
            target_game = entry.get("title") if entry else game
            from actions.computer_control import _focus_window
            if target_game:
                _focus_window(target_game)
                time.sleep(0.2)
            import pyautogui
            pyautogui.typewrite(text, interval=0.03)
            return f"Typed in game: '{text}'."

        # ---- configure in-game settings ---------------------------------
        if action in ("configure", "setting", "settings"):
            setting = str(parameters.get("setting", "") or parameters.get("option", "")).strip().lower()
            target_game = entry.get("title") if entry else game
            from actions.computer_control import _focus_window
            if target_game:
                _focus_window(target_game)
                time.sleep(0.2)
            import pyautogui
            if setting in ("fullscreen", "full_screen"):
                pyautogui.press("f11")
                return "Toggled in-game fullscreen (F11)."
            if setting in ("menu", "pause", "escape"):
                pyautogui.press("esc")
                return "Toggled in-game pause / menu (Esc)."
            if setting in ("chat", "open_chat"):
                pyautogui.press("/")
                return "Opened in-game chat."
            if setting in ("leaderboard", "scores"):
                pyautogui.press("tab")
                return "Toggled in-game leaderboard (Tab)."
            return f"Configured '{setting}' in game."

        # ---- look / inspect screen --------------------------------------
        if action in ("look", "inspect", "state"):
            target_game = entry.get("title") if entry else game
            return _look_game(target_game)

        return (f"Unknown action '{action}'. Valid: play, stop, list, click, press, type, configure, look.")

    except Exception as e:
        return f"game_control failed: {e}"


TOOL = {
    "name": "game_control",
    "description": (
        "Launch, stop, configure, and control games & in-game interfaces. Actions: "
        "play (deep-link Roblox experiences directly into client, launch Steam games, "
        "or open game executables), stop (kill game process), list (show game library), "
        "click (visually find and click in-game menu/button e.g. Play, Settings, Resume), "
        "press (send in-game keys e.g. esc, space, f11), type (type in-game chat/inputs), "
        "configure (toggle fullscreen, pause menu, chat), look (inspect active game screen "
        "with vision and describe menu/HUD/state). Works across Roblox, Steam, Minecraft, and any PC game."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "enum": ["play", "stop", "list", "click", "press", "type", "configure", "look"],
                "description": "What to do.",
            },
            "game": {
                "type": "STRING",
                "description": "Game title, e.g. 'Blox Fruits', 'Minecraft', 'Roblox'. Optional if game is already active.",
            },
            "button": {
                "type": "STRING",
                "description": "Name or appearance of in-game button to click, e.g. 'Play', 'Settings', 'Resume', 'Join'.",
            },
            "key": {
                "type": "STRING",
                "description": "In-game key or shortcut to press, e.g. 'esc', 'space', 'f11', 'tab'.",
            },
            "text": {
                "type": "STRING",
                "description": "Text to type into in-game chat or inputs.",
            },
            "setting": {
                "type": "STRING",
                "description": "Setting to configure, e.g. 'fullscreen', 'menu', 'chat', 'leaderboard'.",
            },
        },
        "required": ["action"],
    },
    "handler": game_control,
}
