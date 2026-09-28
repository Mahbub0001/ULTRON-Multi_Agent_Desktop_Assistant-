"""
Google Calendar & Reminders Sync Plugin for ULTRON / JARVIS.

Allows querying events from Google Calendar, creating new events/reminders,
and getting upcoming schedules.

Uses Google Calendar v3 API via standard oauth2 / service account or fallback
local JSON / web integration if API credentials are not configured.
"""

from __future__ import annotations

import os
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path

def _get_base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent

BASE_DIR = _get_base_dir()
CREDENTIALS_PATH = BASE_DIR / "config" / "google_calendar_credentials.json"
EVENTS_CACHE_PATH = BASE_DIR / "config" / "local_calendar_events.json"

PLUGIN = {
    "name": "calendar_sync",
    "description": (
        "Google Calendar and Reminders integration. Query upcoming events/meetings, "
        "add new calendar events or reminders, and check daily schedule. "
        "Use this tool when the user asks about their schedule, meetings, calendar, "
        "or wants to add an event/reminder."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "The action to perform: 'get_events' (list upcoming events), 'add_event' (create an event/reminder), or 'get_today' (get today's schedule).",
            },
            "summary": {
                "type": "STRING",
                "description": "Title or summary of the event/reminder to add (required for add_event).",
            },
            "start_time": {
                "type": "STRING",
                "description": "Start time for the event in ISO format or relative text (e.g. '2026-09-29T10:00:00' or 'Tomorrow 3pm').",
            },
            "duration_minutes": {
                "type": "INTEGER",
                "description": "Duration of event in minutes (default: 30).",
            },
            "description": {
                "type": "STRING",
                "description": "Optional detailed description or location for the event.",
            },
            "days_ahead": {
                "type": "INTEGER",
                "description": "Number of days ahead to search for get_events (default: 7).",
            },
        },
        "required": ["action"],
    },
}

def _load_local_events() -> list[dict]:
    if EVENTS_CACHE_PATH.exists():
        try:
            with open(EVENTS_CACHE_PATH, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return []
    return []

def _save_local_events(events: list[dict]) -> None:
    EVENTS_CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(EVENTS_CACHE_PATH, "w", encoding="utf-8") as f:
        json.dump(events, f, indent=4, ensure_ascii=False)

def _try_google_api(action: str, params: dict) -> tuple[bool, str]:
    """
    Attempts to call Google Calendar API using google-api-python-client if credentials exist.
    Returns (success, result_message).
    """
    if not CREDENTIALS_PATH.exists():
        return False, "Google credentials file not found."

    try:
        from google.oauth2.credentials import Credentials
        from googleapiclient.discovery import build

        # Load token / credentials
        creds = Credentials.from_authorized_user_file(str(CREDENTIALS_PATH), ["https://www.googleapis.com/auth/calendar"])
        service = build("calendar", "v3", credentials=creds)

        if action in ("get_events", "get_today"):
            now = datetime.utcnow()
            days = 1 if action == "get_today" else params.get("days_ahead", 7)
            time_max = (now + timedelta(days=days)).isoformat() + "Z"
            time_min = now.isoformat() + "Z"

            events_result = service.events().list(
                calendarId="primary",
                timeMin=time_min,
                timeMax=time_max,
                singleEvents=True,
                orderBy="startTime"
            ).execute()

            items = events_result.get("items", [])
            if not items:
                return True, "No events found on your Google Calendar for this period."

            lines = ["Google Calendar Events:"]
            for item in items:
                start = item.get("start", {}).get("dateTime") or item.get("start", {}).get("date")
                summary = item.get("summary", "Untitled Event")
                lines.append(f"- {summary} at {start}")
            return True, "\n".join(lines)

        elif action == "add_event":
            summary = params.get("summary", "New Event")
            start_str = params.get("start_time")
            try:
                start_dt = datetime.fromisoformat(start_str) if start_str else datetime.now() + timedelta(hours=1)
            except Exception:
                start_dt = datetime.now() + timedelta(hours=1)

            duration = params.get("duration_minutes", 30)
            end_dt = start_dt + timedelta(minutes=duration)

            event_body = {
                "summary": summary,
                "description": params.get("description", ""),
                "start": {"dateTime": start_dt.isoformat(), "timeZone": "Asia/Dhaka"},
                "end": {"dateTime": end_dt.isoformat(), "timeZone": "Asia/Dhaka"},
            }

            created_event = service.events().insert(calendarId="primary", body=event_body).execute()
            return True, f"Sir, event '{summary}' successfully added to Google Calendar for {start_dt.strftime('%Y-%m-%d %I:%M %p')}."

    except Exception as e:
        return False, f"Google API error: {e}"

    return False, "Unknown action."

def run(parameters: dict, player=None, session_memory=None) -> str:
    """
    Main plugin execution handler.
    """
    action = parameters.get("action", "").lower()
    summary = parameters.get("summary", "")
    start_time_raw = parameters.get("start_time", "")
    duration = parameters.get("duration_minutes", 30)
    description = parameters.get("description", "")
    days_ahead = parameters.get("days_ahead", 7)

    # 1. Try official Google Calendar API first if credentials exist
    api_success, api_result = _try_google_api(action, parameters)
    if api_success:
        if player:
            try:
                player.write_log(f"ULTRON (Calendar): {api_result}")
            except Exception:
                pass
        return api_result

    # 2. Fallback to Local Calendar Engine & Storage
    local_events = _load_local_events()

    if action == "add_event":
        if not summary:
            return "Sir, please specify an event summary or title."

        try:
            start_dt = datetime.fromisoformat(start_time_raw) if start_time_raw else datetime.now() + timedelta(hours=1)
        except Exception:
            start_dt = datetime.now() + timedelta(hours=1)

        new_event = {
            "id": len(local_events) + 1,
            "summary": summary,
            "start": start_dt.strftime("%Y-%m-%d %H:%M"),
            "duration": duration,
            "description": description,
            "created_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        }

        local_events.append(new_event)
        _save_local_events(local_events)

        result_msg = f"Sir, event '{summary}' has been added to your local calendar for {start_dt.strftime('%B %d at %I:%M %p')}."

    elif action in ("get_events", "get_today"):
        now = datetime.now()
        max_date = now + timedelta(days=1 if action == "get_today" else days_ahead)

        upcoming = []
        for ev in local_events:
            try:
                ev_dt = datetime.strptime(ev["start"], "%Y-%m-%d %H:%M")
                if now <= ev_dt <= max_date:
                    upcoming.append(ev)
            except Exception:
                continue

        if not upcoming:
            result_msg = "Sir, you have no upcoming events or reminders scheduled."
        else:
            period_label = "today" if action == "get_today" else f"the next {days_ahead} days"
            lines = [f"Sir, here are your scheduled events for {period_label}:"]
            for ev in upcoming:
                lines.append(f"- {ev['summary']} - {ev['start']} ({ev['duration']} mins)")
            result_msg = "\n".join(lines)
    else:
        result_msg = f"Sir, unsupported calendar action: '{action}'."

    if player:
        try:
            player.write_log(f"ULTRON (Calendar): {result_msg}")
        except Exception:
            pass

    return result_msg
