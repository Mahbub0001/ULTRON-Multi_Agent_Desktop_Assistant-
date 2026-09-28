import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from actions.browser_control import _normalize_url

class TestBrowserControl(unittest.TestCase):
    def test_normalize_url_web_app_shortcuts(self):
        # LinkedIn
        self.assertEqual(_normalize_url("linkedin notifications"), "https://www.linkedin.com/notifications")
        self.assertEqual(_normalize_url("linkedin/notifications"), "https://www.linkedin.com/notifications")
        self.assertEqual(_normalize_url("linkedin messaging"), "https://www.linkedin.com/messaging")
        self.assertEqual(_normalize_url("linkedin inbox"), "https://www.linkedin.com/messaging")
        self.assertEqual(_normalize_url("linkedin network"), "https://www.linkedin.com/mynetwork")
        self.assertEqual(_normalize_url("linkedin jobs"), "https://www.linkedin.com/jobs")
        self.assertEqual(_normalize_url("linkedin feed"), "https://www.linkedin.com/feed")
        self.assertEqual(_normalize_url("linkedin"), "https://www.linkedin.com")

        # GitHub
        self.assertEqual(_normalize_url("github notifications"), "https://github.com/notifications")
        self.assertEqual(_normalize_url("github pr"), "https://github.com/pulls")
        self.assertEqual(_normalize_url("github issues"), "https://github.com/issues")
        self.assertEqual(_normalize_url("github profile"), "https://github.com/Mahbub0001")

        # YouTube
        self.assertEqual(_normalize_url("youtube subscriptions"), "https://www.youtube.com/feed/subscriptions")
        self.assertEqual(_normalize_url("youtube history"), "https://www.youtube.com/feed/history")

        # Standard URLs
        self.assertEqual(_normalize_url("google.com"), "https://google.com")
        self.assertEqual(_normalize_url("https://example.com/test"), "https://example.com/test")

if __name__ == "__main__":
    unittest.main()
