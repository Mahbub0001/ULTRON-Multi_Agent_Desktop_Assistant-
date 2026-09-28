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
from typing import Optional, Dict, Any, Tuple

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

try:
    import yt_dlp
    _HAS_YTDLP = True
except ImportError:
    _HAS_YTDLP = False


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
        self.volume = 80
        if _HAS_WIN32COM:
            try:
                # Dispatched instance
                self.wmp = win32com.client.Dispatch("WMPlayer.OCX")
                self.wmp.settings.autoStart = True
                self.wmp.settings.volume = 80
            except Exception as e:
                print(f"[MusicSinger] ⚠️ Failed to initialize WMPlayer.OCX: {e}")

    def play_url(self, url: str, title: str = "Audio Track") -> bool:
        self.current_title = title
        self.is_playing = True
        self.is_paused = False
        if not self.wmp:
            return True
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
        self.is_paused = True
        if not self.wmp:
            return True
        try:
            self.wmp.controls.pause()
            return True
        except Exception as e:
            print(f"[MusicSinger] Error pausing: {e}")
            return False

    def resume(self) -> bool:
        self.is_paused = False
        self.is_playing = True
        if not self.wmp:
            return True
        try:
            self.wmp.controls.play()
            return True
        except Exception as e:
            print(f"[MusicSinger] Error resuming: {e}")
            return False

    def stop(self) -> bool:
        self.is_playing = False
        self.is_paused = False
        if not self.wmp:
            return True
        try:
            self.wmp.controls.stop()
            return True
        except Exception as e:
            print(f"[MusicSinger] Error stopping: {e}")
            return False

    def set_volume(self, level: int) -> int:
        try:
            level = max(0, min(100, int(level)))
        except (ValueError, TypeError):
            level = 80
        self.volume = level
        if not self.wmp:
            return level
        try:
            self.wmp.settings.volume = level
            return level
        except Exception:
            return level

    def get_status(self) -> dict:
        vol = self.wmp.settings.volume if (self.wmp and hasattr(self.wmp, "settings")) else self.volume
        return {
            "playing": self.is_playing and not self.is_paused,
            "paused": self.is_paused,
            "current_title": self.current_title,
            "volume": vol
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


def _get_stream_url(query: str) -> tuple[Optional[str], str]:
    """Headless audio stream URL extraction using yt-dlp with zero browser popups."""
    clean_query = query.strip()
    if not clean_query:
        return None, ""

    if _HAS_YTDLP:
        try:
            search_target = clean_query
            if not (search_target.startswith("http://") or search_target.startswith("https://")):
                search_target = f"ytsearch1:{search_target}"

            ydl_opts = {
                "format": "bestaudio/best",
                "noplaylist": True,
                "quiet": True,
                "no_warnings": True,
                "skip_download": True,
            }
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info = ydl.extract_info(search_target, download=False)
                if info:
                    if "entries" in info:
                        entries = info.get("entries") or []
                        if not entries:
                            return None, ""
                        info = entries[0]
                    stream_url = info.get("url")
                    title = info.get("title", clean_query)
                    if stream_url:
                        return stream_url, title
        except Exception as e:
            print(f"[MusicSinger] Error resolving stream URL via yt-dlp: {e}")

    return None, ""


def resolve_original_audio_stream(query: str) -> tuple[bool, str, str]:
    """
    Headless audio stream resolution for original songs with zero browser popups.
    Returns: (success, stream_url_or_empty, title_or_error_message)
    """
    clean_query = query.strip()
    if not clean_query:
        return False, "", "No song title or search query provided."

    url, title = _get_stream_url(clean_query)
    if url:
        return True, url, title
    return False, "", f"Could not resolve audio stream for '{clean_query}'"


def music_singer(
    action: str = "status",
    query: str = "",
    lyrics: str = "",
    level: int = 80,
    parameters: dict | None = None,
    **kwargs: Any,
) -> str:
    """
    Main dispatcher for AI singing and background music engine.
    Supports actions: 'sing', 'play', 'pause', 'resume', 'stop', 'volume', 'status'.
    """
    if parameters and isinstance(parameters, dict):
        action = parameters.get("action", action)
        query = parameters.get("query", query)
        lyrics = parameters.get("lyrics", lyrics)
        level = parameters.get("level", level)

    action = str(action or "status").strip().lower()
    player = NativeAudioPlayer()

    if action == "play":
        if not query or not str(query).strip():
            return "Please specify a song title or artist to play."
        success, url_or_path, title_or_msg = resolve_original_audio_stream(str(query).strip())
        if success:
            if player.play_url(url_or_path, title=title_or_msg):
                return f"Now playing original track: '{title_or_msg}' in the background."
            return f"Failed to play audio stream for '{title_or_msg}'."
        return f"Could not find or stream '{query}': {title_or_msg}"

    elif action == "sing":
        if not query or not str(query).strip():
            return "Please provide a theme, mood, or title for the AI song to compose."
        success, url_or_path, msg = generate_ai_song(str(query).strip(), str(lyrics or "").strip())
        if success:
            if player.play_url(url_or_path, title=f"AI Song: {query}"):
                return f"Generated and singing AI song: {msg}"
            return f"AI song was generated but could not be played: {msg}"
        return f"Failed to compose AI song: {msg}"

    elif action == "pause":
        player.pause()
        return "Audio playback paused."

    elif action == "resume":
        player.resume()
        return "Audio playback resumed."

    elif action == "stop":
        player.stop()
        return "Audio playback stopped."

    elif action == "volume":
        try:
            vol = int(level)
        except (ValueError, TypeError):
            vol = 80
        set_vol = player.set_volume(vol)
        return f"Audio volume set to {set_vol}%."

    elif action == "status":
        status = player.get_status()
        playing_str = "playing" if status.get("playing") else ("paused" if status.get("paused") else "stopped")
        return (
            f"Audio player status: {playing_str}, volume={status.get('volume', 0)}%, "
            f"current_track='{status.get('current_title', 'None')}'."
        )

    return f"Unknown music action '{action}'. Supported actions: sing, play, pause, resume, stop, volume, status."


ACTION: dict[str, Any] = {
    "name": "music_singer",
    "description": (
        "AI Singing and Background Music Engine. Use action='play' to search and play "
        "original songs, tracks, or background music directly in the background with zero browser popups. "
        "Use action='sing' to compose and sing brand new AI songs/tunes. "
        "Use action='pause', 'resume', 'stop', 'volume' (level 0-100), or 'status' to control playback."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "enum": ["sing", "play", "pause", "resume", "stop", "volume", "status"],
                "description": "Action to perform: 'sing' (compose AI song), 'play' (stream original song), 'pause', 'resume', 'stop', 'volume', 'status'"
            },
            "query": {
                "type": "STRING",
                "description": "Song title, artist name, mood, or musical theme (required for 'play' and 'sing')"
            },
            "lyrics": {
                "type": "STRING",
                "description": "Optional lyrics or custom words for the AI song (for action='sing')"
            },
            "level": {
                "type": "INTEGER",
                "description": "Volume level from 0 to 100 (for action='volume', default 80)"
            }
        },
        "required": ["action"]
    },
    "handler": music_singer,
}

TOOL = ACTION

__all__ = [
    "NativeAudioPlayer",
    "generate_ai_song",
    "resolve_original_audio_stream",
    "_get_stream_url",
    "music_singer",
    "ACTION",
    "TOOL",
]
