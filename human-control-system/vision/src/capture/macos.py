"""macOS screen capture implementations."""

from .linux import MSSCapture, MacOSCapture

# Re-export for factory
__all__ = ["MSSCapture", "MacOSCapture"]