"""Tests for vision service."""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock
import grpc

from src.service.vision_service import VisionService
from src.config import VisionConfig


class TestVisionService:
    @pytest.fixture
    def config(self):
        return VisionConfig()

    @pytest.fixture
    def mock_capture(self):
        with patch('src.service.vision_service.CaptureFactory.create') as mock:
            capture = Mock()
            # Use simple objects with attributes for protobuf compatibility
            monitor = Mock()
            monitor.index = 0
            monitor.name = "Monitor 1"
            monitor.x = 0
            monitor.y = 0
            monitor.width = 1920
            monitor.height = 1080
            monitor.is_primary = True
            monitor.scale_factor = 1.0
            capture.get_monitors.return_value = [monitor]
            # Return proper frame data
            frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
            region_mock = Mock()
            region_mock.is_empty.return_value = True
            capture.capture.return_value = Mock(
                frame=frame,
                timestamp_us=1234567890,
                monitor_index=0,
                region=region_mock
            )
            mock.return_value = capture
            yield capture

    @pytest.fixture
    def mock_detector(self):
        with patch('src.service.vision_service.DetectorFactory.create') as mock:
            detector = Mock()
            detection_mock = Mock(
                class_id=0,
                class_name="person",
                confidence=0.95,
                bbox=np.array([100, 100, 200, 300], dtype=np.float32),
                attributes={}
            )
            detector.detect.return_value = Mock(
                detections=[detection_mock],
                inference_time_ms=15.5,
                image_width=640,
                image_height=480
            )
            detector.detect_batch.return_value = [
                Mock(
                    detections=[],
                    inference_time_ms=10.0,
                    image_width=640,
                    image_height=480
                )
            ]
            detector.get_model_info.return_value = {
                "name": "YOLOv8",
                "model_path": "models/yolov8n.onnx",
                "device": "cpu",
                "input_shape": (1, 3, 640, 640),
                "output_shapes": [(1, 84, 8400)],
                "class_names": ["person"] * 80,
                "num_classes": 80
            }
            mock.return_value = detector
            yield detector

    @pytest.fixture
    def mock_ocr(self):
        with patch('src.service.vision_service.OcrFactory.create') as mock:
            ocr = Mock()
            block_mock = Mock(
                text="Hello",
                confidence=0.9,
                bbox=np.array([10, 20, 50, 30], dtype=np.float32),
                polygon=np.array([[10, 20], [60, 20], [60, 50], [10, 50]], dtype=np.float32),
                language="en"
            )
            ocr.recognize.return_value = Mock(
                text_blocks=[block_mock],
                inference_time_ms=25.0,
                image_width=640,
                image_height=480
            )
            ocr.get_engine_info.return_value = {
                "name": "PaddleOCR",
                "version": "2.7",
                "use_gpu": True,
                "languages": ["en"]
            }
            mock.return_value = ocr
            yield ocr

    def test_init(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)
        assert service._capture is not None
        assert service._detector is not None
        assert service._ocr_engine is not None

    def test_capture_screen(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        request = Mock()
        request.monitor_index = 0
        request.region = Mock(width=0, height=0, is_empty=Mock(return_value=True))
        request.format = 1  # BGR
        request.include_cursor = False
        request.window_title = ""

        context = Mock()

        response = service.CaptureScreen(request, context)

        assert response.width == 1920
        assert response.height == 1080
        assert response.monitor_index == 0

    def test_list_monitors(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        request = Mock()
        context = Mock()

        response = service.ListMonitors(request, context)

        assert len(response.monitors) == 1
        assert response.monitors[0].name == "Monitor 1"

    def test_detect_objects(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        # Create a proper image that can be decoded
        image = np.zeros((480, 640, 3), dtype=np.uint8)
        import cv2
        _, encoded = cv2.imencode('.jpg', image)
        image_data = encoded.tobytes()

        request = Mock()
        request.image_data = image_data
        request.width = 640
        request.height = 480
        request.format = 5  # JPEG
        request.confidence_threshold = 0.5
        request.iou_threshold = 0.45
        request.class_filter = []
        request.max_detections = 100

        context = Mock()

        response = service.DetectObjects(request, context)

        assert len(response.detections) == 1
        assert response.detections[0].class_name == "person"
        assert response.detections[0].confidence == pytest.approx(0.95, rel=1e-2)

    def test_batch_detect_objects(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        image = np.zeros((480, 640, 3), dtype=np.uint8)
        import cv2
        _, encoded = cv2.imencode('.jpg', image)
        image_data = encoded.tobytes()

        request = Mock()
        request.requests = [
            Mock(image_data=image_data, width=640, height=480, format=5,
                 confidence_threshold=0.5, iou_threshold=0.45, class_filter=[], max_detections=100)
        ]

        context = Mock()

        response = service.BatchDetectObjects(request, context)

        assert len(response.responses) == 1

    def test_recognize_text(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        image = np.zeros((480, 640, 3), dtype=np.uint8)
        import cv2
        _, encoded = cv2.imencode('.jpg', image)
        image_data = encoded.tobytes()

        request = Mock()
        request.image_data = image_data
        request.width = 640
        request.height = 480
        request.format = 5  # JPEG
        request.region = Mock(width=0, height=0, is_empty=Mock(return_value=True))
        request.languages = ["en"]
        request.detect_orientation = False
        request.confidence_threshold = 0.5

        context = Mock()

        response = service.RecognizeText(request, context)

        assert len(response.text_blocks) == 1
        assert response.text_blocks[0].text == "Hello"

    def test_get_model_info(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        request = Mock()
        context = Mock()

        response = service.GetModelInfo(request, context)

        assert len(response.models) >= 1

    def test_health_check(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        request = Mock()
        context = Mock()

        response = service.HealthCheck(request, context)

        assert response.status == 1  # SERVING
        assert response.version == "1.0.0"
        assert "capture" in response.components
        assert "detection" in response.components
        assert "ocr" in response.components

    def test_get_config(self, config, mock_capture, mock_detector, mock_ocr):
        service = VisionService(config)

        request = Mock()
        context = Mock()

        response = service.GetConfig(request, context)

        assert response.capture.default_fps == 30
        assert response.detection.model_path == "models/yolov8n.onnx"
        assert response.ocr.engine == "paddle"

    def test_health_check_degraded_when_detector_missing(self, config, mock_capture, mock_ocr):
        with patch('src.service.vision_service.DetectorFactory.create', side_effect=Exception("No model")):
            service = VisionService(config)

            request = Mock()
            context = Mock()

            response = service.HealthCheck(request, context)

            # Should still be serving but detection degraded
            assert response.status == 1  # SERVING
            assert response.components["detection"].status == 2  # DEGRADED

    def test_health_check_not_serving_when_capture_missing(self, config):
        # Need to patch before creating service, and not use the fixture mocks
        with patch('src.service.vision_service.CaptureFactory.create', side_effect=Exception("No capture")):
            with patch('src.service.vision_service.DetectorFactory.create') as mock_detector:
                with patch('src.service.vision_service.OcrFactory.create') as mock_ocr:
                    mock_detector.return_value = Mock()
                    mock_ocr.return_value = Mock()
                    # Service creation should raise
                    with pytest.raises(Exception, match="No capture"):
                        VisionService(config)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])