"""Placeholder HealthScalingExpansion 0 takes its ContentTuning ExpansionID on follower maps."""
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'health_scaling_expansion.py'
# Native 70448 rows: Darkflame trash 2718 (10), generic 2702 (-3), 2940 (9), Classic 217 (0).
CONTENT_TUNING = 'ID,Flags,ExpansionID\n2718,0,10\n2702,0,-3\n2940,0,9\n217,0,0\n'


def build_copy(path):
    with closing(sqlite3.connect(path)) as db, db:
        db.executescript("""
            CREATE TABLE follower_lfg_dungeons (id INTEGER, map_id INTEGER, difficulty_id INTEGER);
            INSERT INTO follower_lfg_dungeons VALUES (2655, 2651, 205), (1, 33, 1);
            CREATE TABLE content_creature (guid INTEGER, id1 INTEGER, map INTEGER);
            INSERT INTO content_creature VALUES (1, 210812, 2651), (2, 208743, 2651), (3, 900001, 2651),
                (4, 900002, 2651), (5, 900003, 2651), (6, 210813, 33);
            CREATE TABLE content_creature_template (entry INTEGER PRIMARY KEY, exp INTEGER,
                ContentTuningID INTEGER, HealthScalingExpansion INTEGER);
            -- Royal Wicklighter placeholder, Blazikon sniffed 10, generic -3, contradicted 2940,
            -- and a non-follower map entry with the same placeholder.
            INSERT INTO content_creature_template VALUES (210812, 0, 2718, 0), (208743, 2, 2718, 10),
                (900001, 0, 2702, 0), (900002, 0, 2940, 0), (900003, 0, 217, 0), (210813, 0, 2718, 0);
            CREATE TABLE content_creature_template_difficulty (entry INTEGER, DifficultyID INTEGER,
                exp INTEGER, ContentTuningID INTEGER, HealthScalingExpansion INTEGER);
            INSERT INTO content_creature_template_difficulty VALUES (210812, 1, 0, 2718, 0),
                (210812, 2, 0, 2718, 0), (900002, 1, 0, 2940, 0);
            CREATE TABLE tdb_creature_template_difficulty (Entry INTEGER, DifficultyID INTEGER,
                ContentTuningID INTEGER, HealthScalingExpansion INTEGER);
            INSERT INTO tdb_creature_template_difficulty VALUES (208743, 1, 2718, 10), (208743, 2, 0, 10),
                (210812, 0, 2718, 0), (900010, 1, 2940, 10);
        """)


def run(path, tuning):
    return subprocess.run([sys.executable, '-I', str(SCRIPT), '--db', str(path), '--content-tuning', str(tuning)],
                          check=True, capture_output=True, text=True).stdout


class HealthScalingExpansion(unittest.TestCase):
    def test_follower_placeholders_take_uncontradicted_content_tuning_expansion(self):
        with tempfile.TemporaryDirectory() as directory:
            path, tuning = Path(directory) / 'copy.sqlite', Path(directory) / 'ContentTuning.csv'
            tuning.write_text(CONTENT_TUNING)
            build_copy(path)
            report = run(path, tuning)
            with closing(sqlite3.connect(path)) as db:
                templates = db.execute('SELECT entry, exp, HealthScalingExpansion FROM content_creature_template '
                                       'ORDER BY entry').fetchall()
                difficulties = db.execute('SELECT entry, DifficultyID, exp, HealthScalingExpansion '
                                          'FROM content_creature_template_difficulty ORDER BY 1, 2').fetchall()
                source = db.execute('SELECT table_name, entry, DifficultyID, ContentTuningID, old_value, new_value, '
                                    'policy FROM health_scaling_expansion_source ORDER BY 1, 2, 3').fetchall()
        self.assertEqual(templates, [(208743, 2, 10), (210812, 2, 10), (210813, 0, 0), (900001, 0, 0),
                                     (900002, 0, 0), (900003, 0, 0)])
        self.assertEqual(difficulties, [(210812, 1, 2, 10), (210812, 2, 2, 10), (900002, 1, 0, 0)])
        policy = 'HEALTH_SCALING_EXPANSION_FROM_CONTENT_TUNING'
        self.assertEqual(source, [
            ('content_creature_template', 210812, 0, 2718, 0, 10, policy),
            ('content_creature_template_difficulty', 210812, 1, 2718, 0, 10, policy),
            ('content_creature_template_difficulty', 210812, 2, 2718, 0, 10, policy),
        ])
        self.assertIn('SKIPPED ContentTuning 2702: ExpansionID -3 names no expansion: 1 rows', report)
        self.assertIn('SKIPPED ContentTuning 2940: sniffed {10: 1} contradicts ExpansionID 9: 2 rows', report)

    def test_refuses_canonical_world_db_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'world.db'
            path.touch()
            result = subprocess.run([sys.executable, '-I', str(SCRIPT), '--db', str(path),
                                     '--content-tuning', str(path)], capture_output=True)
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
