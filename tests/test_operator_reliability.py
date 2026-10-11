import unittest
from unittest.mock import patch

from core import universal_operator as operator
from core import uia


class TestOperatorReliability(unittest.TestCase):
    def test_failed_focus_never_controls_other_app(self):
        with (patch.object(operator, "get_active_window", return_value=operator.WindowContext(title="Chrome")),
              patch.object(operator, "focus_app_window", return_value=False),
              patch("core.uia.click") as click):
            result = operator.execute_smart_operation(app="Excel", intent="Save")
        self.assertTrue(result.startswith("Error:"))
        click.assert_not_called()

    def test_all_fallbacks_failed_is_not_reported_as_success(self):
        with (patch.object(operator, "get_active_window", return_value=operator.WindowContext(title="Notepad")),
              patch.object(operator, "_match_intent", return_value=None),
              patch("core.uia.available", return_value=False),
              patch("actions.computer_control._screen_find", return_value=None)):
            result = operator.execute_smart_operation(intent="Unknown control")
        self.assertTrue(result.startswith("Error:"))

    def test_visual_type_fallback_actually_types(self):
        with (patch.object(operator, "get_active_window", return_value=operator.WindowContext(title="Notepad")),
              patch.object(operator, "_match_intent", return_value=None),
              patch("core.uia.available", return_value=False),
              patch("actions.computer_control._screen_find", return_value=(20, 30)),
              patch("actions.computer_control._click"),
              patch("actions.computer_control._clipboard_paste") as paste,
              patch.object(operator.time, "sleep")):
            result = operator.execute_smart_operation(intent="editor", text="বাংলা", action_type="type")
        paste.assert_called_once_with("বাংলা")
        self.assertIn("Typed", result)

    def test_bengali_ui_matching_supports_reordered_words(self):
        self.assertGreater(uia.score("ফাইল সংরক্ষণ", "সংরক্ষণ ফাইল"), 0.3)
