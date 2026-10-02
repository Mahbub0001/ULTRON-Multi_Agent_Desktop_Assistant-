"""Object detection module using YOLOv8 with ONNX Runtime."""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import List, Optional, Dict, Any
import numpy as np
import time


@dataclass
class Detection:
    """Single object detection result."""
    class_id: int
    class_name: str
    confidence: float
    bbox: np.ndarray  # [x, y, width, height] in pixels
    attributes: Dict[str, Any] = None

    def __post_init__(self):
        if self.attributes is None:
            self.attributes = {}


@dataclass
class DetectionResult:
    """Object detection results."""
    detections: List[Detection]
    inference_time_ms: float
    image_width: int
    image_height: int


class BaseDetector(ABC):
    """Abstract base class for object detectors."""

    @abstractmethod
    def detect(self, image: np.ndarray, confidence_threshold: float = 0.5,
               iou_threshold: float = 0.45, class_filter: Optional[List[str]] = None,
               max_detections: int = 100) -> DetectionResult:
        """Run object detection on an image."""
        pass

    @abstractmethod
    def detect_batch(self, images: List[np.ndarray], confidence_threshold: float = 0.5,
                     iou_threshold: float = 0.45, class_filter: Optional[List[str]] = None,
                     max_detections: int = 100) -> List[DetectionResult]:
        """Run object detection on a batch of images."""
        pass

    @abstractmethod
    def warmup(self, input_shape: tuple = (1, 3, 640, 640)):
        """Warm up the model with dummy input."""
        pass

    @abstractmethod
    def get_model_info(self) -> Dict[str, Any]:
        """Get model information."""
        pass

    @abstractmethod
    def close(self):
        """Clean up resources."""
        pass


class DetectorFactory:
    """Factory for creating detector instances."""

    _instances: Dict[str, BaseDetector] = {}

    @classmethod
    def create(cls, name: str, model_path: str, device: str = "cpu",
               batch_size: int = 1, input_width: int = 640, input_height: int = 640,
               use_tensorrt: bool = False, tensorrt_cache_dir: str = "",
               class_names: Optional[List[str]] = None, **kwargs) -> BaseDetector:
        """Create a detector instance."""
        key = f"{name}:{model_path}:{device}"

        if key in cls._instances:
            return cls._instances[key]

        if name.lower() == "yolo" or name.lower() == "yolov8":
            from .yolo import YOLOv8Detector
            detector = YOLOv8Detector(
                model_path=model_path,
                device=device,
                batch_size=batch_size,
                input_width=input_width,
                input_height=input_height,
                use_tensorrt=use_tensorrt,
                tensorrt_cache_dir=tensorrt_cache_dir,
                class_names=class_names,
                **kwargs
            )
        else:
            raise ValueError(f"Unknown detector: {name}")

        cls._instances[key] = detector
        return detector

    @classmethod
    def get_instance(cls, name: str, model_path: str, device: str = "cpu") -> Optional[BaseDetector]:
        """Get existing detector instance."""
        key = f"{name}:{model_path}:{device}"
        return cls._instances.get(key)

    @classmethod
    def clear(cls):
        """Clear all instances."""
        for detector in cls._instances.values():
            detector.close()
        cls._instances.clear()