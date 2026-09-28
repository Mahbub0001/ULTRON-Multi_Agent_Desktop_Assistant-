import unittest
import tempfile
import os
from pathlib import Path
from core.agent_town import AgentTownManager, resolve_agent_path


class TestAgentTownDeveloper(unittest.TestCase):
    def setUp(self):
        self.mgr = AgentTownManager.get_instance()

    def test_resolve_project_relative_path(self):
        p = resolve_agent_path("ui.py")
        self.assertEqual(p.resolve(), (Path.cwd() / "ui.py").resolve())

    def test_resolve_desktop_path(self):
        p = resolve_agent_path("Desktop/my_notes.txt")
        self.assertEqual(p.resolve(), (Path.home() / "Desktop" / "my_notes.txt").resolve())

    def test_resolve_absolute_path(self):
        target = Path.cwd() / "main.py"
        p = resolve_agent_path(str(target))
        self.assertEqual(p.resolve(), target.resolve())

    def test_search_code_finds_occurrence(self):
        # Search for a unique symbol in core/agent_town.py
        result = self.mgr.execute_agent_tool("Bob", "search_code", {"query": "AgentTownManager", "path": "core"})
        self.assertIn("agent_town.py", result)
        self.assertIn("AgentTownManager", result)

    def test_read_file_line_range(self):
        result = self.mgr.execute_agent_tool("Bob", "read_file", {"path": "main.py", "start_line": 1, "end_line": 5})
        lines = result.strip().splitlines()
        self.assertGreater(len(lines), 1)
        self.assertTrue(any("1:" in line or "1 :" in line or "   1:" in line for line in lines))

    def test_edit_file_surgical_replacement(self):
        with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False, encoding="utf-8") as tf:
            tf.write("x = 10\ny = 20\n")
            tmp_path = tf.name

        try:
            res = self.mgr.execute_agent_tool(
                "Bob", "edit_file",
                {"path": tmp_path, "target": "x = 10", "replacement": "x = 999"}
            )
            self.assertIn("Successfully updated", res)
            content = Path(tmp_path).read_text(encoding="utf-8")
            self.assertIn("x = 999", content)
            self.assertNotIn("x = 10", content)
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)

    def test_edit_file_rejects_ambiguity(self):
        with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False, encoding="utf-8") as tf:
            tf.write("val = 1\nval = 1\n")
            tmp_path = tf.name

        try:
            res = self.mgr.execute_agent_tool(
                "Bob", "edit_file",
                {"path": tmp_path, "target": "val = 1", "replacement": "val = 2"}
            )
            self.assertIn("Error:", res)
            self.assertIn("found 2 times", res)
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)

    def test_edit_file_rejects_syntax_error(self):
        with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False, encoding="utf-8") as tf:
            tf.write("def foo():\n    return 42\n")
            tmp_path = tf.name

        try:
            res = self.mgr.execute_agent_tool(
                "Bob", "edit_file",
                {"path": tmp_path, "target": "return 42", "replacement": "return 42 +++"}
            )
            self.assertIn("Error:", res)
            self.assertIn("syntax error", res.lower())
        finally:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)

    def test_run_command_cwd(self):
        res = self.mgr.execute_agent_tool(
            "Bob", "run_command",
            {"command": "python -c 'import os; print(os.getcwd())'", "cwd": "core"}
        )
        self.assertIn("Exit Code: 0", res)
        self.assertIn("core", res.lower())


if __name__ == "__main__":
    unittest.main()
