"""Resolve user-facing Windows folders, including redirected OneDrive folders."""
from __future__ import annotations

import os
from pathlib import Path


def desktop_path() -> Path:
    if os.name == "nt":
        try:
            import ctypes
            import uuid
            from ctypes import wintypes

            class GUID(ctypes.Structure):
                _fields_ = [("Data1", wintypes.DWORD), ("Data2", wintypes.WORD),
                            ("Data3", wintypes.WORD), ("Data4", ctypes.c_ubyte * 8)]

            folder_id = GUID.from_buffer_copy(uuid.UUID("b4bfcc3a-db2c-424c-b029-7fe99a87c641").bytes_le)
            target = ctypes.c_wchar_p()
            shell = ctypes.windll.shell32
            shell.SHGetKnownFolderPath.argtypes = [ctypes.POINTER(GUID), wintypes.DWORD,
                                                    wintypes.HANDLE, ctypes.POINTER(ctypes.c_wchar_p)]
            shell.SHGetKnownFolderPath.restype = ctypes.c_long
            if shell.SHGetKnownFolderPath(ctypes.byref(folder_id), 0, None, ctypes.byref(target)) == 0:
                try:
                    return Path(target.value)
                finally:
                    ctypes.windll.ole32.CoTaskMemFree.argtypes = [ctypes.c_void_p]
                    ctypes.windll.ole32.CoTaskMemFree(ctypes.cast(target, ctypes.c_void_p))
        except Exception:
            pass
    return Path.home() / "Desktop"
