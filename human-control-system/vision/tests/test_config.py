"""Tests for config module."""

import pytest
import tempfile
import os
from pathlib import Path

from src.config import (
    VisionConfig, ServiceConfig, CaptureConfig, DetectionConfig,
    OcrConfig, ModelsConfig, MetricsConfig, LoggingConfig,
    load_config, save_config
)


class TestConfigDataclasses:
    def test_service_config_defaults(self):
        config = ServiceConfig()
        assert config.host == "0.0.0.0"
        assert config.port == 50051
        assert config.workers == 4
        assert config.log_level == "info"

    def test_capture_config_defaults(self):
        config = CaptureConfig()
        assert config.default_fps == 30
        assert config.default_monitor == 0
        assert config.jpeg_quality == 85
        assert config.backend == "auto"

    def test_detection_config_defaults(self):
        config = DetectionConfig()
        assert config.model_path == "models/yolov8n.onnx"
        assert config.device == "tensorrt"
        assert config.batch_size == 1
        assert config.default_confidence == 0.5
        assert len(config.class_names) == 80

    def test_ocr_config_defaults(self):
        config = OcrConfig()
        assert config.engine == "paddle"
        assert config.languages == ["en"]
        assert config.use_gpu is True

    def test_vision_config_defaults(self):
        config = VisionConfig()
        assert isinstance(config.service, ServiceConfig)
        assert isinstance(config.capture, CaptureConfig)
        assert isinstance(config.detection, DetectionConfig)
        assert isinstance(config.ocr, OcrConfig)


class TestLoadConfig:
    def test_load_from_file(self):
        toml_content = """
[service]
host = "127.0.0.1"
port = 50052
workers = 2
log_level = "debug"

[capture]
default_fps = 60
backend = "dxcam"

[detection]
model_path = "custom/yolo.onnx"
device = "cuda"
batch_size = 4

[ocr]
engine = "tesseract"
languages = ["en", "fr"]
"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
            f.write(toml_content)
            temp_path = f.name

        try:
            config = load_config(temp_path)
            assert config.service.host == "127.0.0.1"
            assert config.service.port == 50052
            assert config.service.workers == 2
            assert config.service.log_level == "debug"
            assert config.capture.default_fps == 60
            assert config.capture.backend == "dxcam"
            assert config.detection.model_path == "custom/yolo.onnx"
            assert config.detection.device == "cuda"
            assert config.detection.batch_size == 4
            assert config.ocr.engine == "tesseract"
            assert config.ocr.languages == ["en", "fr"]
        finally:
            os.unlink(temp_path)

    def test_load_missing_file_uses_defaults(self):
        config = load_config("/nonexistent/path.toml")
        assert config.service.port == 50051
        assert config.capture.default_fps == 30


class TestSaveConfig:
    def test_save_and_load_roundtrip(self):
        config = VisionConfig()
        config.service.host = "127.0.0.1"
        config.service.port = 50052
        config.capture.default_fps = 60
        config.detection.model_path = "custom.onnx"
        config.ocr.engine = "tesseract"

        with tempfile.NamedTemporaryFile(suffix='.toml', delete=False) as f:
            temp_path = f.name

        try:
            save_config(config, temp_path)
            loaded = load_config(temp_path)

            assert loaded.service.host == "127.0.0.1"
            assert loaded.service.port == 50052
            assert loaded.capture.default_fps == 60
            assert loaded.detection.model_path == "custom.onnx"
            assert loaded.ocr.engine == "tesseract"
        finally:
            os.unlink(temp_path)


class TestEnvOverrides:
    def test_env_override_port(self, monkeypatch):
        monkeypatch.setenv("VISION_PORT", "50053")
        config = load_config(None)
        assert config.service.port == 50053

    def test_env_override_detection_model(self, monkeypatch):
        monkeypatch.setenv("VISION_DETECTION_MODEL", "env_model.onnx")
        config = load_config(None)
        assert config.detection.model_path == "env_model.onnx"

    def test_env_override_ocr_languages(self, monkeypatch):
        monkeypatch.setenv("VISION_OCR_LANGUAGES", "en,fr,de")
        config = load_config(None)
        assert config.ocr.languages == ["en", "fr", "de"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])