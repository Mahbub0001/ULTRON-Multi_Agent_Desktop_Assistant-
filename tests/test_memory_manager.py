import sys
import unittest
from pathlib import Path
from unittest.mock import patch

root = Path(__file__).resolve().parent.parent
if str(root) not in sys.path:
    sys.path.insert(0, str(root))

from memory.memory_manager import search_memory


class TestMemoryManager(unittest.TestCase):
    def setUp(self):
        self.sample_memory = {
            "identity": {
                "name": {"value": "Nibir sir"},
            },
            "relationships": {
                "mother_name": {"value": "Sabikun Nahar Khuku"},
                "father_name": {"value": "Jakirul Alam Bhuiya"},
            },
            "notes": {
                "father_job": {"value": "Worker at Papri NGO"},
                "mother_job": {"value": "Youth Development Officer"},
            },
        }

    @patch("memory.memory_manager.load_memory")
    def test_search_memory_bengali_and_banglish_synonyms(self, mock_load):
        mock_load.return_value = self.sample_memory

        # Test ammu / mother
        res_ammu = search_memory("ammu")
        self.assertIn("Sabikun Nahar Khuku", res_ammu)

        # Test baba / father
        res_baba = search_memory("baba")
        self.assertIn("Jakirul Alam Bhuiya", res_baba)

        # Test babar pesha / father job
        res_pesha = search_memory("babar pesha")
        self.assertIn("Worker at Papri NGO", res_pesha)

        # Test Bengali script
        res_bn_ammu = search_memory("আম্মুর নাম")
        self.assertIn("Sabikun Nahar Khuku", res_bn_ammu)

        res_bn_baba = search_memory("বাবার পেশা")
        self.assertIn("Worker at Papri NGO", res_bn_baba)

        # Test Hindi
        res_hi_ammi = search_memory("अम्मी का नाम")
        self.assertIn("Sabikun Nahar Khuku", res_hi_ammi)


if __name__ == "__main__":
    unittest.main()
