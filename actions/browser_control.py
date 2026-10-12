
from __future__ import annotations

import asyncio
import concurrent.futures
import os
import platform
import shutil
import subprocess
import sys
import threading
import time
import webbrowser
from pathlib import Path
from typing import Optional
from urllib.parse import urlsplit, quote_plus

if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

from playwright.async_api import (
    async_playwright,
    BrowserContext,
    Page,
    Playwright,
    TimeoutError as PlaywrightTimeout,
)
_OS = platform.system()   # "Windows" | "Darwin" | "Linux"


def _incomplete_linkedin_url(url: str) -> bool:
    """Reject short model-invented paths such as /fee and /not."""
    parsed = urlsplit(_normalize_url(url))
    host = (parsed.hostname or "").casefold()
    if host != "linkedin.com" and not host.endswith(".linkedin.com"):
        return False
    part = parsed.path.strip("/").casefold()
    known = ("feed", "notifications", "messaging", "mynetwork", "jobs")
    return bool(part and len(part) < 5 and any(name.startswith(part) and name != part for name in known))


def _native_browser_window():
    from core.universal_operator import get_active_window
    active = get_active_window()
    if active.app_category != "browser":
        return None
    title = active.title.casefold()
    if "who's using chrome" in title or "choose a profile" in title:
        return None
    return active


def _read_native_url() -> str:
    import pyautogui
    import pyperclip
    previous = pyperclip.paste()
    try:
        pyautogui.hotkey("ctrl", "l")
        pyautogui.hotkey("ctrl", "c")
        import time
        time.sleep(0.08)
        return pyperclip.paste().strip()
    finally:
        pyautogui.press("esc")
        pyperclip.copy(previous)


def _navigate_native_url(url: str) -> str:
    """Navigate the foreground browser tab and read its actual address."""
    if not _native_browser_window():
        return "Error: No browser tab is in the foreground."
    import pyautogui
    import pyperclip
    previous = pyperclip.paste()
    try:
        pyautogui.hotkey("ctrl", "l")
        pyperclip.copy(_normalize_url(url))
        pyautogui.hotkey("ctrl", "v")
        pyautogui.press("enter")
    finally:
        pyperclip.copy(previous)
    return _verify_native_url(url)


def _verify_native_url(url: str) -> str:
    """Bounded read-back; a launch request alone is not navigation evidence."""
    expected_host = (urlsplit(_normalize_url(url)).hostname or "").casefold().removeprefix("www.")
    expected_path = urlsplit(_normalize_url(url)).path.rstrip("/")
    expected_query = urlsplit(_normalize_url(url)).query
    current = ""
    for _ in range(8):
        time.sleep(.35)
        if not _native_browser_window():
            continue
        current = _read_native_url()
        parsed = urlsplit(current)
        host = (parsed.hostname or "").casefold().removeprefix("www.")
        if host == expected_host and parsed.path.rstrip("/") == expected_path and parsed.query == expected_query:
            from core.tool_result import VerifiedResult
            return VerifiedResult(f"Active browser tab address verified: {current}", {"url": current})
    return f"Error: Navigation not verified. Current address: {current or 'unavailable'}. Inspect before retrying."

def _normalize_url(url: str) -> str:
    """
    Bare words like "instagram" → "https://instagram.com"
    Domains like "instagram.com" → "https://instagram.com"
    Web app shortcuts like "linkedin notifications" -> "https://www.linkedin.com/notifications"
    Full URLs pass through unchanged.
    """
    url = url.strip()
    if not url:
        return "about:blank"
    if urlsplit(url).scheme in ("http", "https"):
        return url
    
    u_lower = url.lower()
    # Map common web app shorthand
    if "linkedin" in u_lower:
        if "notification" in u_lower:
            return "https://www.linkedin.com/notifications"
        if any(k in u_lower for k in ["message", "messaging", "inbox", "chat"]):
            return "https://www.linkedin.com/messaging"
        if any(k in u_lower for k in ["network", "connection", "mynetwork"]):
            return "https://www.linkedin.com/mynetwork"
        if "job" in u_lower:
            return "https://www.linkedin.com/jobs"
        if "feed" in u_lower or "home" in u_lower:
            return "https://www.linkedin.com/feed"
        if url in ("linkedin", "linkedin.com", "www.linkedin.com"):
            return "https://www.linkedin.com"

    if "github" in u_lower:
        if "notification" in u_lower:
            return "https://github.com/notifications"
        if "profile" in u_lower or "my profile" in u_lower:
            return "https://github.com/Mahbub0001"
        if "pull" in u_lower or " pr" in u_lower or "/pr" in u_lower or u_lower.endswith("pr"):
            return "https://github.com/pulls"
        if "issue" in u_lower:
            return "https://github.com/issues"

    if "youtube" in u_lower:
        if "subscription" in u_lower:
            return "https://www.youtube.com/feed/subscriptions"
        if "history" in u_lower:
            return "https://www.youtube.com/feed/history"

    if "docs" in u_lower or "google docs" in u_lower:
        return "https://docs.google.com/document/u/0/"
    if "sheets" in u_lower or "google sheets" in u_lower:
        return "https://docs.google.com/spreadsheets/u/0/"
    if "drive" in u_lower or "google drive" in u_lower:
        return "https://drive.google.com"
    if "gmail" in u_lower:
        return "https://mail.google.com"
    if "chatgpt" in u_lower:
        return "https://chatgpt.com"
    if "claude" in u_lower:
        return "https://claude.ai"
    if "whatsapp" in u_lower:
        return "https://web.whatsapp.com"
    if "notion" in u_lower:
        return "https://www.notion.so"
    if "canva" in u_lower:
        return "https://www.canva.com"
    if "reddit" in u_lower:
        return "https://www.reddit.com"
    if "twitter" in u_lower or " x.com" in u_lower or u_lower == "x":
        return "https://x.com"

    if "://" in url:
        return url
    # No dot at all → assume .com  (e.g. "instagram" → "instagram.com")
    if "." not in url:
        url = url + ".com"
    return "https://" + url


def _user_agent() -> str:
    if _OS == "Windows":
        return (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
            "AppleWebKit/537.36 (KHTML, like Gecko) "
            "Chrome/124.0.0.0 Safari/537.36"
        )
    if _OS == "Darwin":
        return (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
            "AppleWebKit/537.36 (KHTML, like Gecko) "
            "Chrome/124.0.0.0 Safari/537.36"
        )
    return (
        "Mozilla/5.0 (X11; Linux x86_64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/124.0.0.0 Safari/537.36"
    )


def _real_profile_dir(browser: str) -> str:
    home  = Path.home()
    local = os.environ.get("LOCALAPPDATA", "")
    roam  = os.environ.get("APPDATA", "")

    candidates: list[Path] = []

    if _OS == "Windows":
        m = {
            "chrome":   [Path(local) / "Google"          / "Chrome"          / "User Data"],
            "edge":     [Path(local) / "Microsoft"        / "Edge"            / "User Data"],
            "brave":    [Path(local) / "BraveSoftware"    / "Brave-Browser"   / "User Data"],
            "vivaldi":  [Path(local) / "Vivaldi"          / "User Data"],
            "opera":    [Path(roam)  / "Opera Software"   / "Opera Stable",
                         Path(local) / "Opera Software"   / "Opera Stable"],
            "operagx":  [Path(roam)  / "Opera Software"   / "Opera GX Stable",
                         Path(local) / "Opera Software"   / "Opera GX Stable"],
        }
        candidates = m.get(browser, [])

    elif _OS == "Darwin":
        lib = home / "Library" / "Application Support"
        m = {
            "chrome":   [lib / "Google"             / "Chrome"],
            "edge":     [lib / "Microsoft Edge"],
            "brave":    [lib / "BraveSoftware"       / "Brave-Browser"],
            "vivaldi":  [lib / "Vivaldi"],
            "opera":    [lib / "com.operasoftware.Opera"],
            "operagx":  [lib / "com.operasoftware.OperaGX"],
        }
        candidates = m.get(browser, [])

    elif _OS == "Linux":
        cfg = home / ".config"
        m = {
            "chrome":   [cfg / "google-chrome", cfg / "chromium"],
            "edge":     [cfg / "microsoft-edge"],
            "brave":    [cfg / "BraveSoftware" / "Brave-Browser"],
            "vivaldi":  [cfg / "vivaldi"],
            "opera":    [cfg / "opera"],
            "operagx":  [cfg / "opera-gx"],
        }
        candidates = m.get(browser, [])

    for p in candidates:
        if p.exists():
            print(f"[Browser] ✅ Real profile found for {browser}: {p}")
            return str(p)

    fallback = home / ".jarvis_profiles" / browser
    fallback.mkdir(parents=True, exist_ok=True)
    print(f"[Browser] ⚠️  Real profile not found for {browser}, using: {fallback}")
    return str(fallback)

def _firefox_profile_dir() -> Optional[str]:
    home = Path.home()

    if _OS == "Windows":
        base = Path(os.environ.get("APPDATA", "")) / "Mozilla" / "Firefox"
    elif _OS == "Darwin":
        base = home / "Library" / "Application Support" / "Firefox"
    else:
        base = home / ".mozilla" / "firefox"

    ini = base / "profiles.ini"
    if not ini.exists():
        return None

    current: dict[str, str] = {}
    default_path: Optional[str] = None

    for line in ini.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.strip()
        if line.startswith("["):
            p = current.get("Path", "")
            if p and current.get("Default") == "1":
                is_rel = current.get("IsRelative", "1") == "1"
                default_path = str(base / p) if is_rel else p
            current = {}
        elif "=" in line:
            k, _, v = line.partition("=")
            current[k.strip()] = v.strip()

    p = current.get("Path", "")
    if p and current.get("Default") == "1":
        is_rel = current.get("IsRelative", "1") == "1"
        default_path = str(base / p) if is_rel else p

    if default_path and Path(default_path).exists():
        print(f"[Browser] Firefox real profile: {default_path}")
        return default_path
    return None

def _find_opera_windows() -> Optional[str]:
    local  = os.environ.get("LOCALAPPDATA", "")
    prog   = os.environ.get("PROGRAMFILES", "")
    prog86 = os.environ.get("PROGRAMFILES(X86)", "")

    candidates = [
        Path(local)  / "Programs" / "Opera"    / "opera.exe",
        Path(local)  / "Programs" / "Opera GX" / "opera.exe",
        Path(prog)   / "Opera"    / "opera.exe",
        Path(prog86) / "Opera"    / "opera.exe",
    ]
    for p in candidates:
        if p.exists():
            print(f"[Browser] Opera found at: {p}")
            return str(p)

    try:
        import winreg
        keys = [
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\opera.exe",
            r"SOFTWARE\Clients\StartMenuInternet\OperaStable\shell\open\command",
            r"SOFTWARE\Clients\StartMenuInternet\OperaGXStable\shell\open\command",
            r"SOFTWARE\Clients\StartMenuInternet\opera\shell\open\command",
        ]
        for key_path in keys:
            for hive in (winreg.HKEY_LOCAL_MACHINE, winreg.HKEY_CURRENT_USER):
                try:
                    k   = winreg.OpenKey(hive, key_path)
                    val = winreg.QueryValue(k, None)
                    winreg.CloseKey(k)
                    exe = val.strip().strip('"').split('"')[0].split(" --")[0].strip()
                    if exe and Path(exe).exists():
                        print(f"[Browser] Opera found via registry: {exe}")
                        return exe
                except Exception:
                    continue
    except Exception:
        pass

    return shutil.which("opera") or None

def _find_exe_windows(prog_name: str) -> Optional[str]:
    try:
        import winreg
        paths_to_try = [
            rf"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{prog_name}.exe",
            rf"SOFTWARE\Clients\StartMenuInternet\{prog_name}\shell\open\command",
        ]
        for key_path in paths_to_try:
            for hive in (winreg.HKEY_LOCAL_MACHINE, winreg.HKEY_CURRENT_USER):
                try:
                    k   = winreg.OpenKey(hive, key_path)
                    val = winreg.QueryValue(k, None)
                    winreg.CloseKey(k)
                    exe = val.strip().strip('"').split('"')[0].split(" --")[0].strip()
                    if exe and Path(exe).exists():
                        return exe
                except Exception:
                    continue
    except Exception:
        pass
    return None

_BROWSER_SPECS: dict[str, dict] = {
    "Windows": {
        "chrome":   {"engine": "chromium", "channel": "chrome",  "bins": []},
        "edge":     {"engine": "chromium", "channel": "msedge",  "bins": []},
        "firefox":  {"engine": "firefox",  "channel": None,      "bins": ["firefox.exe"]},
        "opera":    {"engine": "chromium", "channel": None,      "bins": ["opera.exe"],  "special": "opera_windows"},
        "operagx":  {"engine": "chromium", "channel": None,      "bins": [],             "special": "opera_windows"},
        "brave":    {"engine": "chromium", "channel": None,      "bins": ["brave.exe"]},
        "vivaldi":  {"engine": "chromium", "channel": None,      "bins": ["vivaldi.exe"]},
        "safari":   None,
    },
    "Darwin": {
        "chrome":   {"engine": "chromium", "channel": "chrome",  "bins": []},
        "edge":     {"engine": "chromium", "channel": "msedge",  "bins": ["microsoft-edge"]},
        "firefox":  {"engine": "firefox",  "channel": None,      "bins": ["firefox"]},
        "opera":    {"engine": "chromium", "channel": None,      "bins": ["opera"]},
        "operagx":  {"engine": "chromium", "channel": None,      "bins": ["opera"]},
        "brave":    {"engine": "chromium", "channel": None,      "bins": ["brave browser", "brave"]},
        "vivaldi":  {"engine": "chromium", "channel": None,      "bins": ["vivaldi"]},
        "safari":   {"engine": "webkit",   "channel": None,      "bins": []},
    },
    "Linux": {
        "chrome":   {"engine": "chromium", "channel": None,
                     "bins": ["google-chrome", "google-chrome-stable", "chromium-browser", "chromium"]},
        "edge":     {"engine": "chromium", "channel": None,
                     "bins": ["microsoft-edge", "microsoft-edge-stable"]},
        "firefox":  {"engine": "firefox",  "channel": None, "bins": ["firefox"]},
        "opera":    {"engine": "chromium", "channel": None, "bins": ["opera", "opera-stable"]},
        "operagx":  {"engine": "chromium", "channel": None, "bins": ["opera", "opera-stable"]},
        "brave":    {"engine": "chromium", "channel": None, "bins": ["brave-browser", "brave"]},
        "vivaldi":  {"engine": "chromium", "channel": None, "bins": ["vivaldi-stable", "vivaldi"]},
        "safari":   None,
    },
}

_ALIASES: dict[str, str] = {
    "google chrome":   "chrome",
    "google-chrome":   "chrome",
    "microsoft edge":  "edge",
    "ms edge":         "edge",
    "msedge":          "edge",
    "mozilla firefox": "firefox",
    "opera gx":        "operagx",
    "opera_gx":        "operagx",
}


def _resolve_browser(name: str) -> dict | None:
    name   = _ALIASES.get(name.lower().strip(), name.lower().strip())
    os_map = _BROWSER_SPECS.get(_OS, {})
    spec   = os_map.get(name)
    if spec is None:
        return None

    engine  = spec["engine"]
    channel = spec.get("channel")
    bins    = spec.get("bins", [])
    exe     = None

    if spec.get("special") == "opera_windows":
        exe = _find_opera_windows()
        if not exe:
            print(f"[Browser] ⚠️  Opera executable not found on Windows.")
        return {"engine": engine, "exe": exe, "channel": channel}

    for b in bins:
        found = shutil.which(b)
        if found:
            exe = found
            break

    if not exe and _OS == "Darwin":
        app_names = {
            "chrome":  ["Google Chrome.app"],
            "edge":    ["Microsoft Edge.app"],
            "firefox": ["Firefox.app"],
            "opera":   ["Opera.app", "Opera GX.app"],
            "brave":   ["Brave Browser.app"],
            "vivaldi": ["Vivaldi.app"],
        }
        for app in app_names.get(name, []):
            app_dir = Path("/Applications") / app / "Contents" / "MacOS"
            if app_dir.exists():
                found_bins = list(app_dir.iterdir())
                if found_bins:
                    exe = str(found_bins[0])
                    break

    if not exe and _OS == "Windows" and not channel:
        exe = _find_exe_windows(name)

    return {"engine": engine, "exe": exe, "channel": channel}


def _detect_default_browser() -> str:
    try:
        if _OS == "Windows":
            import winreg
            k = winreg.OpenKey(
                winreg.HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\Shell\Associations"
                r"\UrlAssociations\http\UserChoice",
            )
            prog_id = winreg.QueryValueEx(k, "ProgId")[0].lower()
            winreg.CloseKey(k)
            for kw in ("edge", "firefox", "opera", "brave", "vivaldi", "chrome"):
                if kw in prog_id:
                    return kw
        elif _OS == "Darwin":
            out = subprocess.run(
                ["defaults", "read",
                 "com.apple.LaunchServices/com.apple.launchservices.secure",
                 "LSHandlers"],
                capture_output=True, text=True, timeout=5,
            ).stdout.lower()
            for kw in ("firefox", "opera", "brave", "vivaldi", "safari", "chrome", "edge"):
                if kw in out:
                    return kw
        elif _OS == "Linux":
            out = subprocess.run(
                ["xdg-settings", "get", "default-web-browser"],
                capture_output=True, text=True, timeout=5,
            ).stdout.lower()
            for kw in ("firefox", "opera", "brave", "vivaldi", "chrome", "edge"):
                if kw in out:
                    return kw
    except Exception:
        pass
    return "chrome"


_SEARCH_ENGINES: dict[str, str] = {
    "google":     "https://www.google.com/search?q=",
    "bing":       "https://www.bing.com/search?q=",
    "duckduckgo": "https://duckduckgo.com/?q=",
    "yandex":     "https://yandex.com/search/?text=",
    "youtube":    "https://www.youtube.com/results?search_query=",
}

_MAC_APP_NAMES: dict[str, str] = {
    "chrome":  "Google Chrome",
    "edge":    "Microsoft Edge",
    "firefox": "Firefox",
    "opera":   "Opera",
    "operagx": "Opera GX",
    "brave":   "Brave Browser",
    "vivaldi": "Vivaldi",
    "safari":  "Safari",
}

# Windows registry lookup names for browsers whose spec has no explicit binary
_WIN_EXE_HINTS: dict[str, str] = {"chrome": "chrome", "edge": "msedge"}


def _open_native(url: str, browser_name: Optional[str]) -> str:
    """
    Opens the user's REAL browser normally — with their own profile,
    logged-in accounts and extensions. No automation attaches, so an
    about:blank tab or a blank profile NEVER shows up.
    If url is empty the browser starts with no URL (its own start page /
    session restore) — exactly as if the user had opened it themselves.
    Works on all three of Windows / macOS / Linux.
    """
    url = _normalize_url(url) if url and url.strip() else ""
    if url == "about:blank":
        url = ""

    name = None
    if browser_name:
        name = _ALIASES.get(browser_name.lower().strip(), browser_name.lower().strip())
    elif not url:
        # No URL → only a window will open; needs the default browser's exe
        name = _detect_default_browser()

    # Specific browser → launch its own executable, exactly like the user would.
    if name:
        if _OS == "Darwin":
            app = _MAC_APP_NAMES.get(name)
            if app:
                cmd = ["open", "-a", app] + ([url] if url else [])
                try:
                    subprocess.run(cmd, check=True, timeout=10)
                    return f"Opened in {name}: {url}" if url else f"Opened {name}."
                except Exception as e:
                    print(f"[Browser] 'open -a {app}' failed ({e}), trying binary…")

        spec = _resolve_browser(name)
        exe  = spec.get("exe") if spec else None
        if not exe and _OS == "Windows":
            if name in ("opera", "operagx"):
                exe = _find_opera_windows()
            else:
                exe = _find_exe_windows(_WIN_EXE_HINTS.get(name, name))
        if exe:
            try:
                subprocess.Popen(
                    [exe, url] if url else [exe],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )
                return f"Opened in {name}: {url}" if url else f"Opened {name}."
            except Exception as e:
                print(f"[Browser] Native launch failed for {name}: {e}")
        print(f"[Browser] '{name}' not found — falling back to default browser.")

    if not url:
        return "Could not find a browser to open."

    # Default browser via the OS — exactly like the user clicking a link.
    try:
        if _OS == "Windows":
            os.startfile(url)                       # ShellExecute → default browser
        elif _OS == "Darwin":
            subprocess.run(["open", url], check=True, timeout=10)
        else:
            subprocess.Popen(
                ["xdg-open", url],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
        return f"Opened in your default browser: {url}"
    except Exception:
        try:
            if webbrowser.open(url):
                return f"Opened in your default browser: {url}"
        except Exception:
            pass
        return f"Could not open a browser for: {url}"


class _BrowserSession:
    """
    A full session for one browser instance.
    All browsers open on the real profile via launch_persistent_context.
    """

    def __init__(self, browser_name: str):
        self.browser_name = browser_name
        self._spec        = _resolve_browser(browser_name)

        self._loop:    asyncio.AbstractEventLoop | None = None
        self._thread:  threading.Thread | None          = None
        self._ready    = threading.Event()

        self._pw:      Playwright     | None = None
        self._context: BrowserContext | None = None
        self._page:    Page           | None = None

    def start(self):
        if self._thread and self._thread.is_alive():
            return
        self._thread = threading.Thread(
            target=self._run_loop,
            daemon=True,
            name=f"BrowserThread-{self.browser_name}",
        )
        self._thread.start()
        self._ready.wait(timeout=20)

    def _run_loop(self):
        self._loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self._loop)
        self._loop.run_until_complete(self._async_init())
        self._ready.set()
        self._loop.run_forever()

    async def _async_init(self):
        self._pw = await async_playwright().start()

    def run(self, coro, timeout: int = 60) -> str:
        if not self._loop:
            raise RuntimeError(f"Session for '{self.browser_name}' not started.")
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result(timeout=timeout)

    def close(self):
        if self._loop:
            asyncio.run_coroutine_threadsafe(self._async_close(), self._loop).result(10)

    async def _async_close(self):
        if self._context:
            try:
                await self._context.close()
            except Exception:
                pass
        if self._pw:
            try:
                await self._pw.stop()
            except Exception:
                pass
        self._context = self._page = None

    async def _adopt_page(self) -> Page:
        """
        launch_persistent_context already opens a starting tab.
        Instead of opening a new blank tab (about:blank), it adopts that tab —
        so the user never sees an extra blank tab.
        """
        await asyncio.sleep(0.3)
        pages = self._context.pages
        return pages[0] if pages else await self._context.new_page()

    async def _launch(self):
        """
        Launches the browser with the real user profile.
        Does nothing if the context is already open.
        """
        if self._context is not None:
            return

        if self._spec is None:
            raise RuntimeError(
                f"'{self.browser_name}' bu platformda ({_OS}) desteklenmiyor."
            )

        engine_name = self._spec["engine"]
        exe         = self._spec["exe"]
        channel     = self._spec["channel"]
        engine_obj  = getattr(self._pw, engine_name)

        if engine_name == "firefox":
            profile = _firefox_profile_dir() or str(
                Path.home() / ".jarvis_profiles" / "firefox"
            )
            kwargs: dict = {
                "headless":    False,
                "slow_mo":     0,
                "viewport":    None,
                "no_viewport": True,
                "timeout":     25_000,
            }
            if exe:
                kwargs["executable_path"] = exe
            try:
                self._context = await engine_obj.launch_persistent_context(profile, **kwargs)
            except Exception as e:
                print(f"[Browser] Firefox real profile failed ({e}), using JARVIS profile")
                jarvis = str(Path.home() / ".jarvis_profiles" / "firefox_jarvis")
                Path(jarvis).mkdir(parents=True, exist_ok=True)
                self._context = await engine_obj.launch_persistent_context(jarvis, **kwargs)

            self._page = await self._adopt_page()
            print(f"[Browser] ✅ Firefox launched")
            return

        if engine_name == "webkit":
            safari_profile = str(Path.home() / ".jarvis_profiles" / "safari")
            Path(safari_profile).mkdir(parents=True, exist_ok=True)
            kwargs = {
                "headless":    False,
                "slow_mo":     0,
                "viewport":    None,
                "no_viewport": True,
                "timeout":     25_000,
            }
            self._context = await engine_obj.launch_persistent_context(safari_profile, **kwargs)
            self._page = await self._adopt_page()
            print(f"[Browser] ✅ Safari launched")
            return

        profile = _real_profile_dir(self.browser_name)

        kwargs = {
            "headless":    False,
            "slow_mo":     0,
            "viewport":    None,
            "no_viewport": True,
            "timeout":     25_000,
            "args": [
                "--start-maximized",
                "--disable-blink-features=AutomationControlled",
                "--no-first-run",
                "--disable-default-apps",
                "--no-default-browser-check",
            ],
        }

        if exe:
            kwargs["executable_path"] = exe
        elif channel:
            kwargs["channel"] = channel

        label = (
            f"{self.browser_name}"
            + (f"/{channel}" if channel else "")
            + (f" @ {exe}" if exe else "")
        )

        try:
            self._context = await engine_obj.launch_persistent_context(profile, **kwargs)
            self._page = await self._adopt_page()
            print(f"[Browser] ✅ Launched [{label}] profile={profile}")
            return
        except Exception as e:
            print(f"[Browser] ⚠️  Real profile failed for {label}: {e}")

        # The real profile could not be opened (browser already open / locked
        # profile / newer Chrome versions block the real profile under
        # automation). Fall back to a persistent JARVIS automation profile —
        # accounts logged in here once stay logged in on later sessions too.
        jarvis_profile = str(Path.home() / ".jarvis_profiles" / self.browser_name)
        Path(jarvis_profile).mkdir(parents=True, exist_ok=True)
        print(f"[Browser] Retrying with JARVIS profile: {jarvis_profile}")

        try:
            self._context = await engine_obj.launch_persistent_context(jarvis_profile, **kwargs)
            self._page = await self._adopt_page()
            print(f"[Browser] ✅ Launched [{label}] with JARVIS profile "
                  f"(sign-ins persist across sessions)")
        except Exception as e2:
            raise RuntimeError(f"Could not launch {self.browser_name}: {e2}") from e2


    async def _get_page(self) -> Page:
        await self._launch()
        # If somehow page got closed, open a fresh one
        if self._page is None or self._page.is_closed():
            self._page = await self._context.new_page()
            await asyncio.sleep(0.2)
        return self._page

    async def go_to(self, url: str) -> str:

        url      = _normalize_url(url)
        page     = await self._get_page()
        prev_url = page.url

        async def _do_goto(p: Page) -> str:
            """Attempt navigation and return the resulting URL (may still be blank)."""
            try:
                await p.goto(url, wait_until="domcontentloaded", timeout=30_000)
                await asyncio.sleep(0.3)
            except PlaywrightTimeout:
                pass   # page may have partially loaded — check URL below
            except Exception as e:
                print(f"[Browser] goto exception (non-fatal): {e}")
            return p.url

        result_url = await _do_goto(page)

        if result_url in ("about:blank", "", None, prev_url) and prev_url in ("about:blank", "", None):
            print(f"[Browser] Still blank after goto — retrying on new tab: {url}")
            try:
                new_page   = await self._context.new_page()
                self._page = new_page
                result_url = await _do_goto(new_page)
            except Exception as e:
                print(f"[Browser] New-tab retry failed: {e}")

        if result_url and result_url not in ("about:blank", "", None):
            return f"Opened: {result_url}"
        return f"Could not open: {url}"

    async def search(self, query: str, engine: str = "google") -> str:
        base = _SEARCH_ENGINES.get(engine.lower(), _SEARCH_ENGINES["google"])
        return await self.go_to(base + quote_plus(query))

    async def click(self, selector: str = None, text: str = None) -> str:
        page = await self._get_page()
        try:
            if text:
                loc = page.get_by_text(text, exact=True)
                if await loc.count() != 1:
                    return "Error: Click target is missing or ambiguous; inspect the page and provide a unique selector."
                await loc.click(timeout=8_000)
                return f"Clicked text: '{text}'"
            if selector:
                loc = page.locator(selector)
                if await loc.count() != 1:
                    return "Error: Selector must identify exactly one element."
                await loc.click(timeout=8_000)
                return f"Clicked selector: {selector}"
            return "No selector or text provided."
        except PlaywrightTimeout:
            return "Element not found (timeout)."
        except Exception as e:
            return f"Click error: {e}"

    async def type_text(self, selector: str = None, text: str = "",
                        clear_first: bool = True) -> str:
        page = await self._get_page()
        try:
            el = page.locator(selector) if selector else page.locator(":focus")
            if await el.count() != 1:
                return "Error: Input target is missing or ambiguous."
            if clear_first:
                await el.clear()
            await el.fill(text) if clear_first else await el.type(text, delay=0)
            return "Text typed."
        except Exception as e:
            return f"Type error: {e}"

    async def scroll(self, direction: str = "down", amount: int = 500) -> str:
        page = await self._get_page()
        try:
            if direction not in ("down", "up"):
                return "Error: Scroll direction must be down or up."
            amount = max(120, min(2400, abs(int(amount))))
            positions = """() => [document.scrollingElement, ...document.querySelectorAll('*')]
                .filter((e, i, a) => e && a.indexOf(e) === i && e.scrollHeight > e.clientHeight
                    && e.clientHeight > 100)
                .map(e => ({top:e.scrollTop, height:e.clientHeight, tag:e.tagName}))"""
            before = await page.evaluate(positions)
            url = page.url
            size = await page.evaluate("() => ({width:innerWidth,height:innerHeight})")
            await page.mouse.move(size["width"] / 2, size["height"] / 2)
            y = amount if direction == "down" else -amount
            await page.mouse.wheel(0, y)
            await asyncio.sleep(.35)
            after = await page.evaluate(positions)
            if page.url == url and len(before) == len(after):
                sign = 1 if direction == "down" else -1
                if any((b["top"] - a["top"]) * sign > 0 and a["tag"] == b["tag"]
                       for a, b in zip(before, after)):
                    from core.tool_result import VerifiedResult
                    return VerifiedResult(f"Verified page scrolled {direction}.", {"scroll_positions_changed": True})
            return "Error: Scroll position did not show verified movement; inspect the page or its boundary."
        except Exception as e:
            return f"Scroll error: {e}"

    async def press(self, key: str) -> str:
        page = await self._get_page()
        try:
            await page.keyboard.press(key)
            return f"Pressed: {key}"
        except Exception as e:
            return f"Key error: {e}"

    async def hotkey(self, *keys: str) -> str:
        """Press key combination, e.g. hotkey('ctrl', 'shift', 't')"""
        page = await self._get_page()
        try:
            await page.keyboard.press("+".join(keys))
            return f"Hotkey: {'+'.join(keys)}"
        except Exception as e:
            return f"Hotkey error: {e}"

    # YouTube / media specific shortcuts
    async def youtube_speed(self, direction: str = "up") -> str:
        """Control YouTube playback speed: 'up' (Shift+.), 'down' (Shift+,), 'normal' (Shift+/) """
        page = await self._get_page()
        try:
            if direction == "up":
                await page.keyboard.press("Shift+.")
            elif direction == "down":
                await page.keyboard.press("Shift+,")
            elif direction == "normal":
                await page.keyboard.press("Shift+/")
            else:
                return f"Unknown direction: {direction}. Use 'up', 'down', or 'normal'."
            return f"YouTube speed {direction}."
        except Exception as e:
            return f"YouTube speed error: {e}"

    async def youtube_control(self, action: str) -> str:
        """YouTube player controls: play_pause, next, prev, fullscreen, theater, mini, captions, quality"""
        page = await self._get_page()
        key_map = {
            "play_pause": "k",
            "pause": "k",
            "play": "k",
            "next": "Shift+n",
            "prev": "Shift+p",
            "fullscreen": "f",
            "theater": "t",
            "mini": "i",
            "captions": "c",
            "mute": "m",
            "volume_up": "ArrowUp",
            "volume_down": "ArrowDown",
            "seek_forward": "l",      # 10s forward
            "seek_back": "j",         # 10s back
            "seek_forward_long": "Shift+l",  # frame by frame? actually > key
            "seek_back_long": "Shift+j",
            "restart": "0",
            "speed_up": "Shift+.",
            "speed_down": "Shift+,",
            "speed_normal": "Shift+/",
        }
        key = key_map.get(action.lower())
        if not key:
            return f"Unknown YouTube action: {action}. Available: {', '.join(key_map.keys())}"
        try:
            await page.keyboard.press(key)
            return f"YouTube: {action}"
        except Exception as e:
            return f"YouTube control error: {e}"

    async def get_text(self) -> str:
        page = await self._get_page()
        try:
            text = await page.inner_text("body")
            return text[:4_000]
        except Exception as e:
            return f"Could not get page text: {e}"

    async def get_url(self) -> str:
        page = await self._get_page()
        return page.url

    async def fill_form(self, fields: dict) -> str:
        if not isinstance(fields, dict) or not fields:
            return "Error: Provide a nonempty mapping of field selectors to values."
        page = await self._get_page()
        targets = []
        for selector, value in fields.items():
            try:
                el = page.locator(selector)
                if await el.count() != 1:
                    return f"Error: Field '{selector}' must match exactly one input; no fields changed."
                targets.append((selector, el, str(value)))
            except Exception as exc:
                return f"Error: Could not locate field '{selector}': {exc}; no fields changed."
        completed = []
        for selector, el, value in targets:
            try:
                await el.fill(value, timeout=5000)
                if await el.input_value() != value:
                    raise RuntimeError("Input read-back did not match")
                completed.append(selector)
            except Exception as exc:
                return (f"Error: Form incomplete; field '{selector}' outcome uncertain: {exc}. "
                        f"Verified fields: {completed}. Inspect before retrying.")
        from core.tool_result import VerifiedResult
        return VerifiedResult("Verified form fields: " + ", ".join(completed),
                              {"fields": completed, "values_match": True})

    async def smart_click(self, description: str) -> str:
        page = await self._get_page()
        candidates = [page.get_by_role(role, name=description, exact=True)
                      for role in ("button", "link", "menuitem", "tab")]
        candidates += [page.get_by_text(description, exact=True), page.get_by_label(description, exact=True)]
        for loc in candidates:
            count = await loc.count()
            if count > 1:
                return "Error: Multiple elements match; inspect the page and identify a unique target."
            if count == 1:
                try:
                    await loc.click(timeout=5000)
                    return f"Clicked '{description}'; inspect resulting page state before reporting completion."
                except Exception as exc:
                    return f"Error: Click outcome uncertain: {exc}. Inspect before retrying."
        return f"Error: Could not find element '{description}'."

    async def smart_type(self, description: str, text: str) -> str:
        page = await self._get_page()
        candidates = [page.get_by_label(description, exact=True),
                      page.get_by_placeholder(description, exact=True),
                      page.get_by_role("textbox", name=description, exact=True),
                      page.get_by_role("combobox", name=description, exact=True)]
        for loc in candidates:
            count = await loc.count()
            if count > 1:
                return "Error: Multiple inputs match; use a unique label or selector."
            if count == 1:
                try:
                    await loc.fill(text, timeout=5000)
                    if await loc.input_value() != text:
                        return "Error: Input read-back did not match requested text."
                    from core.tool_result import VerifiedResult
                    return VerifiedResult(f"Verified text in '{description}'.", {"input": description, "value_matches": True})
                except Exception as exc:
                    return f"Error: Input failed: {exc}"
        return f"Error: Could not find input '{description}'."

    async def new_tab(self, url: str = "") -> str:
        page = await self._get_page()
        ctx  = page.context
        new  = await ctx.new_page()
        self._page = new
        if url:
            return await self.go_to(url)
        return "New tab opened."

    async def close_tab(self) -> str:
        page = self._page
        if page and not page.is_closed():
            ctx   = page.context
            await page.close()
            pages = ctx.pages
            self._page = pages[-1] if pages else None
            return "Tab closed."
        return "No active tab to close."

    async def close_tabs_matching(self, domain: str) -> str:
        page = await self._get_page()
        host = (urlsplit(_normalize_url(domain)).hostname or "").casefold().removeprefix("www.")
        if not host:
            return "Error: A valid domain is required to close matching tabs."
        pages = list(page.context.pages)
        matched = [tab for tab in pages if (urlsplit(tab.url).hostname or "").casefold() in (host, "www." + host)]
        if not matched:
            return f"No open tabs matched {host}."
        if len(matched) == len(pages):
            self._page = await page.context.new_page()
        elif self._page in matched:
            self._page = next(tab for tab in pages if tab not in matched)
        for tab in matched:
            await tab.close()
        remaining = [tab for tab in page.context.pages if not tab.is_closed()]
        if not remaining:
            return "Error: Could not preserve a nonmatching tab; inspect browser state."
        return f"Closed {len(matched)} tab(s) on {host}; {len(remaining)} tab(s) remain open."

    async def screenshot(self, path: str = None) -> str:
        page = await self._get_page()
        try:
            save_path = path or str(Path.home() / "Desktop" / "jarvis_screenshot.png")
            await page.screenshot(path=save_path, full_page=False)
            return f"Screenshot saved: {save_path}"
        except Exception as e:
            return f"Screenshot error: {e}"

    async def back(self) -> str:
        page = await self._get_page()
        try:
            await page.go_back(timeout=10_000)
            return f"Navigated back: {page.url}"
        except Exception as e:
            return f"Back error: {e}"

    async def forward(self) -> str:
        page = await self._get_page()
        try:
            await page.go_forward(timeout=10_000)
            return f"Navigated forward: {page.url}"
        except Exception as e:
            return f"Forward error: {e}"

    async def reload(self) -> str:
        page = await self._get_page()
        try:
            await page.reload(timeout=15_000)
            return f"Page reloaded: {page.url}"
        except Exception as e:
            return f"Reload error: {e}"

    async def close_browser(self) -> str:
        await self._async_close()
        return f"{self.browser_name} closed."

class _SessionRegistry:
    """Manages all active browser sessions."""

    def __init__(self):
        self._sessions:        dict[str, _BrowserSession] = {}
        self._active_browser:  str                        = ""
        self._lock             = threading.Lock()
        self._last_native_url: str                        = ""

    def has(self, browser_name: str | None = None) -> bool:
        """Is there an active automation session for this browser (or any)?"""
        with self._lock:
            if not browser_name:
                return bool(self._sessions)
            name = _ALIASES.get(browser_name.lower().strip(), browser_name.lower().strip())
            return name in self._sessions

    def note_native_url(self, url: str) -> None:
        self._last_native_url = url

    def pop_native_url(self) -> str:
        """Returns the last natively-opened URL once (consumed to avoid repeats)."""
        url, self._last_native_url = self._last_native_url, ""
        return url

    def _get_or_create(self, browser_name: str) -> _BrowserSession:
        with self._lock:
            if browser_name not in self._sessions:
                sess = _BrowserSession(browser_name)
                sess.start()
                self._sessions[browser_name] = sess
                print(f"[Registry] New session: {browser_name}")
            return self._sessions[browser_name]

    def get(self, browser_name: str | None = None) -> _BrowserSession:
        if not browser_name:
            browser_name = self._active_browser or _detect_default_browser()
        browser_name = _ALIASES.get(browser_name.lower().strip(), browser_name.lower().strip())
        sess = self._get_or_create(browser_name)
        self._active_browser = browser_name
        return sess

    def switch(self, browser_name: str) -> str:
        browser_name = _ALIASES.get(browser_name.lower().strip(), browser_name.lower().strip())
        self._get_or_create(browser_name)
        self._active_browser = browser_name
        return f"Active browser → {browser_name}"

    def close_one(self, browser_name: str) -> str:
        with self._lock:
            sess = self._sessions.pop(browser_name, None)
        if sess:
            sess.close()
            if self._active_browser == browser_name:
                self._active_browser = ""
            return f"{browser_name} closed."
        return f"No active session for: {browser_name}"

    def close_all(self) -> str:
        with self._lock:
            names    = list(self._sessions.keys())
            sessions = list(self._sessions.values())
            self._sessions.clear()
            self._active_browser = ""
        for s in sessions:
            try:
                s.close()
            except Exception:
                pass
        return "All browsers closed: " + (", ".join(names) if names else "none")

    def list_sessions(self) -> str:
        with self._lock:
            if not self._sessions:
                return "No active browser sessions."
            lines = []
            for name in self._sessions:
                marker = " ◀ active" if name == self._active_browser else ""
                lines.append(f"  • {name}{marker}")
            return "Open browsers:\n" + "\n".join(lines)


_registry = _SessionRegistry()
_scroll_lock = threading.Lock()
_scroll_stop = threading.Event()
_scroll_thread: threading.Thread | None = None


def _continuous_scroll() -> None:
    from core.browser_scroll import scroll_native
    while not _scroll_stop.wait(0.35):
        if not _native_browser_window():
            break
        if str(scroll_native("down", 360)).startswith("Error:"):
            break
    _scroll_stop.set()

def browser_control(
    parameters:    dict = None,
    response=None,
    player=None,
    session_memory=None,
) -> str:
    params  = parameters or {}
    action  = params.get("action", "").lower().strip()
    browser = params.get("browser", "").lower().strip() or None
    result  = "Unknown action."

    if action == "switch":
        target = browser or params.get("target", "").lower().strip()
        result = _registry.switch(target) if target else "Please specify a browser."
        _log(player, result)
        return result

    if action == "list_browsers":
        result = _registry.list_sessions()
        _log(player, result)
        return result

    if action == "stop_scroll":
        _scroll_stop.set()
        return "Continuous browser scrolling stopped."

    if action == "start_scroll":
        if not _native_browser_window():
            return "Error: Focus a browser tab before starting continuous scroll."
        global _scroll_thread
        with _scroll_lock:
            if _scroll_thread and _scroll_thread.is_alive():
                return "Continuous browser scrolling is already running."
            _scroll_stop.clear()
            _scroll_thread = threading.Thread(target=_continuous_scroll, daemon=True,
                                              name="BrowserContinuousScroll")
            _scroll_thread.start()
        return "Continuous browser scrolling started; say stop to stop it."

    if action == "close_all":
        if params.get("scope") != "all_browser_windows":
            return "Error: Closing every browser window requires scope='all_browser_windows'. Site tabs use close_tabs."
        if params.get("domain") or params.get("url"):
            return "Error: close_all closes browser windows. Use close_tabs with domain for site tabs."
        result = _registry.close_all()
        # Also close native browser window if any was open
        try:
            from actions.computer_control import _close_window
            for b in ("Google Chrome", "Chrome", "Microsoft Edge", "Edge", "Firefox", "Opera", "Brave"):
                _close_window(b)
        except Exception:
            pass
        _log(player, result)
        return result

    if action == "close":
        if params.get("scope") != "browser_window":
            return "Error: Closing a browser window requires scope='browser_window'. Site tabs use close_tabs."
        if params.get("domain") or params.get("url"):
            return "Error: close closes a browser window. Use close_tabs with domain for site tabs."
        target = browser or _registry._active_browser
        result = _registry.close_one(target) if (target and _registry.has(target)) else ""
        try:
            from actions.computer_control import _close_window
            b_name = target or "Chrome"
            res_close = _close_window(b_name)
            result = f"{result}. {res_close}".strip(". ") if result else res_close
        except Exception as e:
            if not result:
                result = f"Could not close browser: {e}"
        _log(player, result)
        return result

    if action == "close_tabs":
        domain = str(params.get("domain") or "").strip()
        if not domain:
            return "Error: Provide the exact domain whose tabs should close."
        if not _registry.has(browser):
            return ("Error: Tabs in this native browser cannot be enumerated safely. "
                    "No tabs or windows were closed.")
        try:
            sess = _registry.get(browser)
            result = sess.run(sess.close_tabs_matching(domain))
        except Exception as exc:
            result = f"Error: Could not close matching tabs safely: {exc}"
        _log(player, result)
        return result

    if action == "close_tab" and params.get("domain"):
        # A named site must never be approximated by closing whichever tab is
        # foreground. Native browser enumeration is unavailable here.
        return browser_control({"action": "close_tabs", "domain": params["domain"],
                                "browser": browser or ""}, player=player)

    if action == "new_tab":
        active = _native_browser_window()
        if active:
            try:
                import pyautogui
                url = str(params.get("url") or "").strip()
                if url and _incomplete_linkedin_url(url):
                    return "Error: Incomplete LinkedIn URL; no new tab was opened."
                pyautogui.hotkey("ctrl", "t")
                if url:
                    navigation = _navigate_native_url(url)
                    if navigation.startswith("Error:"):
                        return f"Error: New tab command sent, but {navigation[7:]}"
                return f"Sent new-tab shortcut to active browser '{active.title}'."
            except Exception as exc:
                return f"Error: New-tab outcome uncertain: {exc}. Inspect browser before retrying."
        if _registry.has(browser):
            try:
                sess = _registry.get(browser)
                return sess.run(sess.new_tab(params.get("url", "")))
            except Exception as exc:
                return f"Error: Could not open tab in connected browser: {exc}"
        return "Error: No active browser tab. Focus an existing browser window before opening a new tab."

    # Navigation reuses the foreground native tab. Without one, an explicitly
    # requested URL may be opened by the OS or in an existing connected session.
    if action in ("go_to", "search"):
        if action == "go_to" and _incomplete_linkedin_url(str(params.get("url") or "")):
            return "Error: Incomplete LinkedIn URL. Use smart_click on the visible navigation item or a full URL."
        if _registry.has(browser) and not _native_browser_window():
            sess = _registry.get(browser)
            try:
                if action == "search":
                    result = sess.run(sess.search(params.get("query", ""),
                                                  params.get("engine", "google")))
                else:
                    result = sess.run(sess.go_to(params.get("url", "")))
            except concurrent.futures.TimeoutError:
                result = f"Browser action '{action}' timed out (60s)."
            except Exception as e:
                result = f"Browser error ({action}): {e}"
            _log(player, result)
            return result

        if action == "search":
            base    = _SEARCH_ENGINES.get(params.get("engine", "google").lower(),
                                          _SEARCH_ENGINES["google"])
            nav_url = base + quote_plus(params.get("query", ""))
        else:
            nav_url = params.get("url", "").strip()

        if nav_url and not params.get("new_tab") and _native_browser_window():
            try:
                result = _navigate_native_url(nav_url)
                if not result.startswith("Error:"):
                    _registry.note_native_url(result.partition(": ")[2])
                _log(player, result)
                return result
            except Exception as exc:
                return f"Error: In-tab navigation outcome uncertain: {exc}. No extra browser opened."

        # No browser tab is foreground: ask the OS to open the requested URL.
        result = _open_native(nav_url, browser)
        if result.startswith("Opened") and nav_url:
            try:
                result = _verify_native_url(nav_url)
                if not result.startswith("Error:"):
                    _registry.note_native_url(_normalize_url(nav_url))
            except Exception as exc:
                result = f"Error: Browser launch requested but navigation could not be verified: {exc}."
        _log(player, result)
        return result

    if action == "scroll":
        # A visible native tab takes precedence over an older automation session.
        if _native_browser_window():
            from core.browser_scroll import scroll_native
            result = scroll_native(params.get("direction", "down"), params.get("amount", 500))
        elif _registry.has(browser):
            sess = _registry.get(browser)
            try:
                result = sess.run(sess.scroll(params.get("direction", "down"), params.get("amount", 500)))
            except Exception as exc:
                result = f"Error: Browser scroll failed: {exc}."
        else:
            result = "Error: Focus the intended browser before scrolling."
        _log(player, result)
        return result

    # ── Simple desktop/window navigation fallback ────────────────────────────
    # If the user opened a website in their own native browser (or is looking at
    # an existing browser window) and asks to scroll, press a key, or refresh,
    # execute it directly on the active window via OS input instead of spinning
    # up an isolated automation browser that lacks their login sessions.
    if not _registry.has(browser):
        from core.universal_operator import get_active_window
        active = get_active_window()
        if action not in ("search", "youtube_search", "switch_browser", "list_browsers") and active.app_category != "browser":
            return "Error: No browser is in the foreground. Focus the intended browser before interacting."
        if action == "get_text":
            try:
                from core import uia
                info, controls = uia.list_controls(active.title, limit=80)
                return f"Visible browser UI in {info.title}:\n" + "\n".join(c.name for c in controls)
            except Exception as exc:
                return f"Error: Browser UI could not be inspected: {exc}"
        if action == "get_url":
            return "Error: Current URL is unavailable without a connected DOM session; inspect the visible address bar."
        if params.get("selector") and action in ("click", "type", "smart_click", "smart_type"):
            return "Error: CSS selectors need a connected DOM session. For the current native browser use a visible element description."
        # 1. Profile / direct navigation check
        desc = (params.get("description") or params.get("text") or params.get("selector") or "").lower().strip()
        last_url = (_registry._last_native_url or "").lower()
        cur_title = ""
        try:
            from actions.computer_control import _active_window
            cur_title = _active_window().lower()
        except Exception:
            pass

        # 2. Native click / smart_click on active screen
        if action in ("click", "smart_click"):
            try:
                from actions.computer_control import computer_control as _cc
                if not desc:
                    return "Error: A visible target description is required."
                res = _cc({"action": "screen_click", "description": desc, "title": active.title}, player=player)
                _log(player, res)
                return res
            except Exception as e:
                result = f"Click failed on active screen: {e}"
                _log(player, result)
                return result

        # 3. Native type on active window
        if action in ("type", "smart_type") and not params.get("selector"):
            try:
                from actions.computer_control import computer_control as _cc
                res = _cc({"action": "type", "text": params.get("text", ""),
                           "description": params.get("description", ""), "title": active.title}, player=player)
                _log(player, res)
                return res
            except Exception as e:
                result = f"Typing failed on active screen: {e}"
                _log(player, result)
                return result

        # 4. Scroll, keys, and browser shortcuts (including YouTube controls)
        # These use native OS input on the active browser window — NO Playwright automation session.
        if action in (
            "scroll", "press", "reload", "back", "forward", "new_tab", "close_tab",
            "next_tab", "prev_tab", "reopen_tab", "address_bar", "search_bar",
            "zoom_in", "zoom_out", "zoom_reset", "fullscreen", "search", "youtube_search", "switch_browser", "find",
            "hotkey", "youtube_speed", "youtube_control"
        ):
            # Focus open browser window before sending native keys / scrolls only if not already active
            if action not in ("search", "youtube_search"):
                try:
                    from actions.computer_control import _focus_window, _active_window
                    cur_win = _active_window().lower()
                    browser_kw = ("chrome", "edge", "firefox", "brave", "opera", "youtube", "linkedin", "browser")
                    if not any(k in cur_win for k in browser_kw):
                        for b in ("Google Chrome", "Chrome", "YouTube", "Microsoft Edge", "Edge", "Brave", "Firefox", "Opera"):
                            res = _focus_window(b)
                            if "Focused window:" in res:
                                time.sleep(0.1)
                                break
                except Exception:
                    pass

            try:
                import pyautogui
                old_failsafe = getattr(pyautogui, "FAILSAFE", True)
                try:
                    pyautogui.FAILSAFE = False
                    if action == "press":
                        key = params.get("key", "Enter")
                        pyautogui.press(key.lower())
                        result = f"Pressed {key}."
                    elif action == "reload":
                        pyautogui.hotkey("ctrl", "r")
                        result = "Reloaded page."
                    elif action == "back":
                        pyautogui.hotkey("alt", "left")
                        result = "Navigated back."
                    elif action == "forward":
                        pyautogui.hotkey("alt", "right")
                        result = "Navigated forward."
                    elif action in ("new_tab", "open_tab"):
                        pyautogui.hotkey("ctrl", "t")
                        url = params.get("url")
                        if url:
                            time.sleep(0.3)
                            from actions.computer_control import computer_control as _cc
                            _cc({"action": "smart_type", "text": url})
                            pyautogui.press("enter")
                        result = f"Opened new tab{' with ' + url if url else ''}."
                    elif action == "close_tab":
                        pyautogui.hotkey("ctrl", "w")
                        result = "Closed active tab."
                    elif action == "next_tab":
                        pyautogui.hotkey("ctrl", "tab")
                        result = "Switched to next tab."
                    elif action in ("prev_tab", "previous_tab"):
                        pyautogui.hotkey("ctrl", "shift", "tab")
                        result = "Switched to previous tab."
                    elif action == "reopen_tab":
                        pyautogui.hotkey("ctrl", "shift", "t")
                        result = "Reopened last closed tab."
                    elif action in ("address_bar", "focus_url"):
                        pyautogui.hotkey("ctrl", "l")
                        result = "Focused address bar."
                    elif action == "search_bar":
                        # If on YouTube or web app, '/' focuses in-page search; ctrl+e focuses browser search
                        pyautogui.press("/")
                        result = "Focused search bar."
                    elif action == "find":
                        pyautogui.hotkey("ctrl", "f")
                        query = params.get("text") or params.get("query")
                        if query:
                            time.sleep(0.2)
                            from actions.computer_control import computer_control as _cc
                            _cc({"action": "smart_type", "text": query})
                            pyautogui.press("enter")
                        result = f"Searching page for '{query}'." if query else "Find bar opened."
                    elif action == "zoom_in":
                        pyautogui.hotkey("ctrl", "+")
                        result = "Zoomed in."
                    elif action == "zoom_out":
                        pyautogui.hotkey("ctrl", "-")
                        result = "Zoomed out."
                    elif action == "zoom_reset":
                        pyautogui.hotkey("ctrl", "0")
                        result = "Zoom reset."
                    elif action == "fullscreen":
                        pyautogui.press("f11")
                        result = "Toggled fullscreen."
                    elif action == "hotkey":
                        keys = params.get("keys", "").split("+")
                        keys = [k.strip().lower() for k in keys if k.strip()]
                        if keys:
                            pyautogui.hotkey(*keys)
                            result = f"Hotkey: {'+'.join(keys)}"
                        else:
                            result = "No keys specified for hotkey."
                    elif action == "youtube_speed":
                        direction = params.get("direction", "up").lower().strip()
                        if direction == "up":
                            pyautogui.hotkey("shift", ".")
                        elif direction == "down":
                            pyautogui.hotkey("shift", ",")
                        elif direction == "normal":
                            pyautogui.hotkey("shift", "/")
                        else:
                            result = f"Unknown direction: {direction}. Use 'up', 'down', or 'normal'."
                            _log(player, result)
                            return result
                        result = f"YouTube speed {direction}."
                    elif action == "youtube_control":
                        yt_action = params.get("yt_action", "").lower().strip()
                        key_map = {
                            "play_pause": "k",
                            "pause": "k",
                            "play": "k",
                            "next": "shift+n",
                            "prev": "shift+p",
                            "fullscreen": "f",
                            "theater": "t",
                            "mini": "i",
                            "captions": "c",
                            "mute": "m",
                            "volume_up": "up",
                            "volume_down": "down",
                            "seek_forward": "l",
                            "seek_back": "j",
                            "restart": "0",
                            "speed_up": "shift+.",
                            "speed_down": "shift+,",
                            "speed_normal": "shift+/",
                        }
                        key = key_map.get(yt_action)
                        if not key:
                            result = f"Unknown YouTube action: {yt_action}. Available: {', '.join(key_map.keys())}"
                            _log(player, result)
                            return result
                        if "+" in key:
                            pyautogui.hotkey(*key.split("+"))
                        else:
                            pyautogui.press(key)
                        result = f"YouTube: {yt_action}"
                    elif action in ("search", "youtube_search"):
                        q = params.get("query") or params.get("text") or ""
                        engine = params.get("engine", "google").lower().strip()
                        import urllib.parse
                        import webbrowser
                        low_q = q.lower()
                        if action == "youtube_search" or engine == "youtube" or "youtube" in low_q:
                            import re
                            clean_q = re.sub(r'(?i)\b(on\s+youtube|youtube\s+e|in\s+youtube|youtube)\b', '', q).strip()
                            clean_q = clean_q or q
                            url = f"https://www.youtube.com/results?search_query={urllib.parse.quote(clean_q)}"
                            result = f"Searched YouTube for: {clean_q}"
                        elif engine == "bing":
                            url = f"https://www.bing.com/search?q={urllib.parse.quote(q)}"
                            result = f"Searched Bing for: {q}"
                        elif engine == "duckduckgo":
                            url = f"https://www.duckduckgo.com/?q={urllib.parse.quote(q)}"
                            result = f"Searched DuckDuckGo for: {q}"
                        else:
                            url = f"https://www.google.com/search?q={urllib.parse.quote(q)}"
                            result = f"Searched for: {q}"
                        webbrowser.open(url)
                        time.sleep(0.4)
                        try:
                            from actions.computer_control import _focus_window
                            for b in ("Google Chrome", "Chrome", "YouTube", "Microsoft Edge", "Edge", "Brave", "Firefox"):
                                if "Focused window:" in _focus_window(b):
                                    break
                        except Exception:
                            pass
                    elif action == "switch_browser":
                        from actions.computer_control import _focus_window
                        for b in ("Google Chrome", "Chrome", "YouTube", "Microsoft Edge", "Edge", "Brave", "Firefox"):
                            res = _focus_window(b)
                            if "Focused window:" in res:
                                result = f"Brought {b} to foreground."
                                break
                        else:
                            result = "No open browser found to switch to."
                finally:
                    pyautogui.FAILSAFE = old_failsafe
                _log(player, result)
                return result
            except Exception as e:
                result = f"Native window action '{action}' failed: {e}"
                _log(player, result)
                return result

    # ── Interactive actions (click/type/read…) ───────────────────────────────
    # These require a physically controllable browser; the automation window
    # only opens here, and as soon as it opens it goes to the user's last
    # navigated page — it doesn't sit on a blank page.
    try:
        sess = _registry.get(browser)
    except Exception as e:
        result = f"Could not start browser session: {e}"
        _log(player, result)
        return result

    try:
        last = _registry.pop_native_url()
        if last:
            try:
                sess.run(sess.go_to(last))
            except Exception as e:
                print(f"[Browser] Could not resume last page ({last}): {e}")

        if action == "click":
            result = sess.run(sess.click(params.get("selector"), params.get("text")))
        elif action == "type":
            result = sess.run(sess.type_text(
                params.get("selector"), params.get("text", ""), params.get("clear_first", True)))
        elif action == "fill_form":
            result = sess.run(sess.fill_form(params.get("fields", {})))
        elif action == "smart_click":
            result = sess.run(sess.smart_click(params.get("description", "")))
        elif action == "smart_type":
            result = sess.run(sess.smart_type(params.get("description", ""), params.get("text", "")))
        elif action == "get_text":
            result = sess.run(sess.get_text())
        elif action == "get_url":
            result = sess.run(sess.get_url())
        elif action == "screenshot":
            result = sess.run(sess.screenshot(params.get("path")))
        elif action == "close_tab":
            result = sess.run(sess.close_tab())
        elif action == "back":
            result = sess.run(sess.back())
        elif action == "forward":
            result = sess.run(sess.forward())
        elif action == "reload":
            result = sess.run(sess.reload())
        else:
            result = f"Unknown browser action: '{action}'"

    except concurrent.futures.TimeoutError:
        result = f"Browser action '{action}' timed out (60s)."
    except Exception as e:
        result = f"Browser error ({action}): {e}"

    _log(player, result)
    return result


def _log(player, text: str):
    short = str(text)[:80]
    print(f"[Browser] {short}")
    if player:
        player.write_log(f"[browser] {short[:60]}")


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "browser_control",
    "description": "Controls the active browser. For a visible page navigation item use smart_click with its label, not an invented partial URL. go_to needs a complete URL and uses the current foreground tab. new_tab sends Ctrl+T to the foreground browser and never launches Chrome. start_scroll continues until stop_scroll. close_tabs needs a domain and closes only matching tabs when they can be enumerated safely; never use close or close_all for site tabs.",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "go_to | search | click | type | scroll | start_scroll | stop_scroll | close_tabs | fill_form | smart_click | smart_type | get_text | get_url | press | hotkey | youtube_speed | youtube_control | new_tab | close_tab | screenshot | back | forward | reload | switch | list_browsers | close | close_all"
            },
            "browser": {
                "type": "STRING",
                "description": "Target browser: chrome | edge | firefox | opera | operagx | brave | vivaldi | safari. Omit to use the currently active browser."
            },
            "url": {
                "type": "STRING",
                "description": "URL for go_to / new_tab action"
            },
            "domain": {"type": "STRING", "description": "Exact website domain for close_tabs, e.g. linkedin.com"},
            "scope": {"type": "STRING", "description": "Required only for closing entire windows: browser_window for close, all_browser_windows for close_all. Never use for site tabs."},
            "query": {
                "type": "STRING",
                "description": "Search query for search action"
            },
            "engine": {
                "type": "STRING",
                "description": "Search engine: google | bing | duckduckgo | yandex (default: google)"
            },
            "selector": {
                "type": "STRING",
                "description": "CSS selector for click/type"
            },
            "text": {
                "type": "STRING",
                "description": "Text to click or type"
            },
            "description": {
                "type": "STRING",
                "description": "Element description for smart_click/smart_type"
            },
            "direction": {
                "type": "STRING",
                "description": "up | down for scroll or youtube_speed direction"
            },
            "amount": {
                "type": "INTEGER",
                "description": "Scroll amount in pixels (default: 500)"
            },
            "key": {
                "type": "STRING",
                "description": "Key name for press action (e.g. Enter, Escape, F5)"
            },
            "keys": {
                "type": "STRING",
                "description": "Key combination for hotkey action, e.g. 'ctrl+shift+t' or 'ctrl+alt+delete'"
            },
            "path": {
                "type": "STRING",
                "description": "Save path for screenshot"
            },
            "incognito": {
                "type": "BOOLEAN",
                "description": "Open in private/incognito mode"
            },
            "clear_first": {
                "type": "BOOLEAN",
                "description": "Clear field before typing (default: true)"
            },
            "yt_action": {
                "type": "STRING",
                "description": "YouTube control action: play_pause | next | prev | fullscreen | theater | mini | captions | mute | volume_up | volume_down | seek_forward | seek_back | restart | speed_up | speed_down | speed_normal"
            }
        },
        "required": [
            "action"
        ]
    },
    "handler": browser_control,
}
