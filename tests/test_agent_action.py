import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from actions.agent_town import delegate_agent_task, TOOL


class TestAgentAction(unittest.TestCase):
    def test_tool_declaration(self):
        self.assertEqual(TOOL["name"], "delegate_agent_task")
        self.assertIn("parameters", TOOL)
        self.assertIn("agent", TOOL["parameters"]["properties"])
        self.assertIn("task", TOOL["parameters"]["properties"])

    def test_delegate_agent_task_execution(self):
        dispatched = []
        import core.agent_town as at
        mgr = at.AgentTownManager.get_instance()
        orig_dispatch = mgr.dispatch_task

        def mock_dispatch(agent_name, task, on_complete=None, async_exec=True):
            dispatched.append((agent_name, task))
            return True

        try:
            mgr.dispatch_task = mock_dispatch

            spoken = []
            def mock_speak(text):
                spoken.append(text)

            res = delegate_agent_task(
                {"agent": "Bob", "task": "Write a python script"},
                speak=mock_speak
            )
            self.assertIn("Bob", res)
            self.assertEqual(len(dispatched), 1)
            self.assertEqual(dispatched[0][0], "Bob")
            self.assertEqual(dispatched[0][1], "Write a python script")
            self.assertEqual(spoken, ["Delegating to Bob, sir."])
        finally:
            mgr.dispatch_task = orig_dispatch

    def test_agent_department_status_alice(self):
        from actions.agent_town import agent_department
        res = agent_department({"action": "status", "agent": "Alice"})
        self.assertIn("Alice", res)
        self.assertIn("Senior Research Analyst", res)
        self.assertIn("FREE", res)

    def test_agent_department_list(self):
        from actions.agent_town import agent_department
        res = agent_department({"action": "list"})
        self.assertIn("Alice", res)
        self.assertIn("Bob", res)
        self.assertIn("Carol", res)
        self.assertIn("Dave", res)

    def test_delegate_agent_task_status_fallback(self):
        res = delegate_agent_task({"agent": "Alice", "task": "is free"})
        self.assertIn("Alice", res)
        self.assertIn("FREE", res)

    def test_agent_department_tools_exposed(self):
        from actions.agent_town import TOOLS
        names = [t["name"] for t in TOOLS]
        self.assertIn("delegate_agent_task", names)
        self.assertIn("agent_department", names)


if __name__ == "__main__":
    unittest.main()
