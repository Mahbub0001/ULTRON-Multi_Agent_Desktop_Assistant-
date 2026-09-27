import asyncio
import unittest
from unittest.mock import MagicMock, AsyncMock, patch
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

if __name__ == "__main__":
    unittest.main()
