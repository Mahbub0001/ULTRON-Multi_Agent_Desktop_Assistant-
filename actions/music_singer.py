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
    
    if target_file.exists() and target_file.stat().st_size > 0:
        return True, str(target_file), f"Playing cached AI song: {query}"
        
    try:
        if not _HAS_REQUESTS:
            return False, "", "Requests module unavailable"
        resp = requests.get(url, timeout=35)
        if resp.status_code == 200 and len(resp.content) > 0:
            target_file.write_bytes(resp.content)
            return True, str(target_file), f"Generated and playing AI song for: '{query}'"
    except Exception as e:
        print(f"[MusicSinger] AI generation failed: {e}")
        
    return False, "", "AI song generation service temporarily unreachable"

