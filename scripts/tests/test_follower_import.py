"""Behavioral importer proof against the pinned, locally extracted Retail table."""

import os
import sqlite3
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import follower_import


class FollowerImportTests(unittest.TestCase):
    def test_pinned_ruby_entry_and_authored_spell_rows_survive_sql_application(self):
        source = Path(os.environ["FOLLOWER_LFG_DB2"])
        archive = Path(os.environ["FOLLOWER_WORLD_SQL"])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "followers.sql"
            follower_import.prepare_import(source, archive, output)
            db = sqlite3.connect(":memory:")
            self.addCleanup(db.close)
            db.execute("CREATE TABLE unrelated_state (value TEXT)")
            db.execute("INSERT INTO unrelated_state VALUES ('preserved')")
            db.executescript(output.read_text())
            self.assertEqual(
                db.execute(
                    "SELECT name,map_id,difficulty_id,tanks,healers,damage "
                    "FROM follower_lfg_dungeons WHERE id=2436"
                ).fetchone(),
                ("Ruby Life Pools", 2521, 205, 1, 1, 3),
            )
            self.assertEqual(
                db.execute("SELECT COUNT(*) FROM follower_lfg_dungeons").fetchone()[0],
                27,
            )
            self.assertEqual(
                db.execute(
                    "SELECT COUNT(*) FROM tdb_creature_template_spell"
                ).fetchone()[0],
                9586,
            )
            self.assertEqual(
                db.execute(
                    "SELECT COUNT(*) FROM tdb_creature_template_spell "
                    "WHERE CreatureID IN (209057,209072,209059)"
                ).fetchone()[0],
                0,
            )
            self.assertEqual(
                db.execute(
                    'SELECT "Index",Spell,VerifiedBuild FROM tdb_creature_template_spell '
                    'WHERE CreatureID=48 ORDER BY "Index"'
                ).fetchall(),
                [(0, 9080, 41031), (1, 7165, 41031)],
            )
            db.executescript(output.read_text())
            self.assertEqual(
                db.execute("SELECT COUNT(*) FROM follower_lfg_dungeons").fetchone()[0],
                27,
            )
            self.assertEqual(
                db.execute("SELECT value FROM unrelated_state").fetchall(),
                [("preserved",)],
            )

    def test_wrong_layout_does_not_publish_an_import(self):
        source = Path(os.environ["FOLLOWER_LFG_DB2"])
        with tempfile.TemporaryDirectory() as directory:
            damaged = Path(directory) / "wrong.db2"
            data = bytearray(source.read_bytes())
            data[156:160] = b"\0\0\0\0"
            damaged.write_bytes(data)
            output = Path(directory) / "followers.sql"
            with self.assertRaisesRegex(ValueError, "layout"):
                follower_import.prepare_import(
                    damaged, Path(os.environ["FOLLOWER_WORLD_SQL"]), output
                )
            self.assertFalse(output.exists())

    def test_missing_spell_source_does_not_publish_partial_sql(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "empty.sql"
            source.write_text("-- no creature_template_spell table\n")
            output = Path(directory) / "followers.sql"
            with self.assertRaisesRegex(ValueError, "creature_template_spell"):
                follower_import.prepare_import(
                    Path(os.environ["FOLLOWER_LFG_DB2"]), source, output
                )
            self.assertFalse(output.exists())

    def test_read_only_audit_separates_reference_spawns_and_absent_npc_kits(self):
        world = Path(os.environ["FOLLOWER_WORLD_DB"])
        before = world.stat().st_mtime_ns
        report = follower_import.audit_world(
            world,
            Path(os.environ["FOLLOWER_LFG_DB2"]),
            Path(os.environ["FOLLOWER_WORLD_SQL"]),
        )
        ruby = next(row for row in report["dungeons"] if row["id"] == 2436)
        self.assertEqual(
            ruby["spawn_counts"],
            {"creature": 0, "tdb_creature": 306, "content_creature": 0},
        )
        self.assertEqual(len(report["escort_candidates"]), 37)
        self.assertEqual(
            sum(row["authored_spell_count"] for row in report["escort_candidates"]), 0
        )
        self.assertEqual(
            sum(row["smart_script_count"] for row in report["escort_candidates"]), 0
        )
        self.assertEqual(world.stat().st_mtime_ns, before)

    def test_cli_refuses_to_overwrite_world_database(self):
        with tempfile.TemporaryDirectory() as directory:
            world = Path(directory) / "world.db"
            world.write_bytes(b"must remain unchanged")
            argv = [
                "follower_import.py",
                "--lfg-db2",
                os.environ["FOLLOWER_LFG_DB2"],
                "--world-sql",
                os.environ["FOLLOWER_WORLD_SQL"],
                "--world-db",
                str(world),
                "--output",
                str(world),
            ]
            with (
                patch.object(sys, "argv", argv),
                redirect_stderr(StringIO()),
                self.assertRaises(SystemExit) as error,
            ):
                follower_import.main()
            self.assertEqual(error.exception.code, 2)
            self.assertEqual(world.read_bytes(), b"must remain unchanged")


if __name__ == "__main__":
    unittest.main()
