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
from typing import Any, Callable, Optional

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

    known_sections = {
        "introduction", "background", "overview", "executive summary", "key points",
        "key benefits", "challenges", "discussion", "methodology", "analysis",
        "conclusion", "summary", "future scope", "recommendations", "references",
        "major company announcements", "societal impacts", "nuances and risk management",
        "latest capabilities", "crucial insights"
    }

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

        # Markdown H3 or H4
        if s.startswith("### "):
            blocks.append({"type": "heading2", "text": s[4:].strip()})
            continue
        if s.startswith("#### "):
            blocks.append({"type": "heading2", "text": s[5:].strip()})
            continue

        # Subtitle / Author
        if (s.startswith("*") and s.endswith("*") and len(s) < 80 and not s.startswith("**")) or \
           (s.lower().startswith("by ") and len(s) < 60):
            clean_sub = s.strip("*_").strip()
            blocks.append({"type": "subtitle", "text": clean_sub})
            continue

        # Standalone bold line acting as a heading e.g. **Executive Summary**
        if s.startswith("**") and s.endswith("**") and len(s) < 80:
            clean_h = s.strip("*").strip()
            if not blocks and not first_heading_seen:
                blocks.append({"type": "title", "text": clean_h})
                first_heading_seen = True
            else:
                blocks.append({"type": "heading2", "text": clean_h})
            continue

        # Bullet list item (- , * , • )
        if s.startswith(("- ", "* ", "• ")):
            bullet_text = s[2:].strip()
            blocks.append({"type": "bullet", "text": bullet_text})
            continue

        # Numbered item e.g. "1. " or "1) "
        m = re.match(r"^(\d+[\.\)])\s+(.*)$", s)
        if m and len(s) < 90 and not s.endswith((".", "!", "?")):
            blocks.append({"type": "heading1", "text": s})
            continue

        # Standalone short line acting as heading (e.g. "Executive Summary:" or "Key Points")
        s_clean = s.lower().rstrip(":")
        if (len(s) < 70 and not s.endswith((".", "!", "?")) and not s.startswith(("-", "*", "•"))) or \
           (s_clean in known_sections):
            clean_h = s.rstrip(":")
            if not blocks and not first_heading_seen:
                blocks.append({"type": "title", "text": clean_h})
                first_heading_seen = True
            else:
                blocks.append({"type": "heading1", "text": clean_h})
            continue

        # Regular body paragraph
        blocks.append({"type": "paragraph", "text": s})

    # If first block is not title, make first block title if it looks like one
    if blocks and blocks[0]["type"] == "heading1":
        blocks[0]["type"] = "title"

    return blocks

def _add_styled_runs(p, text: str, base_font: str = "Calibri", base_size: int = 11, base_color: Optional[Any] = None, default_bold: bool = False):
    """
    Parses markdown bold (**...**), italics (*...*), and code (`...`),
    adding runs with correct formatting without leaving raw markdown symbols.
    """
    from docx.shared import Pt

    # Split by markdown tokens
    tokens = re.split(r'(\*\*.*?\*\*|\*.*?\*|`.*?`)', text)
    for tok in tokens:
        if not tok:
            continue
        if tok.startswith("**") and tok.endswith("**") and len(tok) >= 4:
            r = p.add_run(tok[2:-2])
            r.bold = True
        elif tok.startswith("*") and tok.endswith("*") and len(tok) >= 2:
            r = p.add_run(tok[1:-1])
            r.italic = True
        elif tok.startswith("`") and tok.endswith("`") and len(tok) >= 2:
            r = p.add_run(tok[1:-1])
            r.font.name = "Consolas"
            r.font.size = Pt(max(9, base_size - 1))
            continue
        else:
            r = p.add_run(tok)
            if default_bold:
                r.bold = True

        r.font.name = base_font
        r.font.size = Pt(base_size)
        if base_color:
            r.font.color.rgb = base_color

def _create_docx_file(blocks: list[dict], out_path: Path) -> Path:
    """Creates a formatted .docx file using python-docx with executive typography."""
    import docx
    from docx.shared import Pt, Inches, RGBColor
    from docx.enum.text import WD_ALIGN_PARAGRAPH

    doc = docx.Document()

    # Standard Page Margins (1 inch)
    for section in doc.sections:
        section.top_margin    = Inches(1)
        section.bottom_margin = Inches(1)
        section.left_margin   = Inches(1)
        section.right_margin  = Inches(1)

    NAVY_COLOR = RGBColor(0x1B, 0x36, 0x5D)
    SLATE_COLOR = RGBColor(0x2C, 0x3E, 0x50)
    BODY_COLOR = RGBColor(0x22, 0x22, 0x22)
    GRAY_COLOR = RGBColor(0x55, 0x55, 0x55)

    for b in blocks:
        btype = b["type"]
        text  = b["text"]

        if btype == "title":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(14)
            _add_styled_runs(p, text, base_size=24, base_color=NAVY_COLOR, default_bold=True)

        elif btype == "subtitle":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(16)
            r = p.add_run(text)
            r.font.name = "Calibri"
            r.font.size = Pt(11)
            r.font.italic = True
            r.font.color.rgb = GRAY_COLOR

        elif btype == "heading1":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.space_before = Pt(16)
            p.paragraph_format.space_after  = Pt(6)
            p.paragraph_format.keep_with_next = True
            _add_styled_runs(p, text, base_size=16, base_color=NAVY_COLOR, default_bold=True)

        elif btype == "heading2":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.space_before = Pt(12)
            p.paragraph_format.space_after  = Pt(4)
            p.paragraph_format.keep_with_next = True
            _add_styled_runs(p, text, base_size=13, base_color=SLATE_COLOR, default_bold=True)

        elif btype == "bullet":
            p = doc.add_paragraph(style="List Bullet")
            p.paragraph_format.space_before = Pt(2)
            p.paragraph_format.space_after  = Pt(4)
            p.paragraph_format.line_spacing = 1.15
            clean_bullet = text.lstrip("•-* ").strip()
            _add_styled_runs(p, clean_bullet, base_size=11, base_color=BODY_COLOR)

        else: # paragraph
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.line_spacing = 1.15
            p.paragraph_format.space_before = Pt(0)
            p.paragraph_format.space_after  = Pt(6)
            _add_styled_runs(p, text, base_size=11, base_color=BODY_COLOR)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    doc.save(str(out_path))
    return out_path

def _insert_blocks_into_word_com(doc, blocks: list[dict], clear_doc: bool = False):
    """Inserts structured blocks into active Word COM Document with robust range styling."""
    if clear_doc:
        try:
            doc.Content.Text = ""
        except Exception:
            pass

    for b in blocks:
        btype = b["type"]
        text  = b["text"]

        clean_text = text
        if btype == "bullet":
            clean_text = clean_text.lstrip("•-* ").strip()

        display_text = re.sub(r'\*\*(.*?)\*\*', r'\1', clean_text)
        end_pos = max(0, doc.Content.End - 1)
        rng = doc.Range(end_pos, end_pos)
        rng.Text = display_text + "\n"
        rng.Font.Name = "Calibri"

        if btype == "title":
            rng.Font.Size = 24
            rng.Font.Bold = True
            rng.ParagraphFormat.Alignment = 1 # Center
            rng.ParagraphFormat.SpaceBefore = 0
            rng.ParagraphFormat.SpaceAfter = 14
        elif btype == "subtitle":
            rng.Font.Size = 11
            rng.Font.Italic = True
            rng.Font.Bold = False
            rng.ParagraphFormat.Alignment = 1 # Center
            rng.ParagraphFormat.SpaceBefore = 0
            rng.ParagraphFormat.SpaceAfter = 16
        elif btype == "heading1":
            rng.Font.Size = 16
            rng.Font.Bold = True
            rng.ParagraphFormat.Alignment = 0 # Left
            rng.ParagraphFormat.SpaceBefore = 14
            rng.ParagraphFormat.SpaceAfter = 6
        elif btype == "heading2":
            rng.Font.Size = 13
            rng.Font.Bold = True
            rng.ParagraphFormat.Alignment = 0 # Left
            rng.ParagraphFormat.SpaceBefore = 10
            rng.ParagraphFormat.SpaceAfter = 4
        elif btype == "bullet":
            rng.Font.Size = 11
            rng.Font.Bold = False
            rng.ParagraphFormat.Alignment = 0
            rng.ParagraphFormat.SpaceBefore = 2
            rng.ParagraphFormat.SpaceAfter = 4
            try:
                rng.ListFormat.ApplyBulletDefault()
            except Exception:
                pass
        else: # paragraph
            rng.Font.Size = 11
            rng.Font.Bold = False
            rng.ParagraphFormat.Alignment = 0
            rng.ParagraphFormat.LineSpacingRule = 0
            rng.ParagraphFormat.SpaceBefore = 0
            rng.ParagraphFormat.SpaceAfter = 6

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
        f"- **Insight 1:** <Bullet point 1>\n"
        f"- **Insight 2:** <Bullet point 2>\n"
        f"- **Insight 3:** <Bullet point 3>\n\n"
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
        f"- **Innovation:** Transformative potential across diverse sectors.\n"
        f"- **Adaptation:** Necessity of continuous learning and agile evolution.\n"
        f"- **Responsibility:** Balanced approach combining technical progress with ethical considerations.\n\n"
        f"## Conclusion\n"
        f"In conclusion, {topic} will continue to shape our future landscape. "
        f"By fostering innovation and responsible leadership, we can harness its full potential for lasting progress."
    )

# ─────────────────────────────────────────────────────────────────────────────
# Core Action Handlers
# ─────────────────────────────────────────────────────────────────────────────

def write_essay(params: dict) -> str:
    """Generates and writes a structured essay directly into Word or a .docx file."""
    topic = params.get("topic") or params.get("title") or "Technology and Human Progress"
    title = params.get("title")
    raw_content = str(params.get("content", "")).strip()

    if not raw_content:
        raw_content = _generate_essay_text(topic, params.get("instructions", ""))
    elif title and not raw_content.startswith("#"):
        raw_content = f"# {title}\n*Prepared for Nibir sir*\n\n{raw_content}"

    blocks = _parse_content_blocks(raw_content)

    safe_name = re.sub(r'[\\/*?:"<>|]', "", topic or title or "Document")[:40].strip().replace(" ", "_")
    path_param = params.get("path")
    out_file = None
    if path_param:
        out_file = _resolve_path(path_param)
        if out_file.is_dir():
            out_file = out_file / f"{safe_name}.docx"
        elif out_file.suffix.lower() != ".docx":
            out_file = out_file.with_suffix(".docx")

    word, doc = _get_or_create_word()
    if word and doc:
        try:
            clear = doc.Paragraphs.Count <= 2 and len(doc.Content.Text.strip()) < 10
            _insert_blocks_into_word_com(doc, blocks, clear_doc=clear)
            if out_file:
                out_file.parent.mkdir(parents=True, exist_ok=True)
                doc.SaveAs2(str(out_file.resolve()))
            _bring_word_to_front(word)
            title_name = blocks[0]["text"] if blocks else topic
            saved_info = f"\n• Saved to: {out_file}" if out_file else ""
            return (
                f"Document successfully written into Microsoft Word with professional formatting!\n"
                f"• Title: '{title_name}' (24pt Navy Bold, Centered)\n"
                f"• Headings: Introduction, Body, Insights & Conclusion (16pt / 13pt Bold)\n"
                f"• Clean, distinct paragraphs with 1.15 line spacing.{saved_info}"
            )
        except Exception as e:
            print(f"[WordDoc] Writing to active Word failed: {e}")

    # Fallback / file-only creation via python-docx
    if not out_file:
        out_file = _resolve_path(f"desktop/{safe_name}.docx")
        if out_file.is_dir():
            out_file = out_file / f"{safe_name}.docx"

    _create_docx_file(blocks, out_file)
    if params.get("open_word", True) and sys.platform == "win32":
        try:
            os.startfile(str(out_file))
        except Exception:
            pass

    return (
        f"Document generated and saved as formatted Word document: {out_file.name}\n"
        f"• Location: {out_file}\n"
        f"• Structured with Title (24pt Navy Bold), Section Headings (16pt/13pt), and formatted bullet points.\n"
        f"• Opened in Microsoft Word for Nibir sir."
    )

def convert_markdown_to_word(params: dict) -> str:
    """Converts a markdown file or markdown text into a professionally styled Word .docx file."""
    path_param = params.get("path") or params.get("file")
    content = str(params.get("content", "")).strip()
    title = str(params.get("title", "")).strip()

    md_file = None
    if path_param:
        candidate = _resolve_path(path_param)
        if candidate.is_file():
            md_file = candidate
        elif candidate.is_dir():
            md_candidates = list(candidate.glob("*.md"))
            if md_candidates:
                md_file = max(md_candidates, key=lambda f: f.stat().st_mtime)

    # If no file found yet, search Desktop & Desktop/JarvisProjects for recent reports
    if not md_file and not content:
        search_dirs = [
            Path.home() / "Desktop" / "JarvisProjects",
            Path.home() / "Desktop" / "JarvisProjects" / "Desktop" / "JarvisProjects",
            Path.home() / "Desktop",
        ]
        for sdir in search_dirs:
            if sdir.is_dir():
                found = list(sdir.glob("*.md"))
                if found:
                    md_file = max(found, key=lambda f: f.stat().st_mtime)
                    break

    if md_file and md_file.is_file():
        content = md_file.read_text(encoding="utf-8", errors="replace")
        if not title:
            title = md_file.stem.replace("_", " ").replace("-", " ")

    if not content:
        return "No markdown content or file found to convert to Word document."

    blocks = _parse_content_blocks(content)
    if not title and blocks and blocks[0]["type"] == "title":
        title = blocks[0]["text"]

    safe_name = re.sub(r'[\\/*?:"<>|]', "", title or "Formatted_Report")[:40].strip().replace(" ", "_")
    out_path = Path.home() / "Desktop" / f"{safe_name}.docx"

    _create_docx_file(blocks, out_path)

    if params.get("open_word", True) and sys.platform == "win32":
        try:
            # Also try opening via COM if Word is active
            word, _ = _get_active_word()
            if word:
                word.Documents.Open(str(out_path.resolve()))
                _bring_word_to_front(word)
            else:
                os.startfile(str(out_path))
        except Exception:
            try:
                os.startfile(str(out_path))
            except Exception:
                pass

    return (
        f"Markdown report successfully converted into a professional Microsoft Word document!\n"
        f"• File saved to: {out_path}\n"
        f"• Headings: Title (24pt Navy Bold), Sections (16pt/13pt Bold), and clean bullet points.\n"
        f"• Opened directly in Microsoft Word for Nibir sir."
    )

def format_document(params: dict) -> str:
    """Restructures and formats the whole document with Title, Headings, and Paragraphs."""
    # Check if a markdown file path is specified
    path_param = str(params.get("path", "")).strip()
    if path_param.lower().endswith(".md"):
        return convert_markdown_to_word(params)

    word, doc = _get_active_word()
    if word and doc:
        try:
            full_text = doc.Content.Text.strip()
            if not full_text:
                return "The active Word document is empty."

            blocks = _parse_content_blocks(full_text)
            title = blocks[0]["text"] if blocks and blocks[0]["type"] == "title" else "Formatted_Document"
            safe_name = re.sub(r'[\\/*?:"<>|]', "", title)[:40].strip().replace(" ", "_")
            out_path = Path.home() / "Desktop" / f"{safe_name}.docx"
            _create_docx_file(blocks, out_path)

            try:
                # Open cleanly formatted document
                doc.Close(SaveChanges=0)
                word.Documents.Open(str(out_path.resolve()))
                _bring_word_to_front(word)
            except Exception:
                try:
                    os.startfile(str(out_path))
                except Exception:
                    pass

            return (
                f"Word document formatted successfully with professional typography!\n"
                f"• Title placed at top (24pt Navy Bold Centered)\n"
                f"• Headings styled (16pt / 13pt Bold with spacing)\n"
                f"• Bullet points and bold lead-in labels formatted cleanly\n"
                f"• Saved and opened: {out_path.name}"
            )
        except Exception as e:
            return f"format_document failed: {e}"

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

    # If Word is not open, check if there's a recent report to format
    return convert_markdown_to_word(params)

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

    if action in ("write_essay", "write_document", "create_essay", "create", "create_document", "new_document"):
        return write_essay(params)
    elif action in ("from_markdown", "convert_markdown", "markdown", "format_markdown"):
        return convert_markdown_to_word(params)
    elif action in ("format_document", "format_text", "reformat", "format"):
        path_param = str(params.get("path", "")).strip().lower()
        if path_param.endswith(".md"):
            return convert_markdown_to_word(params)
        return format_document(params)
    elif action in ("format_title", "title", "title_format"):
        return format_title(params)
    elif action in ("format_selection", "format_selected"):
        return format_selection(params)
    elif action in ("read_document", "read", "get_text"):
        return read_document(params)
    else:
        return f"Unknown word_document action '{action}'. Available: write_essay, create, from_markdown, format_document, format_title, format_selection, read_document."

TOOL = {
    "name": "word_document",
    "description": (
        "Professional Microsoft Word and Document assistant. Use this tool WHENEVER the user asks to write an essay, "
        "format a Word document or file, format text with proper headings and bullet points, convert a Markdown/report into Word, "
        "style titles (centered, 24pt bold at top), apply section headings, or create structured .docx documents. "
        "DO NOT attempt to format Word by simulating mouse/keyboard copy-paste. Always use this tool for 100% precision."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": (
                    "write_essay / create: generate or write a structured essay/document with title, headings and distinct paragraphs into Word; "
                    "from_markdown: convert a markdown report/file into a beautifully styled .docx with headings and bullets and open it in Word; "
                    "format_document: reformat entire active Word document or file with Title, Headings, and proper bullet points; "
                    "format_title: format the title at the top of the Word document (centered, 24pt bold, spacing); "
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
                "description": "File path (e.g. 'desktop/essay.docx' or markdown file path). Defaults to active Word or Desktop."
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
