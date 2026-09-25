"""
Screen & webcam capture for JARVIS vision.

Provides the two capture entry points main.py uses — `_capture_screen()` and
`_capture_camera()` — plus their helpers (compression, camera auto-detection,
config access). main.py grabs a frame here on demand, then injects it into the
main Gemini Live session; there is no separate vision session here.
"""
from __future__ import annotations

import io
import json
import sys
from pathlib import Path

import numpy as np

if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

try:
    import cv2
    _CV2 = True
except ImportError:
    _CV2 = False

try:
    import mss
    import mss.tools
    _MSS = True
except ImportError:
    _MSS = False

try:
    import PIL.Image
    _PIL = True
except ImportError:
    _PIL = False


def _base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent


_BASE        = _base_dir()
_CONFIG_PATH = _BASE / "config" / "api_keys.json"


def _load_config() -> dict:
    try:
        return json.loads(_CONFIG_PATH.read_text(encoding="utf-8"))
    except Exception:
        return {}


def _save_config_key(key: str, value) -> None:
    try:
        cfg = _load_config()
        cfg[key] = value
        _CONFIG_PATH.write_text(json.dumps(cfg, indent=4), encoding="utf-8")
    except Exception as e:
        print(f"[Vision] ⚠️  Could not save config key '{key}': {e}")


def _get_os() -> str:
    return _load_config().get("os_system", "windows").lower()


_IMG_MAX_W = 1280
_IMG_MAX_H = 720
_JPEG_Q    = 82


def _compress(img_bytes: bytes, source_format: str = "PNG") -> tuple[bytes, str]:
    if not _PIL:
        return img_bytes, f"image/{source_format.lower()}"

    try:
        img = PIL.Image.open(io.BytesIO(img_bytes)).convert("RGB")
        img.thumbnail((_IMG_MAX_W, _IMG_MAX_H), PIL.Image.BILINEAR)
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=_JPEG_Q, optimize=False)
        return buf.getvalue(), "image/jpeg"
    except Exception as e:
        print(f"[Vision] ⚠️  Image compress failed: {e}")
        return img_bytes, f"image/{source_format.lower()}"


def _capture_screen() -> tuple[bytes, str]:
    # 1. Try mss (fastest)
    if _MSS:
        try:
            with mss.mss() as sct:
                monitors = sct.monitors
                target   = monitors[1] if len(monitors) > 1 else monitors[0]
                shot     = sct.grab(target)
                png      = mss.tools.to_png(shot.rgb, shot.size)
                return _compress(png, "PNG")
        except Exception as e:
            print(f"[Vision] ⚠️  mss capture failed: {e}, attempting PIL/ImageGrab fallback")

    # 2. Try PIL ImageGrab
    if _PIL:
        try:
            import PIL.ImageGrab
            img = PIL.ImageGrab.grab(all_screens=True)
            buf = io.BytesIO()
            img.save(buf, format="PNG")
            return _compress(buf.getvalue(), "PNG")
        except Exception as e:
            print(f"[Vision] ⚠️  ImageGrab capture failed: {e}")

    # 3. Try PyAutoGUI screenshot
    try:
        import pyautogui
        img = pyautogui.screenshot()
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        return _compress(buf.getvalue(), "PNG")
    except Exception as e:
        print(f"[Vision] ⚠️  pyautogui screenshot failed: {e}")

    # 4. Bulletproof Synthetic Info Frame (prevents crash if desktop station is locked/restricted)
    try:
        from PIL import Image, ImageDraw
        canvas = Image.new("RGB", (1280, 720), color=(24, 28, 36))
        draw = ImageDraw.Draw(canvas)
        active_title = "Desktop"
        open_wins = []
        try:
            import win32gui
            hwnd = win32gui.GetForegroundWindow()
            if hwnd:
                active_title = win32gui.GetWindowText(hwnd).strip() or "Desktop"
            def _cb(h, _):
                if win32gui.IsWindowVisible(h):
                    t = win32gui.GetWindowText(h).strip()
                    if t and len(t) > 1:
                        open_wins.append(t)
            win32gui.EnumWindows(_cb, None)
        except Exception:
            pass

        draw.text((60, 60), f"Active Window: {active_title}", fill=(255, 255, 255))
        draw.text((60, 110), f"Open Applications: {', '.join(open_wins[:8]) if open_wins else 'Desktop'}", fill=(180, 200, 220))
        draw.text((60, 160), "Note: Direct BitBlt screen grab was restricted by system.", fill=(150, 150, 150))
        buf = io.BytesIO()
        canvas.save(buf, format="JPEG", quality=85)
        return buf.getvalue(), "image/jpeg"
    except Exception as e:
        print(f"[Vision] ⚠️  Fallback canvas creation failed: {e}")

    # Ultimate 1x1 black pixel fallback (valid JPEG bytes)
    return (
        b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x01\x00`\x00`\x00\x00\xff\xdb\x00C\x00"
        b"\x08\x06\x06\x07\x06\x05\x08\x07\x07\x07\t\t\x08\n\x0c\x14\r\x0c\x0b\x0b\x0c\x19"
        b"\x12\x13\x0f\x14\x1d\x1a\x1f\x1e\x1d\x1a\x1c\x1c $.' \",#\x1c\x1c(7),01444\x1f'9=82<.342"
        b"\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00\xff\xc4\x00\x1f\x00\x00\x01\x05"
        b"\x01\x01\x01\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07"
        b"\x08\t\n\x0b\xff\xda\x00\x08\x01\x01\x00\x00?\x00\xbf\x00\xff\xd9",
        "image/jpeg",
    )


def _cv2_backend() -> int:
    """Return the best OpenCV camera backend for the current OS."""
    if not _CV2:
        return 0
    os_name = _get_os()
    if os_name == "windows":
        return cv2.CAP_DSHOW
    if os_name == "mac":
        return cv2.CAP_AVFOUNDATION
    return cv2.CAP_ANY


def _probe_camera(index: int, backend: int, warmup: int = 15) -> bool:

    if not _CV2:
        return False
    cap = cv2.VideoCapture(index, backend)
    if not cap.isOpened():
        cap.release()
        return False
    for _ in range(warmup):
        cap.read()
    ret1, frame1 = cap.read()
    ret2, frame2 = cap.read()
    cap.release()
    if not ret1 or frame1 is None:
        return False

    # Check for static virtual camera test card (such as idle OBS Virtual Camera).
    # Virtual cameras output bit-for-bit identical frames between reads.
    # Real physical sensors always show noise, light fluctuations, or motion.
    if ret2 and frame2 is not None:
        diff = int(np.max(np.abs(frame1.astype(int) - frame2.astype(int))))
        if diff == 0 and np.mean(frame1) > 10:
            print(f"[Vision] ⚠️  Camera index {index}: static placeholder detected (e.g. inactive OBS Virtual Camera)")
            return False

    # Check that the sensor has a non-zero signal (not a completely dead 0.0 frame)
    mean_val = float(np.mean(frame1))
    std_val = float(np.std(frame1))
    if mean_val <= 1.0 and std_val < 0.1:
        return False

    return True


def _detect_camera_index() -> int:

    backend = _cv2_backend()
    print("[Vision] 🔍 Auto-detecting camera...")
    for idx in range(6):
        if _probe_camera(idx, backend):
            print(f"[Vision] ✅ Camera found at index {idx}")
            _save_config_key("camera_index", idx)
            return idx
        print(f"[Vision] ⚠️  Camera index {idx}: no usable frame")

    print("[Vision] ⚠️  No camera found — defaulting to index 0")
    _save_config_key("camera_index", 0)
    return 0


def _get_camera_index() -> int:
    cfg = _load_config()
    if "camera_index" in cfg:
        return int(cfg["camera_index"])
    return _detect_camera_index()


def _capture_camera() -> tuple[bytes, str]:
    if not _CV2:
        raise RuntimeError("OpenCV (cv2) is not installed. Run: pip install opencv-python")

    index   = _get_camera_index()
    backend = _cv2_backend()
    cap     = cv2.VideoCapture(index, backend)

    if not cap.isOpened():
        raise RuntimeError(f"Camera index {index} could not be opened.")

    for _ in range(10):
        cap.read()

    ret, frame = cap.read()
    cap.release()

    if not ret or frame is None:
        raise RuntimeError("Camera returned no frame.")

    if _PIL:
        rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        img = PIL.Image.fromarray(rgb)
        img.thumbnail((_IMG_MAX_W, _IMG_MAX_H), PIL.Image.BILINEAR)
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=_JPEG_Q)
        return buf.getvalue(), "image/jpeg"

    _, buf = cv2.imencode(".jpg", frame, [cv2.IMWRITE_JPEG_QUALITY, _JPEG_Q])
    return buf.tobytes(), "image/jpeg"
