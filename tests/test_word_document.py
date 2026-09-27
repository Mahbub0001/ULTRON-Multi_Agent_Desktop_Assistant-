import os
import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from actions.word_document import (
    word_document,
    _parse_content_blocks,
    _create_docx_file,
    TOOL,
)
from core.agent_town import _clean_agent_path


class TestWordDocument(unittest.TestCase):
    def test_tool_declaration(self):
        self.assertEqual(TOOL["name"], "word_document")
        self.assertIn("parameters", TOOL)
        actions = TOOL["parameters"]["properties"]["action"]["description"]
        self.assertIn("from_markdown", actions)
        self.assertIn("create", actions)

    def test_parse_content_blocks(self):
        raw = (
            "# Main Title\n"
            "### Section Heading\n"
            "Here is a regular paragraph.\n"
            "- **Bullet 1:** Detail text\n"
            "- Regular bullet\n"
        )
        blocks = _parse_content_blocks(raw)
        self.assertEqual(blocks[0]["type"], "title")
        self.assertEqual(blocks[0]["text"], "Main Title")
        self.assertEqual(blocks[1]["type"], "heading2")
        self.assertEqual(blocks[1]["text"], "Section Heading")
        self.assertEqual(blocks[2]["type"], "paragraph")
        self.assertEqual(blocks[3]["type"], "bullet")
        self.assertEqual(blocks[4]["type"], "bullet")

    def test_clean_agent_path_deduplication(self):
        # Should not duplicate Desktop/JarvisProjects
        p1 = _clean_agent_path("Desktop/JarvisProjects/report.md")
        self.assertNotIn("Desktop/JarvisProjects/Desktop", str(p1).replace("\\", "/"))
        self.assertTrue(str(p1).replace("\\", "/").endswith("Desktop/JarvisProjects/report.md"))

        p2 = _clean_agent_path("Desktop/report.docx")
        self.assertTrue(str(p2).replace("\\", "/").endswith("Desktop/report.docx"))

    def test_from_markdown_conversion(self):
        test_md = Path.home() / "Desktop" / "test_temp_sample.md"
        test_docx = Path.home() / "Desktop" / "Test_Temp_Sample.docx"
        try:
            test_md.write_text(
                "# Test Sample Document\n\n"
                "### Key Findings\n"
                "Summary paragraph.\n\n"
                "- **Point A:** First finding\n"
                "- **Point B:** Second finding\n",
                encoding="utf-8",
            )
            res = word_document(
                {
                    "action": "from_markdown",
                    "path": str(test_md),
                    "open_word": False,
                }
            )
            self.assertIn("successfully converted", res)
            self.assertTrue(test_docx.exists())
            self.assertGreater(test_docx.stat().st_size, 1000)
        finally:
            test_md.unlink(missing_ok=True)
            test_docx.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
