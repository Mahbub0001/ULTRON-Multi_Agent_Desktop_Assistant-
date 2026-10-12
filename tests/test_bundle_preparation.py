import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import prepare_app_bundle as bundle


class BundlePreparation(unittest.TestCase):
    def test_private_settings_and_memory_are_not_packaged(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "scratch") as folder:
            root = Path(folder)
            for directory in ("core", "config", "actions", "plugins", "memory", "dist/ULTRON"):
                (root / directory).mkdir(parents=True)
            (root / "config/api_keys.json").write_text('{"gemini_api_key":"private-test-key"}')
            (root / "memory/long_term.json").write_text('{"private":"user data"}')
            (root / "core/face_model.obj").write_text("test mesh")
            (root / "plugins/feature.py").write_text("# packaged tool")
            (root / "plugins/__pycache__").mkdir()
            (root / "plugins/__pycache__/feature.pyc").write_bytes(b"cache")
            (root / "dist/ULTRON/_internal").mkdir()
            (root / "dist/ULTRON/_internal/ucrtbase.dll").write_bytes(b"foreign runtime")
            with patch.object(bundle, "ROOT", root), patch.object(bundle, "DIST_APP", root / "dist/ULTRON"):
                bundle.setup_bundle(root / "dist/ULTRON")
            app = root / "dist/ULTRON"
            self.assertEqual(json.loads((app / "config/api_keys.json").read_text())["gemini_api_key"], "")
            self.assertEqual(json.loads((app / "memory/long_term.json").read_text()), {})
            self.assertTrue((app / "_internal/core/face_model.obj").is_file())
            self.assertFalse((app / "plugins/__pycache__").exists())
            self.assertFalse((app / "_internal/ucrtbase.dll").exists())

    def test_destination_outside_workspace_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "scratch") as folder:
            root = Path(folder) / "workspace"
            root.mkdir()
            with patch.object(bundle, "ROOT", root), patch.object(bundle, "DIST_APP", root / "dist/ULTRON"):
                with self.assertRaises(ValueError):
                    bundle.setup_bundle(Path(folder) / "outside")
