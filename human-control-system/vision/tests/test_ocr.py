"""Tests for OCR module."""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock
import sys

# Mock the heavy dependencies before importing
paddleocr_mock = Mock()
paddleocr_mock.PaddleOCR = Mock()
sys.modules['paddleocr'] = paddleocr_mock

paddlex_mock = Mock()
sys.modules['paddlex'] = paddlex_mock

pytesseract_mock = Mock()
sys.modules['pytesseract'] = pytesseract_mock

from src.ocr import OcrFactory, TextBlock, OcrResult, BaseOcrEngine
from src.ocr.paddle import PaddleOcrEngine
from src.ocr.tesseract import TesseractOcrEngine


class TestTextBlock:
    def test_text_block_creation(self):
        block = TextBlock(
            text="Hello World",
            confidence=0.95,
            bbox=np.array([10, 20, 100, 30], dtype=np.float32),
            polygon=np.array([[10, 20], [110, 20], [110, 50], [10, 50]], dtype=np.float32),
            language="en"
        )
        assert block.text == "Hello World"
        assert block.confidence == 0.95
        assert block.language == "en"


class TestOcrResult:
    def test_result_creation(self):
        block = TextBlock(
            text="Hello",
            confidence=0.9,
            bbox=np.array([0, 0, 50, 20], dtype=np.float32)
        )
        result = OcrResult(
            text_blocks=[block],
            inference_time_ms=25.0,
            image_width=640,
            image_height=480
        )
        assert len(result.text_blocks) == 1
        assert result.inference_time_ms == 25.0


class TestOcrFactory:
    def test_create_paddle(self):
        with patch('paddleocr.PaddleOCR') as mock_paddle:
            mock_instance = Mock()
            mock_instance.ocr.return_value = [[[[[10, 10], [100, 10], [100, 30], [10, 30]], ("Hello", 0.95)]]]
            mock_paddle.return_value = mock_instance

            engine = OcrFactory.create("paddle")
            assert isinstance(engine, PaddleOcrEngine)
            OcrFactory.clear()

    def test_create_tesseract(self):
        with patch('pytesseract.image_to_data') as mock_image_to_data:
            with patch('pytesseract.get_languages') as mock_get_languages:
                mock_image_to_data.return_value = {
                    'text': ['Hello'],
                    'conf': ['95'],
                    'left': [10],
                    'top': [20],
                    'width': [100],
                    'height': [30],
                }
                mock_get_languages.return_value = ['eng', 'osd']

                engine = OcrFactory.create("tesseract")
                assert isinstance(engine, TesseractOcrEngine)
                OcrFactory.clear()

    def test_singleton_pattern(self):
        with patch('paddleocr.PaddleOCR') as mock_paddle:
            mock_instance = Mock()
            mock_instance.ocr.return_value = [[[[[10, 10], [100, 10], [100, 30], [10, 30]], ("Hello", 0.95)]]]
            mock_paddle.return_value = mock_instance

            OcrFactory.clear()
            e1 = OcrFactory.create("paddle")
            e2 = OcrFactory.create("paddle")
            assert e1 is e2
            OcrFactory.clear()


class TestPaddleOcrEngine:
    @patch('paddleocr.PaddleOCR')
    def test_initialization(self, mock_paddle):
        mock_instance = Mock()
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine(
            det_model_dir="models/det",
            rec_model_dir="models/rec",
            use_gpu=False,
            languages=["en"]
        )

        assert engine.languages == ["en"]
        assert engine.use_gpu is False
        mock_paddle.assert_called_once()

    @patch('paddleocr.PaddleOCR')
    def test_recognize(self, mock_paddle):
        mock_instance = Mock()
        mock_instance.ocr.return_value = [[
            [[[10, 10], [100, 10], [100, 30], [10, 30]], ("Hello World", 0.95)]
        ]]
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine(languages=["en"])
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)

        result = engine.recognize(image, confidence_threshold=0.5)

        assert isinstance(result, OcrResult)
        assert len(result.text_blocks) == 1
        assert result.text_blocks[0].text == "Hello World"
        assert result.text_blocks[0].confidence == 0.95

    @patch('paddleocr.PaddleOCR')
    def test_recognize_with_region(self, mock_paddle):
        mock_instance = Mock()
        mock_instance.ocr.return_value = [[
            [[[5, 5], [50, 5], [50, 15], [5, 15]], ("Region Text", 0.9)]
        ]]
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine(languages=["en"])
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)
        region = {"x": 100, "y": 100, "width": 200, "height": 100}

        result = engine.recognize(image, region=region, confidence_threshold=0.5)

        assert len(result.text_blocks) == 1
        # Check region offset applied
        assert result.text_blocks[0].bbox[0] >= 100

    @patch('paddleocr.PaddleOCR')
    def test_recognize_empty_result(self, mock_paddle):
        mock_instance = Mock()
        mock_instance.ocr.return_value = [[]]  # No text detected
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine(languages=["en"])
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)

        result = engine.recognize(image)

        assert len(result.text_blocks) == 0

    @patch('paddleocr.PaddleOCR')
    def test_get_supported_languages(self, mock_paddle):
        mock_instance = Mock()
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine()
        langs = engine.get_supported_languages()

        assert "en" in langs
        assert "ch" in langs

    @patch('paddleocr.PaddleOCR')
    def test_get_engine_info(self, mock_paddle):
        mock_instance = Mock()
        mock_paddle.return_value = mock_instance

        engine = PaddleOcrEngine(
            det_model_dir="models/det",
            rec_model_dir="models/rec",
            use_gpu=True,
            languages=["en", "ch"]
        )
        info = engine.get_engine_info()

        assert info["name"] == "PaddleOCR"
        assert info["use_gpu"] is True
        assert info["languages"] == ["en", "ch"]


class TestTesseractOcrEngine:
    @patch('pytesseract.get_languages')
    def test_initialization(self, mock_get_languages):
        mock_get_languages.return_value = ['eng']

        engine = TesseractOcrEngine(
            tesseract_config="--oem 3 --psm 6",
            languages=["eng"]
        )

        assert engine.languages == ["eng"]
        assert engine.tesseract_config == "--oem 3 --psm 6"

    @patch('pytesseract.image_to_data')
    @patch('pytesseract.get_languages')
    def test_recognize(self, mock_get_languages, mock_image_to_data):
        mock_image_to_data.return_value = {
            'text': ['Hello', 'World', ''],
            'conf': ['95', '90', '-1'],
            'left': [10, 70, 0],
            'top': [20, 20, 0],
            'width': [50, 60, 0],
            'height': [30, 30, 0],
        }
        mock_get_languages.return_value = ['eng']

        engine = TesseractOcrEngine(languages=["eng"])
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)

        result = engine.recognize(image, confidence_threshold=0.5)

        assert isinstance(result, OcrResult)
        assert len(result.text_blocks) == 2
        assert result.text_blocks[0].text == "Hello"
        assert result.text_blocks[1].text == "World"

    @patch('pytesseract.image_to_data')
    @patch('pytesseract.image_to_string')
    @patch('pytesseract.get_languages')
    def test_recognize_fallback(self, mock_get_languages, mock_image_to_string, mock_image_to_data):
        mock_image_to_data.side_effect = Exception("OCR failed")
        mock_image_to_string.return_value = "Fallback text"
        mock_get_languages.return_value = ['eng']

        engine = TesseractOcrEngine(languages=["eng"])
        image = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)

        result = engine.recognize(image)

        assert len(result.text_blocks) == 1
        assert result.text_blocks[0].text == "Fallback text"

    @patch('pytesseract.get_languages')
    def test_get_supported_languages(self, mock_get_languages):
        mock_get_languages.return_value = ['eng', 'fra', 'deu', 'osd']

        engine = TesseractOcrEngine()
        langs = engine.get_supported_languages()

        assert "eng" in langs
        assert "fra" in langs
        assert "osd" not in langs  # Should be filtered out

    @patch('pytesseract.get_tesseract_version')
    @patch('pytesseract.get_languages')
    def test_get_engine_info(self, mock_get_languages, mock_get_version):
        mock_get_version.return_value = "5.3.0"
        mock_get_languages.return_value = ['eng']

        engine = TesseractOcrEngine()
        info = engine.get_engine_info()

        assert info["name"] == "Tesseract"
        assert info["version"] == "5.3.0"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])