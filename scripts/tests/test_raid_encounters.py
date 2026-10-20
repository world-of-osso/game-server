import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from raid_encounters import derive_bindings, prepare


def journal_row(journal_id, dungeon_encounter_id):
    """JournalEncounter layout: [4]=ID, [6]=DungeonEncounterID."""
    return [0, 0, 0, 0, journal_id, 0, dungeon_encounter_id] + [0] * 6


def actor_row(actor_id, journal_id, display_id):
    """JournalEncounterCreature layout: [2]=ID, [3]/[8]=JournalEncounterID, [4]=display."""
    return [0, 0, actor_id, journal_id, display_id, 0, 0, 0, journal_id]


def blood_furnace_world():
    conn = sqlite3.connect(':memory:')
    conn.executescript("""
        CREATE TABLE map (ID INTEGER, InstanceType INTEGER);
        CREATE TABLE dungeon_encounter (ID INTEGER, MapID INTEGER, Bit INTEGER, Name_lang TEXT);
        CREATE TABLE tdb_creature (id INTEGER, map INTEGER);
        CREATE TABLE tdb_creature_template_model (CreatureID INTEGER, CreatureDisplayID INTEGER);
        INSERT INTO map VALUES (542, 1), (1, 0);
        INSERT INTO dungeon_encounter VALUES (1922, 542, 0, 'The Maker');
        INSERT INTO tdb_creature VALUES (17381, 542);
        INSERT INTO tdb_creature_template_model VALUES (17381, 18369);
    """)
    return conn


class DeriveBindingsTest(unittest.TestCase):
    def test_unjoinable_journal_encounter_is_a_named_gap(self):
        with closing(blood_furnace_world()) as conn:
            bindings, gaps = derive_bindings(
                conn,
                [journal_row(555, 1922), journal_row(1756, 2459)],
                [actor_row(1132, 555, 18369), actor_row(3540, 1756, 89000)],
            )
        self.assertEqual(bindings, [(542, 17381, 1922, 0, 555, 1132, 18369)])
        self.assertEqual(gaps, [dict(map_id=None, encounter_id=2459, journal_encounter_id=1756,
                                     reason='no dungeon or raid DungeonEncounter')])


@unittest.skipUnless(os.environ.get('BINDINGS_JOURNAL_DIR'), 'pinned journal binaries')
class PrepareCliTest(unittest.TestCase):
    def test_report_records_declared_journal_build_and_binds_blood_furnace(self):
        journal = Path(os.environ['BINDINGS_JOURNAL_DIR'])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'bindings.sql'
            report = Path(directory) / 'report.json'
            result = subprocess.run(
                [sys.executable, str(Path(__file__).resolve().parents[1] / 'raid_encounters.py'),
                 '--db', os.environ['RAID_WORLD_DB'], '--journal', str(journal / '1240336.db2'),
                 '--creatures', str(journal / '1301155.db2'), '--journal-build', '12.1.5.70448',
                 '--output', str(output), '--report', str(report)],
                capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(report.read_text())
            self.assertEqual(payload['build'], '12.1.5.70448')
            self.assertIn([542, 17381, 1922, 0, 555, 1132, 18369], payload['bindings'])


class RaidEncountersTest(unittest.TestCase):
    def test_authentic_single_boss_join_and_ambiguous_displays_are_not_bound(self):
        root = Path(os.environ['RAID_EVIDENCE'])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'raid.sql'
            report = Path(directory) / 'report.json'
            bindings, gaps = prepare(Path(os.environ['RAID_WORLD_DB']),
                                     root / 'db2/1240336.db2', root / 'db2/1301155.db2',
                                     output, report, os.environ['RAID_JOURNAL_BUILD'])
            self.assertIn((409, 12118, 663, 0, 1519, 3733, 13031), bindings)
            dungeon_maps = {row[0] for row in bindings if row[0] == 43}
            self.assertEqual(dungeon_maps, {43}, 'authentic Wailing Caverns bindings absent')
            self.assertFalse(any(row[0] == 409 and row[2] in (665, 669, 671) for row in bindings))
            self.assertTrue(any(row['encounter_id'] == 665 for row in gaps))
            with closing(sqlite3.connect(':memory:')) as conn:
                conn.executescript(output.read_text())
                self.assertEqual(conn.execute('SELECT creature_entry,encounter_id,bit FROM raid_encounter_creature WHERE map_id=409 AND creature_entry=12118').fetchone(), (12118, 663, 0))
                conn.executescript(output.read_text())
                self.assertEqual(conn.execute('SELECT count(*) FROM raid_encounter_creature').fetchone()[0], len(bindings))


if __name__ == '__main__':
    unittest.main()
