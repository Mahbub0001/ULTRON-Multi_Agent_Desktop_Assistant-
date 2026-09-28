"""
actions/email_assistant.py

Gmail & Email Manager for ULTRON / JARVIS.
Auto-discovered by core/action_loader.py via the module-level TOOL declaration.

Capabilities:
1. Send emails via SMTP (Gmail, Outlook, or custom SMTP server) with attachments.
2. Read & search inbox via IMAP (fetches unread or recent emails with summaries).
3. Draft professional emails and previews.
4. Fallback to webmail compose (browser-based) when credentials are not configured or requested.
5. Setup guidance for Google App Passwords and local configuration.
"""

from __future__ import annotations

import json
import mimetypes
import os
import re
import smtplib
import imaplib
import sys
import webbrowser
from datetime import datetime, date, timedelta
from email import policy
from email.header import decode_header
from email.message import EmailMessage
from email.parser import BytesParser
from pathlib import Path
from urllib.parse import quote_plus


def _base_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent


def _read_config_file() -> dict:
    """Reads configuration from config/email_config.json if present."""
    config_path = _base_dir() / "config" / "email_config.json"
    if not config_path.is_file():
        return {}
    try:
        return json.loads(config_path.read_text(encoding="utf-8"))
    except Exception as e:
        print(f"[EmailAssistant] Error reading email_config.json: {e}")
        return {}


def load_email_config() -> dict:
    """Public helper to load email credentials and server configurations."""
    cfg = _read_config_file()
    email_addr = (cfg.get("email") or "").strip()
    app_pwd = (cfg.get("app_password") or "").strip().replace(" ", "")
    user_name = (cfg.get("user_name") or "").strip()

    smtp_srv = (cfg.get("smtp_server") or "smtp.gmail.com").strip()
    smtp_port = int(cfg.get("smtp_port") or 587)
    imap_srv = (cfg.get("imap_server") or "imap.gmail.com").strip()
    imap_port = int(cfg.get("imap_port") or 993)

    return {
        "email": email_addr,
        "app_password": app_pwd,
        "user_name": user_name,
        "smtp_server": smtp_srv,
        "smtp_port": smtp_port,
        "imap_server": imap_srv,
        "imap_port": imap_port,
        "is_configured": bool(email_addr and app_pwd),
    }


_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
_BENGALI_DIGITS = str.maketrans("০১২৩৪৫৬৭৮৯", "0123456789")
_DEVANAGARI_DIGITS = str.maketrans("०१२३४५६७८९", "0123456789")


def _format_imap_date(dt: date) -> str:
    """Formats a datetime.date object into RFC 3501 IMAP format: DD-Mon-YYYY."""
    return f"{dt.day:02d}-{_MONTHS[dt.month - 1]}-{dt.year}"


def _parse_date_to_imap(date_input: str | None) -> str | None:
    """
    Parses natural language date strings into IMAP DD-Mon-YYYY format.
    Supports:
      - 'today', 'আজ', 'আজকে', 'aaj', 'current'
      - 'yesterday', 'গতকাল', 'গত কাল', 'kal'
      - Day numbers: '28', '28th', '28 তারিখ', '28 तारीख', '২৮ তারিখ'
      - ISO / standard dates: '2026-09-28', '28-09-2026', '28/09/2026', '28-Sep-2026'
    """
    if not date_input or not isinstance(date_input, str):
        return None
    raw = date_input.strip().lower()
    if not raw:
        return None

    # Normalize Bengali and Devanagari numerals to ASCII
    raw = raw.translate(_BENGALI_DIGITS).translate(_DEVANAGARI_DIGITS)

    today = datetime.now().date()

    if raw in ("today", "আজ", "আজকে", "aaj", "current", "আজকের"):
        return _format_imap_date(today)
    if raw in ("yesterday", "গতকাল", "গত কাল", "kal", "গতকালের"):
        return _format_imap_date(today - timedelta(days=1))

    # Try standard date string patterns
    for fmt in ("%Y-%m-%d", "%d-%m-%Y", "%d/%m/%Y", "%d-%b-%Y", "%d %b %Y", "%d %B %Y"):
        try:
            parsed = datetime.strptime(raw, fmt).date()
            return _format_imap_date(parsed)
        except ValueError:
            pass

    # Extract 1 or 2 digit day of current month (e.g. '28 তারিখ', '28 तारीख', '28th', '28')
    m = re.search(r"\b(\d{1,2})\b", raw)
    if m:
        day_num = int(m.group(1))
        if 1 <= day_num <= 31:
            try:
                parsed = date(today.year, today.month, day_num)
                return _format_imap_date(parsed)
            except ValueError:
                pass

    return None


def _decode_mime_str(header_val: str | None) -> str:
    """Decodes MIME encoded header strings (e.g. =?UTF-8?B?...?=)."""
    if not header_val:
        return ""
    decoded_fragments = []
    try:
        for text, encoding in decode_header(header_val):
            if isinstance(text, bytes):
                text = text.decode(encoding or "utf-8", errors="replace")
            decoded_fragments.append(str(text))
        return "".join(decoded_fragments).strip()
    except Exception:
        return str(header_val)


def _extract_body_snippet(msg: EmailMessage, max_chars: int = 250) -> str:
    """Extracts a clean plaintext snippet from an EmailMessage object."""
    try:
        body = ""
        if msg.is_multipart():
            for part in msg.walk():
                ctype = part.get_content_type()
                cdispo = str(part.get("Content-Disposition", ""))
                if ctype == "text/plain" and "attachment" not in cdispo:
                    payload = part.get_payload(decode=True)
                    if payload:
                        charset = part.get_content_charset() or "utf-8"
                        body = payload.decode(charset, errors="replace")
                        break
            if not body:
                # Fallback to text/html if plain text was not found
                for part in msg.walk():
                    if part.get_content_type() == "text/html":
                        payload = part.get_payload(decode=True)
                        if payload:
                            charset = part.get_content_charset() or "utf-8"
                            body = payload.decode(charset, errors="replace")
                            break
        else:
            payload = msg.get_payload(decode=True)
            if payload:
                charset = msg.get_content_charset() or "utf-8"
                body = payload.decode(charset, errors="replace")

        # Strip html tags if html body
        clean_text = " ".join(body.split())
        if len(clean_text) > max_chars:
            return clean_text[:max_chars] + "..."
        return clean_text or "(No text body)"
    except Exception as e:
        return f"(Could not parse body snippet: {e})"


def _setup_guide() -> str:
    return (
        "📧 **ULTRON Email Setup Guide (Gmail & IMAP/SMTP)**\n\n"
        "To enable direct sending and inbox reading without opening a browser:\n"
        "1. **Enable 2-Step Verification** on your Google Account: https://myaccount.google.com/security\n"
        "2. Navigate to **2-Step Verification** -> scroll down to **App passwords** (or visit https://myaccount.google.com/apppasswords) to generate a **Google App Password**.\n"
        "3. Create a new App name (e.g. `ULTRON Assistant`) and copy the generated **16-character password**.\n"
        "4. Create a file at `config/email_config.json` (you can copy `config/email_config.example.json`):\n"
        "```json\n"
        "{\n"
        '    "email": "your_address@gmail.com",\n'
        '    "app_password": "abcd efgh ijkl mnop",\n'
        '    "user_name": "Your Name"\n'
        "}\n"
        "```\n"
        "🔒 *Note: `config/email_config.json` is strictly git-ignored so your credentials remain safe on your local PC.*"
    )


def _open_webmail(recipient: str = "", subject: str = "", body: str = "") -> str:
    """Opens Gmail webmail compose window pre-filled with recipient, subject, and body."""
    params = ["view=cm", "fs=1"]
    if recipient:
        params.append(f"to={quote_plus(recipient)}")
    if subject:
        params.append(f"su={quote_plus(subject)}")
    if body:
        params.append(f"body={quote_plus(body)}")

    query_str = "&".join(params)
    url = f"https://mail.google.com/mail/?{query_str}"

    try:
        webbrowser.open(url)
        return (
            f"Opened webmail composer in your default browser for {recipient or 'new email'}.\n"
            f"URL: {url}"
        )
    except Exception as e:
        return f"Failed to open webmail in browser: {e}"


def _draft_email(recipient: str = "", subject: str = "", body: str = "", attachments: list | None = None) -> str:
    """Prepares and previews an email draft."""
    att_str = ", ".join(str(a) for a in (attachments or [])) if attachments else "None"
    return (
        "📝 **Email Draft Prepared**\n"
        f"**To:** {recipient or '(Not specified)'}\n"
        f"**Subject:** {subject or '(No subject)'}\n"
        f"**Attachments:** {att_str}\n\n"
        f"**Body:**\n{body or '(Empty body)'}\n\n"
        "💡 *Say 'Send this email' to dispatch it via SMTP, or 'Open in Gmail' to review in browser.*"
    )


def _send_email(
    config: dict,
    recipient: str,
    subject: str,
    body: str,
    attachments: list | None = None,
    fallback_to_web: bool = True,
    player=None,
) -> str:
    """Sends an email using SMTP or falls back to Webmail compose."""
    if not recipient:
        return "Recipient email address is required to send an email."
    if not body:
        return "Email body content is required to send an email."

    if not config.get("is_configured"):
        if fallback_to_web:
            _log("Email credentials not configured in config/email_config.json. Falling back to webmail compose.", player)
            web_msg = _open_webmail(recipient=recipient, subject=subject, body=body)
            return (
                "⚠️ Local SMTP credentials not found in `config/email_config.json`.\n"
                f"Webmail composer opened instead:\n{web_msg}\n\n"
                "To send emails directly in background without opening the browser, configure `config/email_config.json`."
            )
        return "Email credentials are not configured. Run the `setup_guide` action for instructions."

    sender_email = config["email"]
    app_pwd = config["app_password"]
    user_name = config.get("user_name")
    smtp_srv = config.get("smtp_server", "smtp.gmail.com")
    smtp_port = int(config.get("smtp_port", 587))

    msg = EmailMessage()
    if user_name:
        msg["From"] = f"{user_name} <{sender_email}>"
    else:
        msg["From"] = sender_email
    msg["To"] = recipient
    msg["Subject"] = subject or "(No Subject)"
    msg.set_content(body)

    # Attachments
    attached_names = []
    if attachments:
        for file_item in attachments:
            file_path = Path(str(file_item).strip())
            if not file_path.is_file():
                continue
            ctype, encoding = mimetypes.guess_type(str(file_path))
            if ctype is None or encoding is not None:
                ctype = "application/octet-stream"
            maintype, subtype = ctype.split("/", 1)
            try:
                with open(file_path, "rb") as fp:
                    file_data = fp.read()
                msg.add_attachment(
                    file_data,
                    maintype=maintype,
                    subtype=subtype,
                    filename=file_path.name,
                )
                attached_names.append(file_path.name)
            except Exception as att_err:
                _log(f"Failed to attach {file_path.name}: {att_err}", player)

    try:
        _log(f"Connecting to SMTP server {smtp_srv}:{smtp_port}...", player)
        if smtp_port == 465:
            with smtplib.SMTP_SSL(smtp_srv, smtp_port, timeout=20) as server:
                server.login(sender_email, app_pwd)
                server.send_message(msg)
        else:
            with smtplib.SMTP(smtp_srv, smtp_port, timeout=20) as server:
                server.starttls()
                server.login(sender_email, app_pwd)
                server.send_message(msg)

        att_notice = f" with {len(attached_names)} attachment(s): {', '.join(attached_names)}" if attached_names else ""
        res = f"✅ Email sent successfully to {recipient} (Subject: '{msg['Subject']}'){att_notice}."
        _log(res, player)
        return res
    except Exception as e:
        err_msg = f"Failed to send email via SMTP: {e}"
        _log(err_msg, player)
        if fallback_to_web:
            web_msg = _open_webmail(recipient=recipient, subject=subject, body=body)
            return f"❌ {err_msg}\nOpened webmail composer as fallback:\n{web_msg}"
        return f"❌ {err_msg}"


def _count_emails(
    config: dict,
    date_input: str | None = None,
    player=None,
) -> str:
    """Counts incoming emails for a specific date or overall inbox unread/total."""
    if not config.get("is_configured"):
        return (
            "⚠️ Email credentials not configured in `config/email_config.json`.\n"
            "Cannot connect to IMAP inbox. Run the `setup_guide` action for step-by-step setup."
        )

    user_email = config["email"]
    app_pwd = config["app_password"]
    imap_srv = config.get("imap_server", "imap.gmail.com")
    imap_port = int(config.get("imap_port", 993))

    imap_date = _parse_date_to_imap(date_input)

    try:
        _log(f"Connecting to IMAP {imap_srv}:{imap_port} for email count...", player)
        imap = imaplib.IMAP4_SSL(imap_srv, imap_port, timeout=25)
        imap.login(user_email, app_pwd)
        status, _ = imap.select("INBOX")
        if status != "OK":
            return "Failed to access INBOX."

        if imap_date:
            # Count total for this specific date
            crit_all = f'(ON "{imap_date}")'
            st_all, data_all = imap.search(None, crit_all)
            all_ids = data_all[0].split() if (st_all == "OK" and data_all and data_all[0]) else []

            # Count unread for this specific date
            crit_unseen = f'(UNSEEN ON "{imap_date}")'
            st_unseen, data_unseen = imap.search(None, crit_unseen)
            unseen_ids = data_unseen[0].split() if (st_unseen == "OK" and data_unseen and data_unseen[0]) else []

            total_count = len(all_ids)
            unread_count = len(unseen_ids)
            read_count = max(0, total_count - unread_count)

            today_str = _format_imap_date(datetime.now().date())
            date_label = f"আজকে ({imap_date})" if imap_date == today_str else f"{imap_date} তারিখে"

            if total_count == 0:
                res = f"📊 **ইমেইল গণনা ({date_label}):**\nএই নির্দিষ্ট তারিখে আপনার ইনবক্সে কোনো ইমেইল পাওয়া যায়নি (০টি)।"
                _log(res, player)
                try:
                    imap.close()
                    imap.logout()
                except Exception:
                    pass
                return res

            # Fetch preview of recent subjects on that date (up to 3)
            recent_ids = all_ids[-3:]
            recent_ids.reverse()
            preview_lines = []
            for mid in recent_ids:
                try:
                    f_res, f_data = imap.fetch(mid, "(BODY.PEEK[HEADER.FIELDS (SUBJECT FROM)])")
                    if f_res == "OK" and f_data:
                        for part in f_data:
                            if isinstance(part, tuple) and len(part) >= 2:
                                raw_header = part[1]
                                msg = BytesParser(policy=policy.default).parsebytes(raw_header)
                                subj = _decode_mime_str(msg.get("subject", "(No Subject)"))
                                frm = _decode_mime_str(msg.get("from", "(Unknown)"))
                                preview_lines.append(f"   • **{subj}** (From: {frm})")
                                break
                except Exception:
                    pass

            try:
                imap.close()
                imap.logout()
            except Exception:
                pass

            previews_str = ("\n**সাম্প্রতিক কয়েকটি ইমেইল:**\n" + "\n".join(preview_lines)) if preview_lines else ""
            res = (
                f"📊 **ইমেইল গণনা ({date_label}):**\n"
                f"• মোট ইমেইল এসেছে: {total_count} টি\n"
                f"• এর মধ্যে অপঠিত (Unread): {unread_count} টি\n"
                f"• পঠিত (Read): {read_count} টি\n"
                f"{previews_str}\n\n"
                "💡 *বিস্তারিত পড়তে চাইলে 'আজকের মেইলগুলো দেখাও' বলতে পারেন।*"
            )
            _log(res, player)
            return res

        else:
            # Overall inbox counts
            st_unseen, data_unseen = imap.search(None, "UNSEEN")
            unseen_ids = data_unseen[0].split() if (st_unseen == "OK" and data_unseen and data_unseen[0]) else []

            st_all, data_all = imap.search(None, "ALL")
            all_ids = data_all[0].split() if (st_all == "OK" and data_all and data_all[0]) else []

            try:
                imap.close()
                imap.logout()
            except Exception:
                pass

            res = (
                f"📊 **ইনবক্স ইমেইল গণনা:**\n"
                f"• মোট অপঠিত (Unread) ইমেইল: {len(unseen_ids)} টি\n"
                f"• ইনবক্সে সর্বমোট সংরক্ষিত ইমেইল: {len(all_ids)} টি\n\n"
                "💡 *আজকের বা নির্দিষ্ট দিনের হিসাব জানতে 'আজকের দিনে কয়টা মেইল আসছে?' বলুন।*"
            )
            _log(res, player)
            return res

    except Exception as e:
        err = f"Failed to count emails via IMAP: {e}"
        _log(err, player)
        return f"❌ {err}"


def _fetch_emails(
    config: dict,
    limit: int = 5,
    unread_only: bool = True,
    query: str | None = None,
    date_input: str | None = None,
    player=None,
) -> str:
    """Fetches and summarizes emails from IMAP inbox with optional date filtering."""
    if not config.get("is_configured"):
        return (
            "⚠️ Email credentials not configured in `config/email_config.json`.\n"
            "Cannot connect to IMAP inbox. Run the `setup_guide` action for step-by-step setup."
        )

    user_email = config["email"]
    app_pwd = config["app_password"]
    imap_srv = config.get("imap_server", "imap.gmail.com")
    imap_port = int(config.get("imap_port", 993))

    imap_date = _parse_date_to_imap(date_input)

    try:
        _log(f"Connecting to IMAP {imap_srv}:{imap_port}...", player)
        imap = imaplib.IMAP4_SSL(imap_srv, imap_port, timeout=25)
        imap.login(user_email, app_pwd)
        status, _ = imap.select("INBOX")
        if status != "OK":
            return "Failed to access INBOX."

        # Build search criteria
        criteria_parts = []
        if imap_date:
            criteria_parts.append(f'ON "{imap_date}"')

        if query and query.strip():
            clean_q = query.strip().replace('"', "")
            criteria_parts.append(f'(OR (SUBJECT "{clean_q}") (FROM "{clean_q}"))')
        elif unread_only:
            criteria_parts.append("UNSEEN")
        elif not imap_date:
            criteria_parts.append("ALL")

        search_crit = f'({" ".join(criteria_parts)})' if len(criteria_parts) > 1 else (criteria_parts[0] if criteria_parts else "ALL")

        status, data = imap.search(None, search_crit)
        if status != "OK" or not data or not data[0]:
            label_parts = []
            if imap_date:
                label_parts.append(f"on {imap_date}")
            if query:
                label_parts.append(f"matching '{query}'")
            if unread_only:
                label_parts.append("unread")
            label = " ".join(label_parts) if label_parts else "recent"
            return f"📭 No {label} emails found in INBOX."

        msg_ids = data[0].split()
        total_found = len(msg_ids)
        # Take the most recent messages up to limit
        selected_ids = msg_ids[-int(limit):]
        selected_ids.reverse()

        parsed_emails = []
        for mid in selected_ids:
            res, msg_data = imap.fetch(mid, "(RFC822)")
            if res != "OK" or not msg_data:
                continue

            for part in msg_data:
                if isinstance(part, tuple) and len(part) >= 2:
                    raw_email_bytes = part[1]
                    try:
                        msg = BytesParser(policy=policy.default).parsebytes(raw_email_bytes)
                        subject = _decode_mime_str(msg.get("subject", "(No Subject)"))
                        from_val = _decode_mime_str(msg.get("from", "(Unknown Sender)"))
                        date_val = _decode_mime_str(msg.get("date", ""))
                        snippet = _extract_body_snippet(msg, max_chars=180)
                        parsed_emails.append({
                            "id": mid.decode() if isinstance(mid, bytes) else str(mid),
                            "subject": subject,
                            "from": from_val,
                            "date": date_val,
                            "snippet": snippet,
                        })
                    except Exception as parse_err:
                        _log(f"Error parsing message {mid}: {parse_err}", player)
                    break

        try:
            imap.close()
            imap.logout()
        except Exception:
            pass

        if not parsed_emails:
            return "No emails could be fetched or parsed from INBOX."

        date_header = f" for {imap_date}" if imap_date else ""
        out_lines = [
            f"📬 **Found {total_found} email(s){date_header} (Showing {len(parsed_emails)}):**\n"
        ]
        for idx, em in enumerate(parsed_emails, 1):
            out_lines.append(
                f"**{idx}. {em['subject']}**\n"
                f"   • **From:** {em['from']}\n"
                f"   • **Date:** {em['date']}\n"
                f"   • **Preview:** {em['snippet']}\n"
            )

        return "\n".join(out_lines)

    except Exception as e:
        err = f"Failed to retrieve emails via IMAP: {e}"
        _log(err, player)
        return f"❌ {err}"


def email_assistant(
    parameters: dict,
    player=None,
    session_memory=None,
) -> str:
    """
    Main handler for email_assistant. Dispatches to send, read, search, draft,
    open_webmail, or setup_guide.
    """
    action = (parameters.get("action") or "read").strip().lower()
    recipient = (parameters.get("recipient") or "").strip()
    subject = (parameters.get("subject") or "").strip()
    body = (parameters.get("body") or "").strip()
    attachments = parameters.get("attachments") or []
    limit = parameters.get("limit", 5)
    unread_only = parameters.get("unread_only", True)
    query = parameters.get("query")
    date_val = parameters.get("date")
    fallback_to_web = parameters.get("fallback_to_web", True)

    config = load_email_config()

    if action == "setup_guide":
        return _setup_guide()

    elif action == "open_webmail":
        return _open_webmail(recipient=recipient, subject=subject, body=body)

    elif action == "draft":
        return _draft_email(recipient=recipient, subject=subject, body=body, attachments=attachments)

    elif action == "send":
        return _send_email(
            config=config,
            recipient=recipient,
            subject=subject,
            body=body,
            attachments=attachments,
            fallback_to_web=fallback_to_web,
            player=player,
        )

    elif action in ("count", "total", "summary"):
        return _count_emails(
            config=config,
            date_input=date_val,
            player=player,
        )

    elif action in ("read", "unread", "inbox"):
        return _fetch_emails(
            config=config,
            limit=limit,
            unread_only=unread_only,
            query=query,
            date_input=date_val,
            player=player,
        )

    elif action == "search":
        return _fetch_emails(
            config=config,
            limit=limit,
            unread_only=False,
            query=query or subject or recipient,
            date_input=date_val,
            player=player,
        )

    else:
        return (
            f"Unknown email action '{action}'. "
            "Supported actions: 'count', 'read', 'search', 'send', 'draft', 'open_webmail', 'setup_guide'."
        )


def _log(message: str, player=None) -> None:
    try:
        print(f"[EmailAssistant] {message}")
    except Exception:
        try:
            safe = message.encode("ascii", errors="replace").decode("ascii")
            print(f"[EmailAssistant] {safe}")
        except Exception:
            pass
    if player:
        try:
            player.write_log(f"ULTRON: {message}")
        except Exception:
            pass


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "email_assistant",
    "description": (
        "Manages email operations: counts emails on specific dates, reads/searches inbox via IMAP, "
        "sends emails via SMTP, drafts email messages, opens webmail composer, or shows setup guide."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": (
                    "Email action to perform: 'count' (count how many emails arrived on a date or overall), "
                    "'read' (fetch unread/recent inbox, optionally by date), "
                    "'search' (search emails by query/date), 'send' (dispatch email), "
                    "'draft' (format a draft), 'open_webmail' (open Gmail in browser), "
                    "or 'setup_guide' (instructions to configure credentials)."
                ),
            },
            "date": {
                "type": "STRING",
                "description": (
                    "Date filter: 'today', 'yesterday', '28', '28 তারিখ', '28 तारीख', '2026-09-28', etc. "
                    "Use when the user asks about emails for today or a specific date."
                ),
            },
            "recipient": {
                "type": "STRING",
                "description": "Recipient email address (e.g. colleague@example.com)",
            },
            "subject": {
                "type": "STRING",
                "description": "Email subject title",
            },
            "body": {
                "type": "STRING",
                "description": "Email body content or text",
            },
            "attachments": {
                "type": "ARRAY",
                "description": "List of file paths to attach to the email",
                "items": {"type": "STRING"},
            },
            "limit": {
                "type": "INTEGER",
                "description": "Number of recent emails to fetch (default: 5)",
            },
            "unread_only": {
                "type": "BOOLEAN",
                "description": "When reading emails, whether to fetch only unread/unseen messages (default: True)",
            },
            "query": {
                "type": "STRING",
                "description": "Search query keyword when searching emails by subject or sender",
            },
            "fallback_to_web": {
                "type": "BOOLEAN",
                "description": "Whether to open browser webmail composer if SMTP credentials are missing (default: True)",
            },
        },
        "required": ["action"],
    },
    "handler": email_assistant,
}
