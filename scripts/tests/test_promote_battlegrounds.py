"""Scoped promotion against an authentic, reflinked world database; no canonical writes."""
import hashlib
import os
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path
import subprocess

SCRIPTS = Path(__file__).resolve().parents[1]
SOURCE = os.environ.get("BATTLEGROUND_PROMOTION_SOURCE")


def digest_rows(conn, table, where="1"):
    digest = hashlib.sha256()
    columns = len(conn.execute(f'SELECT * FROM "{table}" LIMIT 0').description)
    order = ",".join(str(n) for n in range(1, columns + 1))
    for row in conn.execute(f'SELECT * FROM "{table}" WHERE {where} ORDER BY {order}'):
        digest.update(repr(row).encode())
        digest.update(b"\n")
    return digest.hexdigest()


def snapshot(conn):
    """All persistent tables; allowed changes are exact selected-map dependency keys."""
    exclusions = {
        "content_creature": "map=2106",
        "content_gameobject": "map=2106",
        "content_creature_addon": "guid IN (SELECT guid+20000000 FROM tdb_creature WHERE map=2106)",
        "content_creature_template": "entry IN (SELECT id FROM tdb_creature WHERE map=2106)",
        "content_creature_template_model": "CreatureID IN (SELECT id FROM tdb_creature WHERE map=2106)",
        "content_creature_template_addon": "entry IN (SELECT id FROM tdb_creature WHERE map=2106)",
        "content_creature_equip_template": "CreatureID IN (SELECT id FROM tdb_creature WHERE map=2106)",
        "content_gameobject_template": "entry IN (SELECT id FROM tdb_gameobject WHERE map=2106)",
        "content_gameobject_template_addon": "entry IN (SELECT id FROM tdb_gameobject WHERE map=2106)",
    }
    return {
        table: {"all": digest_rows(conn, table),
                "protected": digest_rows(conn, table, f"NOT ({exclusions.get(table, '0')})"),
                "schema": schema}
        for table, schema in conn.execute("SELECT name,sql FROM sqlite_master WHERE type='table' ORDER BY name").fetchall()
    }


@unittest.skipUnless(SOURCE, "set BATTLEGROUND_PROMOTION_SOURCE to authentic world.db")
class PromotionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.db = Path(self.directory.name) / "promotion.sqlite"
        subprocess.run(["cp", "--reflink=auto", SOURCE, str(self.db)], check=True)

    def invoke(self, *maps):
        return subprocess.run([sys.executable, str(SCRIPTS / "promote-battlegrounds.py"),
                               "--db", str(self.db), "--only-maps", *map(str, maps)],
                              text=True, capture_output=True)

    def test_warsong_changes_only_requested_spawns_and_dependencies(self):
        with sqlite3.connect(self.db) as conn:
            before = snapshot(conn)
        result = self.invoke(2106)
        self.assertEqual(result.returncode, 0, result.stderr)
        with sqlite3.connect(self.db) as conn:
            after = snapshot(conn)
            self.assertEqual({k: (v['protected'], v['schema']) for k, v in before.items()},
                             {k: (v['protected'], v['schema']) for k, v in after.items()})
            for table, expected in (("creature", 28), ("gameobject", 14)):
                self.assertEqual(conn.execute(f"SELECT count(*) FROM content_{table} WHERE map=2106").fetchone()[0], expected)
                source_id = "id1" if table == "creature" else "id"
                self.assertEqual(conn.execute(f"SELECT guid+20000000,id,position_x,position_y,position_z FROM tdb_{table} WHERE map=2106 ORDER BY guid").fetchall(),
                                 conn.execute(f"SELECT guid,{source_id},position_x,position_y,position_z FROM content_{table} WHERE map=2106 ORDER BY guid").fetchall())
        self.assertEqual(self.invoke(2106).returncode, 0)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(after, snapshot(conn), "repeat promotion must be row-idempotent")

    def test_mixed_invalid_map_rolls_back_without_changes(self):
        before = hashlib.sha256(self.db.read_bytes()).hexdigest()
        result = self.invoke(2106, 0)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("battleground", result.stderr)
        self.assertEqual(before, hashlib.sha256(self.db.read_bytes()).hexdigest())


if __name__ == "__main__":
    unittest.main()
