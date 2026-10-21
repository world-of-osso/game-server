"""Actual pinned TDB and70448 conversation rows, not invented dialogue/actors."""
import json
import os
from pathlib import Path
import sys
import sqlite3
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import conversation_data


class ConversationDataTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tables = conversation_data.read_source_tables()

    def test_reads_native_orator_templates_actors_and_lines(self):
        self.assertTrue(conversation_data.DEFAULT_SOURCE.is_file(), 'local pinned TDB prerequisite')
        csv_dir = conversation_data.DEFAULT_CSV
        receipt = json.loads((csv_dir / 'receipt.json').read_text())
        self.assertEqual(receipt['identity']['build'], '12.1.5.70448')
        self.assertTrue((csv_dir / 'ConversationLine.csv').is_file())
        tables = self.tables
        templates = tables.get('tdb_conversation_template', ((), ()))[1]
        native = [row for row in templates if row[0] in (24642, 24643, 24644)]
        self.assertEqual(native, [(24642, 67085, 0, 0, '', 57388),
                                  (24643, 67091, 0, 0, '', 57388),
                                  (24644, 67092, 0, 0, '', 57388)])
        actors = tables['tdb_conversation_actors'][1]
        self.assertEqual([row for row in actors if row[0] == 24642],
                         [(24642, 51642, 0, 0, 220769, 116692, 0, 0, 57388)])
        lines = tables['conversation_line'][1]
        self.assertEqual([row[1] for row in lines if row[0] in (67085, 67091, 67092)],
                         [259911, 259917, 259918])

    def test_import_copies_native_rows_without_replacing_other_tables(self):
        with sqlite3.connect(':memory:') as connection:
            connection.execute('CREATE TABLE unrelated (value TEXT)')
            connection.execute("INSERT INTO unrelated VALUES ('keep')")
            conversation_data.import_tables(connection, self.tables)
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchall(), [('keep',)])
            self.assertEqual(connection.execute('SELECT Id,FirstLineId FROM tdb_conversation_template WHERE Id IN (24642,24643,24644) ORDER BY Id').fetchall(),
                             [(24642, 67085), (24643, 67091), (24644, 67092)])
            self.assertEqual(connection.execute('SELECT CreatureId,CreatureDisplayInfoId FROM tdb_conversation_actors WHERE ConversationId=24642').fetchall(),
                             [(220769, 116692)])
            self.assertEqual(connection.execute('SELECT BroadcastTextID,NextConversationLineID FROM conversation_line WHERE ID=67085').fetchall(),
                             [(259911, 0)])
            for table, (_, rows) in self.tables.items():
                self.assertEqual(connection.execute(f'SELECT count(*) FROM "{table}"').fetchone()[0], len(rows))
            self.assertEqual(connection.execute('SELECT table_name,build FROM db2_build ORDER BY table_name').fetchall(),
                             [('BroadcastTextDuration', conversation_data.BUILD), ('ConversationLine', conversation_data.BUILD)])

    def test_publication_sql_replays_exact_source_rows_and_preserves_other_state(self):
        with tempfile.TemporaryDirectory() as directory, sqlite3.connect(':memory:') as connection:
            connection.execute('CREATE TABLE unrelated (value TEXT)')
            connection.execute("INSERT INTO unrelated VALUES ('keep')")
            connection.execute('CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT NOT NULL)')
            connection.execute('INSERT INTO db2_build VALUES (?,?)', ('SpellEffect', conversation_data.BUILD))
            connection.commit()
            path = Path(directory) / 'conversations.sql'
            conversation_data.write_sql(path, self.tables)
            connection.executescript(path.read_text())
            connection.executescript(path.read_text())  # Publication is safely repeatable.
            # The SQL database schema/data is the external publication contract.
            actual = {row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")}
            self.assertEqual(actual, set(self.tables) | {'db2_build', 'unrelated'})
            self.assertEqual(connection.execute('SELECT build FROM db2_build WHERE table_name=?', ('SpellEffect',)).fetchone(),
                             (conversation_data.BUILD,))
            for table, (_, rows) in self.tables.items():
                imported = connection.execute(f'SELECT * FROM "{table}"').fetchall()
                self.assertEqual(sorted(imported), sorted(rows))
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchall(), [('keep',)])

    def test_cli_refuses_canonical_publication_before_reading_source(self):
        # Copy the real entrypoint into an owned mock checkout; never risk the actual
        # canonical database while testing a publication refusal.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / 'scripts/conversation_data.py'
            script.parent.mkdir()
            script.write_bytes((SCRIPTS / 'conversation_data.py').read_bytes())
            database = root / 'data/world.db'
            database.parent.mkdir()
            with sqlite3.connect(database) as connection:
                connection.execute('CREATE TABLE sentinel (value TEXT)')
                connection.execute("INSERT INTO sentinel VALUES ('keep')")
            environment = dict(os.environ, PYTHONPATH=str(SCRIPTS))
            result = subprocess.run([sys.executable, str(script), '--db', str(database)],
                                    capture_output=True, text=True, env=environment, check=False)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('canonical', result.stderr)
            self.assertIn('lead-owned', result.stderr)
            with sqlite3.connect(database) as connection:
                self.assertEqual(connection.execute('SELECT value FROM sentinel').fetchall(), [('keep',)])


if __name__ == '__main__':
    unittest.main()
