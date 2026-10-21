"""Behavioral publication tests using the actual native39174 source row/schema."""
import sqlite3
import unittest
from pathlib import Path

from publish_dreamwalk_tracking_quest import render_sql
from retail_content import QUEST_COLUMNS

SOURCE = Path(__file__).resolve().parents[1] / "data/world.db"
QUEST = 39174


def native_fixture():
    conn = sqlite3.connect(":memory:", uri=True)
    conn.execute("ATTACH DATABASE ? AS native", (f"{SOURCE.resolve().as_uri()}?mode=ro",))
    for table, predicate in [
        ("content_quest_template", "0"),
        ("tdb_quest_template", f"ID={QUEST}"),
        ("spell_effect", "SpellID=193753"),
        ("quest_v2", f"ID={QUEST}"),
        ("content_tuning_level", "0"),
        ("quest_money_reward", "0"),
    ]:
        ddl = conn.execute("SELECT sql FROM native.sqlite_master WHERE name=?", (table,)).fetchone()[0]
        conn.execute(ddl)
        conn.execute(f"INSERT INTO main.{table} SELECT * FROM native.{table} WHERE {predicate}")
    conn.commit()
    return conn


class PublicationTests(unittest.TestCase):
    def test_missing_native_effect_link_is_rejected_without_publication(self):
        with native_fixture() as conn:
            conn.execute("DELETE FROM spell_effect WHERE Effect=16")
            with self.assertRaisesRegex(ValueError, "definition is unavailable"):
                render_sql(conn)
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 0)

    def test_native_tracking_row_is_published_with_existing_quest_mapping_and_is_idempotent(self):
        with native_fixture() as conn:
            sql = render_sql(conn)
            conn.executescript(sql)
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template WHERE ID=?", (QUEST,)).fetchone()[0], 1)
            expected = {target: source for target, source in QUEST_COLUMNS.items() if target != "RewardBonusMoney"}
            targets = ",".join(expected)
            sources = ",".join(expected.values())
            self.assertEqual(
                conn.execute(f"SELECT {targets} FROM content_quest_template WHERE ID=?", (QUEST,)).fetchone(),
                conn.execute(f"SELECT {sources} FROM tdb_quest_template WHERE ID=?", (QUEST,)).fetchone(),
            )
            self.assertEqual(conn.execute("SELECT QuestLevel,MinLevel,RewardMoney FROM content_quest_template WHERE ID=?", (QUEST,)).fetchone(), (-1, 0, 0))
            before = conn.execute("SELECT * FROM content_quest_template WHERE ID=?", (QUEST,)).fetchone()
            conn.executescript(sql)
            self.assertEqual(conn.execute("SELECT * FROM content_quest_template WHERE ID=?", (QUEST,)).fetchone(), before)
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 1)


if __name__ == "__main__":
    unittest.main()
