"""Regression coverage for opening a directory as a VS Code workspace."""
import unittest
from pathlib import Path
from unittest.mock import patch

from actions import file_controller as files


class FolderEditorWorkflowTests(unittest.TestCase):
    def test_folder_target_is_passed_to_vscode_not_explorer(self):
        folder = Path.cwd()
        with patch.object(files, "_open_folder_in_vscode", return_value="Verified") as code, \
                patch.object(files.os, "startfile", create=True) as explorer:
            result = files.file_controller({"action": "open_folder", "path": str(folder),
                                            "with_app": "vscode"})
        self.assertEqual(result, "Verified")
        code.assert_called_once_with(folder)
        explorer.assert_not_called()

    def test_unverified_launch_is_reported_as_error(self):
        folder = Path.cwd()
        with patch.object(files, "_find_vscode_executable", return_value="Code.exe"), \
                patch.object(files, "_vscode_shows_folder", return_value=False), \
                patch.object(files.time, "sleep"), \
                patch.object(files.subprocess, "Popen"):
            result = files._open_folder_in_vscode(folder)
        self.assertTrue(result.startswith("Error:"))
        self.assertIn("could not verify", result)

    def test_verified_launch_has_native_evidence(self):
        folder = Path.cwd()
        with patch.object(files, "_find_vscode_executable", return_value="Code.exe"), \
                patch.object(files, "_vscode_shows_folder", return_value=True), \
                patch.object(files.subprocess, "Popen") as launch:
            result = files._open_folder_in_vscode(folder)
        self.assertEqual(result.evidence["folder"], str(folder.resolve()))
        launch.assert_called_once()
        self.assertEqual(launch.call_args.args[0][-1], str(folder.resolve()))


if __name__ == "__main__":
    unittest.main()
