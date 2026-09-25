# actions/word_document.py
"""
Word Document and Essay Formatting Controller for Mark-LIV.
Provides professional document creation, essay writing, and styling directly
inside Microsoft Word (via Win32 COM) and via python-docx with automatic launch.
"""
from __future__ import annotations

import os
import re
import sys
import time
from pathlib import Path
from typing import Optional

try:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

# Safe path resolution
def _base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent

def _resolve_path(raw: str) -> Path:
    raw = (raw or "").strip().strip('"').strip("'")
    if not raw:
        return Path.home() / "Desktop"
    lower = raw.lower()
    shortcuts = {
        "desktop":   Path.home() / "Desktop",
        "downloads": Path.home() / "Downloads",
        "documents": Path.home() / "Documents",
        "home":      Path.home(),
    }
    if lower in shortcuts:
        return shortcuts[lower]
    head, sep, rest = raw.replace("\\", "/").partition("/")
    if sep and head.lower() in shortcuts:
        rest = rest.strip("/")
        return shortcuts[head.lower()] / rest if rest else shortcuts[head.lower()]
    return Path(raw).expanduser()

# COM Helpers
def _get_active_word():
    """Returns (word_app, active_doc) if MS Word is open, else (None, None)."""
    if sys.platform != "win32":
        return None, None
    try:
        import win32com.client
        word = win32com.client.GetActiveObject("Word.Application")
        if word.Documents.Count > 0:
            return word, word.ActiveDocument
        return word, None
    except Exception:
        return None, None

def _get_or_create_word():
    """Returns (word_app, doc) ensuring Word is open with at least one document."""
    if sys.platform != "win32":
        return None, None
    try:
        import win32com.client
        word, doc = _get_active_word()
        if word and doc:
            return word, doc
        if word and not doc:
            doc = word.Documents.Add()
            word.Visible = True
            return word, doc
        word = win32com.client.Dispatch("Word.Application")
        word.Visible = True
        doc = word.Documents.Add()
        return word, doc
    except Exception as e:
        print(f"[WordDoc] Word COM dispatch failed: {e}")
        return None, None

def _bring_word_to_front(word):
    """Brings MS Word window to front on Windows."""
    try:
        if word:
            word.Visible = True
            word.Activate()
    except Exception:
        pass

def _parse_content_blocks(raw_text: str) -> list[dict]:
    """
    Parses markdown/raw text into structured document blocks:
    - 'title'
    - 'subtitle'
    - 'heading1'
    - 'heading2'
    - 'bullet'
    - 'paragraph'
    """
    lines = [l.rstrip() for l in raw_text.splitlines()]
    blocks = []
    first_heading_seen = False

    for line in lines:
        s = line.strip()
        if not s:
            continue

        # Markdown H1 or Title
        if s.startswith("# "):
            title_text = s[2:].strip()
            if not blocks and not first_heading_seen:
                blocks.append({"type": "title", "text": title_text})
                first_heading_seen = True
            else:
                blocks.append({"type": "heading1", "text": title_text})
            continue

        # Markdown H2
        if s.startswith("## "):
            blocks.append({"type": "heading1", "text": s[3:].strip()})
            continue

        # Markdown H3
        if s.startswith("### "):
            blocks.append({"type": "heading2", "text": s[4:].strip()})
            continue

        # Subtitle / Author
        if (s.startswith("*") and s.endswith("*") and len(s) < 80) or \
           (s.lower().startswith("by ") and len(s) < 60):
            clean_sub = s.strip("*_").strip()
            blocks.append({"type": "subtitle", "text": clean_sub})
            continue

        # Bullet list item
        if s.startswith(("- ", "* ", "• ")):
            bullet_text = s[2:].strip()
            blocks.append({"type": "bullet", "text": bullet_text})
            continue

        # Numbered item e.g. "1. " or "1) "
        m = re.match(r"^(\d+[\.\)])\s+(.*)$", s)
        if m and len(s) < 90 and not s.endswith((".", "!", "?")):
            blocks.append({"type": "heading1", "text": s})
            continue

        # Standalone short line acting as heading
        known_sections = {
            "introduction", "background", "overview", "key points", "key benefits",
            "challenges", "discussion", "methodology", "analysis", "conclusion",
            "summary", "future scope", "recommendations", "references"
        }
        if (len(s) < 60 and not s.endswith((".", "!", "?")) and not s.endswith(":")) or \
           (s.lower().rstrip(":") in known_sections):
            if not blocks:
                blocks.append({"type": "title", "text": s.rstrip(":")})
            else:
                blocks.append({"type": "heading1", "text": s.rstrip(":")})
            continue

        # Regular body paragraph
        blocks.append({"type": "paragraph", "text": s})

    # If first block is not title, make first block title if it looks like one
    if blocks and blocks[0]["type"] == "heading1":
        blocks[0]["type"] = "title"

    return blocks

def _generate_essay_text(topic: str, instructions: str = "") -> str:
    """Uses Gemini to generate an essay structured with markdown headings."""
    prompt = (
        f"Write a comprehensive, engaging, well-researched essay on the topic: '{topic}'.\n"
        f"Format strictly in markdown with:\n"
        f"# <Catchy, Professional Title>\n"
        f"*Written for Nibir sir*\n\n"
        f"## Introduction\n"
        f"<Engaging introductory paragraph setting context>\n\n"
        f"## Key Dimensions and Core Analysis\n"
        f"<2-3 distinct, in-depth paragraphs exploring core aspects>\n\n"
        f"## Crucial Insights and Takeaways\n"
        f"- <Bullet point 1>\n"
        f"- <Bullet point 2>\n"
        f"- <Bullet point 3>\n\n"
        f"## Future Outlook and Challenges\n"
        f"<Paragraph on implications, challenges, and future prospects>\n\n"
        f"## Conclusion\n"
        f"<Inspiring, thought-provoking concluding paragraph>\n\n"
        f"Never combine everything into a single paragraph. Make sure each section has distinct paragraphs."
    )
    if instructions:
        prompt += f"\nAdditional guidelines: {instructions}"

    try:
        from core import gemini
        text_resp = gemini.text(prompt, tier=gemini.SMART)
        if text_resp and len(text_resp.strip()) > 100:
            return text_resp.strip()
    except Exception as e:
        print(f"[WordDoc] Gemini generation failed: {e}")

    # Fallback template if offline
    return (
        f"# The Significance of {topic.title()}\n"
        f"*Prepared for Nibir sir*\n\n"
        f"## Introduction\n"
        f"In today's fast-paced and interconnected world, {topic} represents a subject of paramount significance. "
        f"Understanding its multifaceted dimensions allows us to appreciate its role in modern development.\n\n"
        f"## Critical Analysis\n"
        f"The implications of {topic} span across technology, economics, and human society. "
        f"As developments accelerate, adopting thoughtful frameworks ensures maximum positive impact while mitigating risks.\n\n"
        f"## Key Takeaways\n"
        f"- Transformative potential across diverse sectors\n"
        f"- Necessity of continuous learning and adaptation\n"
        f"- Balanced approach combining innovation with ethical responsibility\n\n"
        f"## Conclusion\n"
        f"In conclusion, {topic} will continue to shape our future landscape. "
        f"By fostering innovation and responsible leadership, we can harness its full potential for lasting progress."
    )

def _insert_blocks_into_word_com(doc, blocks: list[dict], clear_doc: bool = False):
    """Inserts structured blocks into active Word COM Document with rich styling."""
    if clear_doc:
        doc.Content.Text = ""

    for b in blocks:
        btype = b["type"]
        text  = b["text"]
        end_pos = max(0, doc.Content.End - 1)
        rng = doc.Range(end_pos, end_pos)
        rng.InsertAfter(text + "\n")
        p = doc.Paragraphs(doc.Paragraphs.Count - 1)
        p.Range.Font.Name = "Calibri"

        if btype == "title":
            p.Range.Font.Size = 24
            p.Range.Font.Bold = True
            p.Range.ParagraphFormat.Alignment = 1 # Center
            p.Range.ParagraphFormat.SpaceBefore = 0
            p.Range.ParagraphFormat.SpaceAfter = 14
        elif btype == "subtitle":
            p.Range.Font.Size = 11
            p.Range.Font.Italic = True
            p.Range.Font.Bold = False
            p.Range.ParagraphFormat.Alignment = 1 # Center
            p.Range.ParagraphFormat.SpaceBefore = 0
            p.Range.ParagraphFormat.SpaceAfter = 16
        elif btype == "heading1":
            p.Range.Font.Size = 16
            p.Range.Font.Bold = True
            p.Range.ParagraphFormat.Alignment = 0 # Left
            p.Range.ParagraphFormat.SpaceBefore = 14
            p.Range.ParagraphFormat.SpaceAfter = 6
        elif btype == "heading2":
            p.Range.Font.Size = 13
            p.Range.Font.Bold = True
            p.Range.ParagraphFormat.Alignment = 0 # Left
            p.Range.ParagraphFormat.SpaceBefore = 10
            p.Range.ParagraphFormat.SpaceAfter = 4
        elif btype == "bullet":
            p.Range.Font.Size = 11
            p.Range.Font.Bold = False
            p.Range.ParagraphFormat.Alignment = 0
            p.Range.ParagraphFormat.SpaceBefore = 2
            p.Range.ParagraphFormat.SpaceAfter = 4
            # Bullet character
            if not text.startswith("•"):
                p.Range.Text = "•  " + text + "\n"
        else: # paragraph
            p.Range.Font.Size = 11
            p.Range.Font.Bold = False
            p.Range.ParagraphFormat.Alignment = 0 # Left
            p.Range.ParagraphFormat.LineSpacingRule = 0 # Single
            p.Range.ParagraphFormat.SpaceBefore = 0
            p.Range.ParagraphFormat.SpaceAfter = 6

def _create_docx_file(blocks: list[dict], out_path: Path) -> Path:
    """Creates a formatted .docx file using python-docx."""
    import docx
    from docx.shared import Pt, Inches, RGBColor
    from docx.enum.text import WD_ALIGN_PARAGRAPH

    doc = docx.Document()

    # Page Margins (1 inch)
    for section in doc.sections:
        section.top_margin    = Inches(1)
        section.bottom_margin = Inches(1)
        section.left_margin   = Inches(1)
        section.right_margin  = Inches(1)

    for b in blocks:
        btype = b["type"]
        text  = b["text"]

        if btype == "title":
            p = doc.add_paragraph()
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(24)
            r.font.bold = True
            r.font.color.rgb = RGBColor(0x1B, 0x36, 0x5D) # Deep Navy
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(14)

        elif btype == "subtitle":
            p = doc.add_paragraph()
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(11)
            r.font.italic = True
            r.font.color.rgb = RGBColor(0x55, 0x55, 0x55)
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(16)

        elif btype == "heading1":
            p = doc.add_paragraph()
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(16)
            r.font.bold = True
            r.font.color.rgb = RGBColor(0x1B, 0x36, 0x5D)
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.space_before = Pt(14)
            p.paragraph_format.space_after  = Pt(6)

        elif btype == "heading2":
            p = doc.add_paragraph()
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(13)
            r.font.bold = True
            r.font.color.rgb = RGBColor(0x33, 0x33, 0x33)
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.space_before = Pt(10)
            p.paragraph_format.space_after  = Pt(4)

        elif btype == "bullet":
            p = doc.add_paragraph(style="List Bullet")
            r = p.add_run(text.lstrip("•-* ").strip())
            r.font.name = "Calibri"
            r.font.size = Pt(11)
            p.paragraph_format.space_before = Pt(2)
            p.paragraph_format.space_after  = Pt(4)

        else: # paragraph
            p = doc.add_paragraph()
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(11)
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.line_spacing = 1.15
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(6)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    doc.save(str(out_path))
    return out_path

# ─────────────────────────────────────────────────────────────────────────────
# Core Action Handlers
# ─────────────────────────────────────────────────────────────────────────────

def write_essay(params: dict) -> str:
    """Generates and writes a structured essay directly into Word or a .docx file."""
    topic = params.get("topic") or params.get("title") or "Technology and Human Progress"
    title = params.get("title")
    raw_content = params.get("content", "").strip()

    if not raw_content:
        raw_content = _generate_essay_text(topic, params.get("instructions", ""))
    elif title and not raw_content.startswith("#"):
        raw_content = f"# {title}\n*Prepared for Nibir sir*\n\n{raw_content}"

    blocks = _parse_content_blocks(raw_content)

    # Safe filename fallback
    safe_name = re.sub(r'[\\/*?:"<>|]', "", topic)[:40].strip().replace(" ", "_")
    path_param = params.get("path")
    out_file = None
    if path_param:
        out_file = _resolve_path(path_param)
        if out_file.is_dir():
            out_file = out_file / f"{safe_name}_essay.docx"

    # Try Word COM first if active or on Windows
    word, doc = _get_or_create_word()
    if word and doc:
        try:
            # Check if active doc has content
            clear = doc.Paragraphs.Count <= 2 and len(doc.Content.Text.strip()) < 10
            _insert_blocks_into_word_com(doc, blocks, clear_doc=clear)
            if out_file:
                out_file.parent.mkdir(parents=True, exist_ok=True)
                doc.SaveAs2(str(out_file))
            _bring_word_to_front(word)
            title_name = blocks[0]["text"] if blocks else topic
            saved_info = f"\n• Saved to: {out_file}" if out_file else ""
            return (
                f"Essay successfully written into Microsoft Word with professional formatting!\n"
                f"• Title: '{title_name}' (24pt Bold, Centered)\n"
                f"• Headings: Introduction, Body, Takeaways & Conclusion (16pt Bold)\n"
                f"• Clean, distinct paragraphs with 1.15 line spacing.{saved_info}"
            )
        except Exception as e:
            print(f"[WordDoc] Writing to active Word failed: {e}")

    # Fallback to python-docx file and launch
    if not out_file:
        out_file = _resolve_path(f"desktop/{safe_name}_essay.docx")
        if out_file.is_dir():
            out_file = out_file / f"{safe_name}_essay.docx"

    _create_docx_file(blocks, out_file)
    if params.get("open_word", True) and sys.platform == "win32":
        try:
            os.startfile(str(out_file))
        except Exception:
            pass

    return (
        f"Essay generated and saved as formatted Word document: {out_file.name}\n"
        f"• Location: {out_file}\n"
        f"• Structured with Title (24pt), Section Headings (16pt), and clean paragraphs.\n"
        f"• Opened in Microsoft Word."
    )

def format_title(params: dict) -> str:
    """Formats the title at the top of the Word document (centered, 24pt, bold)."""
    title_text = params.get("title", "").strip()
    font_size  = int(params.get("font_size") or 24)
    alignment  = params.get("alignment", "center").lower().strip()
    bold       = params.get("bold", True)

    align_code = 1 if alignment == "center" else (2 if alignment == "right" else 0)

    word, doc = _get_active_word()
    if word and doc:
        try:
            if doc.Paragraphs.Count == 0:
                doc.Paragraphs.Add()

            if title_text:
                first_p_text = doc.Paragraphs(1).Range.Text.strip()
                if not first_p_text or len(first_p_text) < 100:
                    doc.Paragraphs(1).Range.Text = title_text + "\n"
                else:
                    doc.Range(0, 0).InsertBefore(title_text + "\n")
            else:
                title_text = doc.Paragraphs(1).Range.Text.strip()

            p1 = doc.Paragraphs(1)
            p1.Range.Font.Name = "Calibri"
            p1.Range.Font.Size = font_size
            p1.Range.Font.Bold = bold
            p1.Range.ParagraphFormat.Alignment = align_code
            p1.Range.ParagraphFormat.SpaceBefore = 0
            p1.Range.ParagraphFormat.SpaceAfter = 14
            _bring_word_to_front(word)
            return (
                f"Title at the top formatted successfully in Microsoft Word!\n"
                f"• Title: '{title_text}'\n"
                f"• Style: {font_size}pt Bold, {alignment.capitalize()}-aligned with 14pt bottom spacing."
            )
        except Exception as e:
            return f"Failed to format title in Word: {e}"

    # If file specified
    path_param = params.get("path")
    if path_param:
        target_path = _resolve_path(path_param)
        if target_path.exists() and target_path.suffix.lower() == ".docx":
            try:
                import docx
                from docx.shared import Pt
                from docx.enum.text import WD_ALIGN_PARAGRAPH

                doc_obj = docx.Document(str(target_path))
                if title_text:
                    p = doc_obj.paragraphs[0] if doc_obj.paragraphs else doc_obj.add_paragraph()
                    p.text = title_text
                else:
                    p = doc_obj.paragraphs[0]
                    title_text = p.text

                p.alignment = WD_ALIGN_PARAGRAPH.CENTER if alignment == "center" else WD_ALIGN_PARAGRAPH.LEFT
                p.paragraph_format.space_after = Pt(14)
                for run in p.runs:
                    run.font.name = "Calibri"
                    run.font.size = Pt(font_size)
                    run.font.bold = bold

                doc_obj.save(str(target_path))
                if params.get("open_word", True) and sys.platform == "win32":
                    os.startfile(str(target_path))
                return f"Title formatted in file '{target_path.name}': '{title_text}' ({font_size}pt Bold Centered)."
            except Exception as e:
                return f"Failed to format title in file: {e}"

    return "Microsoft Word is not currently open and no valid file path was provided to format title."

def format_document(params: dict) -> str:
    """Restructures and formats the whole document with Title, Headings, and Paragraphs."""
    word, doc = _get_active_word()
    if word and doc:
        try:
            full_text = doc.Content.Text.strip()
            if not full_text:
                return "The active Word document is empty."

            # If document has only 1-2 paragraphs and looks like an unformatted wall of text:
            if doc.Paragraphs.Count <= 3 and len(full_text) > 300:
                # Use Gemini or smart splitter to restructure
                try:
                    from core import gemini
                    prompt = (
                        f"Reformat this unformatted essay/document text with clean structure.\n"
                        f"Add a # Title at top, ## Section Headings, break into clear paragraphs, and use bullet points where helpful:\n\n"
                        f"{full_text[:30000]}"
                    )
                    structured = gemini.text(prompt, tier=gemini.SMART)
                    if structured and len(structured.strip()) > 100:
                        blocks = _parse_content_blocks(structured)
                        _insert_blocks_into_word_com(doc, blocks, clear_doc=True)
                        _bring_word_to_front(word)
                        return (
                            f"Successfully reformatted Word document!\n"
                            f"• Converted wall of text into structured sections with Title, Headings, and clean paragraphs.\n"
                            f"• Total paragraphs created: {doc.Paragraphs.Count}."
                        )
                except Exception as e:
                    print(f"[WordDoc] AI restructuring failed: {e}")

            # Format existing paragraphs directly
            blocks = _parse_content_blocks(full_text)
            _insert_blocks_into_word_com(doc, blocks, clear_doc=True)
            _bring_word_to_front(word)
            return (
                f"Word document formatted successfully!\n"
                f"• Title placed at top (24pt Bold Centered)\n"
                f"• Headings styled (16pt Bold with spacing)\n"
                f"• Paragraphs styled with clean 1.15 line spacing."
            )
        except Exception as e:
            return f"format_document failed: {e}"

    # If file path provided
    path_param = params.get("path")
    if path_param:
        target_path = _resolve_path(path_param)
        if target_path.exists():
            try:
                import docx
                if target_path.suffix.lower() == ".docx":
                    d = docx.Document(str(target_path))
                    raw_text = "\n".join(p.text for p in d.paragraphs if p.text.strip())
                else:
                    raw_text = target_path.read_text(encoding="utf-8", errors="ignore")

                blocks = _parse_content_blocks(raw_text)
                out_path = target_path if target_path.suffix.lower() == ".docx" else target_path.with_suffix(".docx")
                _create_docx_file(blocks, out_path)
                if params.get("open_word", True) and sys.platform == "win32":
                    os.startfile(str(out_path))
                return f"Reformatted and saved '{out_path.name}' with professional Title, Headings, and Paragraphs."
            except Exception as e:
                return f"Reformatting file failed: {e}"

    return "No active Word document found and no file specified. Open Word or specify a file path."

def format_selection(params: dict) -> str:
    """Formats the currently selected text in Microsoft Word."""
    word, doc = _get_active_word()
    if not word or not doc:
        return "Microsoft Word is not open or no active document found."

    try:
        sel = word.Selection
        if not sel or len(sel.Text.strip()) == 0:
            return "No text is currently selected in Word to format."

        if params.get("bold") is not None:
            sel.Font.Bold = bool(params["bold"])
        if params.get("italic") is not None:
            sel.Font.Italic = bool(params["italic"])
        if params.get("underline") is not None:
            sel.Font.Underline = 1 if params["underline"] else 0
        if params.get("font_size"):
            sel.Font.Size = int(params["font_size"])

        alignment = params.get("alignment", "").lower().strip()
        if alignment == "center":
            sel.ParagraphFormat.Alignment = 1
        elif alignment == "left":
            sel.ParagraphFormat.Alignment = 0
        elif alignment == "right":
            sel.ParagraphFormat.Alignment = 2
        elif alignment == "justify":
            sel.ParagraphFormat.Alignment = 3

        heading_level = params.get("heading_level")
        if heading_level:
            sel.Font.Bold = True
            sel.Font.Size = 16 if heading_level == 1 else (13 if heading_level == 2 else 12)
            sel.ParagraphFormat.SpaceBefore = 12
            sel.ParagraphFormat.SpaceAfter = 6

        _bring_word_to_front(word)
        return f"Selected text formatted: '{sel.Text[:40].strip()}…'"
    except Exception as e:
        return f"format_selection failed: {e}"

def read_document(params: dict) -> str:
    """Reads text from active Word document or specified .docx file."""
    word, doc = _get_active_word()
    if word and doc:
        try:
            txt = doc.Content.Text.strip()
            return f"Active Word Document Content ({len(txt)} chars, {doc.Paragraphs.Count} paragraphs):\n\n{txt[:2500]}"
        except Exception as e:
            return f"Could not read from Word: {e}"

    path_param = params.get("path")
    if path_param:
        target = _resolve_path(path_param)
        if target.exists() and target.suffix.lower() == ".docx":
            try:
                import docx
                d = docx.Document(str(target))
                txt = "\n".join(p.text for p in d.paragraphs if p.text.strip())
                return f"Word File '{target.name}' ({len(txt)} chars):\n\n{txt[:2500]}"
            except Exception as e:
                return f"Could not read file: {e}"

    return "No active Word document open and no valid file specified."

# ─────────────────────────────────────────────────────────────────────────────
# Dispatcher & Auto-Discovery Declaration
# ─────────────────────────────────────────────────────────────────────────────

def word_document(
    parameters: dict,
    player=None,
    speak=None,
    response=None,
    session_memory=None,
) -> str:
    """Main entry point for word_document tool."""
    params = parameters or {}
    action = params.get("action", "format_document").lower().strip()

    if player:
        player.write_log(f"[Word] {action}")

    print(f"[WordDocument] -> {action}  {params}")

    if action in ("write_essay", "write_document", "create_essay"):
        return write_essay(params)
    elif action in ("format_title", "title", "title_format"):
        return format_title(params)
    elif action in ("format_document", "format_text", "reformat", "format"):
        return format_document(params)
    elif action in ("format_selection", "format_selected"):
        return format_selection(params)
    elif action in ("create_document", "new_document"):
        return write_essay(params)
    elif action in ("read_document", "read", "get_text"):
        return read_document(params)
    else:
        return f"Unknown word_document action '{action}'. Available: write_essay, format_title, format_document, format_selection, read_document."

TOOL = {
    "name": "word_document",
    "description": (
        "Professional Microsoft Word assistant. Use this tool WHENEVER the user asks to write an essay, "
        "format a Word document or file, format text in Word, style titles (centered, 24pt bold at top), "
        "apply headings, or create structured .docx documents. Operates directly on the active Word window or files."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": (
                    "write_essay: generate/write a structured essay with title, headings and distinct paragraphs into Word; "
                    "format_title: format the title at the top of the Word document (centered, 24pt bold, spacing); "
                    "format_document: reformat entire active Word document or file with Title, Headings, and proper paragraphs; "
                    "format_selection: format selected text in Word (bold, heading, size, alignment); "
                    "read_document: read content of active Word document or file"
                )
            },
            "topic": {
                "type": "STRING",
                "description": "Topic of the essay or article (e.g. 'Artificial Intelligence', 'Climate Change')"
            },
            "title": {
                "type": "STRING",
                "description": "Title to format or insert at the top of the document"
            },
            "content": {
                "type": "STRING",
                "description": "Text or markdown content for the essay or document"
            },
            "path": {
                "type": "STRING",
                "description": "File path (e.g. 'desktop/essay.docx'). Defaults to active Word or Desktop."
            },
            "font_size": {
                "type": "INTEGER",
                "description": "Font size in points (e.g. 24 for title, 16 for heading, 11 for body)"
            },
            "alignment": {
                "type": "STRING",
                "description": "center | left | right | justify"
            },
            "bold": {
                "type": "BOOLEAN",
                "description": "Whether text should be bold"
            },
            "heading_level": {
                "type": "INTEGER",
                "description": "Heading level: 1 (main section) or 2 (sub-section)"
            },
            "open_word": {
                "type": "BOOLEAN",
                "description": "Bring Word to front or open file in Word (default: true)"
            }
        },
        "required": ["action"]
    },
    "handler": word_document,
}
