import os
import sys
import unittest
from pathlib import Path

# Add project root to sys.path
root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

os.environ["QT_QPA_PLATFORM"] = "offscreen"
from PyQt6.QtWidgets import QApplication

class TestAgentTownUI(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance()
        if cls.app is None:
            cls.app = QApplication([])

    def test_agent_card_widget(self):
        from ui import AgentCardWidget
        from core.agent_town import AgentTownManager, AgentState

        mgr = AgentTownManager.get_instance()
        alice = mgr.get_agent("Alice")
        card = AgentCardWidget(alice)
        self.assertEqual(card._agent.name, "Alice")
        self.assertIn("Senior Research", card._role_lbl.text())

        # Test state update
        alice.state = AgentState.WORKING
        card.refresh_ui()
        self.assertIn("WORKING", card._status_lbl.text())

    def test_agent_town_drawer_creation(self):
        from ui import AgentTownDrawer
        from core.agent_town import AgentTownManager

        mgr = AgentTownManager.get_instance()
        drawer = AgentTownDrawer(None)
        self.assertEqual(len(drawer._cards), 4)

if __name__ == "__main__":
    unittest.main()
