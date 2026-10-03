"""gRPC Vision Service Implementation."""

import time
import logging
from concurrent import futures
from pathlib import Path
from typing import Optional, Dict, Any, List
import numpy as np
import cv2

import grpc
from google.protobuf import empty_pb2

from ..capture import CaptureFactory, MonitorInfo, Region, CaptureResult
from ..detection import DetectorFactory, Detection, DetectionResult
from ..ocr import OcrFactory, TextBlock, OcrResult
import sys
sys.path.insert(0, str(Path(__file__).parent.parent))
import vision_pb2, vision_pb2_grpc

from ..config import load_config, VisionConfig

logger = logging.getLogger(__name__)


class VisionService(vision_pb2_grpc.VisionServiceServicer):
    """gRPC service for computer vision operations."""

    def __init__(self, config: VisionConfig):
        self.config = config
        self._start_time = time.time()
        self._capture = None
        self._detector = None
        self._ocr_engine = None
        self._initialize_components()

    def _initialize_components(self):
        """Initialize capture, detection, and OCR components."""
        # Initialize capture
        try:
            self._capture = CaptureFactory.create(self.config.capture.backend)
            logger.info(f"Capture initialized with backend: {self.config.capture.backend}")
        except Exception as e:
            logger.error(f"Failed to initialize capture: {e}")
            raise

        # Initialize detector
        try:
            self._detector = DetectorFactory.create(
                name="yolo",
                model_path=self.config.detection.model_path,
                device=self.config.detection.device,
                batch_size=self.config.detection.batch_size,
                input_width=self.config.detection.input_width,
                input_height=self.config.detection.input_height,
                use_tensorrt=self.config.detection.use_tensorrt,
                tensorrt_cache_dir=self.config.detection.tensorrt_cache_dir,
                class_names=self.config.detection.class_names
            )
            logger.info(f"Detector initialized: {self.config.detection.model_path}")
        except Exception as e:
            logger.warning(f"Failed to initialize detector: {e}")
            self._detector = None

        # Initialize OCR
        try:
            self._ocr_engine = OcrFactory.create(
                self.config.ocr.engine,
                det_model_dir=self.config.ocr.paddle_det_model_dir,
                rec_model_dir=self.config.ocr.paddle_rec_model_dir,
                cls_model_dir=self.config.ocr.paddle_cls_model_dir,
                use_gpu=self.config.ocr.use_gpu,
                gpu_id=self.config.ocr.gpu_id,
                cpu_threads=self.config.ocr.cpu_threads,
                enable_mkldnn=self.config.ocr.enable_mkldnn,
                languages=self.config.ocr.languages,
                tesseract_data_path=self.config.ocr.tesseract_data_path,
                tesseract_config=self.config.ocr.tesseract_config
            )
            logger.info(f"OCR engine initialized: {self.config.ocr.engine}")
        except Exception as e:
            logger.warning(f"Failed to initialize OCR: {e}")
            self._ocr_engine = None

    # ============================================
    # Screen Capture
    # ============================================

    def CaptureScreen(self, request: vision_pb2.CaptureRequest, context) -> vision_pb2.CaptureResponse:
        """Capture a single frame from screen."""
        try:
            region = None
            if request.region.width > 0 and request.region.height > 0:
                region = Region(
                    x=request.region.x,
                    y=request.region.y,
                    width=request.region.width,
                    height=request.region.height
                )

            result: CaptureResult = self._capture.capture(
                monitor_index=request.monitor_index,
                region=region
            )

            # Convert frame to requested format
            frame = result.frame
            format_ = self._convert_format(request.format, frame)

            # Encode if needed
            if request.format in (vision_pb2.IMAGE_FORMAT_JPEG, vision_pb2.IMAGE_FORMAT_PNG):
                image_data = self._encode_image(frame, request.format)
            else:
                image_data = frame.tobytes()

            return vision_pb2.CaptureResponse(
                image_data=image_data,
                width=result.frame.shape[1],
                height=result.frame.shape[0],
                stride=result.frame.strides[0],
                format=request.format,
                timestamp_us=result.timestamp_us,
                monitor_index=result.monitor_index
            )
        except Exception as e:
            logger.error(f"CaptureScreen failed: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            return vision_pb2.CaptureResponse()

    def StreamFrames(self, request: vision_pb2.StreamFramesRequest, context):
        """Stream continuous frames."""
        try:
            region = None
            if request.region.width > 0 and request.region.height > 0:
                region = Region(
                    x=request.region.x,
                    y=request.region.y,
                    width=request.region.width,
                    height=request.region.height
                )

            self._capture.start_stream(
                monitor_index=request.monitor_index,
                region=region,
                target_fps=request.target_fps
            )

            frame_number = 0
            while context.is_active():
                result = self._capture.get_next_frame()
                if result is None:
                    time.sleep(1.0 / request.target_fps)
                    continue

                frame = result.frame
                if request.format in (vision_pb2.IMAGE_FORMAT_JPEG, vision_pb2.IMAGE_FORMAT_PNG):
                    image_data = self._encode_image(frame, request.format)
                else:
                    image_data = frame.tobytes()

                yield vision_pb2.FrameChunk(
                    image_data=image_data,
                    width=frame.shape[1],
                    height=frame.shape[0],
                    stride=frame.strides[0],
                    format=request.format,
                    timestamp_us=result.timestamp_us,
                    frame_number=frame_number
                )
                frame_number += 1

        except Exception as e:
            logger.error(f"StreamFrames failed: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
        finally:
            self._capture.stop_stream()

    def ListMonitors(self, request: empty_pb2.Empty, context) -> vision_pb2.MonitorList:
        """List available monitors."""
        monitors = self._capture.get_monitors()
        return vision_pb2.MonitorList(monitors=[
            vision_pb2.MonitorInfo(
                index=m.index,
                name=m.name,
                x=m.x,
                y=m.y,
                width=m.width,
                height=m.height,
                is_primary=m.is_primary,
                scale_factor=m.scale_factor
            ) for m in monitors
        ])

    # ============================================
    # Object Detection
    # ============================================

    def DetectObjects(self, request: vision_pb2.DetectionRequest, context) -> vision_pb2.DetectionResponse:
        """Run object detection on image."""
        if self._detector is None:
            context.set_code(grpc.StatusCode.FAILED_PRECONDITION)
            context.set_details("Detector not initialized")
            return vision_pb2.DetectionResponse()

        try:
            # Decode image
            image = self._decode_image(request.image_data, request.width, request.height, request.format)

            # Run detection
            result: DetectionResult = self._detector.detect(
                image=image,
                confidence_threshold=request.confidence_threshold or self.config.detection.default_confidence,
                iou_threshold=request.iou_threshold or self.config.detection.default_iou,
                class_filter=list(request.class_filter) if request.class_filter else None,
                max_detections=request.max_detections or 100
            )

            # Convert to proto
            detections = []
            for det in result.detections:
                detections.append(vision_pb2.Detection(
                    class_name=det.class_name,
                    class_id=det.class_id,
                    confidence=det.confidence,
                    bbox=vision_pb2.BoundingBox(
                        x=det.bbox[0],
                        y=det.bbox[1],
                        width=det.bbox[2],
                        height=det.bbox[3]
                    ),
                    attributes=det.attributes or {}
                ))

            return vision_pb2.DetectionResponse(
                detections=detections,
                inference_time_us=int(result.inference_time_ms * 1000),
                image_width=result.image_width,
                image_height=result.image_height
            )
        except Exception as e:
            logger.error(f"DetectObjects failed: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            return vision_pb2.DetectionResponse()

    def BatchDetectObjects(self, request: vision_pb2.BatchDetectionRequest, context) -> vision_pb2.BatchDetectionResponse:
        """Run batch object detection."""
        if self._detector is None:
            context.set_code(grpc.StatusCode.FAILED_PRECONDITION)
            context.set_details("Detector not initialized")
            return vision_pb2.BatchDetectionResponse()

        try:
            start_time = time.perf_counter()
            images = []
            for req in request.requests:
                image = self._decode_image(req.image_data, req.width, req.height, req.format)
                images.append(image)

            results = self._detector.detect_batch(
                images=images,
                confidence_threshold=self.config.detection.default_confidence,
                iou_threshold=self.config.detection.default_iou,
                class_filter=None,
                max_detections=100
            )

            responses = []
            for result in results:
                detections = []
                for det in result.detections:
                    detections.append(vision_pb2.Detection(
                        class_name=det.class_name,
                        class_id=det.class_id,
                        confidence=det.confidence,
                        bbox=vision_pb2.BoundingBox(
                            x=det.bbox[0],
                            y=det.bbox[1],
                            width=det.bbox[2],
                            height=det.bbox[3]
                        ),
                        attributes=det.attributes or {}
                    ))
                responses.append(vision_pb2.DetectionResponse(
                    detections=detections,
                    inference_time_us=int(result.inference_time_ms * 1000),
                    image_width=result.image_width,
                    image_height=result.image_height
                ))

            total_time = (time.perf_counter() - start_time) * 1_000_000
            return vision_pb2.BatchDetectionResponse(
                responses=responses,
                total_inference_time_us=int(total_time)
            )
        except Exception as e:
            logger.error(f"BatchDetectObjects failed: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            return vision_pb2.BatchDetectionResponse()

    # ============================================
    # OCR
    # ============================================

    def RecognizeText(self, request: vision_pb2.OcrRequest, context) -> vision_pb2.OcrResponse:
        """Run OCR on image region."""
        if self._ocr_engine is None:
            context.set_code(grpc.StatusCode.FAILED_PRECONDITION)
            context.set_details("OCR engine not initialized")
            return vision_pb2.OcrResponse()

        try:
            # Decode image
            image = self._decode_image(request.image_data, request.width, request.height, request.format)

            # Prepare region
            region = None
            if request.region.width > 0 and request.region.height > 0:
                region = {
                    "x": request.region.x,
                    "y": request.region.y,
                    "width": request.region.width,
                    "height": request.region.height
                }

            # Run OCR
            result: OcrResult = self._ocr_engine.recognize(
                image=image,
                region=region,
                languages=list(request.languages) if request.languages else None,
                confidence_threshold=request.confidence_threshold or 0.5,
                detect_orientation=request.detect_orientation
            )

            # Convert to proto
            text_blocks = []
            for block in result.text_blocks:
                polygon = []
                if block.polygon is not None:
                    for pt in block.polygon:
                        polygon.append(vision_pb2.Point(x=pt[0], y=pt[1]))

                text_blocks.append(vision_pb2.TextBlock(
                    text=block.text,
                    confidence=block.confidence,
                    bbox=vision_pb2.BoundingBox(
                        x=block.bbox[0],
                        y=block.bbox[1],
                        width=block.bbox[2],
                        height=block.bbox[3]
                    ),
                    polygon=polygon,
                    language=block.language
                ))

            return vision_pb2.OcrResponse(
                text_blocks=text_blocks,
                inference_time_us=int(result.inference_time_ms * 1000),
                image_width=result.image_width,
                image_height=result.image_height
            )
        except Exception as e:
            logger.error(f"RecognizeText failed: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            return vision_pb2.OcrResponse()

    # ============================================
    # Model Management
    # ============================================

    def GetModelInfo(self, request: empty_pb2.Empty, context) -> vision_pb2.ModelList:
        """Get information about loaded models."""
        models = []

        def _to_str_map(d: dict) -> dict:
            """Convert dict values to strings for protobuf map<string, string>."""
            result = {}
            for k, v in d.items():
                if isinstance(v, (list, tuple)):
                    result[k] = str(v)
                elif isinstance(v, np.ndarray):
                    result[k] = str(v.tolist())
                else:
                    result[k] = str(v)
            return result

        if self._detector:
            info = self._detector.get_model_info()
            models.append(vision_pb2.ModelInfo(
                name=info.get("name", "YOLOv8"),
                version="1.0",
                path=self.config.detection.model_path,
                device=self._device_to_proto(self.config.detection.device),
                loaded=True,
                metadata=_to_str_map(info)
            ))

        if self._ocr_engine:
            info = self._ocr_engine.get_engine_info()
            models.append(vision_pb2.ModelInfo(
                name=info.get("name", "OCR"),
                version=info.get("version", "1.0"),
                path="",
                device=vision_pb2.DEVICE_TYPE_CPU,
                loaded=True,
                metadata=_to_str_map(info)
            ))

        return vision_pb2.ModelList(models=models)

    def ReloadModels(self, request: vision_pb2.ReloadModelsRequest, context) -> empty_pb2.Empty:
        """Reload specified models."""
        # TODO: Implement model reloading
        logger.info(f"Reload models requested: {request.model_names}")
        return empty_pb2.Empty()

    # ============================================
    # Health & Config
    # ============================================

    def HealthCheck(self, request: empty_pb2.Empty, context) -> vision_pb2.HealthCheckResponse:
        """Health check endpoint."""
        components = {}

        # Capture health
        components["capture"] = vision_pb2.ComponentHealth(
            status=vision_pb2.COMPONENT_STATUS_HEALTHY if self._capture else vision_pb2.COMPONENT_STATUS_UNHEALTHY,
            message="Capture operational" if self._capture else "Capture not initialized"
        )

        # Detection health
        components["detection"] = vision_pb2.ComponentHealth(
            status=vision_pb2.COMPONENT_STATUS_HEALTHY if self._detector else vision_pb2.COMPONENT_STATUS_DEGRADED,
            message="Detector operational" if self._detector else "Detector not initialized"
        )

        # OCR health
        components["ocr"] = vision_pb2.ComponentHealth(
            status=vision_pb2.COMPONENT_STATUS_HEALTHY if self._ocr_engine else vision_pb2.COMPONENT_STATUS_DEGRADED,
            message="OCR operational" if self._ocr_engine else "OCR not initialized"
        )

        overall_status = vision_pb2.SERVING_STATUS_SERVING
        if not self._capture:
            overall_status = vision_pb2.SERVING_STATUS_NOT_SERVING
        elif not self._detector or not self._ocr_engine:
            overall_status = vision_pb2.SERVING_STATUS_SERVING  # Degraded but serving

        return vision_pb2.HealthCheckResponse(
            status=overall_status,
            version="1.0.0",
            uptime_seconds=int(time.time() - self._start_time),
            components=components
        )

    def GetConfig(self, request: empty_pb2.Empty, context) -> vision_pb2.VisionConfig:
        """Get current configuration."""
        return vision_pb2.VisionConfig(
            capture=vision_pb2.CaptureConfig(
                default_fps=self.config.capture.default_fps,
                default_monitor=self.config.capture.default_monitor,
                jpeg_quality=self.config.capture.jpeg_quality,
                use_hardware_acceleration=self.config.capture.use_hardware_acceleration
            ),
            detection=vision_pb2.DetectionConfig(
                model_path=self.config.detection.model_path,
                device=self._device_to_proto(self.config.detection.device),
                batch_size=self.config.detection.batch_size,
                default_confidence=self.config.detection.default_confidence,
                default_iou=self.config.detection.default_iou,
                input_width=self.config.detection.input_width,
                input_height=self.config.detection.input_height,
                use_tensorrt=self.config.detection.use_tensorrt,
                tensorrt_cache_dir=self.config.detection.tensorrt_cache_dir
            ),
            ocr=vision_pb2.OcrConfig(
                engine=self.config.ocr.engine,
                languages=self.config.ocr.languages,
                paddle_det_model_dir=self.config.ocr.paddle_det_model_dir,
                paddle_rec_model_dir=self.config.ocr.paddle_rec_model_dir,
                paddle_cls_model_dir=self.config.ocr.paddle_cls_model_dir,
                use_gpu=self.config.ocr.use_gpu,
                gpu_id=self.config.ocr.gpu_id,
                cpu_threads=self.config.ocr.cpu_threads,
                enable_mkldnn=self.config.ocr.enable_mkldnn
            )
        )

    def UpdateConfig(self, request: vision_pb2.VisionConfig, context) -> empty_pb2.Empty:
        """Update configuration (requires restart for some changes)."""
        logger.info("Config update requested")
        # TODO: Implement dynamic config updates
        return empty_pb2.Empty()

    # ============================================
    # Helpers
    # ============================================

    def _decode_image(self, data: bytes, width: int, height: int, format_: vision_pb2.ImageFormat) -> np.ndarray:
        """Decode image data to numpy array."""
        if format_ in (vision_pb2.IMAGE_FORMAT_JPEG, vision_pb2.IMAGE_FORMAT_PNG):
            return cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_COLOR)

        # Raw format - assume BGR
        channels = 3
        if format_ == vision_pb2.IMAGE_FORMAT_RGBA:
            channels = 4
        elif format_ == vision_pb2.IMAGE_FORMAT_GRAY:
            channels = 1

        frame = np.frombuffer(data, dtype=np.uint8).reshape(height, width, channels)

        # Convert to BGR for OpenCV
        if format_ == vision_pb2.IMAGE_FORMAT_RGB:
            frame = cv2.cvtColor(frame, cv2.COLOR_RGB2BGR)
        elif format_ == vision_pb2.IMAGE_FORMAT_RGBA:
            frame = cv2.cvtColor(frame, cv2.COLOR_RGBA2BGR)
        elif format_ == vision_pb2.IMAGE_FORMAT_GRAY:
            frame = cv2.cvtColor(frame, cv2.COLOR_GRAY2BGR)

        return frame

    def _encode_image(self, frame: np.ndarray, format_: vision_pb2.ImageFormat) -> bytes:
        """Encode image to bytes."""
        if format_ == vision_pb2.IMAGE_FORMAT_JPEG:
            encode_param = [int(cv2.IMWRITE_JPEG_QUALITY), self.config.capture.jpeg_quality]
            _, buffer = cv2.imencode('.jpg', frame, encode_param)
            return buffer.tobytes()
        elif format_ == vision_pb2.IMAGE_FORMAT_PNG:
            _, buffer = cv2.imencode('.png', frame)
            return buffer.tobytes()
        return frame.tobytes()

    def _convert_format(self, format_: vision_pb2.ImageFormat, frame: np.ndarray) -> vision_pb2.ImageFormat:
        """Convert internal format to requested format."""
        return format_

    def _device_to_proto(self, device: str) -> vision_pb2.DeviceType:
        """Convert device string to proto enum."""
        mapping = {
            "cpu": vision_pb2.DEVICE_TYPE_CPU,
            "cuda": vision_pb2.DEVICE_TYPE_CUDA,
            "tensorrt": vision_pb2.DEVICE_TYPE_TENSORRT,
            "coreml": vision_pb2.DEVICE_TYPE_COREML,
            "directml": vision_pb2.DEVICE_TYPE_DIRECTML,
        }
        return mapping.get(device.lower(), vision_pb2.DEVICE_TYPE_UNSPECIFIED)

    def close(self):
        """Clean up resources."""
        if self._capture:
            self._capture.close()
        if self._detector:
            self._detector.close()
        if self._ocr_engine:
            self._ocr_engine.close()


def serve(config: VisionConfig, host: str = "0.0.0.0", port: int = 50051, workers: int = 4):
    """Start the gRPC server."""
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=workers))
    vision_pb2_grpc.add_VisionServiceServicer_to_server(VisionService(config), server)
    server.add_insecure_port(f"{host}:{port}")
    server.start()
    logger.info(f"Vision service started on {host}:{port}")
    return server