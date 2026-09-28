# AI Singing & In-App Music Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `actions/music_singer.py` to allow ULTRON to generate new AI songs with free cloud audio models and play original songs directly in the background without browser popups, with full voice controls (play, pause, resume, stop, volume).

**Architecture:** An action module (`actions/music_singer.py`) with three specialized sub-components: `NativeAudioPlayer` (Win32 COM `WMPlayer.OCX` headless player), `AISongGenerator` (free cloud text-to-music / audio synthesis), and `OriginalSongStreamer` (headless audio resolver for specific songs). Integrated into ULTRON's dynamic action discovery and `core/prompt.txt`.

**Tech Stack:** Python 3.10+, `win32com.client` (Windows Media Player COM), `urllib`/`requests`, `subprocess`, Python standard library `unittest`.

## Global Constraints
- Zero API cost: No paid API keys or subscriptions required.
- Zero GPU bottleneck: Cloud inference or lightweight streaming; no local heavy PyTorch CUDA requirements on Intel Iris Xe.
- Zero browser popups: No `chrome.exe` or `edge.exe` windows opened for music playback.
- Strict Intent Routing: Specific song title/artist -> `action='play'` (original song). Create/compose new song -> `action='sing'` (AI composition).
- Never say "আমি গান গাইতে পারি না" in `core/prompt.txt`.

---

### Task 1: Native Audio Player Engine (`actions/music_singer.py`)

**Files:**
- Create: `actions/music_singer.py`
- Test: `tests/test_music_singer.py`

**Interfaces:**
- Produces: `NativeAudioPlayer` class with methods:
  - `play_url(url: str) -> bool`
  - `pause() -> bool`
  - `resume() -> bool`
  - `stop() -> bool`
  - `set_volume(level: int) -> int`
  - `get_status() -> dict`

- [ ] **Step 1: Write the failing unit test for `NativeAudioPlayer`**

In `tests/test_music_singer.py`:
```python
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

if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'actions.music_singer'`

- [ ] **Step 3: Implement `NativeAudioPlayer` in `actions/music_singer.py`**

Create `actions/music_singer.py` with `NativeAudioPlayer` and singleton instance management.

```python
"""
actions/music_singer.py
AI Singing and In-App Background Audio Engine for ULTRON.
"""
from __future__ import annotations

import os
import sys
import json
import time
import threading
import urllib.parse
from pathlib import Path
from typing import Optional, Dict, Any

try:
    import win32com.client
    _HAS_WIN32COM = True
except ImportError:
    _HAS_WIN32COM = False

try:
    import requests
    _HAS_REQUESTS = True
except ImportError:
    _HAS_REQUESTS = False


def _get_base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent

BASE_DIR = _get_base_dir()
MUSIC_CACHE_DIR = BASE_DIR / "data" / "music_cache"
MUSIC_CACHE_DIR.mkdir(parents=True, exist_ok=True)


class NativeAudioPlayer:
    """Thread-safe background audio player using Windows Media Player COM."""
    _instance: Optional[NativeAudioPlayer] = None
    _lock = threading.Lock()

    def __new__(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super().__new__(cls)
                cls._instance._init_player()
            return cls._instance

    def _init_player(self):
        self.wmp = None
        self.current_title = "None"
        self.is_playing = False
        self.is_paused = False
        if _HAS_WIN32COM:
            try:
                # Dispatched instance
                self.wmp = win32com.client.Dispatch("WMPlayer.OCX")
                self.wmp.settings.autoStart = True
                self.wmp.settings.volume = 85
            except Exception as e:
                print(f"[MusicSinger] ⚠️ Failed to initialize WMPlayer.OCX: {e}")

    def play_url(self, url: str, title: str = "Audio Track") -> bool:
        if not self.wmp:
            return False
        try:
            self.stop()
            self.current_title = title
            self.wmp.URL = url
            self.wmp.controls.play()
            self.is_playing = True
            self.is_paused = False
            return True
        except Exception as e:
            print(f"[MusicSinger] Error playing URL: {e}")
            return False

    def pause(self) -> bool:
        if not self.wmp or not self.is_playing:
            return False
        try:
            self.wmp.controls.pause()
            self.is_paused = True
            return True
        except Exception as e:
            print(f"[MusicSinger] Error pausing: {e}")
            return False

    def resume(self) -> bool:
        if not self.wmp:
            return False
        try:
            self.wmp.controls.play()
            self.is_paused = False
            self.is_playing = True
            return True
        except Exception as e:
            print(f"[MusicSinger] Error resuming: {e}")
            return False

    def stop(self) -> bool:
        if not self.wmp:
            return False
        try:
            self.wmp.controls.stop()
            self.is_playing = False
            self.is_paused = False
            return True
        except Exception as e:
            print(f"[MusicSinger] Error stopping: {e}")
            return False

    def set_volume(self, level: int) -> int:
        if not self.wmp:
            return 0
        try:
            level = max(0, min(100, int(level)))
            self.wmp.settings.volume = level
            return level
        except Exception:
            return 0

    def get_status(self) -> dict:
        return {
            "playing": self.is_playing and not self.is_paused,
            "paused": self.is_paused,
            "current_title": self.current_title,
            "volume": self.wmp.settings.volume if self.wmp else 0
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit Task 1**

```bash
git add actions/music_singer.py tests/test_music_singer.py
git commit -m "feat(music): add NativeAudioPlayer engine with WMPlayer.OCX integration"
```

---

### Task 2: Free AI Song Generation Pipeline (`sing` Action)

**Files:**
- Modify: `actions/music_singer.py`
- Test: `tests/test_music_singer.py`

**Interfaces:**
- Produces: `generate_ai_song(query: str, lyrics: str = "") -> tuple[bool, str, str]`
- Returns: `(success, audio_filepath_or_url, message)`

- [ ] **Step 1: Write unit test for AI song generation**

In `tests/test_music_singer.py`, add:
```python
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: FAIL with `ImportError: cannot import name 'generate_ai_song'`

- [ ] **Step 3: Implement `generate_ai_song` in `actions/music_singer.py`**

Implement free cloud audio synthesis via free Pollinations / Hugging Face audio APIs with clean disk caching in `data/music_cache/`.

```python
def generate_ai_song(query: str, lyrics: str = "") -> tuple[bool, str, str]:
    """Generates an AI audio track using free cloud inference and caches it locally."""
    clean_prompt = query.strip()
    if lyrics.strip():
        clean_prompt = f"{clean_prompt}. Lyrics: {lyrics.strip()[:200]}"
    
    encoded_prompt = urllib.parse.quote(clean_prompt[:250])
    # 100% free cloud audio generation endpoint
    url = f"https://text.pollinations.ai/{encoded_prompt}?model=audio"
    
    cache_filename = f"ai_song_{abs(hash(clean_prompt)) % 1000000}.mp3"
    target_file = MUSIC_CACHE_DIR / cache_filename
    
    if target_file.exists() and target_file.stat().st_size > 1024:
        return True, str(target_file), f"Playing cached AI song: {query}"
        
    try:
        if not _HAS_REQUESTS:
            return False, "", "Requests module unavailable"
        resp = requests.get(url, timeout=35)
        if resp.status_code == 200 and len(resp.content) > 1024:
            target_file.write_bytes(resp.content)
            return True, str(target_file), f"Generated and playing AI song for: '{query}'"
    except Exception as e:
        print(f"[MusicSinger] AI generation failed: {e}")
        
    return False, "", "AI song generation service temporarily unreachable"
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit Task 2**

```bash
git add actions/music_singer.py tests/test_music_singer.py
git commit -m "feat(music): add free AI song generation and local caching"
```

---

### Task 3: Original Song Background Streamer (`play` Action) & Main Dispatcher

**Files:**
- Modify: `actions/music_singer.py`
- Test: `tests/test_music_singer.py`

**Interfaces:**
- Produces:
  - `resolve_original_audio_stream(query: str) -> tuple[bool, str, str]`
  - `ACTION` dictionary specification
  - `music_singer(action: str, query: str = "", lyrics: str = "", level: int = 80) -> str`

- [ ] **Step 1: Write unit tests for original audio resolution and `music_singer` dispatch**

In `tests/test_music_singer.py`:
```python
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: FAIL with `ImportError: cannot import name 'music_singer'`

- [ ] **Step 3: Implement original stream resolver and `music_singer` entrypoint**

In `actions/music_singer.py`:
- Use `yt-dlp` or headless audio URL resolver to extract direct audio streams without spawning browsers.
- Expose `ACTION` declaration conforming to ULTRON dynamic action loader.
- Expose `def music_singer(action: str, query: str = "", lyrics: str = "", level: int = 80) -> str:`.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m unittest tests/test_music_singer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit Task 3**

```bash
git add actions/music_singer.py tests/test_music_singer.py
git commit -m "feat(music): implement original song background streamer and action dispatcher"
```

---

### Task 4: Prompt Integration & Strict Routing Rules (`core/prompt.txt`)

**Files:**
- Modify: `core/prompt.txt`

- [ ] **Step 1: Add `[MUSIC & SINGING PROTOCOL]` to `core/prompt.txt`**

Specify:
- When asked to sing a specific song by title/artist -> ALWAYS call `music_singer(action='play', query=...)`.
- When asked to create/compose a song -> call `music_singer(action='sing', query=..., lyrics=...)`.
- When asked to pause, resume, stop, or change volume -> call `music_singer(action='pause'|'resume'|'stop'|'volume')`.
- STRICT RULE: NEVER say "আমি গান গাইতে পারি না" (I cannot sing) or refuse singing requests.

- [ ] **Step 2: Commit Task 4**

```bash
git add core/prompt.txt
git commit -m "chore(prompt): add music and singing protocol with strict routing rules"
```

---

### Task 5: Full Test Suite Verification

**Files:**
- Test: All test files in `tests/`

- [ ] **Step 1: Run full unit test suite**

Run: `python -m unittest discover -s tests -p "test_*.py" -v`  
Expected: All 48+ tests PASS.

- [ ] **Step 2: Git push to origin/main**

Run: `git push origin main`
