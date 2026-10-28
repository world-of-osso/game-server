import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class TopLogSimcProfileTest(unittest.TestCase):
    def test_guardian_items_include_ids_and_options(self):
        source = ROOT / 'scripts/reference/healer-tank-top-logs/104.profile.json'
        profile = json.loads(source.read_text())
        output = subprocess.check_output(
            [sys.executable, str(ROOT / 'scripts/top_log_simc_profile.py'), str(source)],
            text=True,
        )
        lines = dict(line.split('=', 1) for line in output.splitlines())
        self.assertEqual(lines['head'], ',id=271528,ilevel=334,bonus_id=13334/6652/13696/13847/13692/13698/12854,enchant_id=7961')
        self.assertIn('gem_id=240890', lines['wrists'])
        self.assertIn('crafted_stats=32/36', lines['wrists'])
        self.assertEqual(sum(item['slot'] in lines for item in profile['equipment']), 15)
        self.assertEqual(lines['talents'], profile['talents'])


if __name__ == '__main__':
    unittest.main()
