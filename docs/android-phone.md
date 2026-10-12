# Android phone features

ULTRON controls one explicitly selected Android phone through ADB. Android 13
is supported. USB and paired Wireless Debugging both work; no root is required.
The desktop assistant must be running for voice commands and background alerts.

## Connect the phone

1. Install Android Platform Tools and enable USB debugging on the phone.
2. Connect USB and accept the computer's debugging authorization. Run
   `adb devices -l` to check that the device state is `device`.
3. In ULTRON's plugin settings → **ANDROID PHONE CONTROL**, select an ADB
   device serial or set a wireless `IP:port`. The serial takes priority over
   the plugin Wi-Fi IP, then the main Android IP setting. With no selection,
   exactly one authorized connected device is selected automatically.
4. For Android 13 Wireless Debugging, pair first using the pairing dialog's
   address/port: `adb pair IP:PAIRING_PORT`; then connect using the separate
   debugging address/port: `adb connect IP:DEBUGGING_PORT`. The pairing port
   and debugging port are different and may change. Do not assume port 5555.

The phone must be awake/unlocked when navigating apps. Some Xiaomi/MIUI phones
also need **USB debugging (Security settings)** for ADB input injection. ULTRON
reports permission failures; it does not bypass the lock screen.

## Install the optional companion

The [phone-companion](../phone-companion) Android project uses native Java and
Android APIs, without third-party runtime dependencies. Build it with JDK 17+
(tested with JDK 21), Android SDK platform 35 and build tools 35.0.0:

```powershell
cd phone-companion
# Set ANDROID_HOME to your SDK, or create ignored local.properties with sdk.dir.
.\gradlew.bat assembleDebug lintDebug
adb -s YOUR_DEVICE_SERIAL install -r app\build\outputs\apk\debug\app-debug.apk
```

Open **ULTRON Phone** on the phone, copy its private connection token into
the desktop's Android Phone Control plugin settings, and tap **Start connection**.
Then enable only the permissions you want using the app's buttons:

- **Screen control:** enable ULTRON Phone control in Accessibility settings.
  Supports visible UI inspection, clicks, long presses, scrolling and Unicode
  text entry. Unicode typing replaces the focused editable field's contents;
  password fields are excluded. Use `phone_clear_text` to clear the focused field.
- **Notification access:** enable ULTRON Phone notifications to read active
  notifications and use apps' direct-reply actions. ULTRON does not store a
  notification history. Sensitive content may be hidden by Android or the app.
- **Flashlight / Bluetooth / alerts:** grant camera, Bluetooth and notification
  permissions as needed. Flashlight can fail while another app owns the camera.

The connection server listens only on the phone's loopback address. The desktop
creates an ADB port forward and authenticates each request with the token. It
does not expose an unauthenticated server on the local Wi-Fi network. Stop the
connection from the app or ongoing notification. Resetting the token revokes the
old token; save the new one on the desktop. The connection does not auto-start
after a reboot. Tokens live in ignored `config/api_keys.json`, never in Git.

## Commands and examples

| Feature | Example voice request | Tool actions |
|---|---|---|
| Apps | “Phone e YouTube kholo”; “phone er app list dekhao” | `phone_open_app`, `phone_close_app`, `phone_list_apps` |
| Search | “Phone er YouTube e Python tutorial search koro” | `phone_search_youtube`, `phone_search` |
| Navigation | “Niche scroll koro”; “ditiyo video ta chalao”; “back jao” | `phone_screen_elements`, `phone_tap_element`, `phone_scroll`, `phone_long_press`, `phone_press_key` |
| Typing | “Phone e Banglay eta likho” | `phone_type_text` (Unicode requires companion) |
| Status | “Phone er battery, Wi-Fi ar Bluetooth status bolo” | `phone_status`, `phone_battery_status`, `phone_companion_status` |
| Quick controls | “Brightness 40 percent”; “volume 20 percent”; “flashlight on” | `phone_set_brightness`, `phone_set_volume`, `phone_volume_up/down`, `phone_mute`, `phone_flashlight` |
| Media | “Gaan pause koro”; “porer gaan dao” | `phone_media_control` |
| Connectivity | “Bluetooth on koro” | `phone_set_wifi`, `phone_set_bluetooth` |
| Capture/lock | “Phone er screenshot nao”; “screen lock koro” | `phone_screenshot`, `phone_lock_screen` |
| Transfer | “Ei PDF phone er Download folder e pathao”; “phone er ei photo PC te ano” | `phone_push_file`, `phone_pull_file`, `phone_list_files` |
| Alarm | “Kal shokal 7:30 e phone e alarm dao” | `phone_set_alarm` |
| Routines | “Phone e sleep routine chalao, YouTube bondho koro, 7:30 alarm dao” | `phone_run_routine`, `phone_save_routine`, `phone_list_routines`, `phone_delete_routine` |
| Notifications | “Phone er notification pore shonao”; “ei notification e reply dao” | `phone_notifications`, `phone_reply_notification` |
| Alerts | “Battery 20 percent er niche gele bolo”; “WhatsApp er important notification janabe” | `phone_configure_alerts` |

The model first inspects the current UI, identifies the requested item by its
actual text/description, and selects its current index. An element index is not
an ordinal video number and becomes stale when the screen changes. Companion
UI actions accept `expected_package` from the inspected elements and refuse
input if another app becomes active. `phone_clear_text` can also use
`expected_text` to preserve a draft changed since inspection. Custom app
controls, games, protected screens and WebViews may not expose useful nodes;
ULTRON reports these limitations instead of inventing a successful result.

Brightness switches to manual mode and uses the standard Android 0..255 setting;
some manufacturers use a different brightness scale. Exact media volume uses the
phone's actual stream range. Media commands go to the active media session; a
key dispatch alone does not prove playback changed. Wi-Fi off is refused over
wireless ADB to preserve the control connection; select a USB serial first.

File listings return JSON pages (`offset`, `page_size`, `next_offset`) and preserve
Unicode filenames. File transfers accept regular files in `/sdcard` or `/storage/emulated/0`, require
an explicit remote filename, preserve existing destination files, and verify
destination size. `phone_pull_file` defaults to ignored `downloads/phone/`.
No app-private files are accessed. A failed transfer can leave a partial file;
inspect it and choose a new destination or remove it before retrying.

Alarm requests open the phone's clock UI so the user can review/confirm them.
Android clock apps differ; opening the intent is not proof an alarm was saved.
Alarm days use Android's convention: 1=Sunday through 7=Saturday. The phone's
local clock/timezone is used.

## Routines

Built-in `sleep` pauses media, sets media volume to 20%, and brightness to 10%.
Optional `apps` close the named apps and optional `time` opens the alarm UI.
Custom routines are saved in ignored `config/phone_routines.json` and can use
app open/close, Back/Home/Recent/Enter/Wake/Sleep, screen lock, media, brightness,
volume and alarm actions. Message sending and nested routines are excluded.

```json
{
  "action": "phone_save_routine",
  "routine_name": "study",
  "steps": [
    {"action": "phone_media_control", "parameters": {"command": "pause"}},
    {"action": "phone_set_brightness", "parameters": {"percent": 40}}
  ]
}
```

Routines stop at the first failed step and report completed steps; earlier changes
remain applied. ULTRON does not rerun successful steps automatically.

## Notification and battery alerts

Alerts are off by default. Enable them explicitly:

```json
{"action":"phone_configure_alerts","battery_alerts":true,"battery_threshold":20}
```

```json
{"action":"phone_configure_alerts","notification_alerts":true,"notification_packages":["com.whatsapp"]}
```

Only allowlisted notifications with Android importance >= 3 are spoken. Existing
notifications form a baseline; new/changed notifications are deduplicated. Low
battery alerts fire once per discharge episode, rearming after charging or
recovering above threshold + 5%. Polling occurs every 30 seconds while ULTRON is
awake and the conversation is idle. Offline/permission errors stay quiet. Alerts
do not wake or reconnect a phone, execute notification instructions, or auto-reply.

Direct replies require the exact current notification key, the app's free-form
reply action, and a human click in ULTRON's confirmation panel. A successful
dispatch is reported as dispatch, not proof of delivery. Disable an alert by
setting its corresponding `battery_alerts` or `notification_alerts` to `false`.

## Validation

Desktop tests:

```powershell
.venv\Scripts\python.exe -m pytest tests/test_android_control.py -q --basetemp=scratch/pytest-phone
```

Android build/lint: `gradlew.bat assembleDebug lintDebug`. For device validation,
use a dedicated test file and editable field, record and restore any changed
brightness/volume, and never send test messages to real contacts.

Validated on Android 13: battery/connectivity status, brightness and exact volume
with restoration, Unicode/special-character filenames in a PC → phone → PC
transfer with SHA-256 comparison, authenticated companion access and rejection of
an invalid token, launcher-app discovery, tap/scroll, Bangla typing/clearing,
foreground-app and changed-draft guards, flashlight, and a local notification
direct reply through Android RemoteInput. Wi-Fi/Bluetooth controls were checked
by requesting their existing states. Phone alert deduplication, routine failures,
alarm extra formats and the confirmation lifecycle are covered by regression tests.
Real contact messaging, clock-specific alarm saving and disconnecting Wi-Fi were
not exercised by the device diagnostics.
