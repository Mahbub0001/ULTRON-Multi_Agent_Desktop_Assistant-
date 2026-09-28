"""
Prepares the complete standalone application bundle in dist/ULTRON/
Copies assets, code modules, sanitized config templates, and empty data directories.
Ensures zero developer secrets / private API keys are bundled.
"""
import os
import shutil
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST_APP = ROOT / "dist" / "ULTRON"

def setup_bundle():
    print(f"[*] Preparing standalone bundle at: {DIST_APP}")
    assert DIST_APP.exists(), f"Directory {DIST_APP} does not exist. Run PyInstaller first."

    # 1. Core directory
    core_dest = DIST_APP / "core"
    core_dest.mkdir(parents=True, exist_ok=True)
    for f in ["face_model.obj", "prompt.txt"]:
        src = ROOT / "core" / f
        if src.exists():
            shutil.copy2(src, core_dest / f)
            print(f"  [+] Copied core/{f}")
        else:
            print(f"  [!] Warning: core/{f} missing")

    # 2. Config directory (clean & sanitized)
    config_dest = DIST_APP / "config"
    config_dest.mkdir(parents=True, exist_ok=True)
    
    # Copy icon
    if (ROOT / "config" / "jarvis.ico").exists():
        shutil.copy2(ROOT / "config" / "jarvis.ico", config_dest / "jarvis.ico")
        print("  [+] Copied config/jarvis.ico")
        
    # Copy safe config files
    for f in ["agents.json", "email_config.example.json", "api_keys.example.json"]:
        src = ROOT / "config" / f
        if src.exists():
            shutil.copy2(src, config_dest / f)
            print(f"  [+] Copied config/{f}")

    # Write clean contacts.json
    (config_dest / "contacts.json").write_text(json.dumps({}, indent=4), encoding="utf-8")
    print("  [+] Created clean config/contacts.json")

    # Write clean local_calendar_events.json
    (config_dest / "local_calendar_events.json").write_text("[]", encoding="utf-8")
    print("  [+] Created clean config/local_calendar_events.json")

    # Write sanitized api_keys.json (NO developer keys!)
    clean_api_keys = {
        "gemini_api_key": "",
        "os_system": "windows",
        "camera_index": 0,
        "user_name": "Nibir",
        "assistant_name": "ULTRON",
        "android_device_ip": "",
        "proactive_audio": False,
        "turn_tuning": {
            "enabled": False
        }
    }
    (config_dest / "api_keys.json").write_text(json.dumps(clean_api_keys, indent=4), encoding="utf-8")
    print("  [+] Created clean sanitized config/api_keys.json (NO private keys)")

    # 3. Actions directory
    actions_dest = DIST_APP / "actions"
    if actions_dest.exists():
        shutil.rmtree(actions_dest)
    shutil.copytree(ROOT / "actions", actions_dest, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    print("  [+] Copied actions/ tree")

    # 4. Plugins directory
    plugins_dest = DIST_APP / "plugins"
    if plugins_dest.exists():
        shutil.rmtree(plugins_dest)
    shutil.copytree(ROOT / "plugins", plugins_dest, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    print("  [+] Copied plugins/ tree")

    # 5. Memory directory
    memory_dest = DIST_APP / "memory"
    memory_dest.mkdir(parents=True, exist_ok=True)
    for py_file in (ROOT / "memory").glob("*.py"):
        shutil.copy2(py_file, memory_dest / py_file.name)
    (memory_dest / "long_term.json").write_text("{}", encoding="utf-8")
    (memory_dest / "agent_town_memory.json").write_text("{}", encoding="utf-8")
    print("  [+] Copied memory/ modules and initialized clean memories")

    # 6. Data directory
    data_dest = DIST_APP / "data"
    data_dest.mkdir(parents=True, exist_ok=True)
    print("  [+] Initialized clean data/ directory")

    print("\n[SUCCESS] Standalone bundle prepared successfully at dist/ULTRON/")

if __name__ == "__main__":
    setup_bundle()
