"""Prepare authentic follower templates and the user-approved player-spec spell bridge.

Reads world.db only. The lead applies emitted SQL; agents may apply to a private copy.
Identity/spec labels are Spell.Description_lang (not NameSubtext_lang) in 69933.
These player spell priorities are an explicit workaround, not authored NPC kits.
"""

import argparse
import sqlite3
from contextlib import closing
from pathlib import Path

import retail_content

# entry, identity spell, class, spec, role (0 tank / 1 healer / 2 damage),
# primary attack, cooldown attack, heal, taunt, preparation aura, melee.
PROFILES = (
    (209057, 458306, 2, 66, 0, 275779, 31935, 0, 62124, 0, 1),
    (215517, 458309, 1, 73, 0, 23922, 20243, 0, 355, 0, 1),
    (209072, 458313, 11, 105, 1, 5176, 0, 8936, 0, 0, 0),
    (215011, 458314, 5, 257, 1, 585, 0, 2061, 0, 0, 0),
    (209059, 458342, 8, 63, 2, 133, 108853, 0, 0, 0, 0),
    (209053, 458346, 7, 262, 2, 188196, 51505, 0, 0, 0, 0),
    (209065, 458348, 3, 253, 2, 193455, 217200, 0, 0, 0, 0),
    (215240, 458349, 13, 1467, 2, 361469, 356995, 0, 0, 0, 0),
    (227734, 458660, 10, 269, 2, 100780, 107428, 0, 0, 0, 1),
    (230861, 464927, 9, 266, 2, 686, 105174, 0, 0, 0, 0),
    (232800, 469014, 11, 103, 2, 5221, 1822, 0, 0, 768, 1),
)
COLUMNS = (
    "entry",
    "identity_spell",
    "class_id",
    "spec_id",
    "role",
    "attack_spell",
    "cooldown_spell",
    "heal_spell",
    "taunt_spell",
    "form_spell",
    "melee",
)


class SqlStatements:
    """Reuse the existing Retail template conversion without duplicating its policy."""

    def __init__(self):
        self.statements = []

    def execute(self, sql):
        # The SQL file has no Python UDF; preserve health_expansion's exact -1/0..2 rule.
        expansion = "CASE WHEN d.HealthScalingExpansion<0 THEN 2 ELSE MIN(d.HealthScalingExpansion,2) END"
        self.statements.append(
            sql.replace("health_expansion(d.HealthScalingExpansion)", expansion) + ";"
        )


def read_profiles(db):
    rows = []
    for values in PROFILES:
        entry, identity, class_id, spec_id, role, *spells, _melee = values
        npc = db.execute(
            "SELECT name FROM tdb_creature_template WHERE entry=? AND subname=?",
            (entry, "Expedition Escort"),
        ).fetchone()
        label = db.execute(
            "SELECT n.Name_lang,s.Description_lang FROM spell_name n JOIN spell s USING(ID) WHERE ID=?",
            (identity,),
        ).fetchone()
        spec = db.execute(
            "SELECT ClassID,Role FROM chr_specialization WHERE ID=?", (spec_id,)
        ).fetchone()
        if not npc or not label or npc[0] != label[0] or spec != (class_id, role):
            raise ValueError(
                f"follower identity/spec mismatch: entry {entry}, spell {identity}, spec {spec_id}"
            )
        if not label[1]:
            raise ValueError(f"follower {entry} has no local class/spec description")
        for spell in [458324, *spells]:
            if (
                spell
                and not db.execute(
                    "SELECT 1 FROM spell_name WHERE ID=?", (spell,)
                ).fetchone()
            ):
                raise ValueError(f"follower {entry} player spell {spell} missing")
        if not db.execute(
            "SELECT 1 FROM tdb_creature_template_model WHERE CreatureID=?", (entry,)
        ).fetchone():
            raise ValueError(f"follower {entry} has no authentic display")
        rows.append((*values, npc[0], label[1]))
    return rows


def prepare_profiles(world_db, output):
    world_db, output = Path(world_db), Path(output)
    if output.resolve() == world_db.resolve():
        raise ValueError("refusing to overwrite source database")
    with closing(
        sqlite3.connect(world_db.resolve().as_uri() + "?mode=ro", uri=True)
    ) as db:
        rows = read_profiles(db)
    with closing(sqlite3.connect(":memory:")) as staging:
        ddl = ",".join(column + " INTEGER NOT NULL" for column in COLUMNS)
        staging.execute(
            f"CREATE TABLE follower_profiles ({ddl},name TEXT NOT NULL,label TEXT NOT NULL,PRIMARY KEY(entry))"
        )
        staging.executemany(
            "INSERT INTO follower_profiles VALUES ("
            + ",".join("?" for _ in rows[0])
            + ")",
            rows,
        )
        statements = list(staging.iterdump())[1:-1]
    entries = "SELECT entry FROM follower_profiles"
    converter = SqlStatements()
    retail_content.insert_tdb_templates(converter, entries, replace_legacy=True)
    statements += [
        "DROP TABLE IF EXISTS temp.tdb_difficulty;",
        f"CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE DifficultyID=0 AND Entry IN ({entries});",
        f"DELETE FROM content_creature_template WHERE entry IN ({entries});",
        *converter.statements,
    ]
    for suffix, key in [
        ("creature_template_model", "CreatureID"),
        ("creature_template_addon", "entry"),
        ("creature_equip_template", "CreatureID"),
    ]:
        columns = (
            "*"
            if suffix == "creature_template_model"
            else (
                "entry," + retail_content.ADDON_COLUMNS
                if suffix == "creature_template_addon"
                else "CreatureID,ID,ItemID1,AppearanceModID1,ItemID2,AppearanceModID2,ItemID3,AppearanceModID3"
            )
        )
        statements += [
            f"DELETE FROM content_{suffix} WHERE {key} IN ({entries});",
            f"INSERT INTO content_{suffix} SELECT {columns} FROM tdb_{suffix} WHERE {key} IN ({entries});",
        ]
    output.write_text(
        "BEGIN;\nDROP TABLE IF EXISTS follower_profiles;\n"
        + "\n".join(statements)
        + "\nCOMMIT;\n"
    )
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    count = prepare_profiles(args.world_db, args.output)
    print(
        f"Prepared {count} approved follower profiles: {args.output}; Rogue/DK/DH absent"
    )


if __name__ == "__main__":
    main()
