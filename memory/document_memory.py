"""Lossless local text memory for documents the user explicitly asks to remember."""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

DOCUMENT_DIR = Path(__file__).resolve().parent / "documents"
MAX_DOCUMENT_CHARS = 1_000_000


def remember_document(path: Path, pages: list[str]) -> dict:
    pages = [str(page or "") for page in pages]
    text = "\n".join(pages)
    if not text.strip():
        raise ValueError("No readable text was found; the document may be scanned and need OCR")
    if len(text) > MAX_DOCUMENT_CHARS:
        raise ValueError("Document is too large to remember in full; nothing was stored")
    source = str(path.resolve())
    document_id = hashlib.sha256(source.casefold().encode("utf-8")).hexdigest()[:16]
    record = {"id": document_id, "name": path.name, "source": source,
              "pages": pages, "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest()}
    DOCUMENT_DIR.mkdir(parents=True, exist_ok=True)
    destination = DOCUMENT_DIR / f"{document_id}.json"
    temporary = destination.with_suffix(".tmp")
    temporary.write_text(json.dumps(record, ensure_ascii=False), encoding="utf-8")
    temporary.replace(destination)
    saved = json.loads(destination.read_text(encoding="utf-8"))
    if saved["sha256"] != record["sha256"] or saved["pages"] != pages:
        raise OSError("Stored document failed read-back verification")
    from memory.memory_manager import update_memory
    kind = "CV/resume" if re.search(r"\b(cv|resume|résumé)\b", path.stem, re.I) else "document"
    pointer = (f"Full {kind} '{path.name}' stored locally ({len(pages)} pages, {len(text)} characters). "
               "Use recall_memory with a topic or the document name for details.")
    update_memory({"notes": {f"document_{document_id}": pointer}})
    from memory.memory_manager import load_memory
    if load_memory().get("notes", {}).get(f"document_{document_id}", {}).get("value") != pointer:
        raise OSError("Document index failed read-back verification")
    return {"name": path.name, "id": document_id, "pages": len(pages),
            "characters": len(text), "sha256": record["sha256"]}


def search_documents(query: str, limit: int = 8) -> list[str]:
    if not DOCUMENT_DIR.is_dir():
        return []
    words = [word for word in re.findall(r"[\w\u0980-\u09FF]+", query.casefold()) if len(word) > 1]
    translations = {"শিক্ষা": "education", "শিক্ষাগত": "education", "যোগ্যতা": "qualification",
                    "পড়াশোনা": "education", "অভিজ্ঞতা": "experience", "দক্ষতা": "skill",
                    "ইমেইল": "email", "ফোন": "phone", "ঠিকানা": "address",
                    "প্রজেক্ট": "project", "কাজ": "work"}
    words = [translations.get(word, word) for word in words
             if word not in {"আমার", "আমি", "তার", "কি", "কী", "বলো", "দেখাও", "the", "my", "in"}]
    broad = {"cv", "resume", "résumé", "document", "pdf", "সিভি", "রেজুমে"}
    results = []
    for file in DOCUMENT_DIR.glob("*.json"):
        try:
            record = json.loads(file.read_text(encoding="utf-8"))
            name = str(record["name"])
            pages = record["pages"]
            if not isinstance(pages, list):
                continue
            if not words:
                results.append((1, f"Stored document: {name} ({len(pages)} pages). Search by topic to read details."))
                continue
            for page_no, page in enumerate(pages, 1):
                for offset in range(0, len(page), 650):
                    chunk = page[offset:offset + 650]
                    score = sum(2 for word in words if word in chunk.casefold())
                    if any(word in name.casefold() for word in words):
                        score += 1
                    if words and all(word in broad for word in words):
                        score += 1
                    if not words:
                        score = 1 if offset == 0 else 0
                    if score:
                        results.append((score, f"{name} page {page_no}: {chunk}"))
        except (OSError, ValueError, KeyError, TypeError):
            continue
    results.sort(key=lambda item: -item[0])
    return [line for _, line in results[:max(1, limit)]]
