"""Linux/macOS screen capture using mss."""

import time
import numpy as np
from typing import Optional, List
from . import BaseCapture, MonitorInfo, Region, CaptureResult


class MSSCapture(BaseCapture):
    """Cross-platform capture using mss (works on Linux, macOS, Windows)."""

    def __init__(self):
        import mss
        self._sct = mss.mss()
        self._monitors: List[MonitorInfo] = []
        self._streaming = False
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
        # Could implement a background thread for true streaming

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


# For macOS, we can also use CoreGraphics directly for better performance
# This is a placeholder for future optimization
class MacOSCapture(BaseCapture):
    """macOS-specific capture using CoreGraphics (placeholder for future implementation)."""

    def __init__(self):
        # For now, fall back to MSSCapture
        self._impl = MSSCapture()

    def get_monitors(self) -> List[MonitorInfo]:
        return self._impl.get_monitors()

    def capture(self, monitor_index: int = 0, region: Optional[Region] = None) -> CaptureResult:
        return self._impl.capture(monitor_index, region)

    def start_stream(self, monitor_index: int = 0, region: Optional[Region] = None, target_fps: int = 30):
        self._impl.start_stream(monitor_index, region, target_fps)

    def stop_stream(self):
        self._impl.stop_stream()

    def get_next_frame(self) -> Optional[CaptureResult]:
        return self._impl.get_next_frame()

    def close(self):
        self._impl.close()