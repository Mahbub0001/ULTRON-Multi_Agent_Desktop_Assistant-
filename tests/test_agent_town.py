import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from core.agent_town import AgentTownManager, AgentState


class TestAgentTown(unittest.TestCase):
    def setUp(self):
        mgr = AgentTownManager.get_instance()
        for a in mgr.get_all_agents():
            a.state = AgentState.IDLE
            a.current_task = ""
            a.latest_result = ""

    def test_load_default_agents(self):
        mgr = AgentTownManager.get_instance()
        agents = mgr.get_all_agents()
        self.assertEqual(len(agents), 4)
        names = [a.name for a in agents]
        self.assertIn("Alice", names)
        self.assertIn("Bob", names)
        self.assertIn("Carol", names)
        self.assertIn("Dave", names)
        self.assertEqual(mgr.get_agent("Alice").state, AgentState.IDLE)

    def test_dispatch_task_sync_mock(self):
        mgr = AgentTownManager.get_instance()

        import core.gemini as gemini
        orig_call = getattr(gemini, "call", None)

        def mock_call(contents, tier="fast", timeout_ms=30000):
            return "Mock research report from agent."

        try:
            gemini.call = mock_call
            finished = []

            def on_complete(agent, result):
                finished.append((agent.name, result))

            mgr.dispatch_task("Alice", "Find recent AI agent news", on_complete=on_complete, async_exec=False)

            alice = mgr.get_agent("Alice")
            self.assertEqual(alice.state, AgentState.COMPLETED)
            self.assertEqual(alice.latest_result, "Mock research report from agent.")
            self.assertEqual(len(finished), 1)
            self.assertEqual(finished[0][0], "Alice")
        finally:
            if orig_call:
                gemini.call = orig_call


if __name__ == "__main__":
    unittest.main()
