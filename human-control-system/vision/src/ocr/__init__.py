"""OCR module with PaddleOCR and Tesseract support."""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import List, Optional, Dict, Any
import numpy as np


@dataclass
class TextBlock:
    """Single text detection/recognition result."""
    text: str
    confidence: float
    bbox: np.ndarray  # [x, y, width, height]
    polygon: Optional[np.ndarray] = None  # [[x1,y1], [x2,y2], ...] for rotated text
    language: str = "en"


@dataclass
class OcrResult:
    """OCR results."""
    text_blocks: List[TextBlock]
    inference_time_ms: float
    image_width: int
    image_height: int


class BaseOcrEngine(ABC):
    """Abstract base class for OCR engines."""

    @abstractmethod
    def recognize(self, image: np.ndarray, region: Optional[Dict[str, int]] = None,
                  languages: Optional[List[str]] = None, confidence_threshold: float = 0.5,
                  detect_orientation: bool = False) -> OcrResult:
        """Run OCR on an image."""
        pass

    @abstractmethod
    def get_supported_languages(self) -> List[str]:
        """Get list of supported language codes."""
        pass

    @abstractmethod
    def get_engine_info(self) -> Dict[str, Any]:
        """Get engine information."""
        pass

    @abstractmethod
    def close(self):
        """Clean up resources."""
        pass


class OcrFactory:
    """Factory for creating OCR engine instances."""

    _instances: Dict[str, BaseOcrEngine] = {}

    @classmethod
    def create(cls, name: str, **kwargs) -> BaseOcrEngine:
        """Create an OCR engine instance."""
        if name in cls._instances:
            return cls._instances[name]

        if name.lower() == "paddle":
            from .paddle import PaddleOcrEngine
            engine = PaddleOcrEngine(**kwargs)
        elif name.lower() == "tesseract":
            from .tesseract import TesseractOcrEngine
            engine = TesseractOcrEngine(**kwargs)
        else:
            raise ValueError(f"Unknown OCR engine: {name}")

        cls._instances[name] = engine
        return engine

    @classmethod
    def get_instance(cls, name: str) -> Optional[BaseOcrEngine]:
        """Get existing engine instance."""
        return cls._instances.get(name)

    @classmethod
    def clear(cls):
        """Clear all instances."""
        for engine in cls._instances.values():
            engine.close()
        cls._instances.clear()