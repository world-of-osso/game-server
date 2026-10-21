"""Actual pinned TDB and70448 conversation rows, not invented dialogue/actors."""
import json
from pathlib import Path
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import conversation_data


class ConversationDataTests(unittest.TestCase):
    def test_reads_native_orator_templates_actors_and_lines(self):
        self.assertTrue(conversation_data.DEFAULT_SOURCE.is_file(), 'local pinned TDB prerequisite')
        csv_dir = conversation_data.DEFAULT_CSV
        receipt = json.loads((csv_dir / 'receipt.json').read_text())
        self.assertEqual(receipt['identity']['build'], '12.1.5.70448')
        self.assertTrue((csv_dir / 'ConversationLine.csv').is_file())
        tables = conversation_data.read_source_tables()
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


if __name__ == '__main__':
    unittest.main()
