import unittest
from unittest.mock import MagicMock, patch
import sys
from pathlib import Path

# Add project root to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

class TestMusicSingerPlayer(unittest.TestCase):
    def setUp(self):
        from actions.music_singer import NativeAudioPlayer
        NativeAudioPlayer._instance = None

    @patch("win32com.client.Dispatch")
    def test_player_controls(self, mock_dispatch):
        mock_wmp = MagicMock()
        mock_dispatch.return_value = mock_wmp
        
        from actions.music_singer import NativeAudioPlayer
        player = NativeAudioPlayer()
        
        # Test play_url
        player.play_url("http://example.com/audio.mp3")
        self.assertEqual(mock_wmp.URL, "http://example.com/audio.mp3")
        mock_wmp.controls.play.assert_called()
        
        # Test pause
        player.pause()
        mock_wmp.controls.pause.assert_called()
        
        # Test resume
        player.resume()
        mock_wmp.controls.play.assert_called()
        
        # Test stop
        player.stop()
        mock_wmp.controls.stop.assert_called()
        
        # Test volume
        player.set_volume(80)
        self.assertEqual(mock_wmp.settings.volume, 80)

        # Test status
        status = player.get_status()
        self.assertIn("playing", status)
        self.assertIn("paused", status)
        self.assertEqual(status["volume"], 80)

    @patch("actions.music_singer.requests.get")
    def test_generate_ai_song_success(self, mock_get):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.content = b"ID3\x03fake_mp3_data"
        mock_get.return_value = mock_response

        from actions.music_singer import generate_ai_song
        success, path, msg = generate_ai_song("cheerful birthday tune", "Happy birthday to you")
        self.assertTrue(success)
        self.assertTrue(Path(path).exists())

    @patch("actions.music_singer._get_stream_url")
    def test_music_singer_play_original(self, mock_stream):
        mock_stream.return_value = ("http://stream.example/song.mp3", "Tum Hi Ho - Arijit Singh")
        from actions.music_singer import music_singer
        res = music_singer(action="play", query="Tum Hi Ho")
        self.assertIn("Tum Hi Ho", res)

    def test_music_singer_volume_and_controls(self):
        from actions.music_singer import music_singer
        self.assertIn("paused", music_singer(action="pause").lower())
        self.assertIn("resumed", music_singer(action="resume").lower())
        self.assertIn("stopped", music_singer(action="stop").lower())
        self.assertIn("volume", music_singer(action="volume", level=75).lower())

    def test_music_singer_action_schema_compliance(self):
        from actions.music_singer import ACTION, TOOL, music_singer
        self.assertEqual(ACTION, TOOL)
        self.assertEqual(ACTION["name"], "music_singer")
        self.assertTrue(len(ACTION["description"]) > 0)
        self.assertEqual(ACTION["parameters"]["type"], "OBJECT")
        props = ACTION["parameters"]["properties"]
        self.assertIn("action", props)
        self.assertIn("query", props)
        self.assertIn("lyrics", props)
        self.assertIn("level", props)
        self.assertEqual(
            props["action"]["enum"],
            ["sing", "play", "pause", "resume", "stop", "volume", "status"]
        )
        self.assertIs(ACTION["handler"], music_singer)

        # Test ULTRON dynamic action loader compatibility
        from core.action_loader import _validate
        import actions.music_singer as mod
        rec = _validate(mod, "actions/music_singer.py")
        self.assertTrue(rec.valid, f"Action validation failed: {rec.error}")

if __name__ == "__main__":
    unittest.main()

