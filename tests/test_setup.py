"""Setup must resolve its inputs independently of the caller's directory."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("ultron_setup", ROOT / "setup.py")
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class TestSetup(unittest.TestCase):
    def test_installer_uses_absolute_requirements_path(self):
        with patch.object(setup, "_run") as run, \
                patch.object(setup, "_check_python"), \
                patch.object(setup, "_check_assets"), \
                patch.object(setup, "OS", "Test"), \
                patch("builtins.print"):
            setup.main()
        requirements_path = Path(run.call_args_list[0].args[1][-1])
        self.assertTrue(requirements_path.is_absolute())
        self.assertEqual(requirements_path, ROOT / "requirements.txt")

    def test_package_metadata_reads_lowercase_readme(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Path(directory)
            (project / "readme.md").write_text("Package documentation", encoding="utf-8")
            with patch.object(setup, "HERE", project), \
                    patch("setuptools.setup") as build, \
                    patch("setuptools.find_packages", return_value=[]):
                setup._run_setuptools()
            self.assertEqual(build.call_args.kwargs["long_description"], "Package documentation")
