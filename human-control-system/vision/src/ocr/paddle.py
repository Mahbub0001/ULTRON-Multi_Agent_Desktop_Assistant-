"""PaddleOCR engine implementation."""

import time
import numpy as np
from typing import List, Optional, Dict, Any
import cv2

from . import BaseOcrEngine, TextBlock, OcrResult, OcrFactory


class PaddleOcrEngine(BaseOcrEngine):
    """PaddleOCR engine for text detection and recognition."""

    def __init__(self, det_model_dir: str = "", rec_model_dir: str = "", cls_model_dir: str = "",
                 use_gpu: bool = True, gpu_id: int = 0, cpu_threads: int = 4,
                 enable_mkldnn: bool = True, languages: Optional[List[str]] = None,
                 **kwargs):
        self.det_model_dir = det_model_dir
        self.rec_model_dir = rec_model_dir
        self.cls_model_dir = cls_model_dir
        self.use_gpu = use_gpu
        self.gpu_id = gpu_id
        self.cpu_threads = cpu_threads
        self.enable_mkldnn = enable_mkldnn
        self.languages = languages or ["en"]
        self._ocr = None
        self._initialize()

    def _initialize(self):
        """Initialize PaddleOCR."""
        try:
            from paddleocr import PaddleOCR
        except ImportError:
            raise RuntimeError("PaddleOCR not available. Install with: pip install paddleocr")

        # Build model paths if provided
        det_path = self.det_model_dir if self.det_model_dir else None
        rec_path = self.rec_model_dir if self.rec_model_dir else None
        cls_path = self.cls_model_dir if self.cls_model_dir else None

        self._ocr = PaddleOCR(
            det_model_dir=det_path,
            rec_model_dir=rec_path,
            cls_model_dir=cls_path,
            use_gpu=self.use_gpu,
            gpu_id=self.gpu_id,
            cpu_threads=self.cpu_threads,
            enable_mkldnn=self.enable_mkldnn,
            lang=self.languages[0] if self.languages else "en",
            show_log=False,
            use_angle_cls=True,
            **{
                k: v for k, v in {
                    "det_algorithm": "DB",
                    "rec_algorithm": "SVTR_LCNet",
                }.items()
            }
        )

    def recognize(self, image: np.ndarray, region: Optional[Dict[str, int]] = None,
                  languages: Optional[List[str]] = None, confidence_threshold: float = 0.5,
                  detect_orientation: bool = False) -> OcrResult:
        """Run OCR on image."""
        start_time = time.perf_counter()

        # Crop region if specified
        if region:
            x, y, w, h = region.get("x", 0), region.get("y", 0), region.get("width", 0), region.get("height", 0)
            if w > 0 and h > 0:
                image = image[y:y+h, x:x+w]

        orig_h, orig_w = image.shape[:2]

        # Run OCR
        result = self._ocr.ocr(image, cls=detect_orientation)

        text_blocks = []
        if result and result[0]:
            for line in result[0]:
                # line format: [[[x1,y1],[x2,y2],[x3,y3],[x4,y4]], (text, confidence)]
                polygon = np.array(line[0], dtype=np.float32)
                text, confidence = line[1]

                if confidence < confidence_threshold:
                    continue

                # Calculate bounding box from polygon
                x_coords = polygon[:, 0]
                y_coords = polygon[:, 1]
                x, y = x_coords.min(), y_coords.min()
                w, h = x_coords.max() - x, y_coords.max() - y

                # Adjust for region offset
                if region:
                    x += region.get("x", 0)
                    y += region.get("y", 0)

                text_blocks.append(TextBlock(
                    text=text,
                    confidence=float(confidence),
                    bbox=np.array([x, y, w, h], dtype=np.float32),
                    polygon=polygon,
                    language=languages[0] if languages else (self.languages[0] if self.languages else "en")
                ))

        inference_time_ms = (time.perf_counter() - start_time) * 1000

        return OcrResult(
            text_blocks=text_blocks,
            inference_time_ms=inference_time_ms,
            image_width=orig_w,
            image_height=orig_h
        )

    def get_supported_languages(self) -> List[str]:
        """Get supported languages."""
        return [
            "en", "ch", "chinese_cht", "ta", "te", "ka", "ja", "ko",
            "cyrillic", "arabic", "devanagari", "latin", "hebrew", "thai"
        ]

    def get_engine_info(self) -> Dict[str, Any]:
        """Get engine information."""
        return {
            "name": "PaddleOCR",
            "version": "2.7+",
            "det_model_dir": self.det_model_dir,
            "rec_model_dir": self.rec_model_dir,
            "cls_model_dir": self.cls_model_dir,
            "use_gpu": self.use_gpu,
            "languages": self.languages
        }

    def close(self):
        """Clean up resources."""
        self._ocr = None


# Register with factory
OcrFactory.create("paddle")