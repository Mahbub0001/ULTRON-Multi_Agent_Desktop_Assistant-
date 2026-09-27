import os
import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

os.environ["QT_QPA_PLATFORM"] = "offscreen"
from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import QPoint


class TestTownCanvas(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance()
        if cls.app is None:
            cls.app = QApplication([])

    def test_town_office_canvas_init(self):
        from ui import TownOfficeCanvas
        from core.agent_town import AgentTownManager

        mgr = AgentTownManager.get_instance()
        canvas = TownOfficeCanvas(None)
        self.assertIsNotNone(canvas)
        self.assertEqual(len(canvas._desk_rects), 4)

        # Advance tick
        canvas._tick()
        self.assertTrue(canvas._anim_frame > 0)

    def test_town_office_canvas_desk_hit_test(self):
        from ui import TownOfficeCanvas

        canvas = TownOfficeCanvas(None)
        canvas.resize(800, 500)
        canvas._layout_desks(800, 500)

        # Alice is top-left
        alice_desk = canvas._find_desk_at(QPoint(150, 120))
        self.assertIsNotNone(alice_desk)
        self.assertEqual(alice_desk._agent.name, "Alice")


if __name__ == "__main__":
    unittest.main()
