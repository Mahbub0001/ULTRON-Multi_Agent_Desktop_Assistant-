import unittest
from pathlib import Path
from core.agent_town import resolve_agent_path


class TestAgentTownDeveloper(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
