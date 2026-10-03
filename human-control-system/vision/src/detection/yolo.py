"""ONNX Runtime wrapper for YOLOv8 inference."""

import os
import time
import numpy as np
from typing import List, Optional, Dict, Any, Tuple
import onnxruntime as ort

from . import BaseDetector, Detection, DetectionResult, DetectorFactory


class ONNXRuntimeWrapper:
    """Wrapper for ONNX Runtime session with TensorRT support."""

    def __init__(self, model_path: str, device: str = "cpu",
                 use_tensorrt: bool = False, tensorrt_cache_dir: str = "",
                 providers: Optional[List[str]] = None):
        self.model_path = model_path
        self.device = device
        self.use_tensorrt = use_tensorrt
        self.tensorrt_cache_dir = tensorrt_cache_dir
        self.session: Optional[ort.InferenceSession] = None
        self.input_name: Optional[str] = None
        self.output_names: List[str] = []
        self._providers = providers or self._get_default_providers()
        self._initialize_session()

    def _get_default_providers(self) -> List[str]:
        """Get default execution providers based on device."""
        if self.device == "tensorrt" and self.use_tensorrt:
            return [
                ("TensorrtExecutionProvider", {
                    "trt_engine_cache_enable": True,
                    "trt_engine_cache_path": self.tensorrt_cache_dir or "./tensorrt_cache",
                    "trt_fp16_enable": True,
                }),
                "CUDAExecutionProvider",
                "CPUExecutionProvider",
            ]
        elif self.device == "cuda":
            return ["CUDAExecutionProvider", "CPUExecutionProvider"]
        elif self.device == "directml":
            return ["DmlExecutionProvider", "CPUExecutionProvider"]
        elif self.device == "coreml":
            return ["CoreMLExecutionProvider", "CPUExecutionProvider"]
        else:
            return ["CPUExecutionProvider"]

    def _initialize_session(self):
        """Initialize ONNX Runtime session."""
        if not os.path.exists(self.model_path):
            raise FileNotFoundError(f"Model not found: {self.model_path}")

        sess_options = ort.SessionOptions()
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        sess_options.enable_cpu_mem_arena = True
        sess_options.enable_mem_pattern = True

        self.session = ort.InferenceSession(
            self.model_path,
            sess_options=sess_options,
            providers=self._providers
        )

        self.input_name = self.session.get_inputs()[0].name
        self.output_names = [out.name for out in self.session.get_outputs()]

    def infer(self, input_data: np.ndarray) -> List[np.ndarray]:
        """Run inference."""
        if self.session is None:
            raise RuntimeError("Session not initialized")

        return self.session.run(self.output_names, {self.input_name: input_data})

    def get_input_shape(self) -> Tuple[int, ...]:
        """Get model input shape."""
        if self.session is None:
            return (1, 3, 640, 640)
        return tuple(self.session.get_inputs()[0].shape)

    def get_output_shapes(self) -> List[Tuple[int, ...]]:
        """Get model output shapes."""
        if self.session is None:
            return []
        return [tuple(out.shape) for out in self.session.get_outputs()]

    def close(self):
        """Clean up."""
        self.session = None


class YOLOv8Detector(BaseDetector):
    """YOLOv8 object detector using ONNX Runtime."""

    def __init__(self, model_path: str, device: str = "cpu",
                 batch_size: int = 1, input_width: int = 640, input_height: int = 640,
                 use_tensorrt: bool = False, tensorrt_cache_dir: str = "",
                 class_names: Optional[List[str]] = None, **kwargs):
        self.model_path = model_path
        self.device = device
        self.batch_size = batch_size
        self.input_width = input_width
        self.input_height = input_height
        self.class_names = class_names or self._get_default_class_names()

        self._ort = ONNXRuntimeWrapper(
            model_path=model_path,
            device=device,
            use_tensorrt=use_tensorrt,
            tensorrt_cache_dir=tensorrt_cache_dir
        )

        # Warm up
        self.warmup()

    def _get_default_class_names(self) -> List[str]:
        """Get default COCO class names."""
        return [
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
        ]

    def _preprocess(self, image: np.ndarray) -> np.ndarray:
        """Preprocess image for YOLOv8 input."""
        # Letterbox resize to maintain aspect ratio
        h, w = image.shape[:2]
        scale = min(self.input_width / w, self.input_height / h)
        new_w, new_h = int(w * scale), int(h * scale)

        # Resize
        resized = cv2.resize(image, (new_w, new_h), interpolation=cv2.INTER_LINEAR)

        # Create letterboxed image
        letterboxed = np.full((self.input_height, self.input_width, 3), 114, dtype=np.uint8)
        y_offset = (self.input_height - new_h) // 2
        x_offset = (self.input_width - new_w) // 2
        letterboxed[y_offset:y_offset+new_h, x_offset:x_offset+new_w] = resized

        # Convert BGR to RGB, normalize, transpose to CHW
        rgb = letterboxed[:, :, ::-1].astype(np.float32) / 255.0
        chw = rgb.transpose(2, 0, 1)

        # Add batch dimension
        return np.expand_dims(chw, axis=0), scale, (x_offset, y_offset)

    def _postprocess(self, outputs: List[np.ndarray], scale: float,
                     padding: Tuple[int, int], orig_w: int, orig_h: int,
                     confidence_threshold: float, iou_threshold: float,
                     class_filter: Optional[List[str]], max_detections: int) -> List[Detection]:
        """Post-process YOLOv8 outputs."""
        # YOLOv8 output format: [batch, num_boxes, 4 + num_classes] or [batch, 4 + num_classes, num_boxes]
        predictions = outputs[0]

        # Handle different output formats
        if predictions.ndim == 3:
            # [batch, num_boxes, 4+classes] -> [num_boxes, 4+classes]
            predictions = predictions[0]
        elif predictions.ndim == 2:
            # [num_boxes, 4+classes] or [4+classes, num_boxes]
            if predictions.shape[0] > predictions.shape[1]:
                predictions = predictions.T

        # predictions shape: [num_boxes, 4 + num_classes]
        # 4 = x_center, y_center, width, height (normalized to input size)

        boxes = []
        scores = []
        class_ids = []

        for pred in predictions:
            # Class scores (after box coordinates)
            class_scores = pred[4:]
            class_id = np.argmax(class_scores)
            confidence = class_scores[class_id]

            if confidence < confidence_threshold:
                continue

            class_name = self.class_names[class_id] if class_id < len(self.class_names) else f"class_{class_id}"

            if class_filter and class_name not in class_filter:
                continue

            # Extract box coordinates
            x_center, y_center, width, height = pred[:4]

            # Convert from normalized input coordinates to original image coordinates
            x_center = (x_center - padding[0]) / scale
            y_center = (y_center - padding[1]) / scale
            width = width / scale
            height = height / scale

            # Convert to x, y, w, h (top-left)
            x = x_center - width / 2
            y = y_center - height / 2

            # Clip to image bounds
            x = max(0, min(x, orig_w - 1))
            y = max(0, min(y, orig_h - 1))
            width = max(0, min(width, orig_w - x))
            height = max(0, min(height, orig_h - y))

            boxes.append([x, y, width, height])
            scores.append(float(confidence))
            class_ids.append(class_id)

        # Apply NMS
        if boxes:
            indices = cv2.dnn.NMSBoxes(boxes, scores, confidence_threshold, iou_threshold)
            if len(indices) > 0:
                indices = indices.flatten()[:max_detections]
                boxes = [boxes[i] for i in indices]
                scores = [scores[i] for i in indices]
                class_ids = [class_ids[i] for i in indices]
            else:
                boxes, scores, class_ids = [], [], []

        detections = []
        for box, score, class_id in zip(boxes, scores, class_ids):
            detections.append(Detection(
                class_id=class_id,
                class_name=self.class_names[class_id] if class_id < len(self.class_names) else f"class_{class_id}",
                confidence=score,
                bbox=np.array(box, dtype=np.float32)
            ))

        return detections

    def detect(self, image: np.ndarray, confidence_threshold: float = 0.5,
               iou_threshold: float = 0.45, class_filter: Optional[List[str]] = None,
               max_detections: int = 100) -> DetectionResult:
        """Run detection on a single image."""
        import cv2
        start_time = time.perf_counter()

        orig_h, orig_w = image.shape[:2]

        # Preprocess
        input_tensor, scale, padding = self._preprocess(image)

        # Inference
        outputs = self._ort.infer(input_tensor)

        # Postprocess
        detections = self._postprocess(
            outputs, scale, padding, orig_w, orig_h,
            confidence_threshold, iou_threshold, class_filter, max_detections
        )

        inference_time_ms = (time.perf_counter() - start_time) * 1000

        return DetectionResult(
            detections=detections,
            inference_time_ms=inference_time_ms,
            image_width=orig_w,
            image_height=orig_h
        )

    def detect_batch(self, images: List[np.ndarray], confidence_threshold: float = 0.5,
                     iou_threshold: float = 0.45, class_filter: Optional[List[str]] = None,
                     max_detections: int = 100) -> List[DetectionResult]:
        """Run detection on a batch of images."""
        results = []
        for image in images:
            results.append(self.detect(image, confidence_threshold, iou_threshold, class_filter, max_detections))
        return results

    def warmup(self, input_shape: tuple = (1, 3, 640, 640)):
        """Warm up the model."""
        dummy_input = np.random.randn(*input_shape).astype(np.float32)
        self._ort.infer(dummy_input)

    def get_model_info(self) -> Dict[str, Any]:
        """Get model information."""
        return {
            "name": "YOLOv8",
            "model_path": self.model_path,
            "device": self.device,
            "input_shape": self._ort.get_input_shape(),
            "output_shapes": self._ort.get_output_shapes(),
            "class_names": self.class_names,
            "num_classes": len(self.class_names)
        }

    def close(self):
        """Clean up resources."""
        self._ort.close()


# Import cv2 at module level for preprocessing
import cv2