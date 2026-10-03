"""Configuration loading for vision service."""

import os
from dataclasses import dataclass, field
from typing import List, Optional
import tomli
import tomli_w


@dataclass
class ServiceConfig:
    host: str = "0.0.0.0"
    port: int = 50051
    workers: int = 4
    log_level: str = "info"


@dataclass
class CaptureConfig:
    default_fps: int = 30
    default_monitor: int = 0
    jpeg_quality: int = 85
    use_hardware_acceleration: bool = True
    backend: str = "auto"


@dataclass
class DetectionConfig:
    model_path: str = "models/yolov8n.onnx"
    device: str = "tensorrt"
    batch_size: int = 1
    default_confidence: float = 0.5
    default_iou: float = 0.45
    input_width: int = 640
    input_height: int = 640
    use_tensorrt: bool = True
    tensorrt_cache_dir: str = "models/tensorrt_cache"
    class_names: List[str] = field(default_factory=lambda: [
        "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck",
        "boat", "traffic light", "fire hydrant", "stop sign", "parking meter", "bench",
        "bird", "cat", "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra",
        "giraffe", "backpack", "umbrella", "handbag", "tie", "suitcase", "frisbee",
        "skis", "snowboard", "sports ball", "kite", "baseball bat", "baseball glove",
        "skateboard", "surfboard", "tennis racket", "bottle", "wine glass", "cup",
        "fork", "knife", "spoon", "bowl", "banana", "apple", "sandwich", "orange",
        "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair", "couch",
        "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
        "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink",
        "refrigerator", "book", "clock", "vase", "scissors", "teddy bear", "hair drier",
        "toothbrush"
    ])


@dataclass
class OcrConfig:
    engine: str = "paddle"
    languages: List[str] = field(default_factory=lambda: ["en"])
    use_gpu: bool = True
    gpu_id: int = 0
    cpu_threads: int = 4
    enable_mkldnn: bool = True
    paddle_det_model_dir: str = "models/paddleocr/det"
    paddle_rec_model_dir: str = "models/paddleocr/rec"
    paddle_cls_model_dir: str = "models/paddleocr/cls"
    tesseract_data_path: str = ""
    tesseract_config: str = "--oem 3 --psm 6"


@dataclass
class ModelsConfig:
    model_dir: str = "models"
    auto_download: bool = True


@dataclass
class MetricsConfig:
    enabled: bool = True
    port: int = 9090
    path: str = "/metrics"


@dataclass
class LoggingConfig:
    format: str = "json"
    level: str = "info"
    output: str = "stdout"


@dataclass
class VisionConfig:
    service: ServiceConfig = field(default_factory=ServiceConfig)
    capture: CaptureConfig = field(default_factory=CaptureConfig)
    detection: DetectionConfig = field(default_factory=DetectionConfig)
    ocr: OcrConfig = field(default_factory=OcrConfig)
    models: ModelsConfig = field(default_factory=ModelsConfig)
    metrics: MetricsConfig = field(default_factory=MetricsConfig)
    logging: LoggingConfig = field(default_factory=LoggingConfig)


def load_config(config_path: Optional[str] = None) -> VisionConfig:
    """Load configuration from TOML file."""
    config = VisionConfig()

    if config_path is None:
        # Default locations
        default_paths = [
            "config/vision.toml",
            "vision/config/vision.toml",
            "/etc/hcs/vision.toml",
            os.path.expanduser("~/.config/hcs/vision.toml")
        ]
        for path in default_paths:
            if os.path.exists(path):
                config_path = path
                break

    if config_path and os.path.exists(config_path):
        with open(config_path, "rb") as f:
            data = tomli.load(f)
        _apply_config(config, data)

    # Override with environment variables
    _apply_env_overrides(config)

    return config


def _apply_config(config: VisionConfig, data: dict):
    """Apply configuration from dict."""
    if "service" in data:
        for k, v in data["service"].items():
            if hasattr(config.service, k):
                setattr(config.service, k, v)

    if "capture" in data:
        for k, v in data["capture"].items():
            if hasattr(config.capture, k):
                setattr(config.capture, k, v)

    if "detection" in data:
        for k, v in data["detection"].items():
            if hasattr(config.detection, k):
                setattr(config.detection, k, v)

    if "ocr" in data:
        for k, v in data["ocr"].items():
            if hasattr(config.ocr, k):
                setattr(config.ocr, k, v)

    if "models" in data:
        for k, v in data["models"].items():
            if hasattr(config.models, k):
                setattr(config.models, k, v)

    if "metrics" in data:
        for k, v in data["metrics"].items():
            if hasattr(config.metrics, k):
                setattr(config.metrics, k, v)

    if "logging" in data:
        for k, v in data["logging"].items():
            if hasattr(config.logging, k):
                setattr(config.logging, k, v)


def _apply_env_overrides(config: VisionConfig):
    """Apply environment variable overrides."""
    env_mappings = {
        "VISION_HOST": ("service", "host"),
        "VISION_PORT": ("service", "port", int),
        "VISION_WORKERS": ("service", "workers", int),
        "VISION_LOG_LEVEL": ("service", "log_level"),
        "VISION_CAPTURE_BACKEND": ("capture", "backend"),
        "VISION_CAPTURE_FPS": ("capture", "default_fps", int),
        "VISION_DETECTION_MODEL": ("detection", "model_path"),
        "VISION_DETECTION_DEVICE": ("detection", "device"),
        "VISION_DETECTION_BATCH_SIZE": ("detection", "batch_size", int),
        "VISION_OCR_ENGINE": ("ocr", "engine"),
        "VISION_OCR_LANGUAGES": ("ocr", "languages", lambda x: x.split(",")),
    }

    for env_var, mapping in env_mappings.items():
        value = os.environ.get(env_var)
        if value is not None:
            section, key = mapping[0], mapping[1]
            converter = mapping[2] if len(mapping) > 2 else str
            try:
                converted = converter(value)
                section_obj = getattr(config, section)
                if hasattr(section_obj, key):
                    setattr(section_obj, key, converted)
            except (ValueError, TypeError):
                pass


def save_config(config: VisionConfig, config_path: str):
    """Save configuration to TOML file."""
    data = {
        "service": {
            "host": config.service.host,
            "port": config.service.port,
            "workers": config.service.workers,
            "log_level": config.service.log_level,
        },
        "capture": {
            "default_fps": config.capture.default_fps,
            "default_monitor": config.capture.default_monitor,
            "jpeg_quality": config.capture.jpeg_quality,
            "use_hardware_acceleration": config.capture.use_hardware_acceleration,
            "backend": config.capture.backend,
        },
        "detection": {
            "model_path": config.detection.model_path,
            "device": config.detection.device,
            "batch_size": config.detection.batch_size,
            "default_confidence": config.detection.default_confidence,
            "default_iou": config.detection.default_iou,
            "input_width": config.detection.input_width,
            "input_height": config.detection.input_height,
            "use_tensorrt": config.detection.use_tensorrt,
            "tensorrt_cache_dir": config.detection.tensorrt_cache_dir,
            "class_names": config.detection.class_names,
        },
        "ocr": {
            "engine": config.ocr.engine,
            "languages": config.ocr.languages,
            "use_gpu": config.ocr.use_gpu,
            "gpu_id": config.ocr.gpu_id,
            "cpu_threads": config.ocr.cpu_threads,
            "enable_mkldnn": config.ocr.enable_mkldnn,
            "paddle_det_model_dir": config.ocr.paddle_det_model_dir,
            "paddle_rec_model_dir": config.ocr.paddle_rec_model_dir,
            "paddle_cls_model_dir": config.ocr.paddle_cls_model_dir,
            "tesseract_data_path": config.ocr.tesseract_data_path,
            "tesseract_config": config.ocr.tesseract_config,
        },
        "models": {
            "model_dir": config.models.model_dir,
            "auto_download": config.models.auto_download,
        },
        "metrics": {
            "enabled": config.metrics.enabled,
            "port": config.metrics.port,
            "path": config.metrics.path,
        },
        "logging": {
            "format": config.logging.format,
            "level": config.logging.level,
            "output": config.logging.output,
        },
    }

    os.makedirs(os.path.dirname(config_path), exist_ok=True)
    with open(config_path, "wb") as f:
        tomli_w.dump(data, f)