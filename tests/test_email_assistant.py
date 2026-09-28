import json
import os
import sys
import unittest
from email.message import EmailMessage
from pathlib import Path
from unittest.mock import MagicMock, patch

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from actions.email_assistant import (
    email_assistant,
    load_email_config,
    TOOL,
)
from core.action_loader import discover_actions


class TestEmailAssistant(unittest.TestCase):
    def setUp(self):
        self.dummy_config = {
            "email": "testuser@gmail.com",
            "app_password": "abcd efgh ijkl mnop",
            "smtp_server": "smtp.gmail.com",
            "smtp_port": 587,
            "imap_server": "imap.gmail.com",
            "imap_port": 993,
            "user_name": "Test User",
        }

    def test_tool_declaration(self):
        self.assertEqual(TOOL["name"], "email_assistant")
        self.assertIn("parameters", TOOL)
        props = TOOL["parameters"]["properties"]
        self.assertIn("action", props)
        self.assertIn("recipient", props)
        self.assertIn("subject", props)
        self.assertIn("body", props)
        self.assertIn("attachments", props)
        self.assertIn("limit", props)
        self.assertIn("query", props)
        self.assertTrue(callable(TOOL["handler"]))

    def test_action_loader_discovers_email_assistant(self):
        actions_dir = root / "actions"
        registry = discover_actions(actions_dir=actions_dir, logger=lambda x: None)
        self.assertTrue(registry.has("email_assistant"))

    def test_setup_guide(self):
        res = email_assistant({"action": "setup_guide"})
        self.assertIn("Google App Password", res)
        self.assertIn("email_config.json", res)

    def test_draft_email(self):
        res = email_assistant({
            "action": "draft",
            "recipient": "colleague@example.com",
            "subject": "Sprint Review",
            "body": "Here is the sprint summary for this week.",
        })
        self.assertIn("Draft Prepared", res)
        self.assertIn("colleague@example.com", res)
        self.assertIn("Sprint Review", res)

    def test_send_missing_parameters(self):
        res = email_assistant({"action": "send"})
        self.assertIn("recipient", res.lower())

        res2 = email_assistant({"action": "send", "recipient": "colleague@example.com"})
        self.assertIn("body", res2.lower())

    @patch("webbrowser.open")
    def test_open_webmail(self, mock_browser):
        mock_browser.return_value = True
        res = email_assistant({
            "action": "open_webmail",
            "recipient": "colleague@example.com",
            "subject": "Hello",
            "body": "World",
        })
        self.assertIn("Opened webmail", res)
        mock_browser.assert_called_once()
        opened_url = mock_browser.call_args[0][0]
        self.assertIn("mail.google.com", opened_url)
        self.assertIn("colleague%40example.com", opened_url)

    @patch("actions.email_assistant._read_config_file")
    @patch("webbrowser.open")
    def test_send_fallback_when_unconfigured(self, mock_browser, mock_read_cfg):
        mock_read_cfg.return_value = {}
        mock_browser.return_value = True

        res = email_assistant({
            "action": "send",
            "recipient": "boss@example.com",
            "subject": "Report",
            "body": "Please find the report.",
            "fallback_to_web": True,
        })
        self.assertIn("Webmail composer opened", res)
        mock_browser.assert_called_once()

    @patch("actions.email_assistant._read_config_file")
    @patch("smtplib.SMTP")
    def test_send_smtp_success(self, mock_smtp_cls, mock_read_cfg):
        mock_read_cfg.return_value = self.dummy_config
        mock_server = MagicMock()
        mock_smtp_cls.return_value.__enter__.return_value = mock_server

        res = email_assistant({
            "action": "send",
            "recipient": "boss@company.com",
            "subject": "Weekly Update",
            "body": "All deliverables are completed.",
        })
        self.assertIn("Email sent successfully", res)
        self.assertIn("boss@company.com", res)
        mock_server.starttls.assert_called_once()
        mock_server.login.assert_called_once_with("testuser@gmail.com", "abcdefghijklmnop")
        mock_server.send_message.assert_called_once()

    @patch("actions.email_assistant._read_config_file")
    @patch("actions.email_assistant.imaplib.IMAP4_SSL")
    def test_read_inbox_success(self, mock_imap_cls, mock_read_cfg):
        mock_read_cfg.return_value = self.dummy_config
        mock_imap = MagicMock()
        mock_imap_cls.return_value = mock_imap

        # Mock select
        mock_imap.select.return_value = ("OK", [b"1"])
        # Mock search: return 1 message id
        mock_imap.search.return_value = ("OK", [b"101"])

        # Construct raw email bytes
        msg = EmailMessage()
        msg["From"] = "boss@company.com"
        msg["Subject"] = "Project Alpha Status"
        msg["Date"] = "Mon, 28 Sep 2026 10:00:00 +0000"
        msg.set_content("Please share the latest slides.")
        raw_email = msg.as_bytes()

        mock_imap.fetch.return_value = ("OK", [(b"101 (RFC822 {100})", raw_email)])

        res = email_assistant({
            "action": "read",
            "limit": 5,
            "unread_only": True,
        })
        self.assertIn("Project Alpha Status", res)
        self.assertIn("boss@company.com", res)
        self.assertIn("Please share the latest slides.", res)
        mock_imap.login.assert_called_once_with("testuser@gmail.com", "abcdefghijklmnop")

    @patch("actions.email_assistant._read_config_file")
    @patch("actions.email_assistant.imaplib.IMAP4_SSL")
    def test_search_emails_success(self, mock_imap_cls, mock_read_cfg):
        mock_read_cfg.return_value = self.dummy_config
        mock_imap = MagicMock()
        mock_imap_cls.return_value = mock_imap

        mock_imap.select.return_value = ("OK", [b"1"])
        mock_imap.search.return_value = ("OK", [b"202"])

        msg = EmailMessage()
        msg["From"] = "finance@company.com"
        msg["Subject"] = "Invoice #9821"
        msg["Date"] = "Mon, 28 Sep 2026 11:00:00 +0000"
        msg.set_content("Attached invoice for Q3.")
        raw_email = msg.as_bytes()

        mock_imap.fetch.return_value = ("OK", [(b"202 (RFC822 {100})", raw_email)])

        res = email_assistant({
            "action": "search",
            "query": "Invoice",
        })
        self.assertIn("Invoice #9821", res)
        self.assertIn("finance@company.com", res)

    @patch("actions.email_assistant._read_config_file")
    @patch("smtplib.SMTP")
    def test_send_with_attachment(self, mock_smtp_cls, mock_read_cfg):
        mock_read_cfg.return_value = self.dummy_config
        mock_server = MagicMock()
        mock_smtp_cls.return_value.__enter__.return_value = mock_server

        temp_file = root / "temp_test_attachment.txt"
        try:
            temp_file.write_text("Attachment content here", encoding="utf-8")
            res = email_assistant({
                "action": "send",
                "recipient": "colleague@example.com",
                "subject": "With Attachment",
                "body": "See attached file.",
                "attachments": [str(temp_file)],
            })
            self.assertIn("Email sent successfully", res)
            self.assertIn("1 attachment(s)", res)
            self.assertIn("temp_test_attachment.txt", res)
            mock_server.send_message.assert_called_once()
        finally:
            temp_file.unlink(missing_ok=True)

    @patch("actions.email_assistant._read_config_file")
    @patch("smtplib.SMTP_SSL")
    def test_send_ssl_port_465(self, mock_smtp_ssl_cls, mock_read_cfg):
        cfg = dict(self.dummy_config)
        cfg["smtp_port"] = 465
        mock_read_cfg.return_value = cfg
        mock_server = MagicMock()
        mock_smtp_ssl_cls.return_value.__enter__.return_value = mock_server

        res = email_assistant({
            "action": "send",
            "recipient": "partner@example.com",
            "subject": "Secure Email",
            "body": "Testing port 465.",
        })
        self.assertIn("Email sent successfully", res)
        mock_server.login.assert_called_once_with("testuser@gmail.com", "abcdefghijklmnop")

    @patch("actions.email_assistant._read_config_file")
    def test_load_email_config_normalization(self, mock_read_cfg):
        mock_read_cfg.return_value = {
            "email": "  user@domain.com  ",
            "app_password": " 1234  5678 9012 3456 ",
        }
        cfg = load_email_config()
        self.assertEqual(cfg["email"], "user@domain.com")
        self.assertEqual(cfg["app_password"], "1234567890123456")
        self.assertEqual(cfg["smtp_server"], "smtp.gmail.com")
        self.assertEqual(cfg["smtp_port"], 587)
        self.assertTrue(cfg["is_configured"])


if __name__ == "__main__":
    unittest.main()
