"""Fresh observations and deterministic postconditions for task execution."""
import asyncio
from pathlib import Path

from core.task_runtime import is_read_only, validate_value


async def observe_desktop() -> dict:
    from core.universal_operator import get_active_window
    try:
        return await asyncio.wait_for(asyncio.to_thread(lambda: get_active_window().as_dict()), 3)
    except Exception as exc:
        return {"error": f"Desktop observation unavailable: {type(exc).__name__}"}


class Verifier:
    def validate(self, check: dict, catalog: dict):
        kind = check.get("kind")
        if kind in {"file_exists", "file_contains"}:
            path = check.get("path")
            if not isinstance(path, str) or not Path(path).is_absolute():
                raise ValueError("Verification needs an absolute file path")
            if kind == "file_contains" and not check.get("text"):
                raise ValueError("File verification needs expected text")
            if kind == "file_exists" and not isinstance(check.get("exists", True), bool):
                raise ValueError("exists must be boolean")
        elif kind == "window_title":
            if not isinstance(check.get("text"), str) or not check["text"].strip():
                raise ValueError("Window verification needs expected title text")
        elif kind == "tool":
            name, args = check.get("tool"), check.get("arguments", {})
            if name not in catalog or not isinstance(args, dict) or not is_read_only(name, args):
                raise ValueError("Verification must use a read-only catalog tool")
            validate_value(args, catalog[name].get("parameters", {"type": "OBJECT"}))
            if not isinstance(check.get("contains"), str) or not check["contains"].strip():
                raise ValueError("Verification needs expected state text")
        else:
            raise ValueError("Unknown verification kind")

    async def __call__(self, check, call, observe):
        try:
            kind = check["kind"]
            if kind == "file_exists":
                path = Path(check["path"])
                exists = await asyncio.to_thread(path.exists)
                return exists == check.get("exists", True), f"{path}: exists={exists}"
            if kind == "file_contains":
                path = Path(check["path"])
                def read():
                    if path.stat().st_size > 2_000_000:
                        raise ValueError("File too large for text verification")
                    return path.read_text(encoding="utf-8")
                content = await asyncio.to_thread(read)
                matched = check["text"] in content
                return matched, f"{path}: expected text found={matched}"
            if kind == "window_title":
                snapshot = await observe()
                return check["text"].casefold() in snapshot.get("title", "").casefold(), str(snapshot)
            outcome = await call(check["tool"], check.get("arguments", {}))
            passed = outcome["status"] == "executed" and check["contains"].casefold() in str(outcome["result"]).casefold()
            return passed, outcome["result"]
        except Exception as exc:
            return False, f"Verification failed: {exc}"
