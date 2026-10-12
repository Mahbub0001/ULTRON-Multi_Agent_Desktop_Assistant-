"""Noninteractive frozen-package validation; does not start audio or a session."""
import importlib
import json
from pathlib import Path


def check_bundle(base_dir, report_path):
    base = Path(base_dir)
    result = {"ok": True, "imports": [], "files": [], "errors": []}
    modules = (
        "ui", "sounddevice", "google.genai", "PyQt6.QtWidgets", "playwright.async_api",
        "pyautogui", "pyperclip", "psutil", "win32gui", "win32process", "pywinauto",
        "core.avatar_mesh", "core.browser_scroll", "core.desktop_target", "core.uia", "core.universal_operator",
        "actions.open_app", "actions.computer_control", "actions.browser_control", "actions.app_control",
        "plugins.android_control", "plugins._android_features", "memory.config_manager",
    )
    for module in modules:
        try:
            importlib.import_module(module)
            result["imports"].append(module)
        except Exception as exc:
            result["errors"].append(f"{module}: {type(exc).__name__}: {exc}")
    for name in ("core/prompt.txt", "core/face_model.obj", "config/jarvis.ico",
                 "actions/browser_control.py", "plugins/android_control.py", "plugins/_android_features.py"):
        if (base / name).is_file():
            result["files"].append(name)
        else:
            result["errors"].append("Missing bundled file: " + name)
    if not result["errors"]:
        from actions.browser_control import TOOL as browser
        from actions.app_control import TOOL as app
        from plugins.android_control import PLUGIN
        if "hover" not in browser["parameters"]["properties"]["action"]["description"]:
            result["errors"].append("Browser hover action missing")
        if "verify_state" not in app["parameters"]["properties"]["action"]["enum"]:
            result["errors"].append("App state verification action missing")
        if "phone_notifications" not in json.dumps(PLUGIN, default=str):
            result["errors"].append("Phone notification action missing")
    result["ok"] = not result["errors"]
    Path(report_path).write_text(json.dumps(result, indent=2), encoding="utf-8")
    return 0 if result["ok"] else 1
