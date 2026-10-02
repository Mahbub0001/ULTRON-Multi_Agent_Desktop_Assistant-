# actions/presentation.py
"""
PowerPoint / slide-deck controller for Mark-LIV.

Two modes, chosen automatically:

  * FILE mode (python-pptx) — build a real .pptx on disk: set the title,
    add N slides with headings and bullets, then open it. Works even when
    PowerPoint is not running yet; no COM required.
  * LIVE mode (win32com) — when PowerPoint is already open, drive the
    running app directly: add slides, write into placeholders, jump to a
    slide, save. What the user sees changes as the tool runs.

Typical request "open PowerPoint, make two pages and write something":

    presentation {action:"create", title:"My Deck",
                  slides:[{title:"Page 1", bullets:["Hello"]},
                          {title:"Page 2", bullets:["World"]}]}

which creates the two-slide deck AND opens it in PowerPoint in one step;
then any follow-up edits run in live mode against the open window.
"""
from __future__ import annotations

import sys
import time
from pathlib import Path
from typing import Optional

# ppSlideLayout constants (avoid importing oletools just for these)
_PP_LAYOUT_TITLE = 1        # title slide
_PP_LAYOUT_TEXT = 2         # title + body
_PP_LAYOUT_BLANK = 12


def _default_dir() -> Path:
    d = Path.home() / "Documents" / "ULTRON"
    try:
        d.mkdir(parents=True, exist_ok=True)
    except Exception:
        d = Path.home()
    return d


def _safe_filename(title: str) -> str:
    keep = "".join(ch if ch.isalnum() or ch in " -_" else "_" for ch in title)
    keep = keep.strip() or "Presentation"
    return keep[:60] + ".pptx"


# ---------------------------------------------------------------------------
# LIVE mode helpers (PowerPoint COM)
# ---------------------------------------------------------------------------

def _get_active_powerpoint():
    """(ppt_app, active_presentation) when PowerPoint is open, else (None, None)."""
    if sys.platform != "win32":
        return None, None
    try:
        import win32com.client
        try:
            ppt = win32com.client.GetActiveObject("PowerPoint.Application")
        except Exception:
            return None, None
        pres = None
        try:
            if ppt.Presentations.Count > 0:
                pres = ppt.ActivePresentation
        except Exception:
            pres = None
        return ppt, pres
    except Exception:
        return None, None


def _com_session():
    """COM must be initialised on the executor thread actions run on."""
    if sys.platform != "win32":
        return None
    try:
        import pythoncom
        pythoncom.CoInitialize()
        return pythoncom
    except Exception:
        return None


def _fill_slide(slide, title: str, bullets: list[str] | str) -> None:
    """Write a heading + body into a slide's placeholders (live mode)."""
    if title:
        try:
            slide.Shapes.Title.TextFrame.TextRange.Text = title
        except Exception:
            pass
    if isinstance(bullets, str):
        bullets = [bullets] if bullets else []
    if bullets:
        body = "\n".join(bullets)
        for idx in (2, 1):                     # body placeholder, then first
            try:
                slide.Shapes.Placeholders(idx).TextFrame.TextRange.Text = body
                return
            except Exception:
                continue
        # No placeholder (blank layout) — drop a text box instead.
        try:
            box = slide.Shapes.AddTextbox(1, 60, 120, 600, 400)  # horizontal text
            box.TextFrame.TextRange.Text = body
        except Exception:
            pass


def _open_in_powerpoint(path: Path) -> str:
    """Open the file in PowerPoint (live COM when possible, else shell)."""
    com = _com_session()
    if com is not None:
        try:
            import win32com.client
            try:
                ppt = win32com.client.GetActiveObject("PowerPoint.Application")
            except Exception:
                ppt = win32com.client.Dispatch("PowerPoint.Application")
            ppt.Visible = True
            pres = ppt.Presentations.Open(str(path), WithWindow=True)
            try:
                pres.Windows(1).Activate()
            except Exception:
                pass
            return f"Opened '{path.name}' in PowerPoint ({pres.Slides.Count} slides)."
        except Exception:
            pass
        finally:
            try:
                com.CoUninitialize()
            except Exception:
                pass
    if sys.platform == "win32":
        import os
        os.startfile(str(path))  # noqa: S606
        return f"Opened '{path.name}' in the default presentation app."
    return f"Created '{path.name}' (open it manually on this OS)."


# ---------------------------------------------------------------------------
# FILE mode helpers (python-pptx)
# ---------------------------------------------------------------------------

def _create_pptx(title: str, slides: list, path: Path) -> Path:
    from pptx import Presentation
    from pptx.util import Inches

    prs = Presentation()
    if not slides:
        slides = [{"title": title or "Slide 1", "bullets": []}]

    for i, spec in enumerate(slides):
        if isinstance(spec, str):
            spec = {"title": spec, "bullets": []}
        if not isinstance(spec, dict):
            spec = {"title": str(spec), "bullets": []}
        spec_title = str(spec.get("title", "") or "")
        spec_bullets = spec.get("bullets") or spec.get("content") or []
        if isinstance(spec_bullets, str):
            spec_bullets = [ln for ln in spec_bullets.split("\n") if ln.strip()]

        layout = prs.slide_layouts[0 if i == 0 and not spec_bullets else 1]
        slide = prs.slides.add_slide(layout)

        # Title
        if slide.shapes.title is not None:
            slide.shapes.title.text = spec_title or (title if i == 0 else f"Slide {i+1}")

        # Body bullets
        body = None
        for ph in slide.placeholders:
            if ph.placeholder_format.idx != 0:
                body = ph
                break
        if body is not None and spec_bullets:
            body.text_frame.text = ""
            tf = body.text_frame
            for j, line in enumerate(spec_bullets):
                if j == 0:
                    tf.text = line
                else:
                    p = tf.add_paragraph()
                    p.text = line

    # A deck with no slides at all would be invalid — guarantee one.
    if len(prs.slides._sldIdLst) == 0:
        prs.slides.add_slide(prs.slide_layouts[0])

    path.parent.mkdir(parents=True, exist_ok=True)
    prs.save(str(path))
    return path


def _append_slide_file(path: Path, title: str, bullets: list[str]) -> int:
    """Add a slide to a .pptx on disk (file mode). Returns new slide count."""
    from pptx import Presentation
    prs = Presentation(str(path))
    layout = prs.slide_layouts[1] if bullets else prs.slide_layouts[5]
    slide = prs.slides.add_slide(layout)
    if slide.shapes.title is not None and title:
        slide.shapes.title.text = title
    if bullets:
        for ph in slide.placeholders:
            if ph.placeholder_format.idx != 0:
                ph.text_frame.text = "\n".join(bullets)
                break
    prs.save(str(path))
    return len(prs.slides._sldIdLst)


def _write_slide_file(path: Path, slide_no: int, title: str, text: str) -> str:
    """Overwrite a slide's title/body in a .pptx on disk (file mode)."""
    from pptx import Presentation
    prs = Presentation(str(path))
    if slide_no < 1 or slide_no > len(prs.slides._sldIdLst):
        raise ValueError(f"Slide {slide_no} does not exist "
                         f"(deck has {len(prs.slides._sldIdLst)}).")
    slide = prs.slides[slide_no - 1]
    if title and slide.shapes.title is not None:
        slide.shapes.title.text = title
    if text:
        placed = False
        for ph in slide.placeholders:
            if ph.placeholder_format.idx != 0:
                lines = text if isinstance(text, list) else str(text).split("\n")
                ph.text_frame.text = "\n".join(lines)
                placed = True
                break
        if not placed:
            box = slide.shapes.add_textbox(60, 120, 600, 400)
            box.text_frame.text = text if isinstance(text, str) else "\n".join(text)
    prs.save(str(path))
    return f"Wrote slide {slide_no}."


def _newest_our_pptx() -> Optional[Path]:
    d = _default_dir()
    try:
        files = sorted(d.glob("*.pptx"), key=lambda p: p.stat().st_mtime,
                       reverse=True)
    except Exception:
        return None
    return files[0] if files else None


# ---------------------------------------------------------------------------
# Handler
# ---------------------------------------------------------------------------

def presentation(parameters: dict, response=None, player=None,
                 session_memory=None) -> str:
    action = str(parameters.get("action", "")).strip().lower()
    title = str(parameters.get("title", "") or "").strip()
    path_raw = str(parameters.get("path", "") or "").strip().strip('"')
    bullets_raw = parameters.get("bullets", [])
    slide_no = int(parameters.get("slide", 0) or 0)

    if isinstance(bullets_raw, str):
        bullets = [ln for ln in bullets_raw.split("\n") if ln.strip()]
    elif isinstance(bullets_raw, list):
        bullets = [str(b) for b in bullets_raw]
    else:
        bullets = []

    if not action:
        return "Error: 'action' is required."
    _valid = ("create", "open", "add_slide", "write", "goto", "save", "status")
    if action not in _valid:
        return (f"Unknown action '{action}'. Valid: " + ", ".join(_valid) + ".")

    slides = parameters.get("slides") or []
    path = Path(path_raw).expanduser() if path_raw else None
    com = _com_session()
    try:
        ppt, pres = _get_active_powerpoint()

        # ---- create: build the deck and open it -------------------------
        if action == "create":
            if not path:
                path = _default_dir() / _safe_filename(title or "Presentation")
            if path.suffix.lower() != ".pptx":
                path = path.with_suffix(".pptx")

            # Normalise slides: allow plain strings or {title, bullets}.
            norm = []
            for s in (slides if isinstance(slides, list) else []):
                if isinstance(s, dict):
                    norm.append({
                        "title": str(s.get("title", "") or ""),
                        "bullets": ([str(b) for b in s["bullets"]]
                                    if isinstance(s.get("bullets"), list)
                                    else ([str(s["content"])]
                                           if s.get("content") else [])),
                    })
                else:
                    norm.append({"title": str(s), "bullets": []})

            if not norm:
                # No content given: two blank-ish pages is the common ask
                # ("take two pages") — default to one titled slide.
                norm = [{"title": title or "Slide 1", "bullets": bullets}]
            elif title:
                norm[0].setdefault("title", "")
                if not norm[0]["title"]:
                    norm[0]["title"] = title

            _create_pptx(title, norm, path)
            msg = _open_in_powerpoint(path)
            return f"Created {len(norm)}-slide deck at {path}. {msg}"

        # ---- open: launch an existing file ------------------------------
        if action == "open":
            target = path or _newest_our_pptx()
            if target is None or not target.exists():
                return "No presentation file found to open — give a path or use create."
            return _open_in_powerpoint(target)

        # Everything below needs a deck: live presentation or a file on disk.
        live = pres is not None
        if not live and path is None:
            path = _newest_our_pptx()
        if not live and (path is None or not path.exists()):
            return ("No presentation open and no file found. "
                    "Use action='create' first.")

        # ---- add_slide --------------------------------------------------
        if action in ("add_slide", "add"):
            if live:
                n = pres.Slides.Count + 1
                layout = _PP_LAYOUT_TEXT if bullets else _PP_LAYOUT_BLANK
                slide = pres.Slides.Add(n, layout)
                _fill_slide(slide, title, bullets)
                try:
                    pres.Slides(n).Select()
                except Exception:
                    pass
                return f"Added slide {n} to '{pres.Name}' (now {n} slides)."
            n = _append_slide_file(path, title, bullets)
            return f"Added slide {n} to {path.name} ({n} slides total)."

        # ---- write: put text on an existing slide -----------------------
        if action in ("write", "set_text", "edit"):
            text = str(parameters.get("text", "") or "")
            lines = ([ln for ln in text.split("\n") if ln]
                     if text else bullets)
            if slide_no < 1:
                slide_no = pres.Slides.Count if live else 1
            if live:
                if slide_no > pres.Slides.Count:
                    return (f"Slide {slide_no} does not exist — deck has "
                            f"{pres.Slides.Count} slides.")
                slide = pres.Slides(slide_no)
                _fill_slide(slide, title, lines)
                try:
                    pres.Slides(slide_no).Select()
                except Exception:
                    pass
                return f"Wrote into slide {slide_no} of '{pres.Name}'."
            return _write_slide_file(path, slide_no, title, "\n".join(lines))

        # ---- goto -------------------------------------------------------
        if action in ("goto", "select"):
            if not live:
                return (f"PowerPoint is not running — {path.name if path else 'the deck'} "
                        "is on disk only. Open it with action='open'.")
            if slide_no < 1 or slide_no > pres.Slides.Count:
                return (f"Slide {slide_no} out of range "
                        f"(1–{pres.Slides.Count}).")
            pres.Slides(slide_no).Select()
            return f"Showing slide {slide_no} of '{pres.Name}'."

        # ---- save -------------------------------------------------------
        if action == "save":
            if not live:
                return f"{path.name if path else 'Deck'} is already saved on disk."
            pres.Save()
            return f"Saved '{pres.Name}'."

        # ---- status -----------------------------------------------------
        if action in ("status", "state"):
            if live:
                try:
                    current = pres.SlideShowWindow.View.CurrentSlideIndex
                except Exception:
                    try:
                        # pres.ActiveWindow is unreliable over pywin32; the
                        # application-level one answers correctly.
                        current = ppt.ActiveWindow.View.Slide.SlideIndex
                    except Exception:
                        current = 1
                return (f"PowerPoint open with '{pres.Name}': "
                        f"{pres.Slides.Count} slides, currently on slide {current}.")
            if path and path.exists():
                from pptx import Presentation
                count = len(Presentation(str(path)).slides._sldIdLst)
                return f"File {path.name}: {count} slides (PowerPoint not running)."
            return "No presentation open or found."

        return (f"Unknown action '{action}'. Valid: create, open, add_slide, "
                "write, goto, save, status.")

    except Exception as e:
        return f"presentation failed: {e}"
    finally:
        if com is not None:
            try:
                com.CoUninitialize()
            except Exception:
                pass


TOOL = {
    "name": "presentation",
    "description": (
        "Create and edit slide decks (PowerPoint). Actions: create (build a "
        ".pptx with a title and a slides list — each slide {title, bullets} — "
        "and open it in PowerPoint in one call), open (open an existing .pptx), "
        "add_slide (append a slide, live in PowerPoint when it is open), write "
        "(put title/bullets text into slide N), goto (show slide N), save, "
        "status. Use this instead of app_control for 'make a presentation', "
        "'open PowerPoint and add two slides', or 'write something on slide 2'."),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "enum": ["create", "open", "add_slide", "write", "goto",
                         "save", "status"],
                "description": "What to do.",
            },
            "title": {
                "type": "STRING",
                "description": "Deck title (create) or slide heading (add_slide/write).",
            },
            "slides": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "title": {"type": "STRING"},
                        "bullets": {"type": "ARRAY", "items": {"type": "STRING"}},
                    },
                },
                "description": "Slide specs for create: [{title, bullets}, ...].",
            },
            "bullets": {
                "type": "ARRAY",
                "items": {"type": "STRING"},
                "description": "Bullet lines for add_slide/write.",
            },
            "text": {
                "type": "STRING",
                "description": "Body text for write (newline-separated lines).",
            },
            "slide": {
                "type": "NUMBER",
                "description": "1-based slide number (write/goto).",
            },
            "path": {
                "type": "STRING",
                "description": "Optional .pptx file path (default: Documents/ULTRON).",
            },
        },
        "required": ["action"],
    },
    "handler": presentation,
}
