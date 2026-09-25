"""
Android phone control plugin for JARVIS using wireless ADB.

Controls the user's PHONE over ADB (not the PC).
Allows the user to open/close apps, type text, tap UI elements, send WhatsApp messages,
adjust volume, mute, take screenshots, lock screen, query battery status,
and inspect connected phone devices.

All screen tapping dynamically inspects the active UI hierarchy via uiautomator dump;
no coordinates are ever hardcoded.
"""

from __future__ import annotations

import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import urllib.parse
import xml.etree.ElementTree as ET
from datetime import datetime
from pathlib import Path

if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

from memory.config_manager import get_android_ip, save_android_ip

# Windows process creation flag to prevent cmd console window popup
if platform.system() == "Windows":
    _WIN_HIDE = {"creationflags": subprocess.CREATE_NO_WINDOW}
else:
    _WIN_HIDE = {}

def _get_base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent

BASE_DIR = _get_base_dir()
SCREENSHOTS_DIR = BASE_DIR / "screenshots"
CONTACTS_PATH = BASE_DIR / "config" / "contacts.json"


def _find_adb() -> str | None:
    found = shutil.which("adb")
    if found:
        return found
    candidates = [
        Path.home() / "AppData" / "Local" / "Microsoft" / "WinGet" / "Links" / "adb.exe",
        Path.home() / "AppData" / "Local" / "Android" / "Sdk" / "platform-tools" / "adb.exe",
        Path(r"C:\platform-tools\adb.exe"),
        Path(r"C:\Program Files\Android\platform-tools\adb.exe"),
    ]
    winget_pkg = Path.home() / "AppData" / "Local" / "Microsoft" / "WinGet" / "Packages"
    if winget_pkg.exists():
        try:
            for p in winget_pkg.glob("**/platform-tools/adb.exe"):
                if p.exists():
                    return str(p)
        except Exception:
            pass
    for c in candidates:
        if c.exists():
            return str(c)
    return None


_ADB_PATH = _find_adb()
if not _ADB_PATH:
    try:
        print("[android_control] [!] 'adb' not found on PATH. Install Android Platform Tools and add adb to PATH.")
    except Exception:
        pass

# Common app names to Android package names
_APP_PACKAGES: dict[str, str] = {
    "whatsapp": "com.whatsapp",
    "youtube": "com.google.android.youtube",
    "chrome": "com.android.chrome",
    "browser": "com.android.chrome",
    "camera": "com.google.android.GoogleCamera",
    "gallery": "com.google.android.apps.photos",
    "photos": "com.google.android.apps.photos",
    "settings": "com.android.settings",
    "spotify": "com.spotify.music",
    "facebook": "com.facebook.katana",
    "instagram": "com.instagram.android",
    "telegram": "org.telegram.messenger",
    "messenger": "com.facebook.orca",
    "maps": "com.google.android.apps.maps",
    "google maps": "com.google.android.apps.maps",
    "gmail": "com.google.android.gm",
    "calculator": "com.google.android.calculator",
    "clock": "com.google.android.deskclock",
    "contacts": "com.google.android.contacts",
    "phone": "com.google.android.dialer",
    "dialer": "com.google.android.dialer",
    "messages": "com.google.android.apps.messaging",
    "play store": "com.android.vending",
    "playstore": "com.android.vending",
}


def _run_adb(args: list[str], timeout: float = 7.0, capture_binary: bool = False) -> tuple[int, str | bytes, str]:
    """Execute an adb command with a strict timeout. Never raises unhandled exceptions."""
    adb_cmd = _ADB_PATH or "adb"
    cmd = [adb_cmd] + args
    try:
        if capture_binary:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                timeout=timeout,
                **_WIN_HIDE,
            )
            return proc.returncode, proc.stdout, proc.stderr.decode("utf-8", errors="replace")
        else:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=timeout,
                **_WIN_HIDE,
            )
            return proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "ADB command timed out."
    except FileNotFoundError:
        return -1, "", "ADB executable not found."
    except Exception as e:
        return -1, "", str(e)


def _get_target_endpoint() -> tuple[str | None, str | None]:
    """Retrieve the configured IP:port endpoint, or error message."""
    ip = get_android_ip()
    if not ip:
        return None, "Phone IP address is not configured. Please set the phone's IP in settings, sir."
    target = ip if ":" in ip else f"{ip}:5555"
    return target, None


def _ensure_device_connected(target: str) -> tuple[bool, str]:
    """Ensure the target device is connected and authorized over wireless ADB."""
    if not _ADB_PATH:
        return False, "ADB is not installed or not found on PATH, sir. Please install Android Platform Tools."

    # Check connected devices
    rc, stdout, stderr = _run_adb(["devices"])
    if rc != 0:
        return False, f"Could not query ADB devices: {stderr or 'Unknown error'}."

    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    device_states: dict[str, str] = {}
    for line in lines[1:]:  # skip 'List of devices attached'
        parts = line.split()
        if len(parts) >= 2:
            device_states[parts[0]] = parts[1]

    # If target is already connected and authorized
    if target in device_states:
        state = device_states[target]
        if state == "device":
            return True, ""
        elif state == "unauthorized":
            return False, "Phone is connected but unauthorized, sir. Please accept the debugging prompt on your phone screen."
        elif state == "offline":
            _run_adb(["disconnect", target], timeout=2.0)

    # Attempt connection
    rc, stdout, stderr = _run_adb(["connect", target], timeout=5.0)
    out_lower = (stdout + " " + stderr).lower()

    if "connected to" in out_lower:
        time.sleep(0.3)
        rc2, stdout2, _ = _run_adb(["devices"])
        for line in stdout2.splitlines()[1:]:
            parts = line.strip().split()
            if len(parts) >= 2 and parts[0] == target:
                if parts[1] == "unauthorized":
                    return False, "Phone is connected but unauthorized, sir. Please accept the debugging prompt on your phone screen."
                return True, ""
        return True, ""

    if "cannot connect" in out_lower or "failed to connect" in out_lower:
        return False, f"Phone connected na, ADB wireless setup age koro ({target}). Check Wi-Fi and Wireless Debugging."

    return False, f"Could not connect to phone at {target}: {stdout.strip() or stderr.strip()}"


# ── UI Automator Dump & Coordinate Resolver ──────────────────────────────────

def _dump_ui_hierarchy(target: str) -> str | None:
    """Run uiautomator dump on the device and retrieve the active XML hierarchy."""
    dump_remote = "/data/local/tmp/uidump.xml"
    # Run dump
    rc, stdout, stderr = _run_adb(["-s", target, "shell", "uiautomator", "dump", dump_remote], timeout=6.0)
    out_all = stdout + " " + stderr
    if rc != 0 and "dumped to" not in out_all.lower():
        # Fallback to default path
        _run_adb(["-s", target, "shell", "uiautomator", "dump"], timeout=6.0)
        dump_remote = "/sdcard/window_dump.xml"

    # Read dumped XML
    rc_cat, xml_text, _ = _run_adb(["-s", target, "shell", "cat", dump_remote], timeout=5.0)
    if rc_cat == 0 and "<hierarchy" in str(xml_text):
        return str(xml_text)
    return None


def _find_element_bounds(xml_content: str, description: str) -> tuple[int, int] | None:
    """
    Search XML hierarchy for a node whose text, content-desc, or resource-id
    contains description (case-insensitive partial match).
    Computes and returns the center coordinates (cx, cy) from node bounds.
    """
    if not xml_content:
        return None

    try:
        root = ET.fromstring(xml_content)
    except Exception as e:
        print(f"[Android] XML parse error: {e}")
        return None

    desc_lower = description.lower().strip()
    matches: list[tuple[int, int, int]] = []  # (priority, cx, cy)

    for node in root.iter("node"):
        text = (node.attrib.get("text") or "").lower()
        content_desc = (node.attrib.get("content-desc") or "").lower()
        res_id = (node.attrib.get("resource-id") or "").lower()
        bounds = node.attrib.get("bounds") or ""

        if desc_lower in text or desc_lower in content_desc or desc_lower in res_id:
            m = re.search(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = map(int, m.groups())
                if x2 > x1 and y2 > y1:
                    cx = (x1 + x2) // 2
                    cy = (y1 + y2) // 2
                    # Prioritize exact matches and clickable nodes
                    clickable = node.attrib.get("clickable", "false").lower() == "true"
                    exact = (desc_lower == text or desc_lower == content_desc)
                    priority = (2 if exact else 0) + (1 if clickable else 0)
                    matches.append((priority, cx, cy))

    if not matches:
        return None

    matches.sort(key=lambda item: item[0], reverse=True)
    return matches[0][1], matches[0][2]


# ── Contacts Resolver for WhatsApp ───────────────────────────────────────────

def _resolve_contact_number(contact: str) -> tuple[str | None, str | None]:
    """
    Resolve contact name or phone number.
    Checks config/contacts.json first. If already a number, formats digits.
    Returns (phone_number_digits, error_message).
    """
    clean = contact.strip()
    if not clean:
        return None, "No contact name or phone number provided, sir."

    contacts_data = {}
    if CONTACTS_PATH.exists():
        try:
            contacts_data = json.loads(CONTACTS_PATH.read_text(encoding="utf-8"))
        except Exception as e:
            print(f"[Android] Error reading contacts.json: {e}")

    # Case-insensitive lookup in contacts.json
    clean_lower = clean.lower()
    for name, num in contacts_data.items():
        if name.lower() == clean_lower:
            num_clean = re.sub(r"[^\d]", "", str(num))
            if num_clean.startswith("01") and len(num_clean) == 11:
                num_clean = "88" + num_clean
            return num_clean, None

    # Check if user directly provided a phone number
    digits = re.sub(r"[^\d]", "", clean)
    if len(digits) >= 8:
        if digits.startswith("01") and len(digits) == 11:
            digits = "88" + digits
        return digits, None

    return None, (
        f"Contact '{clean}' was not found in config/contacts.json, sir. "
        "Please add their number to contacts.json or provide the phone number directly."
    )


# ── Action Handlers ───────────────────────────────────────────────────────────

def _handle_open_app(target: str, app_name: str) -> str:
    name_clean = app_name.lower().strip()
    if not name_clean:
        return "Please specify which app to open on your phone, sir."

    package = _APP_PACKAGES.get(name_clean, name_clean)

    # Special handling for Camera (universal intent)
    if name_clean in ("camera", "video"):
        rc, _, _ = _run_adb(["-s", target, "shell", "am", "start", "-a", "android.media.action.STILL_IMAGE_CAMERA"])
        if rc == 0:
            time.sleep(2.0)
            return "Opened camera on your phone, sir."

    # Special handling for Settings (universal intent)
    if name_clean == "settings":
        rc, _, _ = _run_adb(["-s", target, "shell", "am", "start", "-a", "android.settings.SETTINGS"])
        if rc == 0:
            time.sleep(2.0)
            return "Opened settings on your phone, sir."

    # Launch package via monkey
    rc, stdout, _ = _run_adb([
        "-s", target, "shell", "monkey",
        "-p", package,
        "-c", "android.intent.category.LAUNCHER", "1"
    ])

    if rc == 0 and "No activities found" not in stdout:
        time.sleep(2.0)  # Delay for MIUI/HyperOS app launch animations
        return f"Opened {app_name} on your phone, sir."

    # Fallback: check if package is installed
    rc_pm, pm_out, _ = _run_adb(["-s", target, "shell", "pm", "path", package])
    if rc_pm == 0 and "package:" in pm_out:
        return f"Package '{package}' is installed, but launcher activity could not be started directly, sir."

    return f"Could not find or launch '{app_name}' on your phone, sir."


def _handle_close_app(target: str, app_name: str) -> str:
    name_clean = app_name.lower().strip()
    if not name_clean:
        return "Please specify which app to close on your phone, sir."

    package = _APP_PACKAGES.get(name_clean, name_clean)
    rc, stdout, stderr = _run_adb(["-s", target, "shell", "am", "force-stop", package])
    if rc != 0:
        return f"Failed to close '{app_name}' on phone: {stderr}"

    time.sleep(1.5)  # Delay for screen animation
    return f"Closed {app_name} on your phone, sir."


def _handle_type_text(target: str, text: str) -> str:
    if not text:
        return "No text provided to type, sir."

    # adb shell input text treats spaces as %s and requires escaping shell special chars
    escaped = ""
    for ch in text:
        if ch == " ":
            escaped += "%s"
        elif ch in ('\\', '$', '"', "'", '`', '(', ')', '<', '>', '&', '|', ';', '*', '?', '~'):
            escaped += f"\\{ch}"
        else:
            escaped += ch

    rc, stdout, stderr = _run_adb(["-s", target, "shell", "input", "text", f'"{escaped}"'])
    combined = (str(stdout) + " " + str(stderr)).lower()
    if "securityexception" in combined or "inject_events" in combined:
        return "Permission needed: Please enable 'USB debugging (Security settings)' in Developer Options on your phone, sir."
    if rc != 0:
        return f"Failed to type on phone: {stderr}"

    time.sleep(1.5)  # Delay for typing / keyboard animation
    return f"Typed on phone: '{text}', sir."


def _handle_tap_element(target: str, description: str) -> str:
    desc_clean = description.strip()
    if not desc_clean:
        return "Please specify what element or button to tap on your phone, sir."

    xml_text = _dump_ui_hierarchy(target)
    if not xml_text:
        return "Could not inspect phone screen elements via uiautomator, sir."

    coords = _find_element_bounds(xml_text, desc_clean)
    if not coords:
        return f"Could not find element matching '{desc_clean}' on your phone screen, sir."

    cx, cy = coords
    rc, stdout, stderr = _run_adb(["-s", target, "shell", "input", "tap", str(cx), str(cy)])
    combined = (str(stdout) + " " + str(stderr)).lower()
    if "securityexception" in combined or "inject_events" in combined:
        return "Permission needed: Please enable 'USB debugging (Security settings)' in Developer Options on your phone, sir."
    if rc != 0:
        return f"Failed to tap element at ({cx}, {cy}): {stderr}"

    time.sleep(1.8)  # Delay for UI transition
    return f"Tapped '{desc_clean}' at ({cx}, {cy}) on your phone, sir."


def _handle_send_whatsapp_message(target: str, contact: str, message: str) -> str:
    if not message:
        return "Please specify what message to send, sir."

    phone_number, err = _resolve_contact_number(contact)
    if err:
        return err

    encoded_msg = urllib.parse.quote(message)
    wa_url = f"https://wa.me/{phone_number}?text={encoded_msg}"

    # Launch WhatsApp chat with pre-filled message
    rc, stdout, stderr = _run_adb([
        "-s", target, "shell", "am", "start",
        "-a", "android.intent.action.VIEW",
        "-d", wa_url
    ])
    if rc != 0:
        return f"Failed to open WhatsApp on phone: {stderr}"

    # Wait for WhatsApp to open and load the chat with pre-filled text
    time.sleep(2.5)

    # Use uiautomator dump to locate the send button
    xml_text = _dump_ui_hierarchy(target)
    send_coords = None
    if xml_text:
        for candidate in ("com.whatsapp:id/send", "send", "পাঠান", "submit"):
            send_coords = _find_element_bounds(xml_text, candidate)
            if send_coords:
                break

    # Retry once after 1 second if not found on first pass
    if not send_coords:
        time.sleep(1.0)
        xml_text = _dump_ui_hierarchy(target)
        if xml_text:
            for candidate in ("com.whatsapp:id/send", "send", "পাঠান"):
                send_coords = _find_element_bounds(xml_text, candidate)
                if send_coords:
                    break

    if not send_coords:
        return f"Opened WhatsApp chat with {contact} and pre-filled the message, but could not detect the send button to tap, sir."

    cx, cy = send_coords
    rc_tap, stdout_tap, stderr_tap = _run_adb(["-s", target, "shell", "input", "tap", str(cx), str(cy)])
    combined = (str(stdout_tap) + " " + str(stderr_tap)).lower()
    if "securityexception" in combined or "inject_events" in combined:
        return "Opened WhatsApp with the message pre-filled, but tapping Send requires 'USB debugging (Security settings)' enabled in Developer Options, sir."
    if rc_tap != 0:
        return f"Failed to tap WhatsApp send button: {stderr_tap}"

    time.sleep(1.8)  # Delay after sending
    return f"WhatsApp message sent to {contact}: '{message}', sir."


def _handle_volume(target: str, direction: str) -> str:
    key_map = {
        "up": "24",     # KEYCODE_VOLUME_UP
        "down": "25",   # KEYCODE_VOLUME_DOWN
        "mute": "164",  # KEYCODE_VOLUME_MUTE
    }
    code = key_map.get(direction)
    if not code:
        return f"Unknown volume action: {direction}."

    rc, stdout, stderr = _run_adb(["-s", target, "shell", "input", "keyevent", code])
    combined = (str(stdout) + " " + str(stderr)).lower()
    if "securityexception" in combined or "inject_events" in combined:
        return "Permission needed: Please enable 'USB debugging (Security settings)' in Developer Options on your phone, sir."

    if rc != 0:
        return f"Failed to adjust phone volume: {stderr}"

    time.sleep(0.5)
    if direction == "up":
        return "Phone volume increased, sir."
    elif direction == "down":
        return "Phone volume decreased, sir."
    else:
        return "Phone muted, sir."


def _handle_screenshot(target: str) -> str:
    SCREENSHOTS_DIR.mkdir(parents=True, exist_ok=True)
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"phone_screenshot_{ts}.png"
    filepath = SCREENSHOTS_DIR / filename

    rc, stdout_bytes, stderr = _run_adb(["-s", target, "exec-out", "screencap", "-p"], capture_binary=True)
    if rc != 0 or not isinstance(stdout_bytes, bytes) or len(stdout_bytes) < 500:
        return f"Phone screenshot capture failed: {stderr or 'Incomplete image data'}."

    try:
        filepath.write_bytes(stdout_bytes)
        return f"Phone screenshot captured and saved: {filepath.name}"
    except Exception as e:
        return f"Failed to save phone screenshot: {e}"


def _handle_lock_screen(target: str) -> str:
    rc, stdout, stderr = _run_adb(["-s", target, "shell", "input", "keyevent", "26"]) # KEYCODE_POWER
    combined = (str(stdout) + " " + str(stderr)).lower()
    if "securityexception" in combined or "inject_events" in combined:
        return "Permission needed: Please enable 'USB debugging (Security settings)' in Developer Options on your phone, sir."
    if rc != 0:
        return f"Failed to lock phone screen: {stderr}"
    time.sleep(1.0)
    return "Phone screen locked, sir."


def _handle_battery_status(target: str) -> str:
    rc, stdout, stderr = _run_adb(["-s", target, "shell", "dumpsys", "battery"])
    if rc != 0:
        return f"Could not retrieve phone battery status: {stderr}"

    level_m = re.search(r"level:\s*(\d+)", stdout)
    status_m = re.search(r"status:\s*(\d+)", stdout)
    ac_m = re.search(r"AC powered:\s*(true|false)", stdout, re.IGNORECASE)
    usb_m = re.search(r"USB powered:\s*(true|false)", stdout, re.IGNORECASE)
    wireless_m = re.search(r"Wireless powered:\s*(true|false)", stdout, re.IGNORECASE)

    level = level_m.group(1) if level_m else "Unknown"
    is_charging = False

    if status_m and status_m.group(1) in ("2", "5"):
        is_charging = True
    elif (ac_m and ac_m.group(1).lower() == "true") or \
         (usb_m and usb_m.group(1).lower() == "true") or \
         (wireless_m and wireless_m.group(1).lower() == "true"):
        is_charging = True

    if level != "Unknown":
        charge_str = "charging" if is_charging else "discharging"
        return f"Phone battery is at {level}% and currently {charge_str}, sir."

    return f"Phone battery info: {stdout.strip()[:100]}"


def _handle_list_devices() -> str:
    rc, stdout, stderr = _run_adb(["devices", "-l"])
    if rc != 0:
        return f"Could not list ADB devices: {stderr}"

    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    if len(lines) <= 1:
        return "No Android devices connected via ADB, sir."

    device_lines = lines[1:]
    return f"Connected Android devices ({len(device_lines)}):\n" + "\n".join(device_lines)


# ── Plugin Declaration ────────────────────────────────────────────────────────

PLUGIN = {
    "name": "android_control",
    "description": (
        "Controls the user's PHONE over ADB (not the PC). Use whenever the user asks to control "
        "their mobile/phone device or send WhatsApp messages via phone (e.g., 'phone e WhatsApp e message pathao', "
        "'Mamuni ke WhatsApp e message pathao', 'phone e type koro', 'phone er app close koro', "
        "'phone e YouTube kholo', 'phone er volume', 'phone e tap koro', 'phone er screenshot', 'phone er battery'). "
        "For PC/desktop operations, use desktop tools instead. "
        "Actions: phone_open_app | phone_close_app | phone_send_whatsapp_message | phone_type_text | "
        "phone_tap_element | phone_volume_up | phone_volume_down | phone_mute | phone_screenshot | "
        "phone_lock_screen | phone_battery_status | phone_list_devices."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": (
                    "The exact phone action to perform. Pick one of: "
                    "phone_open_app | phone_close_app | phone_send_whatsapp_message | "
                    "phone_type_text | phone_tap_element | phone_volume_up | "
                    "phone_volume_down | phone_mute | phone_screenshot | "
                    "phone_lock_screen | phone_battery_status | phone_list_devices"
                ),
            },
            "app_name": {
                "type": "STRING",
                "description": (
                    "App name to launch or close on the phone (e.g. 'whatsapp', 'youtube', 'chrome', 'camera', 'gallery', 'settings'). "
                    "Used with phone_open_app and phone_close_app."
                ),
            },
            "contact": {
                "type": "STRING",
                "description": (
                    "Contact name (looked up in config/contacts.json, e.g. 'Mamuni', 'Abbu') or phone number with country code. "
                    "Used with phone_send_whatsapp_message."
                ),
            },
            "message": {
                "type": "STRING",
                "description": "Message text to send via WhatsApp on phone. Used with phone_send_whatsapp_message.",
            },
            "text": {
                "type": "STRING",
                "description": "Text to type on the phone keyboard. Used with phone_type_text.",
            },
            "description": {
                "type": "STRING",
                "description": (
                    "Visible text, content-description, or resource-id of the UI element to tap on the phone screen. "
                    "Used with phone_tap_element."
                ),
            },
        },
        "required": ["action"],
    },
}

PLUGIN_SETTINGS = {
    "namespace": "android_control",
    "title": "ANDROID PHONE CONTROL",
    "fields": [
        {
            "key": "device_ip",
            "label": "Phone Wi-Fi IP Address (e.g. 192.168.0.105)",
            "type": "text",
            "placeholder": "192.168.1.xxx",
            "default": "",
        }
    ],
}


def run(parameters: dict, player=None, session_memory=None) -> str:
    """
    Main entry point for android_control plugin called by JARVIS plugin loader.
    """
    params = parameters or {}
    raw_action = params.get("action", "").lower().strip()

    # Normalize action: accept both phone_<action> and legacy <action>
    action = raw_action
    if not action.startswith("phone_") and f"phone_{action}" in (
        "phone_open_app", "phone_close_app", "phone_send_whatsapp_message",
        "phone_type_text", "phone_tap_element", "phone_volume_up",
        "phone_volume_down", "phone_mute", "phone_screenshot",
        "phone_lock_screen", "phone_battery_status", "phone_list_devices"
    ):
        action = f"phone_{action}"

    if not _ADB_PATH:
        return "ADB is not installed or not found on PATH, sir. Please install Android Platform Tools and add adb to PATH."

    if player:
        try:
            player.write_log(f"[Android] Action: {action}")
        except Exception:
            pass

    print(f"[Android] > Action: {action}  Params: {params}")

    # phone_list_devices does not require a pre-configured IP
    if action == "phone_list_devices":
        return _handle_list_devices()

    # Verify device IP & connection
    target, err = _get_target_endpoint()
    if err:
        return err

    ok, conn_err = _ensure_device_connected(target)
    if not ok:
        return conn_err

    try:
        if action == "phone_open_app":
            app_name = params.get("app_name") or params.get("name") or ""
            return _handle_open_app(target, app_name)

        elif action == "phone_close_app":
            app_name = params.get("app_name") or params.get("name") or ""
            return _handle_close_app(target, app_name)

        elif action == "phone_send_whatsapp_message":
            contact = params.get("contact") or params.get("contact_name_or_number") or params.get("to") or ""
            message = params.get("message") or params.get("text") or ""
            return _handle_send_whatsapp_message(target, contact, message)

        elif action == "phone_type_text":
            text_to_type = params.get("text") or params.get("query") or ""
            return _handle_type_text(target, text_to_type)

        elif action == "phone_tap_element":
            element_desc = params.get("description") or params.get("element") or params.get("target") or ""
            return _handle_tap_element(target, element_desc)

        elif action == "phone_volume_up":
            return _handle_volume(target, "up")

        elif action == "phone_volume_down":
            return _handle_volume(target, "down")

        elif action == "phone_mute":
            return _handle_volume(target, "mute")

        elif action == "phone_screenshot":
            return _handle_screenshot(target)

        elif action == "phone_lock_screen":
            return _handle_lock_screen(target)

        elif action == "phone_battery_status":
            return _handle_battery_status(target)

        else:
            return (
                f"Unknown Android action: '{raw_action}'. "
                "Available: phone_open_app, phone_close_app, phone_send_whatsapp_message, "
                "phone_type_text, phone_tap_element, phone_volume_up, phone_volume_down, "
                "phone_mute, phone_screenshot, phone_lock_screen, phone_battery_status, phone_list_devices."
            )
    except Exception as e:
        print(f"[Android] [ERR] in {action}: {e}")
        return f"Phone action '{action}' failed, sir: {e}"
