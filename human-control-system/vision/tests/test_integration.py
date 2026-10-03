"""Integration tests for vision service."""

import pytest
import numpy as np
from unittest.mock import Mock, patch
import cv2

from src.config import VisionConfig
from src.service.vision_service import VisionService


class TestIntegration:
    """Integration tests with mocked components."""

    @pytest.fixture
    def config(self):
        return VisionConfig()

    def test_full_capture_detect_pipeline(self, config):
        """Test capture -> detect pipeline."""
        with patch('src.service.vision_service.CaptureFactory.create') as mock_cap_factory:
            with patch('src.service.vision_service.DetectorFactory.create') as mock_det_factory:
                with patch('src.service.vision_service.OcrFactory.create') as mock_ocr_factory:
                    # Setup capture mock
                    mock_capture = Mock()
                    frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
                    region_mock = Mock()
                    region_mock.is_empty.return_value = True
                    mock_capture.capture.return_value = Mock(
                        frame=frame,
                        timestamp_us=1234567890,
                        monitor_index=0,
                        region=region_mock
                    )
                    mock_capture.get_monitors.return_value = [
                        Mock(index=0, name="Monitor 1", x=0, y=0, width=1920, height=1080, is_primary=True, scale_factor=1.0)
                    ]
                    mock_cap_factory.return_value = mock_capture

                    # Setup detector mock
                    detection_mock = Mock(
                        class_id=0,
                        class_name="person",
                        confidence=0.95,
                        bbox=np.array([100, 100, 200, 300], dtype=np.float32),
                        attributes={}
                    )
                    mock_detector = Mock()
                    mock_detector.detect.return_value = Mock(
                        detections=[detection_mock],
                        inference_time_ms=15.5,
                        image_width=1920,
                        image_height=1080
                    )
                    mock_detector.get_model_info.return_value = {
                        "name": "YOLOv8",
                        "model_path": "models/yolov8n.onnx",
                        "device": "cpu",
                        "num_classes": 80
                    }
                    mock_det_factory.return_value = mock_detector

                    # Setup OCR mock
                    mock_ocr = Mock()
                    mock_ocr.recognize.return_value = Mock(
                        text_blocks=[],
                        inference_time_ms=5.0,
                        image_width=1920,
                        image_height=1080
                    )
                    mock_ocr.get_engine_info.return_value = {
                        "name": "PaddleOCR",
                        "version": "2.7"
                    }
                    mock_ocr_factory.return_value = mock_ocr

                    # Create service
                    service = VisionService(config)

                    # Test capture
                    request = Mock()
                    request.monitor_index = 0
                    request.region = Mock(width=0, height=0, is_empty=Mock(return_value=True))
                    request.format = 1
                    context = Mock()

                    cap_response = service.CaptureScreen(request, context)
                    assert cap_response.width == 1920
                    assert cap_response.height == 1080

                    # Test detection
                    _, encoded = cv2.imencode('.jpg', frame)
                    det_request = Mock()
                    det_request.image_data = encoded.tobytes()
                    det_request.width = 1920
                    det_request.height = 1080
                    det_request.format = 5
                    det_request.confidence_threshold = 0.5
                    det_request.iou_threshold = 0.45
                    det_request.class_filter = []
                    det_request.max_detections = 100

                    det_response = service.DetectObjects(det_request, context)
                    assert len(det_response.detections) == 1
                    assert det_response.detections[0].class_name == "person"

    def test_health_check_reports_correct_status(self, config):
        """Test health check reflects component status."""
        with patch('src.service.vision_service.CaptureFactory.create') as mock_cap_factory:
            with patch('src.service.vision_service.DetectorFactory.create') as mock_det_factory:
                with patch('src.service.vision_service.OcrFactory.create') as mock_ocr_factory:
                    # All components working
                    mock_cap_factory.return_value = Mock()
                    mock_det_factory.return_value = Mock(get_model_info=Mock(return_value={}))
                    mock_ocr_factory.return_value = Mock(get_engine_info=Mock(return_value={}))

                    service = VisionService(config)
                    response = service.HealthCheck(Mock(), Mock())

                    assert response.status == 1  # SERVING
                    assert all(c.status == 1 for c in response.components.values())


if __name__ == "__main__":
    pytest.main([__file__, "-v"])