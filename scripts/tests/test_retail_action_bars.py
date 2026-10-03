"""Relocate unpaged bonus-bar buttons of playercreateinfo_action rows."""

import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import retail_action_bars  # noqa: E402

SPELL, ITEM = 0, 128


def actions(rows):
    conn = sqlite3.connect(":memory:")
    conn.execute("""CREATE TABLE tdb_playercreateinfo_action (race INTEGER, class INTEGER, button INTEGER,
        action INTEGER, type INTEGER, PRIMARY KEY (race, class, button))""")
    conn.executemany("INSERT INTO tdb_playercreateinfo_action VALUES (?, ?, ?, ?, ?)", rows)
    return conn


def table(conn):
    return conn.execute("SELECT * FROM tdb_playercreateinfo_action ORDER BY race, class, button").fetchall()


class RelocateUnpagedButtonsTest(unittest.TestCase):
    def test_human_warrior_attack_leaves_the_battle_stance_bar(self):
        conn = actions([(1, 1, 0, 1464, SPELL), (1, 1, 9, 59752, SPELL), (1, 1, 10, 1231411, SPELL),
                        (1, 1, 72, 88163, SPELL), (1, 1, 81, 59752, SPELL)])
        self.assertEqual(retail_action_bars.relocate_unpaged_buttons(conn), {1: (1, 1)})
        self.assertEqual(table(conn), [(1, 1, 0, 1464, SPELL), (1, 1, 1, 88163, SPELL),
                                       (1, 1, 9, 59752, SPELL), (1, 1, 10, 1231411, SPELL)])
        self.assertEqual(retail_action_bars.relocate_unpaged_buttons(conn), {})

    def test_item_keeps_its_page_one_slot_when_empty(self):
        conn = actions([(11, 8, 0, 116, SPELL), (11, 8, 83, 4540, ITEM)])
        retail_action_bars.relocate_unpaged_buttons(conn)
        self.assertEqual(table(conn), [(11, 8, 0, 116, SPELL), (11, 8, 11, 4540, ITEM)])

    def test_rogue_stealth_and_druid_form_bars_stay(self):
        rows = [(1, 4, 72, 1833, SPELL), (28, 11, 96, 33917, SPELL), (28, 11, 108, 8921, SPELL)]
        conn = actions(rows)
        self.assertEqual(retail_action_bars.relocate_unpaged_buttons(conn), {})
        self.assertEqual(table(conn), rows)


if __name__ == "__main__":
    unittest.main()
