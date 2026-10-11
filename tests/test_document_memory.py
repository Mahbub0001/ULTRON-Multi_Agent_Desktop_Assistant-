import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

from actions.file_processor import _process_pdf
from memory.memory_manager import search_memory, remember


class TestDocumentMemory(unittest.TestCase):
    def test_pdf_full_text_survives_and_is_recalled(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "my_cv.pdf"
            source.write_bytes(b"test")
            page_text = "Education: Example University.\n" + "Background section.\n" * 3500 + "Final skill: data visualization."
            page = MagicMock()
            page.extract_text.return_value = page_text
            opened = MagicMock()
            opened.__enter__.return_value.pages = [page]
            with patch("pdfplumber.open", return_value=opened), \
                    patch("memory.document_memory.DOCUMENT_DIR", root / "documents"), \
                    patch("memory.memory_manager.MEMORY_PATH", root / "long_term.json"):
                result = _process_pdf(source, "remember", {})
                self.assertIn("Verified full text", result)
                saved = list((root / "documents").glob("*.json"))
                self.assertEqual(len(saved), 1)
                self.assertIn("Final skill: data visualization.", saved[0].read_text(encoding="utf-8"))
                self.assertIn("data visualization", search_memory("visualization"))
                self.assertIn("Example University", search_memory("আমার সিভিতে শিক্ষাগত যোগ্যতা"))
                self.assertIn("Stored document: my_cv.pdf", search_memory(""))

    def test_scanned_pdf_does_not_claim_memory(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "scan.pdf"
            source.write_bytes(b"test")
            page = MagicMock()
            page.extract_text.return_value = None
            opened = MagicMock()
            opened.__enter__.return_value.pages = [page]
            with patch("pdfplumber.open", return_value=opened), \
                    patch("memory.document_memory.DOCUMENT_DIR", root / "documents"), \
                    patch("memory.memory_manager.MEMORY_PATH", root / "long_term.json"):
                self.assertTrue(_process_pdf(source, "remember", {}).startswith("Error:"))
                self.assertFalse((root / "documents").exists())

    def test_long_fact_is_preserved_without_silent_truncation(self):
        with tempfile.TemporaryDirectory() as folder:
            value = ("Detail " * 200).strip()
            with patch("memory.memory_manager.MEMORY_PATH", Path(folder) / "memory.json"), \
                    patch("memory.document_memory.DOCUMENT_DIR", Path(folder) / "documents"):
                remember("biography", value, "notes")
                self.assertIn(value, search_memory("biography"))
