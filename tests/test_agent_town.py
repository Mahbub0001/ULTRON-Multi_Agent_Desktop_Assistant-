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
            a.status_message = ""

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

    def test_agent_status_message_and_ambient_thought(self):
        mgr = AgentTownManager.get_instance()
        alice = mgr.get_agent("Alice")
        self.assertTrue(hasattr(alice, "status_message"))
        self.assertTrue(hasattr(alice, "get_ambient_thought"))
        thought = alice.get_ambient_thought()
        self.assertIsInstance(thought, str)
        self.assertTrue(len(thought) > 0)

    def test_agent_tool_execution(self):
        mgr = AgentTownManager.get_instance()
        test_dir = root / "scratch" / "agent_test"
        test_dir.mkdir(parents=True, exist_ok=True)
        test_file = test_dir / "sample.py"
        if test_file.exists():
            test_file.unlink()

        res = mgr.execute_agent_tool(
            agent_name="Bob",
            tool_name="write_file",
            arguments={"path": str(test_file), "content": "print('hello from Bob')"}
        )
        self.assertIn("Successfully wrote", res)
        self.assertTrue(test_file.exists())
        self.assertEqual(test_file.read_text(encoding="utf-8").strip(), "print('hello from Bob')")

        # Test reading file
        read_res = mgr.execute_agent_tool(
            agent_name="Bob",
            tool_name="read_file",
            arguments={"path": str(test_file)}
        )
        self.assertIn("print('hello from Bob')", read_res)

        # Clean up
        if test_file.exists():
            test_file.unlink()

    def test_agent_react_loop_with_tool(self):
        mgr = AgentTownManager.get_instance()
        import core.gemini as gemini
        orig_text = getattr(gemini, "text", None)

        test_file = root / "scratch" / "react_sample.py"
        if test_file.exists():
            test_file.unlink()

        turns = [
            # Turn 1: Call write_file tool
            '```json\n{"thought": "Writing the requested script", "tool": "write_file", "arguments": {"path": "' + str(test_file).replace('\\', '/') + '", "content": "# script\\nx = 42"}}\n```',
            # Turn 2: Provide final answer
            '```json\n{"thought": "Verified output", "final_answer": "Created the script successfully."}\n```'
        ]
        turn_idx = [0]

        def mock_text(contents, **kwargs):
            idx = turn_idx[0]
            turn_idx[0] += 1
            if idx < len(turns):
                return turns[idx]
            return "Done"

        try:
            gemini.text = mock_text
            mgr.dispatch_task("Bob", "Create react_sample.py", async_exec=False)
            bob = mgr.get_agent("Bob")
            self.assertEqual(bob.state, AgentState.COMPLETED)
            self.assertIn("Created the script successfully", bob.latest_result)
            self.assertTrue(test_file.exists())
            self.assertIn("x = 42", test_file.read_text(encoding="utf-8"))
        finally:
            if orig_text:
                gemini.text = orig_text
            if test_file.exists():
                test_file.unlink()

    def test_dispatch_task_sync_mock(self):
        mgr = AgentTownManager.get_instance()

        import core.gemini as gemini
        orig_text = getattr(gemini, "text", None)

        def mock_text(contents, **kwargs):
            return "Mock research report from agent."

        try:
            gemini.text = mock_text
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
            if orig_text:
                gemini.text = orig_text


if __name__ == "__main__":
    unittest.main()
