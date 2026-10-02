"""Base capture interface and factory."""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional, List
import numpy as np


@dataclass
class MonitorInfo:
    """Information about a display monitor."""
    index: int
    name: str
    x: int
    y: int
    width: int
    height: int
    is_primary: bool
    scale_factor: float = 1.0


@dataclass
class Region:
    """Screen region to capture."""
    x: int = 0
    y: int = 0
    width: int = 0
    height: int = 0

    def is_empty(self) -> bool:
        return self.width <= 0 or self.height <= 0


@dataclass
class CaptureResult:
    """Result of a screen capture operation."""
    frame: np.ndarray  # BGR format by default
    timestamp_us: int
    monitor_index: int
    region: Region


class BaseCapture(ABC):
    """Abstract base class for screen capture implementations."""

    @abstractmethod
    def get_monitors(self) -> List[MonitorInfo]:
        """Get list of available monitors."""
        pass

    @abstractmethod
    def capture(self, monitor_index: int = 0, region: Optional[Region] = None) -> CaptureResult:
        """Capture a single frame from the specified monitor/region."""
        pass

    @abstractmethod
    def start_stream(self, monitor_index: int = 0, region: Optional[Region] = None, target_fps: int = 30):
        """Start continuous frame capture stream."""
        pass

    @abstractmethod
    def stop_stream(self):
        """Stop the capture stream."""
        pass

    @abstractmethod
    def get_next_frame(self) -> Optional[CaptureResult]:
        """Get the next frame from the stream (blocking)."""
        pass

    @abstractmethod
    def close(self):
        """Clean up resources."""
        pass


class CaptureFactory:
    """Factory for creating platform-appropriate capture implementation."""

    _instance: Optional[BaseCapture] = None
    _backend: Optional[str] = None

    @classmethod
    def create(cls, backend: str = "auto") -> BaseCapture:
        """Create a capture instance for the current platform."""
        import sys

        if cls._instance is not None and cls._backend == backend:
            return cls._instance

        if backend == "auto":
            if sys.platform == "win32":
                backend = "dxcam"
            elif sys.platform == "linux":
                backend = "mss"
            elif sys.platform == "darwin":
                backend = "mss"
            else:
                backend = "mss"

        cls._backend = backend

        if backend == "dxcam" and sys.platform == "win32":
            from .windows import WindowsCapture
            cls._instance = WindowsCapture()
        elif backend == "mss":
            from .linux import MSSCapture
            cls._instance = MSSCapture()
        elif backend == "gdi" and sys.platform == "win32":
            from .windows import GDICapture
            cls._instance = GDICapture()
        else:
            from .linux import MSSCapture
            cls._instance = MSSCapture()

        return cls._instance

    @classmethod
    def get_instance(cls) -> Optional[BaseCapture]:
        """Get the current capture instance."""
        return cls._instance

    @classmethod
    def reset(cls):
        """Reset the factory (for testing)."""
        if cls._instance:
            cls._instance.close()
        cls._instance = None
        cls._backend = None