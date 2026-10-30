import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from generate_class_follower_profiles import generate


class ClassFollowerProfilesTest(unittest.TestCase):
    def test_simc_takes_priority_and_parity_fills_missing_spec(self):
        db = sqlite3.connect(":memory:")
        db.executescript("""
            CREATE TABLE chr_classes(ID INTEGER, Filename TEXT);
            CREATE TABLE chr_specialization(ID INTEGER, Name_lang TEXT, ClassID INTEGER, Role INTEGER);
            CREATE TABLE chr_races(ID INTEGER, Name_lang TEXT);
            INSERT INTO chr_classes VALUES(2, 'PALADIN');
            INSERT INTO chr_specialization VALUES(66, 'Protection', 2, 0), (65, 'Holy', 2, 1);
            INSERT INTO chr_races VALUES(1, 'Human'), (3, 'Dwarf');
        """)
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            (root / 'profiles/MID2').mkdir(parents=True)
            (root / 'profiles/MID2/MID2_Paladin_Protection.simc').write_text(
                'paladin="Test"\nspec=protection\nrace=dwarf\ntalents=abc\n'
            )
            parity = root / 'parity'
            parity.mkdir()
            (parity / '66.profile.json').write_text(json.dumps({
                'spec_id': 66, 'class': 'paladin', 'spec': 'protection', 'race': 'Human', 'talents': 'other'
            }))
            (parity / '65.profile.json').write_text(json.dumps({
                'spec_id': 65, 'class': 'paladin', 'spec': 'holy', 'race': 'Human', 'talents': 'healer'
            }))
            rows = generate(db, root, parity)
        self.assertEqual([(row['spec_id'], row['race'], row['role']) for row in rows], [(65, 1, 1), (66, 3, 0)])
        self.assertEqual([row['race_source'] for row in rows], ['RACE_FROM_PARITY_PROFILE', 'RACE_FROM_SIMC_PROFILE'])
        self.assertEqual(rows[1]['talents'], 'abc')


if __name__ == '__main__':
    unittest.main()
