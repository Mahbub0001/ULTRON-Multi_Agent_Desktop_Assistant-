import unittest

from core.uia import Control, WindowInfo, available, best_match, score


class TestMatching(unittest.TestCase):
    def test_exact_match_is_perfect(self):
        self.assertEqual(score("Sign In", "Sign In"), 1.0)
        self.assertEqual(score("  Sign In  ", "sign in"), 1.0)

    def test_substring_scores_high(self):
        self.assertGreater(score("Sign In Button", "Sign In"), 0.7)

    def test_token_overlap_scores_mid(self):
        s = score("Submit Form Button", "Form Submit")
        self.assertGreater(s, 0.3)
        self.assertLess(s, 0.7)

    def test_unrelated_is_zero(self):
        self.assertEqual(score("Volume Slider", "Delete File"), 0.0)

    def test_empty_inputs(self):
        self.assertEqual(score("", "x"), 0.0)
        self.assertEqual(score("x", ""), 0.0)

    def test_best_match_prefers_exact(self):
        got = best_match(["Button Sign In", "Sign In", "Sign Out"], "sign in")
        self.assertEqual(got, "Sign In")

    def test_best_match_fuzzy(self):
        got = best_match(["New Slide", "Slide Layout", "Delete"], "new slides")
        self.assertEqual(got, "New Slide")

    def test_best_match_none_when_too_far(self):
        self.assertIsNone(best_match(["Volume", "Brightness"], "Delete File"))

    def test_best_match_min_score_gate(self):
        self.assertIsNone(best_match(["abc"], "xyz"))

    def test_best_match_tie_prefers_shorter(self):
        got = best_match(["Play Game Now", "Play"], "play")
        self.assertEqual(got, "Play")


class TestRecords(unittest.TestCase):
    def test_control_center(self):
        c = Control(name="OK", rect=(10, 20, 30, 60))
        self.assertEqual(c.center, (20, 40))
        d = c.as_dict()
        self.assertEqual(d["name"], "OK")
        self.assertEqual(d["center"], [20, 40])

    def test_window_info_dict(self):
        w = WindowInfo(title="Notepad", rect=(0, 0, 100, 50))
        self.assertEqual(w.as_dict()["title"], "Notepad")

    def test_available_is_bool(self):
        self.assertIsInstance(available(), bool)


if __name__ == "__main__":
    unittest.main()
