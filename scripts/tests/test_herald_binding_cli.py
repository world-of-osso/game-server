"""Real CLI binding contract: pinned TC inputs and active70448 row snapshots."""
import csv
import hashlib
import json
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import encounter_cpp_bindings

FIXTURE = Path(__file__).parent / 'fixtures/herald-binding'
POLICY = 'ENCOUNTER_CREDIT_FROM_RETAIL_DUNGEONENCOUNTER'


class HeraldBindingCliTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.db = self.root / 'copy.sqlite'
        self.sql = self.root / 'binding.sql'
        self.report = self.root / 'report.json'
        self.tc = self.root / 'tc'
        shutil.copytree(FIXTURE / 'tc', self.tc)
        self.snapshot = json.loads((FIXTURE / 'retail70448.json').read_text())
        self.write_retail_snapshot()
        self.create_database()

    def write_retail_snapshot(self):
        for name, rows in self.snapshot['rows'].items():
            with (self.root / name).open('w', newline='') as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
                writer.writeheader()
                writer.writerows(rows)

    def create_database(self):
        with sqlite3.connect(self.db) as connection:
            connection.executescript('''
                CREATE TABLE dungeon_encounter(ID INTEGER,MapID INTEGER,Bit INTEGER);
                CREATE TABLE tdb_creature_template(entry INTEGER,ScriptName TEXT);
                CREATE TABLE raid_encounter_creature(map_id INTEGER,creature_entry INTEGER,
                    encounter_id INTEGER,bit INTEGER,journal_encounter_id INTEGER,
                    journal_creature_id INTEGER,display_id INTEGER,PRIMARY KEY(map_id,creature_entry));
                CREATE TABLE content_creature(guid INTEGER PRIMARY KEY,id1 INTEGER,map INTEGER);
                INSERT INTO dungeon_encounter VALUES(1891,543,0),(1892,543,1),(1893,543,2);
                INSERT INTO tdb_creature_template VALUES(17307,'boss_vazruden_the_herald'),
                    (17536,''),(17537,'boss_vazruden'),(17517,''),(18435,'');
                INSERT INTO raid_encounter_creature VALUES
                    (543,17308,1891,0,528,1105,18237),(543,17306,1893,2,527,1104,18236);
                INSERT INTO content_creature VALUES
                    (20202693,17307,543),(20202705,17517,543),(20202706,17517,543);
            ''')

    def run_binding(self, build='12.1.5.70448'):
        return subprocess.run(
            [sys.executable, encounter_cpp_bindings.__file__, '--encounter', 'herald',
             '--db', str(self.db), '--tc-dir', str(self.tc),
             '--journal', str(self.root / 'JournalEncounter.csv'),
             '--creatures', str(self.root / 'JournalEncounterCreature.csv'),
             '--dungeon-encounter', str(self.root / 'DungeonEncounter.csv'),
             '--journal-build', build, '--output', str(self.sql), '--report', str(self.report)],
            capture_output=True, text=True, check=False,
        )

    def test_70448_cli_exports_primary_only_and_preserves_existing_credit_and_population(self):
        with sqlite3.connect(self.db) as connection:
            population = connection.execute('SELECT * FROM content_creature ORDER BY guid').fetchall()
        for _ in range(2):
            before = self.db.read_bytes()
            result = self.run_binding()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(self.db.read_bytes(), before, 'Generator never mutates its input database')
            with sqlite3.connect(self.db) as connection:
                connection.executescript(self.sql.read_text())
        with sqlite3.connect(self.db) as connection:
            self.assertEqual(connection.execute(
                'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
                'FROM raid_encounter_creature WHERE map_id=543 ORDER BY bit').fetchall(),
                [(17308,1891,0,528,1105,18237),(17307,1892,1,529,1106,18944),
                 (17306,1893,2,527,1104,18236)])
            self.assertEqual(connection.execute('SELECT * FROM content_creature ORDER BY guid').fetchall(),
                             population, 'Binding cannot manufacture or suppress source actors')
        report = json.loads(self.report.read_text())
        self.assertEqual(report['build'], '12.1.5.70448')
        self.assertEqual(report['credit_identity_policy'], POLICY)
        expected_hashes = {str(self.root / name): hashlib.sha256((self.root / name).read_bytes()).hexdigest()
                           for name in ['DungeonEncounter.csv', 'JournalEncounter.csv',
                                        'JournalEncounterCreature.csv']}
        self.assertEqual(report['retail_source_sha256'], expected_hashes)

    def write_row_override(self, name, field, value):
        rows = [dict(row) for row in self.snapshot['rows'][name]]
        rows[0][field] = value
        with (self.root / name).open('w', newline='') as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)

    def test_missing_or_disagreeing_retail_identity_has_named_gap_and_no_outputs_changed(self):
        cases = [('old-build', None, None, None),
                 ('missing', 'DungeonEncounter.csv', None, None),
                 ('wrong-bit', 'DungeonEncounter.csv', 'Bit', '2'),
                 ('wrong-journal', 'JournalEncounter.csv', 'DungeonEncounterID', '1893'),
                 ('wrong-actor', 'JournalEncounterCreature.csv', 'CreatureDisplayInfoID', '18812')]
        for name, file, field, value in cases:
            with self.subTest(case=name):
                self.write_retail_snapshot()
                if file and field:
                    self.write_row_override(file, field, value)
                elif file:
                    (self.root / file).unlink()
                self.sql.write_text('retained SQL')
                self.report.write_text('retained report')
                before = self.db.read_bytes()
                result = self.run_binding('12.1.5.70391' if name == 'old-build' else '12.1.5.70448')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('HERALD_RETAIL_IDENTITY_GAP', result.stderr)
                self.assertEqual(self.db.read_bytes(), before)
                self.assertEqual(self.sql.read_text(), 'retained SQL')
                self.assertEqual(self.report.read_text(), 'retained report')

    def test_changed_pinned_source_is_refused_before_database_or_output_changes(self):
        header = self.tc / 'Outland/HellfireCitadel/HellfireRamparts/hellfire_ramparts.h'
        header.write_bytes(header.read_bytes() + b'\n')
        before = self.db.read_bytes()
        result = self.run_binding()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('pinned TC source hash changed', result.stderr)
        self.assertEqual(self.db.read_bytes(), before)
        self.assertFalse(self.sql.exists())
        self.assertFalse(self.report.exists())


if __name__ == '__main__':
    unittest.main()
