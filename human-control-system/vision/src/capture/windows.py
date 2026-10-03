"""Windows screen capture implementations."""

import time
import numpy as np
from typing import Optional, List
from . import BaseCapture, MonitorInfo, Region, CaptureResult, CaptureFactory


class WindowsCapture(BaseCapture):
    """High-performance Windows capture using DXGI/Duplication API via dxcam."""

    def __init__(self):
        self._camera = None
        self._monitors: List[MonitorInfo] = []
        self._streaming = False
        self._target_fps = 30
        self._initialize()

    def _initialize(self):
        """Initialize dxcam camera."""
        try:
            import dxcam
            self._camera = dxcam.create()
            self._refresh_monitors()
        except ImportError:
            raise RuntimeError("dxcam not available. Install with: pip install dxcam")
        except Exception as e:
            raise RuntimeError(f"Failed to initialize dxcam: {e}")

    def _refresh_monitors(self):
        """Refresh monitor information."""
        if not self._camera:
            return

        try:
            monitors = self._camera.get_monitor_list()
            self._monitors = []
            for i, mon in enumerate(monitors):
                # dxcam returns (left, top, right, bottom)
                left, top, right, bottom = mon
                self._monitors.append(MonitorInfo(
                    index=i,
                    name=f"Monitor {i}",
                    x=left,
                    y=top,
                    width=right - left,
                    height=bottom - top,
                    is_primary=(i == 0)
                ))
        except Exception:
            # Fallback to primary monitor
            self._monitors = [MonitorInfo(
                index=0,
                name="Primary",
                x=0, y=0,
                width=1920, height=1080,
                is_primary=True
            )]

    def get_monitors(self) -> List[MonitorInfo]:
        return self._monitors.copy()

    def capture(self, monitor_index: int = 0, region: Optional[Region] = None) -> CaptureResult:
        if not self._camera:
            raise RuntimeError("Camera not initialized")

        # Switch to target monitor if needed
        if monitor_index < len(self._monitors):
            self._camera.set_monitor(monitor_index)

        # Capture frame
        frame = self._camera.grab(region=(region.x, region.y, region.x + region.width, region.y + region.height) if region and not region.is_empty() else None)

        if frame is None:
            raise RuntimeError("Failed to capture frame")

        timestamp_us = int(time.time() * 1_000_000)

        return CaptureResult(
            frame=frame,
            timestamp_us=timestamp_us,
            monitor_index=monitor_index,
            region=region or Region()
        )

    def start_stream(self, monitor_index: int = 0, region: Optional[Region] = None, target_fps: int = 30):
        if not self._camera:
            raise RuntimeError("Camera not initialized")

        if monitor_index < len(self._monitors):
            self._camera.set_monitor(monitor_index)

        self._target_fps = target_fps
        region_tuple = (region.x, region.y, region.x + region.width, region.y + region.height) if region and not region.is_empty() else None
        self._camera.start(target_fps=target_fps, region=region_tuple)
        self._streaming = True

    def stop_stream(self):
        if self._camera and self._streaming:
            self._camera.stop()
            self._streaming = False

    def get_next_frame(self) -> Optional[CaptureResult]:
        if not self._camera or not self._streaming:
            return None

        frame = self._camera.get_latest_frame()
        if frame is None:
            return None

        timestamp_us = int(time.time() * 1_000_000)
        return CaptureResult(
            frame=frame,
            timestamp_us=timestamp_us,
            monitor_index=0,  # dxcam doesn't expose current monitor in stream mode
            region=Region()
        )

    def close(self):
        self.stop_stream()
        if self._camera:
            try:
                self._camera.release()
            except Exception:
                pass
            self._camera = None


class GDICapture(BaseCapture):
    """Fallback Windows capture using GDI (slower but more compatible)."""

    def __init__(self):
        self._monitors: List[MonitorInfo] = []
        self._streaming = False
        import mss
        self._sct = mss.mss()
        self._refresh_monitors()

    def _refresh_monitors(self):
        monitors = self._sct.monitors
        self._monitors = []
        # mss.monitors[0] is the "all monitors" combined view
        for i, mon in enumerate(monitors[1:], 1):
            self._monitors.append(MonitorInfo(
                index=i - 1,
                name=f"Monitor {i}",
                x=mon["left"],
                y=mon["top"],
                width=mon["width"],
                height=mon["height"],
                is_primary=(i == 1)
            ))

    def get_monitors(self) -> List[MonitorInfo]:
        return self._monitors.copy()

    def capture(self, monitor_index: int = 0, region: Optional[Region] = None) -> CaptureResult:
        if monitor_index >= len(self._monitors):
            monitor_index = 0

        mon = self._sct.monitors[monitor_index + 1]  # +1 because [0] is all monitors

        capture_region = {
            "left": mon["left"] + (region.x if region else 0),
            "top": mon["top"] + (region.y if region else 0),
            "width": region.width if region and not region.is_empty() else mon["width"],
            "height": region.height if region and not region.is_empty() else mon["height"],
        }

        screenshot = self._sct.grab(capture_region)
        frame = np.array(screenshot)
        # mss returns BGRA, convert to BGR
        if frame.shape[2] == 4:
            frame = frame[:, :, :3]

        timestamp_us = int(time.time() * 1_000_000)

        return CaptureResult(
            frame=frame,
            timestamp_us=timestamp_us,
            monitor_index=monitor_index,
            region=region or Region()
        )

    def start_stream(self, monitor_index: int = 0, region: Optional[Region] = None, target_fps: int = 30):
        self._streaming = True
        # For mss, we just capture on demand in get_next_frame

    def stop_stream(self):
        self._streaming = False

    def get_next_frame(self) -> Optional[CaptureResult]:
        if not self._streaming:
            return None
        return self.capture()

    def close(self):
        self.stop_stream()
        if self._sct:
            self._sct.close()
            self._sct = None