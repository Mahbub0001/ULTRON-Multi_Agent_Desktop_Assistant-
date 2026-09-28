import unittest
from unittest.mock import MagicMock, patch
import sys
from pathlib import Path

# Add project root to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

class TestMusicSingerPlayer(unittest.TestCase):
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

if __name__ == "__main__":

    unittest.main()
