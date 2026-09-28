# Specification: AI Singing & In-App Music Engine for ULTRON

**Date:** 2026-09-28  
**Author:** Pair Programming with Nibir  
**Status:** Approved  
**Topic:** AI Song Generation & In-App Background Audio Engine (`music_singer`)

---

## 1. Overview & Objective
Enable ULTRON to perform and play songs for Nibir sir with zero external API subscription costs and zero GPU hardware bottleneck on Intel Iris Xe graphics.

The system enforces a strict distinction based on user intent:
1. **AI Song Creation (`action='sing'`):** When Nibir sir asks ULTRON to create, compose, or make an AI song ("নিজে একটা গান বানিয়ে গাও", "আমার জন্য একটা গান বানাও", "AI দিয়ে গান ক্রিয়েট করো"), ULTRON composes lyrics and uses free cloud AI music generation endpoints to generate a new melodic song.
2. **Original Song Playback (`action='play'`):** When Nibir sir mentions the name of a specific known song or singer (e.g. "Tum Hi Ho গাও/শোনাও", "আমার সোনার বাংলা শোনাও", "Shape of You গাও"), ULTRON **never** recreates it with AI; instead, ULTRON directly streams and plays the **original audio recording** in the background without opening browser pop-ups.
3. **In-App Audio Controls (`action='pause'|'resume'|'stop'|'volume'|'status'`):** All playback runs headless inside Windows using native COM `WMPlayer.OCX` (Windows Media Player COM) or native background audio stream, completely controllable via voice.

---

## 2. Architecture & Components

```
User Voice Request ("Tum Hi Ho শোনাও" / "নিজে একটা গান বানিয়ে গাও")
       │
       ▼
   Gemini Live / ULTRON Router
       │
       ├─── Specific song name mentioned? ──► action='play', query="Tum Hi Ho"
       │                                            │
       │                                            ▼
       │                                     Direct Audio Extractor / Streamer
       │                                            │
       │                                            ▼
       └─── Request to compose/create? ────► action='sing', query="...", lyrics="..."
                                                    │
                                                    ▼
                                            Free Cloud AI Audio Generation
                                            (Hugging Face / Free AI Inference)
                                                    │
                                                    ▼
                                            Cached MP3/WAV in data/music_cache/
                                                    │
                                                    ▼
                                            In-App Native Audio Player (WMPlayer.OCX)
                                            (Background play, pause, resume, volume)
```

### Components:
1. **`actions/music_singer.py`**:
   - Discovers as an active ULTRON action.
   - Declares `music_singer` schema with actions: `sing`, `play`, `pause`, `resume`, `stop`, `volume`, `status`.
   - Houses `NativeAudioPlayer`: A singleton thread-safe wrapper around Windows `WMPlayer.OCX` providing:
     - `play_file(filepath)`
     - `play_stream(url)`
     - `pause()`
     - `resume()`
     - `stop()`
     - `set_volume(0-100)`
     - `get_status()`
   - Houses `AISongGenerator`:
     - Free cloud music inference querying open Gradio/HF endpoints or direct audio generation without required paid API keys.
     - Saves generated audio to `data/music_cache/`.
   - Houses `OriginalSongStreamer`:
     - Resolves the audio URL for the original song query via headless yt-dlp / audio scrapers.
     - Caches or streams directly to `NativeAudioPlayer` with zero browser popups.

2. **`core/prompt.txt`**:
   - Adds `[MUSIC & SINGING PROTOCOL]`:
     - Explicit routing instructions: If user mentions specific song title/artist -> `music_singer(action='play', query=...)`.
     - If user asks to compose/create a new song -> `music_singer(action='sing', query=..., lyrics=...)`.
     - Playback controls: `pause`, `resume`, `stop`, `volume`.
     - Strict rule: Never say "আমি গান গাইতে পারি না".

3. **`tests/test_music_singer.py`**:
   - Unit tests covering action declaration, player state transitions, query routing, and argument parsing.

---

## 3. Interfaces & Data Formats

### Tool Declaration
```json
{
  "name": "music_singer",
  "description": "Generate AI songs with melody, vocals, and lyrics, or play original songs/music in background without browser popups. Supports play, pause, resume, stop, and volume.",
  "parameters": {
    "type": "OBJECT",
    "properties": {
      "action": {
        "type": "STRING",
        "enum": ["sing", "play", "pause", "resume", "stop", "volume", "status"],
        "description": "Singing or playback action to execute."
      },
      "query": {
        "type": "STRING",
        "description": "Song name, artist, topic, or musical mood."
      },
      "lyrics": {
        "type": "STRING",
        "description": "Optional customized lyrics for AI composition."
      },
      "level": {
        "type": "INTEGER",
        "description": "Volume percentage from 0 to 100 for action='volume'."
      }
    },
    "required": ["action"]
  }
}
```

---

## 4. Error Handling & Edge Cases
- **No Internet / Network Error:** If cloud generation fails, cleanly falls back to original stream or notifies user without crashing.
- **Audio Overlap:** If a song is already playing when a new song is requested, the current track is cleanly stopped and released before starting the new track.
- **Background Execution:** All download, generation, and streaming operations run in asynchronous daemon threads so the UI and Gemini Live voice loop remain completely responsive.
- **Zero Browser Intrusion:** No browser windows (`chrome.exe` or `edge.exe`) are spawned during playback.

---

## 5. Testing Plan
- Test action discovery and JSON schema compliance.
- Test NativeAudioPlayer initialization and mock state transitions (play, pause, resume, stop, volume).
- Test routing: verify `action='play'` triggers original stream and `action='sing'` triggers AI generation pipeline.
- Verify full test suite passes with zero regressions.
