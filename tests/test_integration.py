import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from core.action_loader import discover_actions


class TestAgentTownIntegration(unittest.TestCase):
    def test_action_loader_discovers_agent_town(self):
        actions_dir = root / "actions"
        registry = discover_actions(actions_dir)
        self.assertTrue(registry.has("delegate_agent_task"))

        decls = registry.get_tool_declarations()
        names = [d["name"] for d in decls]
        self.assertIn("delegate_agent_task", names)

        town_decl = next(d for d in decls if d["name"] == "delegate_agent_task")
        self.assertIn("Alice", town_decl["description"])


if __name__ == "__main__":
    unittest.main()
