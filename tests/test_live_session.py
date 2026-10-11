import asyncio
import unittest
from pathlib import Path
from unittest.mock import MagicMock, AsyncMock, patch
from google.genai import types
from main import LIVE_MODEL
from main import JarvisLive

class TestLiveSession(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.mock_ui = MagicMock()
        self.mock_ui.muted = False
        self.live = JarvisLive(ui=self.mock_ui)
        self.live._awake = True

    async def test_session_state_reset_on_connect(self):
        """Verify speaking and echo state is clean on session start."""
        self.live._is_speaking = True
        self.live._tail_until = 999999.0
        self.live.set_speaking(False, reset_tail=True)
        self.assertFalse(self.live._is_speaking)
        self.assertEqual(self.live._tail_until, 0.0)

    async def test_multi_turn_receive_loop(self):
        """Verify _receive_audio loops across turns without raising RuntimeError."""
        self.live.session = MagicMock()
        
        # Simulate 2 turns of LiveServerMessages
        msg_turn1 = MagicMock()
        msg_turn1.session_resumption_update = None
        msg_turn1.data = None
        msg_turn1.tool_call = None
        msg_turn1.server_content = MagicMock()
        msg_turn1.server_content.output_transcription = None
        msg_turn1.server_content.input_transcription = None
        msg_turn1.server_content.turn_complete = True

        msg_turn2 = MagicMock()
        msg_turn2.session_resumption_update = None
        msg_turn2.data = None
        msg_turn2.tool_call = None
        msg_turn2.server_content = MagicMock()
        msg_turn2.server_content.output_transcription = None
        msg_turn2.server_content.input_transcription = None
        msg_turn2.server_content.turn_complete = True

        call_count = 0
        async def fake_receive():
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                yield msg_turn1
            elif call_count == 2:
                yield msg_turn2
            else:
                # Cancel after 2 turns to finish test
                raise asyncio.CancelledError()

        self.live.session.receive = fake_receive
        self.live.audio_in_queue = asyncio.Queue()

        with self.assertRaises(asyncio.CancelledError):
            await self.live._receive_audio()

        self.assertEqual(call_count, 3)

    def test_set_speaking_updates_ui_state(self):
        """Verify set_speaking(False) transitions UI back to LISTENING if awake."""
        self.live._is_speaking = True
        self.live.set_speaking(False)
        self.assertFalse(self.live._is_speaking)
        self.mock_ui.set_state.assert_called_with("LISTENING")


class TestGemini38Config(unittest.TestCase):
    def test_gemini_38_live_config_preserves_sequential_tools(self):
        self.assertEqual(LIVE_MODEL, "gemini-3.8-live")
        live = JarvisLive.__new__(JarvisLive)
        live._resume_handle = None
        live._tuned_live = True
        live._action_registry = MagicMock()
        live._plugin_registry = MagicMock()
        declarations = [{"name": "background_task", "description": "Run later",
                         "behavior": "NON_BLOCKING", "parameters": {"type": "OBJECT", "properties": {}}}]
        with (patch("main.API_CONFIG_PATH", Path("missing-api-config.json")),
              patch("main.load_memory", return_value={}),
              patch("main.format_memory_for_prompt", return_value=""),
              patch("main._load_system_prompt", return_value=""),
              patch("main.get_turn_tuning", return_value={"enabled": False}),
              patch("main.get_media_resolution", return_value="default"),
              patch("main.get_response_language", return_value="Bengali"),
              patch.object(live._action_registry, "get_tool_declarations", return_value=declarations),
              patch.object(live._plugin_registry, "get_tool_declarations", return_value=[])):
            config = live._build_config()

        self.assertIsInstance(config, types.LiveConnectConfig)
        self.assertIn("[STABLE RESPONSE LANGUAGE]", config.system_instruction)
        self.assertIn("Reply in Bengali", config.system_instruction)
        self.assertIn("recognition errors", config.system_instruction)
        self.assertIsNone(config.thinking_config)
        self.assertIsNone(config.proactivity)
        behaviors = {decl.name: decl.behavior for decl in config.tools[0].function_declarations}
        self.assertEqual(behaviors["background_task"], types.Behavior.NON_BLOCKING)
        self.assertTrue(all(behavior != types.Behavior.UNSPECIFIED for behavior in behaviors.values()))
        self.assertTrue(all(behavior == types.Behavior.BLOCKING
                            for name, behavior in behaviors.items() if name != "background_task"))

if __name__ == "__main__":
    unittest.main()
