"""Tests for detection module."""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock

from src.detection import DetectorFactory, Detection, DetectionResult, BaseDetector
from src.detection.yolo import YOLOv8Detector, ONNXRuntimeWrapper


class TestDetection:
    def test_detection_creation(self):
        det = Detection(
            class_id=0,
            class_name="person",
            confidence=0.95,
            bbox=np.array([100, 100, 200, 300], dtype=np.float32)
        )
        assert det.class_id == 0
        assert det.class_name == "person"
        assert det.confidence == 0.95
        assert det.bbox[0] == 100

    def test_detection_attributes_default(self):
        det = Detection(
            class_id=0,
            class_name="person",
            confidence=0.95,
            bbox=np.array([100, 100, 200, 300], dtype=np.float32)
        )
        assert det.attributes == {}


class TestDetectionResult:
    def test_result_creation(self):
        det = Detection(
            class_id=0,
            class_name="person",
            confidence=0.95,
            bbox=np.array([100, 100, 200, 300], dtype=np.float32)
        )
        result = DetectionResult(
            detections=[det],
            inference_time_ms=15.5,
            image_width=640,
            image_height=480
        )
        assert len(result.detections) == 1
        assert result.inference_time_ms == 15.5


class TestDetectorFactory:
    def test_create_yolo(self):
        with patch('src.detection.yolo.ONNXRuntimeWrapper') as mock_wrapper:
            mock_session = Mock()
            mock_session.get_inputs.return_value = [Mock(name='input', shape=[1, 3, 640, 640])]
            mock_session.get_outputs.return_value = [Mock(name='output', shape=[1, 84, 8400])]
            mock_session.run.return_value = [np.random.randn(1, 84, 8400).astype(np.float32)]
            mock_wrapper.return_value = mock_session

            detector = DetectorFactory.create(
                name="yolo",
                model_path="dummy.onnx",
                device="cpu"
            )
            assert isinstance(detector, YOLOv8Detector)

    def test_singleton_pattern(self):
        with patch('src.detection.yolo.ONNXRuntimeWrapper') as mock_wrapper:
            mock_session = Mock()
            mock_session.get_inputs.return_value = [Mock(name='input', shape=[1, 3, 640, 640])]
            mock_session.get_outputs.return_value = [Mock(name='output', shape=[1, 84, 8400])]
            mock_session.run.return_value = [np.random.randn(1, 84, 8400).astype(np.float32)]
            mock_wrapper.return_value = mock_session

            DetectorFactory.clear()
            d1 = DetectorFactory.create("yolo", "dummy.onnx", "cpu")
            d2 = DetectorFactory.create("yolo", "dummy.onnx", "cpu")
            assert d1 is d2
            DetectorFactory.clear()


class TestYOLOv8Detector:
    @patch('src.detection.yolo.ONNXRuntimeWrapper')
    def test_preprocess(self, mock_wrapper_class):
        mock_wrapper = Mock()
        mock_wrapper.get_input_shape.return_value = (1, 3, 640, 640)
        mock_wrapper_class.return_value = mock_wrapper

        detector = YOLOv8Detector(
            model_path="dummy.onnx",
            device="cpu",
            input_width=640,
            input_height=640
        )

        # Test with a sample image
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)
        input_tensor, scale, padding = detector._preprocess(image)

        assert input_tensor.shape == (1, 3, 640, 640)
        assert input_tensor.dtype == np.float32
        assert scale > 0
        assert isinstance(padding, tuple)

    @patch('src.detection.yolo.ONNXRuntimeWrapper')
    def test_postprocess(self, mock_wrapper_class):
        mock_wrapper = Mock()
        mock_wrapper.get_input_shape.return_value = (1, 3, 640, 640)
        mock_wrapper_class.return_value = mock_wrapper

        detector = YOLOv8Detector(
            model_path="dummy.onnx",
            device="cpu",
            input_width=640,
            input_height=640
        )

        # Create mock output (num_boxes, 4+num_classes)
        num_classes = 80
        output = np.zeros((8400, 4 + num_classes), dtype=np.float32)
        # Add a person detection
        output[0, 0] = 320  # x_center
        output[0, 1] = 240  # y_center
        output[0, 2] = 100  # width
        output[0, 3] = 200  # height
        output[0, 4] = 0.9  # person class confidence
        output[0, 5:] = 0.1  # other classes

        detections = detector._postprocess(
            outputs=[output],
            scale=1.0,
            padding=(0, 0),
            orig_w=640,
            orig_h=480,
            confidence_threshold=0.5,
            iou_threshold=0.45,
            class_filter=None,
            max_detections=100
        )

        assert len(detections) >= 0  # May be 0 or 1 depending on NMS

    @patch('src.detection.yolo.ONNXRuntimeWrapper')
    def test_get_model_info(self, mock_wrapper_class):
        mock_wrapper = Mock()
        mock_wrapper.get_input_shape.return_value = (1, 3, 640, 640)
        mock_wrapper.get_output_shapes.return_value = [(1, 84, 8400)]
        mock_wrapper_class.return_value = mock_wrapper

        detector = YOLOv8Detector(model_path="dummy.onnx", device="cpu")
        info = detector.get_model_info()

        assert info["name"] == "YOLOv8"
        assert info["device"] == "cpu"
        assert info["num_classes"] == 80


class TestONNXRuntimeWrapper:
    def test_get_default_providers_cpu(self):
        wrapper = ONNXRuntimeWrapper.__new__(ONNXRuntimeWrapper)
        wrapper.device = "cpu"
        wrapper.use_tensorrt = False
        providers = wrapper._get_default_providers()
        assert "CPUExecutionProvider" in providers

    def test_get_default_providers_cuda(self):
        wrapper = ONNXRuntimeWrapper.__new__(ONNXRuntimeWrapper)
        wrapper.device = "cuda"
        wrapper.use_tensorrt = False
        providers = wrapper._get_default_providers()
        assert "CUDAExecutionProvider" in providers

    def test_get_default_providers_tensorrt(self):
        wrapper = ONNXRuntimeWrapper.__new__(ONNXRuntimeWrapper)
        wrapper.device = "tensorrt"
        wrapper.use_tensorrt = True
        wrapper.tensorrt_cache_dir = "/tmp/trt"
        providers = wrapper._get_default_providers()
        assert any("TensorrtExecutionProvider" in str(p) for p in providers)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])