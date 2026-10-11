"""Agent execution contracts shared by Alice, Bob, Carol and Dave."""
import unittest
from unittest.mock import patch

from core.agent_town import AgentState, AgentTownManager


class TestAgentTownWorkflow(unittest.TestCase):
    def setUp(self):
        self.manager = AgentTownManager()
        self.memory = patch.object(self.manager, "_save_memory")
        self.memory.start()
        self.addCleanup(self.memory.stop)

    def test_busy_agent_keeps_original_assignment(self):
        for agent in self.manager.get_all_agents():
            agent.state = AgentState.WORKING
            agent.current_task = "Original task"
            with self.subTest(agent=agent.name):
                self.assertFalse(self.manager.dispatch_task(agent.name, "New task", async_exec=False))
                self.assertEqual(agent.current_task, "Original task")

    def test_tool_failure_requires_repair_before_success(self):
        for agent in self.manager.get_all_agents():
            agent.state = AgentState.IDLE
            answers = iter([
                '{"tool":"write_file","arguments":{"path":"x","content":"ok"}}',
                '{"final_answer":"Done"}',
                '{"tool":"read_file","arguments":{"path":"x"}}',
                '{"final_answer":"Verified after repair"}',
            ])
            outputs = iter(["Error: disk full", "ok"])
            with self.subTest(agent=agent.name), patch("core.gemini.text", side_effect=lambda *_a, **_k: next(answers)), \
                    patch.object(self.manager, "execute_agent_tool", side_effect=lambda *_a: next(outputs)):
                self.assertTrue(self.manager.dispatch_task(agent.name, "Create x", async_exec=False))
                self.assertEqual(agent.state, AgentState.COMPLETED)
                self.assertEqual(agent.latest_result, "Verified after repair")

    def test_large_tool_result_is_bounded_in_next_prompt(self):
        agent = self.manager.get_agent("Alice")
        prompts = []
        def answer(prompt, **_kwargs):
            prompts.append(prompt)
            if len(prompts) == 1:
                return '{"tool":"web_search","arguments":{"query":"topic"}}'
            return '{"final_answer":"Summary"}'
        with patch("core.gemini.text", side_effect=answer), \
                patch.object(self.manager, "execute_agent_tool", return_value="a" * 100_000):
            self.manager.dispatch_task("Alice", "Research topic", async_exec=False)
        self.assertLess(len(prompts[1]), 8_000)

    def test_nonzero_command_exit_cannot_be_reported_as_success(self):
        agent = self.manager.get_agent("Bob")
        answers = iter(['{"tool":"run_command","arguments":{"command":"test"}}',
                        '{"final_answer":"Tests passed"}'])
        with patch("core.gemini.text", side_effect=lambda *_a, **_k: next(answers, "")), \
                patch.object(self.manager, "execute_agent_tool", return_value="Exit Code: 1\nFAILED"):
            self.manager.dispatch_task("Bob", "Run tests", async_exec=False)
        self.assertEqual(agent.state, AgentState.ERROR)

