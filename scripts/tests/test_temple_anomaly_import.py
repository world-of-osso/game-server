"""Qualify the inferred Static Anomaly flag during template regeneration."""
import json
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import retail_content


class TempleAnomalyImportTests(unittest.TestCase):
    def test_import_only_changes_the_reviewed_source_flags(self):
        fixture = json.loads((Path(__file__).parent / 'fixtures/log-population-tdb.json').read_text())
        with sqlite3.connect(':memory:') as db:
            for table in ('content_creature_template', 'tdb_creature_template',
                          'tdb_creature_template_difficulty', 'content_tuning_level'):
                db.execute(fixture[table]['sql'])
            db.execute('CREATE TABLE creature_template (entry INTEGER PRIMARY KEY)')
            db.executemany(
                'INSERT INTO tdb_creature_template (entry,name,unit_flags) VALUES (?,?,?)',
                [(134691, 'Static Anomaly', 33554960), (134692, 'Control', 33554960)],
            )
            db.executemany(
                'INSERT INTO tdb_creature_template_difficulty (Entry,DifficultyID) VALUES (?,0)',
                [(134691,), (134692,)],
            )
            db.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty')
            db.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
            retail_content.insert_tdb_templates(db, 'SELECT entry FROM tdb_creature_template')
            self.assertEqual(db.execute(
                'SELECT entry,unit_flags FROM content_creature_template ORDER BY entry'
            ).fetchall(), [(134691, 528), (134692, 33554960)])
            db.execute('DELETE FROM content_creature_template WHERE entry=134691')
            db.execute('UPDATE tdb_creature_template SET unit_flags=33554961 WHERE entry=134691')
            retail_content.insert_tdb_templates(db, 'SELECT entry FROM tdb_creature_template')
            self.assertEqual(db.execute(
                'SELECT unit_flags FROM content_creature_template WHERE entry=134691'
            ).fetchone(), (33554961,))


if __name__ == '__main__':
    unittest.main()
