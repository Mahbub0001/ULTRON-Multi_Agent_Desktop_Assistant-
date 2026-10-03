"""HCS Vision Package."""

from .config import VisionConfig, load_config, save_config
from .capture import CaptureFactory, MonitorInfo, Region, CaptureResult
from .detection import DetectorFactory, Detection, DetectionResult
from .ocr import OcrFactory, TextBlock, OcrResult

__all__ = [
    "VisionConfig",
    "load_config",
    "save_config",
    "CaptureFactory",
    "MonitorInfo",
    "Region",
    "CaptureResult",
    "DetectorFactory",
    "Detection",
    "DetectionResult",
    "OcrFactory",
    "TextBlock",
    "OcrResult",
]