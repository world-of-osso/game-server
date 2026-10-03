#!/usr/bin/env python3
"""Move new-character bonus-bar buttons the Retail client never pages to onto the main bar.

  retail_action_bars.py --db data/world.db.next

TDB1210 playercreateinfo_action (unchanged through TrinityCore a352b1fa) still holds the
WotLK stance/presence/Shadowform bar rows: a Warrior's Attack 88163 sits on button 72,
Battle Stance's bonus bar 1. Retail pages the main bar only for a shapeshift form with a
non-zero SpellShapeshiftForm.BonusActionBar: Rogue Stealth (form 30) and Shadow Dance (13),
Druid Cat (1), Bear (5, 8) and Moonkin (31, 35). Retail Warrior stances 386164/386196/386208
apply no SPELL_AURA_MOD_SHAPESHIFT and Shadowform 232698 sets form 28 (BonusActionBar 0), so
every other class's buttons 72-119 are never shown.

Each such button moves to its page-1 slot (button % 12). A button whose action already sits on
the main bar is dropped; one whose slot is taken goes to the first empty main bar slot.
Idempotent: a second run finds no buttons left to move.
"""

import argparse
import sqlite3
from pathlib import Path

ROGUE, DRUID = 4, 11
PAGED_CLASSES = (ROGUE, DRUID)
MAIN_BAR = range(12)
BONUS_BARS = range(72, 120)


def relocate_unpaged_buttons(conn):
    """Rewrite tdb_playercreateinfo_action; return {class: (moved, dropped)}."""
    placeholders = ",".join("?" * len(PAGED_CLASSES))
    rows = conn.execute(
        f"""SELECT race, class, button, action, type FROM tdb_playercreateinfo_action
            WHERE button BETWEEN ? AND ? AND class NOT IN ({placeholders}) ORDER BY race, class, button""",
        (BONUS_BARS.start, BONUS_BARS.stop - 1, *PAGED_CLASSES)).fetchall()
    counts = {}
    for race, class_id, button, action, kind in rows:
        main_bar = {slot: (a, t) for slot, a, t in conn.execute(
            "SELECT button, action, type FROM tdb_playercreateinfo_action WHERE race = ? AND class = ? AND button < ?",
            (race, class_id, len(MAIN_BAR)))}
        conn.execute("DELETE FROM tdb_playercreateinfo_action WHERE race = ? AND class = ? AND button = ?",
                     (race, class_id, button))
        moved, dropped = counts.get(class_id, (0, 0))
        if (action, kind) in main_bar.values():
            counts[class_id] = (moved, dropped + 1)
            continue
        slot = button % len(MAIN_BAR)
        if slot in main_bar:
            slot = next((s for s in MAIN_BAR if s not in main_bar), None)
            if slot is None:
                raise ValueError(f"race {race} class {class_id}: no empty main bar slot for button {button}")
        conn.execute("INSERT INTO tdb_playercreateinfo_action (race, class, button, action, type) VALUES (?, ?, ?, ?, ?)",
                     (race, class_id, slot, action, kind))
        counts[class_id] = (moved + 1, dropped)
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", required=True, type=Path)
    args = parser.parse_args()
    with sqlite3.connect(args.db) as conn:
        counts = relocate_unpaged_buttons(conn)
    for class_id, (moved, dropped) in sorted(counts.items()):
        print(f"class {class_id}: {moved} moved to the main bar, {dropped} already on it dropped")


if __name__ == "__main__":
    main()
