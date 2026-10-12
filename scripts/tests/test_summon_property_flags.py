"""Real readable CASC69933 records; three encrypted records are not invented."""
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2_casc
import import_summon_property_flags as flags


class SummonPropertyFlagsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.payload = Path(os.environ['TOY_SUMMON_PROPERTIES_DB2']).read_bytes()
        _, cls.source = db2_casc.decode(cls.payload)

    def database(self):
        conn = sqlite3.connect(':memory:')
        conn.execute('CREATE TABLE summon_properties (ID INTEGER PRIMARY KEY, Control INTEGER, Faction INTEGER, Title INTEGER, Slot INTEGER)')
        for record_id in [64, 3077, 3135]:
            conn.execute('INSERT INTO summon_properties VALUES (?,?,?,?,?)', self.source[record_id][:5])
        conn.execute('CREATE TABLE unrelated (ID INTEGER PRIMARY KEY, Name TEXT)')
        conn.execute("INSERT INTO unrelated VALUES (42,'preserved fixture')")
        conn.commit()
        return conn

    def test_imports_zero_private_and_creature_level_flags_without_changing_identity(self):
        with self.database() as conn:
            flags.import_flags(conn, self.payload)
            for record_id in [64, 3077, 3135]:
                row = conn.execute('SELECT * FROM summon_properties WHERE ID=?', (record_id,)).fetchone()
                self.assertEqual(row, tuple(self.source[record_id]))
            self.assertEqual(conn.execute('SELECT * FROM unrelated').fetchall(), [(42, 'preserved fixture')])
            self.assertEqual(len(self.source), 3610)
            self.assertEqual(self.source[3077][5:], [16, 0])
            self.assertEqual(self.source[3135][5:], [133384, 0])

    def test_rejects_layout_drift_before_schema_or_row_changes(self):
        payload = bytearray(self.payload)
        payload[156:160] = (0).to_bytes(4, 'little')
        with self.database() as conn:
            before = conn.execute('SELECT * FROM summon_properties').fetchall()
            with self.assertRaisesRegex(ValueError, 'layout'):
                flags.import_flags(conn, payload)
            self.assertEqual(conn.execute('SELECT * FROM summon_properties').fetchall(), before)
            self.assertEqual(len(conn.execute('PRAGMA table_info(summon_properties)').fetchall()), 5)


if __name__ == '__main__':
    unittest.main()
