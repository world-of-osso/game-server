import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import migrate_creature_static_flags
import retail_content


class CreatureStaticFlagsTests(unittest.TestCase):
    def fixture(self):
        conn = sqlite3.connect(":memory:")
        conn.executescript("""
            CREATE TABLE difficulty (ID INTEGER, FallbackDifficultyID INTEGER);
            INSERT INTO difficulty VALUES (205, 1), (1, 0);
            CREATE TABLE tdb_creature_template_difficulty (
                Entry INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER,
                HealthScalingExpansion INTEGER, LevelScalingDeltaMin INTEGER, LevelScalingDeltaMax INTEGER,
                HealthModifier REAL, ManaModifier REAL, ArmorModifier REAL, DamageModifier REAL,
                StaticFlags1 INTEGER, StaticFlags2 INTEGER, StaticFlags3 INTEGER, StaticFlags4 INTEGER,
                StaticFlags5 INTEGER, StaticFlags6 INTEGER, StaticFlags7 INTEGER, StaticFlags8 INTEGER);
            INSERT INTO tdb_creature_template_difficulty VALUES
                (244041, 0, 0, 0, 0, 0, 1, 1, 1, 1, 805306624, 2, 3, 4, 5, 6, 7, 8),
                (244041, 1, 0, 0, 0, 0, 1, 1, 1, 1, 256, 12, 13, 14, 15, 16, 17, 18);
            CREATE TABLE content_tuning_level (ID INTEGER, minlevel INTEGER, maxlevel INTEGER);
            CREATE TABLE content_creature_template (entry INTEGER PRIMARY KEY);
            INSERT INTO content_creature_template VALUES (244041), (999999);
            CREATE TEMP TABLE spawn_row (entry INTEGER, difficulty INTEGER, row_difficulty INTEGER);
            INSERT INTO spawn_row VALUES (244041, 205, 1);
        """)
        conn.create_function("health_expansion", 1, lambda value: value)
        return conn

    def test_import_carries_all_difficulty_flags_from_resolved_source(self):
        conn = self.fixture()
        retail_content.build_template_difficulties(conn, None)
        row = conn.execute("SELECT StaticFlags1,StaticFlags2,StaticFlags3,StaticFlags4,"
                           "StaticFlags5,StaticFlags6,StaticFlags7,StaticFlags8 "
                           "FROM content_creature_template_difficulty WHERE entry=244041 AND DifficultyID=205").fetchone()
        self.assertEqual(row, (256, 12, 13, 14, 15, 16, 17, 18))

    def test_migration_uses_difficulty_fallback_and_preserves_non_tdb_rows(self):
        conn = self.fixture()
        conn.executescript("""
            CREATE TABLE content_creature_template_difficulty (entry INTEGER, DifficultyID INTEGER,
                PRIMARY KEY(entry, DifficultyID));
            INSERT INTO content_creature_template_difficulty VALUES (244041,205);
        """)
        migrate_creature_static_flags.project_static_flags(conn)
        columns = ",".join(migrate_creature_static_flags.FLAGS)
        self.assertEqual(conn.execute(f"SELECT {columns} FROM content_creature_template_difficulty").fetchone(),
                         (256, 12, 13, 14, 15, 16, 17, 18))
        self.assertEqual(conn.execute(f"SELECT {columns} FROM content_creature_template WHERE entry=244041").fetchone(),
                         (256, 12, 13, 14, 15, 16, 17, 18))
        self.assertEqual(conn.execute(f"SELECT {columns} FROM content_creature_template WHERE entry=999999").fetchone(),
                         (0,) * 8)


if __name__ == "__main__":
    unittest.main()
