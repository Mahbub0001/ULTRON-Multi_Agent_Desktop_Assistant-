"""Tesseract OCR engine implementation."""

import time
import numpy as np
from typing import List, Optional, Dict, Any
import cv2

from . import BaseOcrEngine, TextBlock, OcrResult, OcrFactory


class TesseractOcrEngine(BaseOcrEngine):
    """Tesseract OCR engine for text recognition."""

    def __init__(self, tesseract_data_path: str = "", tesseract_config: str = "--oem 3 --psm 6",
                 languages: Optional[List[str]] = None, **kwargs):
        self.tesseract_data_path = tesseract_data_path
        self.tesseract_config = tesseract_config
        self.languages = languages or ["eng"]
        self._tess = None
        self._initialize()

    def _initialize(self):
        """Initialize Tesseract."""
        try:
            import pytesseract
            self._tess = pytesseract
            if self.tesseract_data_path:
                self._tess.pytesseract.tesseract_cmd = self.tesseract_data_path
        except ImportError:
            raise RuntimeError("pytesseract not available. Install with: pip install pytesseract")

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

        # Convert to RGB if needed
        if len(image.shape) == 3 and image.shape[2] == 3:
            rgb = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
        else:
            rgb = image

        # Build language string
        lang_str = "+".join(languages) if languages else "+".join(self.languages)

        # Run OCR with detailed output
        config = self.tesseract_config
        if detect_orientation:
            config += " --psm 0"

        try:
            data = self._tess.image_to_data(rgb, lang=lang_str, config=config, output_type=self._tess.Output.DICT)
        except Exception as e:
            # Fallback to simple OCR
            text = self._tess.image_to_string(rgb, lang=lang_str, config=config)
            inference_time_ms = (time.perf_counter() - start_time) * 1000
            return OcrResult(
                text_blocks=[TextBlock(
                    text=text.strip(),
                    confidence=0.0,
                    bbox=np.array([0, 0, orig_w, orig_h], dtype=np.float32),
                    language=lang_str
                )] if text.strip() else [],
                inference_time_ms=inference_time_ms,
                image_width=orig_w,
                image_height=orig_h
            )

        text_blocks = []
        n_boxes = len(data.get('text', []))

        for i in range(n_boxes):
            text = data['text'][i].strip()
            if not text:
                continue

            conf = float(data['conf'][i]) / 100.0
            if conf < confidence_threshold:
                continue

            x, y, w, h = data['left'][i], data['top'][i], data['width'][i], data['height'][i]

            # Adjust for region offset
            if region:
                x += region.get("x", 0)
                y += region.get("y", 0)

            # Create polygon from bbox
            polygon = np.array([
                [x, y],
                [x + w, y],
                [x + w, y + h],
                [x, y + h]
            ], dtype=np.float32)

            text_blocks.append(TextBlock(
                text=text,
                confidence=conf,
                bbox=np.array([x, y, w, h], dtype=np.float32),
                polygon=polygon,
                language=lang_str
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
        try:
            langs = self._tess.get_languages(config='')
            return [l for l in langs if l not in ('osd', 'equ')]
        except Exception:
            return ["eng", "osd"]

    def get_engine_info(self) -> Dict[str, Any]:
        """Get engine information."""
        return {
            "name": "Tesseract",
            "version": self._tess.get_tesseract_version() if hasattr(self._tess, 'get_tesseract_version') else "unknown",
            "tesseract_data_path": self.tesseract_data_path,
            "tesseract_config": self.tesseract_config,
            "languages": self.languages
        }

    def close(self):
        """Clean up resources."""
        self._tess = None