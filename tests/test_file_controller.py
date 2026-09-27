import unittest
import os
import shutil
import platform
from pathlib import Path
from actions.file_controller import (
    _is_safe_path,
    _resolve_path,
    create_folder,
    delete_file,
    create_file,
    read_file,
)

class TestFileController(unittest.TestCase):
    def test_safe_path_drives(self):
        """Verify user drives are allowed while OS system directories are blocked."""
        # Allowed user locations
        self.assertTrue(_is_safe_path(Path.home() / "Desktop"))
        self.assertTrue(_is_safe_path(Path("E:/ultron")))
        self.assertTrue(_is_safe_path(Path("E:/test_folder/nested")))
        self.assertTrue(_is_safe_path(Path("D:/my_data")))

        # Blocked critical system directories on Windows
        if platform.system() == "Windows":
            self.assertFalse(_is_safe_path(Path(r"C:\Windows")))
            self.assertFalse(_is_safe_path(Path(r"C:\Windows\System32\calc.exe")))
            self.assertFalse(_is_safe_path(Path(r"C:\Program Files")))
            self.assertFalse(_is_safe_path(Path(r"C:\Program Files\App")))
            self.assertFalse(_is_safe_path(Path(r"E:\System Volume Information")))
            self.assertFalse(_is_safe_path(Path(r"E:\$Recycle.Bin")))

    def test_resolve_path_formats(self):
        """Verify drive names, drive letters, and shortcuts are properly resolved."""
        self.assertEqual(_resolve_path("desktop"), Path.home() / "Desktop")
        self.assertEqual(_resolve_path("documents"), Path.home() / "Documents")
        
        if platform.system() == "Windows":
            self.assertEqual(_resolve_path("E:"), Path("E:\\"))
            self.assertEqual(_resolve_path("e:"), Path("E:\\"))
            self.assertEqual(_resolve_path("E:\\"), Path("E:\\"))
            self.assertEqual(_resolve_path("E:/"), Path("E:\\"))
            self.assertEqual(_resolve_path("e drive"), Path("E:\\"))
            self.assertEqual(_resolve_path("E drive"), Path("E:\\"))
            self.assertEqual(_resolve_path("E drive/my_sub"), Path("E:\\my_sub"))
            self.assertEqual(_resolve_path("e:my_sub"), Path("E:\\my_sub"))

    def test_create_and_delete_folder_on_e_drive(self):
        """Test creating and deleting a folder on E drive."""
        folder_name = "test_ultron_temp_unit_test"
        res = create_folder("E:", folder_name)
        expected_path = Path(f"E:\\{folder_name}")
        try:
            self.assertIn("Folder created", res)
            self.assertTrue(expected_path.exists())
            self.assertTrue(expected_path.is_dir())
        finally:
            if expected_path.exists():
                shutil.rmtree(expected_path, ignore_errors=True)

    def test_bengali_suffix_handling_in_create_folder(self):
        """Test creating folder with Bengali suffix words like 'নামায়' or 'নামে'."""
        res = create_folder("E:", "test_bengali_suffix_নামায়")
        expected_path = Path("E:\\test_bengali_suffix")
        try:
            self.assertIn("Folder created", res)
            self.assertTrue(expected_path.exists())
        finally:
            if expected_path.exists():
                shutil.rmtree(expected_path, ignore_errors=True)

if __name__ == "__main__":
    unittest.main()
