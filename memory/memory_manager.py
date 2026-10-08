import json
import re
from datetime import datetime
from threading import Lock
from pathlib import Path
import sys


def get_base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent


BASE_DIR         = get_base_dir()
MEMORY_PATH      = BASE_DIR / "memory" / "long_term.json"
_lock            = Lock()
MAX_VALUE_LENGTH = 380

# ── Why there are two very different numbers here ────────────────────────────
#
# There used to be one: MEMORY_MAX_CHARS = 2200, applied to the whole store. It
# was a *storage* limit, and it existed only because the entire memory was
# pasted into the system prompt on every connect — so growing the memory grew
# every single request. When it filled, _trim_to_limit() deleted the oldest
# entries and printed one line to a console nobody reads. A memory described as
# "deeply remembers projects, preferences and personal context" was in practice
# two pages long, and quietly forgot your sister's name after a few weeks.
#
# Storage and prompt budget are now separate concerns:
#
#   MEMORY_MAX_CHARS  — a runaway guard, not a feature limit. Nothing normal
#                       reaches it; a bug writing in a loop does.
#   PROMPT_CORE_CHARS — what actually rides in the system prompt every session.
#                       Smaller than the old whole-memory dump, so sessions
#                       start *faster* than before, not slower.
#
# Everything above the core stays on disk and is fetched on demand by the
# recall_memory tool — see search_memory() and format_memory_for_prompt().
MEMORY_MAX_CHARS  = 500_000
PROMPT_CORE_CHARS = 16_000
PROMPT_INDEX_CHARS = 2_000
# Most entries any one category may contribute to the core block
PROMPT_MAX_PER_CATEGORY = 40

_CATEGORY_ALIASES: dict[str, str] = {
    "family": "relationships",
    "relatives": "relationships",
    "friends": "relationships",
    "parents": "relationships",
    "personal": "identity",
    "user": "identity",
    "me": "identity",
    "work": "projects",
    "goals": "projects",
    "habits": "preferences",
    "likes": "preferences",
    "favorite": "preferences",
    "favourite": "preferences",
    "wish": "wishes",
    "plans": "wishes",
}

def _empty_memory() -> dict:
    return {
        "identity":      {},
        "preferences":   {},
        "projects":      {},
        "relationships": {},
        "wishes":        {},
        "notes":         {},
    }

def load_memory() -> dict:
    if not MEMORY_PATH.exists():
        return _empty_memory()
    with _lock:
        try:
            data = json.loads(MEMORY_PATH.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                base = _empty_memory()
                for key in base:
                    if key not in data:
                        data[key] = {}
                return data
            return _empty_memory()
        except Exception as e:
            print(f"[Memory] ⚠️ Load error: {e}")
            return _empty_memory()

def _all_entries(memory: dict) -> list[tuple]:
    entries = []
    for cat, items in memory.items():
        if not isinstance(items, dict):
            continue
        for key, entry in items.items():
            if isinstance(entry, dict) and "value" in entry:
                entries.append((cat, key, entry))
    return entries


# Set by main.py so a trim can reach the activity log. Deleting something a
# person told you and mentioning it only on stdout is how a memory loses trust.
_trim_notifier = None


def set_trim_notifier(fn) -> None:
    """Register a callable(str) that surfaces trims to the user."""
    global _trim_notifier
    _trim_notifier = fn


def _trim_to_limit(memory: dict) -> dict:
    if len(json.dumps(memory, ensure_ascii=False)) <= MEMORY_MAX_CHARS:
        return memory
    entries = _all_entries(memory)
    entries.sort(key=lambda t: t[2].get("updated", "0000-00-00"))
    dropped = []
    for cat, key, _ in entries:
        if len(json.dumps(memory, ensure_ascii=False)) <= MEMORY_MAX_CHARS:
            break
        del memory[cat][key]
        dropped.append(f"{cat}/{key}")
        print(f"[Memory] 🗑️  Trimmed {cat}/{key}")
    if dropped and _trim_notifier:
        try:
            _trim_notifier(
                f"SYS: Memory full — forgot {len(dropped)} oldest entries "
                f"({', '.join(dropped[:3])}{'…' if len(dropped) > 3 else ''})"
            )
        except Exception:
            pass
    return memory

def save_memory(memory: dict) -> None:
    if not isinstance(memory, dict):
        return
    memory = _trim_to_limit(memory)
    MEMORY_PATH.parent.mkdir(parents=True, exist_ok=True)
    with _lock:
        MEMORY_PATH.write_text(
            json.dumps(memory, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )


def _truncate_value(val: str) -> str:
    if isinstance(val, str) and len(val) > MAX_VALUE_LENGTH:
        return val[:MAX_VALUE_LENGTH].rstrip() + "…"
    return val


def _recursive_update(target: dict, updates: dict) -> bool:
    changed = False
    for key, value in updates.items():
        if value is None:
            continue
        if isinstance(value, str) and not value.strip():
            continue
        if isinstance(value, dict) and "value" not in value:
            if key not in target or not isinstance(target[key], dict):
                target[key] = {}
                changed = True
            if _recursive_update(target[key], value):
                changed = True
        else:
            new_val  = _truncate_value(str(value["value"] if isinstance(value, dict) else value))
            entry    = {"value": new_val, "updated": datetime.now().strftime("%Y-%m-%d")}
            existing = target.get(key, {})
            if not isinstance(existing, dict) or existing.get("value") != new_val:
                target[key] = entry
                changed = True
    return changed


def update_memory(memory_update: dict) -> dict:
    if not isinstance(memory_update, dict) or not memory_update:
        return load_memory()
    # Normalize aliases in top-level categories
    normalized: dict = {}
    for cat, val in memory_update.items():
        canonical = _CATEGORY_ALIASES.get(str(cat).lower().strip(), str(cat))
        normalized[canonical] = val
    memory = load_memory()
    if _recursive_update(memory, normalized):
        save_memory(memory)
        print(f"[Memory] 💾 Saved: {list(normalized.keys())}")
    return memory

def _entry_value(entry) -> str:
    """Accept both the {'value': ..., 'updated': ...} shape and a bare string,
    because early versions of the store wrote plain strings."""
    if isinstance(entry, dict):
        return str(entry.get("value", "") or "").strip()
    return str(entry or "").strip()


def _pretty(key: str) -> str:
    return key.replace("_", " ").strip()


# Identity is always in the prompt; these categories are ordered by priority
_CATEGORY_ORDER = ["relationships", "preferences", "projects", "notes", "wishes"]
_CATEGORY_LABELS = {
    "relationships": "People in their life & Family",
    "preferences":   "Preferences & Habits",
    "projects":      "Active projects / goals",
    "notes":         "Notes & Important Facts",
    "wishes":        "Wishes / plans",
}

_IDENTITY_FIELDS = [
    "name", "full_name", "nickname", "age", "birthday", "city", "home_district", "job",
    "language", "school", "university", "education_field", "nationality", "github_username", "github_profile"
]


def format_memory_for_prompt(memory: dict | None) -> str:
    """Build the memory block that goes into the system prompt.

    Sends:
      1. IDENTITY  - always, in full.
      2. ACTIVE MEMORY - facts from relationships, preferences, projects, notes, wishes.
         Generous budget ensures critical personal and family facts are always visible.
      3. AN INDEX  - the *keys* of any overflow beyond PROMPT_CORE_CHARS.
    """
    if not memory:
        return ""

    core_lines: list[str] = []

    # 1. Identity - always, in full
    identity = memory.get("identity", {}) or {}
    for field in _IDENTITY_FIELDS:
        val = _entry_value(identity.get(field))
        if not val:
            continue
        if field == "language":
            core_lines.append(
                f"Has spoken to you in: {val} (an observation about the past — "
                f"always answer in the language of their CURRENT message)")
        else:
            core_lines.append(f"{_pretty(field).title()}: {val}")
    for key, entry in identity.items():
        if key in _IDENTITY_FIELDS:
            continue
        val = _entry_value(entry)
        if val:
            core_lines.append(f"{_pretty(key).title()}: {val}")

    # 2. Everything else, organized by category priority
    rest: list[tuple[str, str, str, str]] = []   # (updated, cat, key, value)
    for cat in _CATEGORY_ORDER:
        cat_items = []
        for key, entry in (memory.get(cat, {}) or {}).items():
            val = _entry_value(entry)
            if not val:
                continue
            updated = (entry.get("updated", "") if isinstance(entry, dict) else "") or "0000-00-00"
            cat_items.append((updated, cat, key, val))
        cat_items.sort(key=lambda t: t[0], reverse=True)
        rest.extend(cat_items)

    used    = sum(len(l) + 1 for l in core_lines)
    shown: dict[str, list[str]] = {}
    overflow: dict[str, list[str]] = {}

    per_cat_used: dict[str, int] = {}
    for _updated, cat, key, val in rest:
        line = f"  - {_pretty(key).title()}: {val}"
        if (per_cat_used.get(cat, 0) < PROMPT_MAX_PER_CATEGORY
                and used + len(line) + 1 <= PROMPT_CORE_CHARS):
            shown.setdefault(cat, []).append(line)
            per_cat_used[cat] = per_cat_used.get(cat, 0) + 1
            used += len(line) + 1
        else:
            overflow.setdefault(cat, []).append(_pretty(key))

    # The index is a table of contents
    indexed: list[str] = []
    if overflow:
        cats  = [c for c in _CATEGORY_ORDER if overflow.get(c)]
        cursor = {c: 0 for c in cats}
        while cats:
            for cat in list(cats):
                i = cursor[cat]
                if i >= len(overflow[cat]):
                    cats.remove(cat)
                    continue
                indexed.append(overflow[cat][i])
                cursor[cat] = i + 1

    for cat in _CATEGORY_ORDER:
        label = _CATEGORY_LABELS.get(cat, cat.title())
        if shown.get(cat):
            core_lines.append("")
            core_lines.append(f"{label}:")
            core_lines.extend(shown[cat])

    if not core_lines and not indexed:
        return ""

    out = [
        "[WHAT YOU KNOW ABOUT THIS PERSON — use naturally, never recite like a list]",
        *core_lines,
    ]

    # 3. The index of what is on disk but not in this prompt
    if indexed:
        budget, names = PROMPT_INDEX_CHARS, []
        for n in indexed:
            if budget - len(n) - 2 < 0:
                break
            names.append(n)
            budget -= len(n) + 2
        if names:
            out.append("")
            out.append(
                "[ALSO REMEMBERED — values not shown here. Call recall_memory "
                "with a keyword to read any of these before saying you do not know]"
            )
            out.append(", ".join(names)
                       + (f" (+{len(indexed) - len(names)} more)"
                          if len(indexed) > len(names) else ""))

    return "\n".join(out) + "\n"


# ── Multilingual query expansion for recall ──────────────────────────────────
_SYNONYM_GROUPS: dict[str, list[str]] = {
    "mother": [
        "ammu", "amma", "ammur", "ammar", "ma", "maa", "mar",
        "mom", "mother", "mom's", "mother's", "matri", "mata", "mamuni",
        "আম্মু", "আম্মুর", "আম্মা", "আম্মার", "মা", "মার", "মাতা", "মাতৃ", "মামুনি",
        "अम्मी", "माँ", "माता", "अम्मीका"
    ],
    "father": [
        "baba", "abbu", "abba", "babar", "abbur", "abbar",
        "dad", "father", "dad's", "father's", "pitri", "pita", "abuji",
        "বাবা", "বাবার", "আব্বু", "আব্বুর", "আব্বা", "আব্ব্বার", "পিতা", "পিতৃ",
        "अब्बा", "पिता", "पापा", "बापू", "पिताजी", "बाबा"
    ],
    "job": [
        "job", "work", "profession", "occupation", "career", "employment", "works", "worker", "post", "role", "service",
        "pesha", "chakri", "kormokorta", "koren", "korchen", "chakuri", "chakrite", "peshay", "officer", "ngo",
        "পেশা", "চাকরি", "কর্মকর্তা", "কাজ", "কর্ম", "চাকুরে", "চাকরী", "কর্মরত", "অফিসার", "এনজিও",
        "पेशा", "नौकरी", "काम", "कार्यकर्ता", "व्यवसाय", "अधिकारी"
    ],
    "brother": [
        "brother", "bro", "bhai", "vai", "bhaiya", "vaiya", "bhaiye", "vaire", "bhaier", "vaier",
        "ভাই", "ভাইয়া", "ভাইয়ের", "ভ্রাতা", "ভাইরে",
        "भाई", "भैया"
    ],
    "sister": [
        "sister", "sis", "bon", "apu", "api", "boner", "apur", "apir",
        "বোন", "আপু", "আপি", "বোনের", "ভগিনী",
        "बहन", "दीदी"
    ],
    "family": [
        "family", "relative", "relatives", "parents", "poribar", "poribare", "poribarer", "attiyo",
        "পরিবার", "পরিবারের", "পিতামাতা", "আত্মীয়", "স্বজন",
        "परिवार", "रिश्तेदार"
    ],
    "wife": [
        "wife", "spouse", "partner", "bou", "stree",
        "বউ", "স্ত্রী", "ওয়াইফ", "সহধর্মিনী",
        "पत्नी", "बीवी"
    ],
    "husband": [
        "husband", "shami", "pati",
        "স্বামী", "পতি",
        "पति", "शौहर"
    ],
    "son": [
        "son", "child", "kid", "chele", "putro", "cheler",
        "ছেলে", "পুত্র", "সন্তান",
        "बेटा", "पुत्र"
    ],
    "daughter": [
        "daughter", "konna", "meye", "meyer",
        "মেয়ে", "কন্যা",
        "बेटी", "पुत्री"
    ],
    "name": [
        "name", "named", "names", "nam", "naam", "namti", "namta", "daknam",
        "নাম", "নামটি", "নামটা", "ডাকনাম", "শুভনাম",
        "नाम"
    ],
    "city": [
        "city", "town", "location", "place", "bari", "thake", "basha", "desh", "district", "hometown", "jela",
        "brahmanbaria", "dhaka", "bangladesh",
        "বাড়ি", "বাসা", "শহর", "দেশ", "থাকেন", "থাকা", "জেলা", "গ্রাম", "ব্রাহ্মণবাড়িয়া", "ঢাকা",
        "शहर", "घर", "रहते", "स्थान"
    ],
    "education": [
        "school", "college", "university", "varsity", "study", "student", "class", "degree", "education",
        "porashona", "poralekha", "shikhon", "porasuna",
        "স্কুল", "কলেজ", "বিশ্ববিদ্যালয়", "ভার্সিটি", "পড়াশোনা", "পড়ালেখা", "শ্রেণী", "ক্লাস", "শিক্ষা",
        "स्कूल", "कॉलेज", "विश्वविद्यालय", "पढ़ाई"
    ],
    "preference": [
        "favorite", "favourite", "preference", "likes", "fav", "love", "hobby",
        "priyo", "pochondo", "bhalo lage", "bhalobashe",
        "প্রিয়", "প্রিয়", "পছন্দ", "ভালোবাসা", "শখ",
        "पसंद", "प्रिय"
    ],
    "food": [
        "food", "dish", "meal", "khabar", "khana", "ranna", "biryani", "kacchi",
        "খাবার", "খাওয়া", "রান্না", "বিরিয়ানি", "কাচ্চি",
        "खाना", "भोजन"
    ],
    "project": [
        "project", "code", "coding", "repo", "github", "ultron", "jarvis", "assistant", "mark",
        "প্রজেক্ট", "কোড", "কাজ", "কাজের",
        "प्रोजेक्ट", "काम"
    ]
}


def _expand_query_words(words: list[str]) -> list[str]:
    expanded = set(words)
    for w in words:
        for canonical, syns in _SYNONYM_GROUPS.items():
            if w == canonical or w in syns or any(s in w for s in syns if len(s) >= 3):
                expanded.add(canonical)
                for s in syns:
                    if len(s) >= 3:
                        expanded.add(s)
    return list(expanded)


# ── Recall ────────────────────────────────────────────────────────────────────

def _score(query_words: list[str], cat: str, key: str, value: str) -> int:
    """Intelligent concept-based lexical relevance. No external models needed —
    evaluates query words and synonym concepts against key, value, and category."""
    hay_key = _pretty(key).lower()
    hay_val = value.lower()
    key_tokens = set(re.findall(r"[\w\u0980-\u09FF\u0900-\u097F]+", hay_key))
    val_tokens = set(re.findall(r"[\w\u0980-\u09FF\u0900-\u097F]+", hay_val))

    score = 0
    for w in query_words:
        if not w:
            continue
        concept_words = {w}
        for canonical, syns in _SYNONYM_GROUPS.items():
            if w == canonical or w in syns or any(s in w for s in syns if len(s) >= 3):
                concept_words.add(canonical)
                concept_words.update(syns)

        # Match against key
        if any(cw == hay_key for cw in concept_words):
            score += 25
        elif any(cw in key_tokens for cw in concept_words):
            score += 15
        elif any(cw in hay_key for cw in concept_words if len(cw) >= 3):
            score += 8

        # Match against value
        if any(cw in val_tokens for cw in concept_words):
            score += 10
        elif any(cw in hay_val for cw in concept_words if len(cw) >= 3):
            score += 4

        # Match against category
        if any(cw in cat for cw in concept_words):
            score += 3

    return score


def search_memory(query: str, limit: int = 10) -> str:
    """Find stored facts matching `query`. Backs the recall_memory tool.

    An empty query is treated as "show me everything you know", capped - the
    model asks that when the user says "what do you remember about me?"."""
    memory = load_memory()
    raw_words = [w for w in re.findall(r"[\w\u0980-\u09FF\u0900-\u097F]+", (query or "").lower()) if len(w) > 1]

    rows: list[tuple[int, str, str, str]] = []
    for cat, items in memory.items():
        if not isinstance(items, dict):
            continue                     # skip 'sessions', which is a list
        for key, entry in items.items():
            val = _entry_value(entry)
            if not val:
                continue
            s = _score(raw_words, cat, key, val) if raw_words else 1
            if s > 0:
                rows.append((s, cat, key, val))

    if not rows:
        return (f"Nothing stored about '{query}'." if query
                else "I have not stored anything about this person yet.")

    rows.sort(key=lambda r: (-r[0], r[2]))
    lines = [f"{cat}/{_pretty(key)}: {val}" for _s, cat, key, val in rows[:max(1, limit)]]
    head  = (f"Stored facts matching '{query}':" if query
             else "Everything currently stored:")
    more  = (f"\n(+{len(rows) - len(lines)} more — search with a narrower keyword)"
             if len(rows) > len(lines) else "")
    return head + "\n" + "\n".join(lines) + more


def all_entries_for_ui() -> list[dict]:
    """Flat list for the memory panel: what JARVIS knows, and when it learned it.
    Sorted newest first so the panel opens on what changed most recently."""
    memory = load_memory()
    rows = []
    for cat, items in memory.items():
        if not isinstance(items, dict):
            continue
        for key, entry in items.items():
            val = _entry_value(entry)
            if not val:
                continue
            rows.append({
                "category": cat,
                "key":      key,
                "value":    val,
                "updated":  (entry.get("updated", "") if isinstance(entry, dict) else ""),
            })
    rows.sort(key=lambda r: (r["updated"] or "0000-00-00"), reverse=True)
    return rows

def remember(key: str, value: str, category: str = "notes") -> str:
    category = _CATEGORY_ALIASES.get(str(category).lower().strip(), str(category))
    valid = {"identity", "preferences", "projects", "relationships", "wishes", "notes"}
    if category not in valid:
        category = "notes"
    update_memory({category: {key: {"value": value}}})
    return f"Remembered: {category}/{key} = {value}"


def forget(key: str, category: str = "notes") -> str:
    category = _CATEGORY_ALIASES.get(str(category).lower().strip(), str(category))
    memory = load_memory()
    cat    = memory.get(category, {})
    if key in cat:
        del cat[key]
        memory[category] = cat
        save_memory(memory)
        return f"Forgotten: {category}/{key}"
    return f"Not found: {category}/{key}"


forget_memory = forget


# ── Auto Memory Ingestion ────────────────────────────────────────────────────

_MEMORY_CUES = [
    # Bengali explicit memory commands
    "মনে রাখ", "মনে রেখ", "মনে রাখিস", "মনে রাখবেন", "সেভ কর", "নোট কর", "লিখে রাখ",
    # Bengali personal possessives & relations
    "আমার বাবা", "আমার আব্বু", "আমার আম্মু", "আমার মা", "আমার বোন", "আমার ভাই",
    "আমার নাম", "আমার প্রিয়", "আমার প্রিয়", "আমার পছন্দ", "আমার বয়স", "আমার জন্মদিন",
    "আমার পেশা", "আমার চাকরি", "আমার কাজ", "আমার স্কুল", "আমার কলেজ", "আমার ভার্সিটি",
    "আমার বাসা", "আমার বাড়ি", "আমার জেলা", "আম্মুর পেশা", "আম্মুর চাকরি", "আম্মুর নাম",
    "বাবার পেশা", "বাবার চাকরি", "বাবার নাম", "বোনের নাম", "ভাইয়ের নাম",
    # Banglish cues
    "mone rakh", "mone rekho", "save kor", "save koro", "note kor",
    "amar baba", "amar abbu", "amar ammu", "amar ma", "amar bon", "amar vai",
    "amar nam", "amar priyo", "amar pochondo", "amar basha", "amar bari",
    "amar chakri", "amar pesha", "ammur pesha", "ammur chakri", "babar pesha",
    # English cues
    "remember", "keep in mind", "save this", "take note", "don't forget",
    "my father", "my dad", "my mother", "my mom", "my sister", "my brother",
    "my name is", "my favorite", "my favourite", "my job is", "i work as", "i live in",
]


def extract_and_save_heuristic(text: str) -> tuple[str, str, str] | None:
    """Fast regex-based extractor for high-confidence common Bengali/English memory patterns."""
    t = text.strip()

    # Mother's job
    m = re.search(r"(?:আম্মুর|মায়ের|ammu[r]?|ma[r]?)\s+(?:পেশা|কাজ|চাকরি|job|pesha)\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("mother_job", val, "relationships")
        return ("relationships", "mother_job", val)

    # Father's job
    m = re.search(r"(?:বাবার|পিতার|আব্বুর|baba[r]?|abbu[r]?)\s+(?:পেশা|কাজ|চাকরি|job|pesha)\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("father_job", val, "relationships")
        return ("relationships", "father_job", val)

    # Mother's name
    m = re.search(r"(?:আম্মুর|মায়ের|ammu[r]?|ma[r]?)\s+নাম\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("mother_name", val, "relationships")
        return ("relationships", "mother_name", val)

    # Father's name
    m = re.search(r"(?:বাবার|পিতার|আব্বুর|baba[r]?|abbu[r]?)\s+নাম\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("father_name", val, "relationships")
        return ("relationships", "father_name", val)

    # Favorite food
    m = re.search(r"(?:আমার\s+)?(?:প্রিয়|প্রিয়|priyo)\s+(?:খাবার|food)\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("favorite_food", val, "preferences")
        return ("preferences", "favorite_food", val)

    # Favorite color
    m = re.search(r"(?:আমার\s+)?(?:প্রিয়|প্রিয়|priyo)\s+(?:রং|colour|color)\s*(?:হলো|হচ্ছে|হল|is|[:=])?\s*(.+)", t, re.I)
    if m:
        val = m.group(1).strip().rstrip(".।")
        remember("favorite_color", val, "preferences")
        return ("preferences", "favorite_color", val)

    return None


async def auto_extract_and_save_memory(text: str) -> dict | None:
    """Background auto-extractor: extracts personal facts and stores them permanently."""
    if not text or len(text.strip()) < 5:
        return None

    low = text.lower()
    if not any(cue in low for cue in _MEMORY_CUES):
        return None

    # 1. Try fast heuristic first (<1ms)
    try:
        h = extract_and_save_heuristic(text)
        if h:
            cat, key, val = h
            print(f"[AutoMemory] ⚡ Fast-captured: {cat}/{key} = {val}")
            return {"category": cat, "key": key, "value": val}
    except Exception as e:
        print(f"[AutoMemory] Heuristic notice: {e}")

    # 2. Asynchronous LLM Extraction via Gemini
    try:
        import asyncio
        from core import gemini
        prompt = (
            "You are an expert memory extractor for an AI personal assistant. "
            "Analyze what the user said to the assistant:\n"
            f'"{text}"\n\n'
            "Did the user state a personal fact about themselves, their family, their preferences, their projects, or instruct the assistant to remember something? "
            "If YES, return ONLY a JSON object:\n"
            '{\n'
            '  "category": "identity"|"relationships"|"preferences"|"projects"|"notes"|"wishes",\n'
            '  "key": "snake_case_key (e.g. mother_profession, favorite_food, sister_name, birth_year)",\n'
            '  "value": "concise accurate value in the user\'s language or English"\n'
            '}\n'
            "If NO personal fact or instruction to remember was given (e.g. general commands, questions, chit-chat), return null.\n"
            "Output JSON only, no markdown, no explanation."
        )
        res = await asyncio.to_thread(gemini.as_json, prompt, gemini.FAST, None, 10000)
        if isinstance(res, dict) and "key" in res and "value" in res:
            cat = str(res.get("category", "notes")).lower().strip()
            cat = _CATEGORY_ALIASES.get(cat, cat)
            valid = {"identity", "preferences", "projects", "relationships", "wishes", "notes"}
            if cat not in valid:
                cat = "notes"
            key = str(res["key"]).strip().lower().replace(" ", "_").replace("-", "_")
            val = str(res["value"]).strip()
            if key and val:
                remember(key, val, cat)
                print(f"[AutoMemory] 🧠 LLM-captured: {cat}/{key} = {val}")
                return {"category": cat, "key": key, "value": val}
    except Exception as e:
        print(f"[AutoMemory] LLM extract error: {e}")

    return None


# ── Session memory ─────────────────────────────────────────────────────────────

_SESSION_MAX = 3   # safety cap — in practice 0-1 entries after pop


def save_session_summary(summary: str, language: str = "") -> None:
    """Append a 1-2 sentence session summary to long_term.json['sessions']."""
    summary = (summary or "").strip()
    if not summary:
        return
    memory   = load_memory()
    sessions = memory.get("sessions", [])
    if not isinstance(sessions, list):
        sessions = []
    entry: dict = {
        "date":    datetime.now().strftime("%Y-%m-%d"),
        "summary": summary[:280],
    }
    if language:
        entry["language"] = language
    sessions.append(entry)
    memory["sessions"] = sessions[-_SESSION_MAX:]
    with _lock:
        MEMORY_PATH.parent.mkdir(parents=True, exist_ok=True)
        MEMORY_PATH.write_text(
            json.dumps(memory, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
    print(f"[Memory] 📝 Session saved ({entry['date']}): {summary[:60]}…")


def pop_last_session() -> dict | None:
    """
    Return AND remove the most recent session entry.
    Calling this consumes the entry so it is never repeated in future briefings.
    """
    with _lock:
        if not MEMORY_PATH.exists():
            return None
        try:
            memory   = json.loads(MEMORY_PATH.read_text(encoding="utf-8"))
            sessions = memory.get("sessions", [])
            if not isinstance(sessions, list) or not sessions:
                return None
            entry = sessions.pop()          # remove the last entry
            memory["sessions"] = sessions
            MEMORY_PATH.write_text(
                json.dumps(memory, indent=2, ensure_ascii=False),
                encoding="utf-8",
            )
            return entry
        except Exception as e:
            print(f"[Memory] ⚠️ pop_last_session error: {e}")
            return None