"""Source-qualified Wild Imp closure on a COPY, never fabricated template/model rows."""
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest


class WildImpTemplatePolicyTests(unittest.TestCase):
    def test_prepares_authentic_wild_imp_only_and_preserves_models_on_second_run(self):
        root = Path(__file__).resolve().parents[1]
        source_path = root / 'data/world.db'
        with tempfile.TemporaryDirectory(prefix='wild-imp-policy-') as directory:
            prepared_path = Path(directory) / 'prepared.sqlite'
            with sqlite3.connect(f'file:{source_path}?mode=ro', uri=True) as source:
                with sqlite3.connect(prepared_path) as prepared:
                    source.backup(prepared)
            command = [sys.executable, str(root / 'scripts/add_guardian_templates.py'), '--db', str(prepared_path)]
            subprocess.run(command, check=True, capture_output=True, text=True)
            with sqlite3.connect(prepared_path) as prepared:
                actual = prepared.execute('SELECT name FROM content_creature_template WHERE entry=55659').fetchall()
                self.assertEqual(actual, [('Wild Imp',)], 'source-qualified Wild Imp must not remain an unprepared template')
                native_models = prepared.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=55659 ORDER BY Idx').fetchall()
                models = prepared.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=55659 ORDER BY Idx').fetchall()
                self.assertGreater(len(native_models), 0, 'actual archived model rows are the precondition')
                self.assertEqual(models, native_models, 'all source models/builds/scales preserved, no Imp416 substitution')
                self.assertEqual(prepared.execute('SELECT COUNT(*) FROM content_creature_template WHERE entry=98035').fetchone()[0], 0,
                                 'unqualified Dreadstalker must not be published by the Wild Imp slice')
            subprocess.run(command, check=True, capture_output=True, text=True)
            with sqlite3.connect(prepared_path) as prepared:
                repeated = prepared.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=55659 ORDER BY Idx').fetchall()
                self.assertEqual(repeated, models, 'rerunning closure must not duplicate models')


if __name__ == '__main__':
    unittest.main()
