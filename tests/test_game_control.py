import unittest
from unittest.mock import patch

from actions.game_control import GAMES, TOOL, game_control
from core.action_loader import _validate


class TestToolShape(unittest.TestCase):
    def test_tool_validates(self):
        rec = _validate(TOOL, "game_control.py")
        self.assertTrue(rec.valid, rec.error)
        self.assertEqual(rec.name, "game_control")

    def test_required_action(self):
        self.assertIn("'action' is required", game_control({}))


class TestLibrary(unittest.TestCase):
    def test_list_shows_library(self):
        out = game_control({"action": "list"})
        self.assertIn("Blox Fruits", out)
        self.assertIn("roblox", out)

    def test_blox_fruits_asset_id(self):
        self.assertEqual(GAMES["blox fruits"]["asset_id"], 2753915549)

    def test_play_without_game_errors(self):
        out = game_control({"action": "play"})
        self.assertIn("'game' is required", out)

    def test_unknown_action(self):
        out = game_control({"action": "respawn", "game": "blox fruits"})
        self.assertIn("Unknown action", out)


class TestPlay(unittest.TestCase):
    def test_fuzzy_title_resolves_to_blox_fruits(self):
        """'blox fruit' (singular) must still hit the Roblox entry."""
        with patch("actions.game_control._play_roblox",
                   return_value="LAUNCHED") as play:
            out = game_control({"action": "play", "game": "blox fruit"})
        self.assertEqual(out, "LAUNCHED")
        entry = play.call_args[0][0]
        self.assertEqual(entry["asset_id"], 2753915549)
        self.assertEqual(entry["engine"], "roblox")

    def test_unknown_game_falls_back_to_app_launch(self):
        with patch("actions.open_app.open_app",
                   return_value="opened as app") as opener:
            out = game_control({"action": "play", "game": "Chess Titans"})
        self.assertIn("not in the game library", out)
        self.assertIn("opened as app", out)
        opener.assert_called_once()

    def test_app_engine_launches_via_open_app(self):
        with patch("actions.game_control._launch_app",
                   return_value="Minecraft opened."), \
             patch("actions.game_control._wait_for", return_value=True):
            out = game_control({"action": "play", "game": "minecraft"})
        self.assertIn("running", out)


class TestStop(unittest.TestCase):
    def test_stop_when_not_running(self):
        with patch("actions.game_control._kill", return_value=[]) as kill:
            out = game_control({"action": "stop", "game": "blox fruits"})
        self.assertIn("not running", out)
        kill.assert_called_once()

    def test_stop_reports_killed_process(self):
        with patch("actions.game_control._kill",
                   return_value=["robloxplayerbeta.exe (4242)"]):
            out = game_control({"action": "stop", "game": "roblox"})
        self.assertIn("robloxplayerbeta.exe (4242)", out)


if __name__ == "__main__":
    unittest.main()
