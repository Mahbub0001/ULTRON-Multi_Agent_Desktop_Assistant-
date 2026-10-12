"""Android commands are tested without real contacts, device data or permissions."""
import json
import shlex
from types import SimpleNamespace

import pytest

from plugins import android_control as android
from plugins import _android_features as features


@pytest.fixture(autouse=True)
def isolated_config(monkeypatch, tmp_path):
    monkeypatch.setattr(android, "BASE_DIR", tmp_path)
    monkeypatch.setattr(android, "_ADB_PATH", "adb")
    monkeypatch.setattr(android, "get_android_ip", lambda: "")
    monkeypatch.setattr(android, "get_plugin_config", lambda _: {})
    monkeypatch.setattr(features, "get_plugin_config", lambda _: {})
    monkeypatch.setattr(features, "save_plugin_config", lambda *_: None)
    features._forwards.clear()


def phone():
    return features.PhoneFeatures(android, "test-usb")


def test_shell_values_are_quoted_once(monkeypatch):
    captured = []
    def run(args, **kwargs):
        captured.append(args)
        return SimpleNamespace(returncode=0, stdout="ok", stderr="")
    monkeypatch.setattr(android.subprocess, "run", run)
    value = "বাংলা file & $(touch nope); 'quoted'"
    android._run_adb(["-s", "test-usb", "shell", "am", "start", "--es", "label", value])
    assert shlex.split(captured[0][-1]) == ["am", "start", "--es", "label", value]
    assert captured[0][0:4] == ["adb", "-s", "test-usb", "shell"]


def test_usb_selection_and_multiple_devices(monkeypatch):
    monkeypatch.setattr(android, "_run_adb", lambda _: (0, "List of devices attached\none device\n", ""))
    assert android._get_target_endpoint() == ("one", None)
    monkeypatch.setattr(android, "_run_adb", lambda _: (0, "List of devices attached\none device\ntwo device\n", ""))
    assert android._get_target_endpoint()[0] is None
    monkeypatch.setattr(android, "get_plugin_config", lambda _: {"device_serial": "two", "device_ip": "192.0.2.1"})
    assert android._get_target_endpoint() == ("two", None)


def test_offline_after_connect_is_not_success(monkeypatch):
    calls = iter([(0, "List of devices attached\n", ""), (0, "connected to 192.0.2.1:5555", ""),
                  (0, "List of devices attached\n192.0.2.1:5555 offline\n", "")])
    monkeypatch.setattr(android, "_run_adb", lambda *a, **k: next(calls))
    monkeypatch.setattr(android.time, "sleep", lambda _: None)
    assert android._ensure_device_connected("192.0.2.1:5555")[0] is False


def test_lock_does_not_toggle_awake_screen(monkeypatch):
    calls = []
    monkeypatch.setattr(android, "_run_adb", lambda args, **kw: (calls.append(args) or 0, "", ""))
    monkeypatch.setattr(android.time, "sleep", lambda _: None)
    android._handle_lock_screen("test-usb")
    assert calls[0][-1] == "223"


def test_unicode_uses_companion_not_input_text(monkeypatch):
    calls = []
    monkeypatch.setattr(features.PhoneFeatures, "companion", lambda self, command, **p: calls.append((command, p)))
    assert android._handle_type_text("test-usb", "হ্যালো বাংলাদেশ") == "Phone text entered."
    assert calls == [("text", {"text": "হ্যালো বাংলাদেশ"})]


def test_unsupported_key_cannot_be_a_shell_argument(monkeypatch):
    monkeypatch.setattr(android, "_run_adb", lambda *a, **k: pytest.fail("Must not execute an invalid key"))
    assert "Unsupported" in android._handle_press_key("test-usb", "66; reboot")


def test_scroll_scales_to_override_screen(monkeypatch):
    calls = []
    def shell(*args, **kwargs):
        calls.append(args)
        return "Physical size: 1080x2400\nOverride size: 720x1600" if args[:2] == ("wm", "size") else ""
    p = phone()
    monkeypatch.setattr(p, "shell", shell)
    p.scroll("down", 2)
    assert calls[1:] == [("input", "swipe", 360, 1200, 360, 400, 400)] * 2


def test_inspection_indices_exclude_passwords_and_disabled_nodes(monkeypatch):
    monkeypatch.setattr(android, "_dump_ui_hierarchy", lambda _: '''<hierarchy>
      <node text="Secret" password="true"/><node text="disabled" enabled="false"/>
      <node text="Video one" enabled="true" clickable="true" bounds="[0,100][100,200]"/>
      <node text="Video two" enabled="true" clickable="true" bounds="[0,200][100,300]"/>
    </hierarchy>''')
    p = phone()
    assert [n["text"] for n in p.elements()] == ["Video one", "Video two"]
    calls = []
    monkeypatch.setattr(p, "shell", lambda *a: calls.append(a) or "")
    p.tap(index=2)
    assert calls == [("input", "tap", 50, 250)]


def test_companion_action_error_never_retries_as_adb(monkeypatch):
    p = phone()
    def denied(*a, **k):
        raise features.PhoneError("Access denied")
    monkeypatch.setattr(p, "companion", denied)
    monkeypatch.setattr(p, "shell", lambda *a: pytest.fail("Must not repeat rejected command"))
    with pytest.raises(features.PhoneError, match="Access denied"):
        p.scroll("down")


def test_guarded_tap_does_not_fall_back_to_unverified_coordinates(monkeypatch):
    p = phone()
    monkeypatch.setattr(p, "elements", lambda: pytest.fail("Guarded action must not fall back"))
    with pytest.raises(features.PhoneError, match="Guarded"):
        p.tap("Send", expected_package="com.example.chat")


def test_clear_text_preserves_expected_app_and_exact_content(monkeypatch):
    p = phone(); calls = []
    monkeypatch.setattr(p, "companion", lambda command, **params: calls.append((command, params)) or {"text_entered": True})
    p.dispatch("phone_clear_text", {"expected_package": "com.example.chat", "expected_text": "draft"})
    assert calls == [("text", {"text": "", "expected_package": "com.example.chat", "expected_text": "draft"})]


def test_lost_mutation_response_is_uncertain_not_adb_fallback(monkeypatch):
    import contextlib
    import urllib.error
    calls = []
    monkeypatch.setattr(features, "get_plugin_config", lambda _: {"companion_token": "test-only-token"})
    monkeypatch.setattr(android, "_run_adb", lambda *a, **k: (0, "12345", ""))
    class Opener:
        def open(self, req, **kwargs):
            command = json.loads(req.data)["command"]
            calls.append(command)
            if command == "status":
                return contextlib.nullcontext(SimpleNamespace(read=lambda n: b'{"ok":true,"data":{}}'))
            raise urllib.error.URLError("Response lost")
    monkeypatch.setattr(features.urllib.request, "build_opener", lambda *_: Opener())
    with pytest.raises(features.PhoneError, match="uncertain") as exc:
        features.bridge(android, "test-usb", "tap", description="Play")
    assert not isinstance(exc.value, features.BridgeUnavailable)
    assert calls == ["status", "tap"]


def test_failed_ui_dump_never_reads_old_xml(monkeypatch):
    calls = []
    def adb(args, **kwargs):
        calls.append(args)
        return (0, "ERROR: could not get idle state", "") if "uiautomator" in args else (0, "", "")
    monkeypatch.setattr(android, "_run_adb", adb)
    assert android._dump_ui_hierarchy("test-usb") is None
    assert len(calls) == 2 and not any("cat" in c for c in calls)


@pytest.mark.parametrize("path", ["/data/private", "/sdcard/../data/file", "relative", "/sdcard/a\nfile", "/sdcard\\a"])
def test_remote_transfer_rejects_unsafe_paths(path):
    with pytest.raises(features.PhoneError):
        features.remote_path(path)


def test_push_preserves_existing_destination(monkeypatch, tmp_path):
    source = tmp_path / "file & name.txt"; source.write_text("test")
    p = phone()
    monkeypatch.setattr(p, "shell", lambda *a: "exists")
    monkeypatch.setattr(android, "_run_adb", lambda *a, **k: pytest.fail("Must not overwrite"))
    with pytest.raises(features.PhoneError, match="already exists"):
        p.transfer("push", str(source), "/sdcard/Download/file & name.txt")


def test_pull_reports_verified_file_path(monkeypatch, tmp_path):
    dest = tmp_path / "new.txt"
    p = phone()
    monkeypatch.setattr(p, "shell", lambda *a: "4" if a[0] == "stat" else "file")
    def adb(args, **kwargs):
        assert args == ["-s", "test-usb", "pull", "/sdcard/Download/new.txt", str(dest)]
        dest.write_bytes(b"test")
        return 0, "pulled", ""
    monkeypatch.setattr(android, "_run_adb", adb)
    assert str(dest) in p.transfer("pull", str(dest), "/sdcard/Download/new.txt")


def test_file_listing_preserves_unicode_and_paginates(monkeypatch):
    p = phone()
    monkeypatch.setattr(p, "shell", lambda *a: "/sdcard/Download/b & বাংলা.txt\0/sdcard/Download/a.txt\0")
    first = json.loads(p.dispatch("phone_list_files", {"page_size": 1}))
    assert first["files"] == ["/sdcard/Download/a.txt"] and first["next_offset"] == 1
    second = json.loads(p.dispatch("phone_list_files", {"page_size": 1, "offset": 1}))
    assert second["files"] == ["/sdcard/Download/b & বাংলা.txt"] and second["next_offset"] is None


def test_global_navigation_uses_companion(monkeypatch):
    calls = []
    monkeypatch.setattr(features.PhoneFeatures, "companion", lambda self, command, **p: calls.append((command, p)))
    monkeypatch.setattr(android, "_run_adb", lambda *a, **k: pytest.fail("Companion should handle global navigation"))
    assert "Pressed" in android._handle_press_key("test-usb", "back")
    assert calls == [("global", {"action": "back"})]


def test_wifi_off_refuses_wireless_disconnection(monkeypatch):
    p = features.PhoneFeatures(android, "192.0.2.1:5555")
    monkeypatch.setattr(p, "shell", lambda *a: pytest.fail("Must not disconnect control link"))
    with pytest.raises(features.PhoneError, match="USB"):
        p.network("wifi", False)


def test_brightness_verifies_and_validates(monkeypatch):
    p = phone(); calls = []
    monkeypatch.setattr(p, "shell", lambda *a: calls.append(a) or ("128" if a[:2] == ("settings", "get") else ""))
    assert "50%" in p.brightness(50)
    assert calls[0] == ("settings", "put", "system", "screen_brightness_mode", 0)
    with pytest.raises(features.PhoneError): p.brightness(101)
    with pytest.raises(features.PhoneError): p.brightness(True)


def test_alarm_requires_real_time_and_does_not_claim_saved(monkeypatch):
    p = phone(); calls = []
    monkeypatch.setattr(p, "shell", lambda *a: calls.append(a) or "")
    result = p.alarm("07:30", "Wake & study", [2, 3])
    assert "Check/confirm" in result
    assert "Wake & study" in calls[0] and "2,3" in calls[0] and "--eial" in calls[0]
    with pytest.raises(features.PhoneError): p.alarm("25:99")


def test_routine_persistence_without_connected_device():
    steps = [{"action": "phone_media_control", "parameters": {"command": "pause"}}]
    assert "saved" in android.run({"action": "phone_save_routine", "routine_name": "study", "steps": steps})
    assert json.loads(android.run({"action": "phone_list_routines"}))["saved"]["study"] == steps
    assert "deleted" in android.run({"action": "phone_delete_routine", "routine_name": "study"})


def test_invalid_routine_cannot_recurse_or_message():
    for action in ("phone_run_routine", "phone_reply_notification", "phone_send_whatsapp_message"):
        with pytest.raises(features.PhoneError): phone().validate_steps([{"action": action}])


def test_routine_stops_on_partial_failure(monkeypatch):
    p = phone()
    p.routine("phone_save_routine", {"routine_name": "study", "steps": [
        {"action": "phone_media_control", "parameters": {"command": "pause"}},
        {"action": "phone_set_brightness", "parameters": {"percent": 40}}]})
    monkeypatch.setattr(p, "media", lambda _: "paused")
    def fail(_): raise features.PhoneError("permission denied")
    monkeypatch.setattr(p, "brightness", fail)
    result = json.loads(p.routine("phone_run_routine", {"routine_name": "study"}))
    assert result["ok"] is False and result["failed_step"] == 2 and len(result["completed"]) == 1


def test_notifications_reply_requires_human_confirmation(monkeypatch):
    p = phone(); calls = []
    notification = {"key": "key1", "package": "com.example.chat", "title": "A", "can_reply": True}
    monkeypatch.setattr(p, "companion", lambda command, **params: calls.append(command) or [notification])
    from core import confirm
    monkeypatch.setattr(confirm, "request", lambda key, title, detail, run: "Confirmation pending")
    assert p.dispatch("phone_reply_notification", {"notification_key": "key1", "message": "Hello"}) == "Confirmation pending"
    assert calls == ["notifications"]


def test_reply_callback_runs_only_after_confirm_and_cancel_does_not_send(monkeypatch):
    import threading
    from core import confirm
    calls, dispatched = [], threading.Event()
    notification = {"key": "key1", "package": "com.example.chat", "title": "A", "can_reply": True}
    p = phone()
    def companion(command, **params):
        calls.append(command)
        if command == "notifications": return [notification]
        dispatched.set()
        return {"reply_dispatched": True}
    monkeypatch.setattr(p, "companion", companion)
    monkeypatch.setattr(confirm, "_show_cb", lambda *_: None)
    monkeypatch.setattr(confirm, "_hide_cb", lambda: None)
    monkeypatch.setattr(confirm, "_log_cb", lambda *_: None)
    monkeypatch.setattr(confirm, "_pending", None)
    params = {"notification_key": "key1", "message": "Hello"}
    p.dispatch("phone_reply_notification", params)
    assert calls == ["notifications"]
    confirm.resolve(False)
    assert not dispatched.is_set()
    p.dispatch("phone_reply_notification", params)
    assert calls == ["notifications", "notifications"]
    confirm.resolve(True)
    assert dispatched.wait(2)
    assert calls == ["notifications", "notifications", "reply"]


def test_alert_opt_in_needs_notification_allowlist():
    with pytest.raises(features.PhoneError, match="notification_packages"):
        phone().dispatch("phone_configure_alerts", {"notification_alerts": True})


def test_monitor_battery_transition_and_notification_dedup(monkeypatch):
    cfg = {"battery_alerts": True, "notification_alerts": True, "notification_packages": ["com.example.chat"]}
    monkeypatch.setattr(features, "get_plugin_config", lambda _: cfg)
    monkeypatch.setattr(android, "_get_target_endpoint", lambda: ("test-usb", None))
    monkeypatch.setattr(android, "_run_adb", lambda _: (0, "List of devices attached\ntest-usb device", ""))
    from memory import config_manager
    monkeypatch.setattr(config_manager, "get_plugin_enabled", lambda _: True)
    status = {"battery_percent": 19, "charging": False}
    notifications = [{"key": "one", "package": "com.example.chat", "importance": 4, "title": "Hi", "text": "Old", "time": 1}]
    monkeypatch.setattr(features.PhoneFeatures, "status", lambda _: status)
    monkeypatch.setattr(features.PhoneFeatures, "companion", lambda *a, **k: notifications)
    monitor = features.PhoneAlertMonitor()
    assert len(monitor.check(android)) == 1  # low battery; pre-existing notifications form baseline
    assert monitor.check(android) == []
    notifications[0]["text"] = "New"; notifications[0]["time"] = 2
    assert "notification" in monitor.check(android)[0]
    assert monitor.check(android) == []
    status["charging"] = True
    assert monitor.check(android) == []
    status["charging"] = False
    assert len(monitor.check(android)) == 1


def test_schema_exposes_all_dispatch_actions():
    enum = android.PLUGIN["parameters"]["properties"]["action"]["enum"]
    assert set(features.FEATURE_ACTIONS) <= set(enum)
    assert all(key in android.PLUGIN["parameters"]["properties"] for key in ("index", "remote_path", "steps", "notification_key"))
