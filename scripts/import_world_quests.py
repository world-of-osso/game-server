#!/usr/bin/env python3
"""Import source-backed durations/templates into a NEW private world.db copy.

The duration receipt is audit_sources.py's local CASC source-census.json.
Missing durations stay excluded. This command never modifies its source DB.
"""
import argparse
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from retail_content import QUEST_COLUMNS, map_quest_objectives
from content_common import Report

WORLD_QUEST_MODIFIER = 0x100


def select_definitions(conn, receipt):
    states = receipt["managed_world_states"]
    rows = conn.execute("""SELECT q.ID, q.QuestSortID, q.ManagedWorldStateID
        FROM tdb_quest_template q JOIN quest_info i ON i.ID=q.QuestInfoID
        JOIN quest_v2 v ON v.ID=q.ID WHERE (i.Modifiers & ?) != 0 ORDER BY q.ID""",
        (WORLD_QUEST_MODIFIER,)).fetchall()
    definitions = []
    for quest, zone, state_id in rows:
        state = states.get(str(state_id))
        if state is None:
            continue
        duration = state["UpTimeSecs"]
        if state["ID"] != state_id or not isinstance(duration, int) or not 0 < duration <= 0xFFFFFFFF:
            raise ValueError(f"quest {quest}: invalid authentic duration definition")
        definitions.append((quest, zone, duration, state_id))
    return definitions


def insert_templates(conn, quests):
    columns = ",".join(QUEST_COLUMNS)
    sources = ",".join(f"q.{name}" for name in QUEST_COLUMNS.values())
    difficulty = "CASE q.RewardMoneyDifficulty " + " ".join(
        f"WHEN {index} THEN m.Difficulty_{index}" for index in range(10)) + " ELSE 0 END"
    conn.execute(f"""INSERT INTO content_quest_template ({columns}, QuestLevel, MinLevel, RewardMoney)
        SELECT {sources}, -1, COALESCE(l.minlevel,0),
        COALESCE(CAST({difficulty} * q.RewardMoneyMultiplier AS INTEGER),0)
        FROM tdb_quest_template q LEFT JOIN content_tuning_level l ON l.ID=q.ContentTuningID
        LEFT JOIN quest_money_reward m ON m.ID=l.maxlevel WHERE q.ID IN ({quests})""")
    report = Report()
    map_quest_objectives(conn, quests, report)
    if report.rows:
        raise ValueError(f"world quest has unsupported objectives: {report.rows}")
    for quest, name in conn.execute(f"""SELECT o.QuestID,t.name FROM tdb_quest_objectives o
        JOIN tdb_creature_template t ON t.entry=o.ObjectID WHERE o.Type=0 AND o.QuestID IN ({quests})"""):
        conn.execute("UPDATE content_quest_template SET ObjectiveText1=? WHERE ID=? AND ObjectiveText1=''", (f"{name} slain",quest))


def insert_pins(conn, quests):
    conn.execute(f"""INSERT INTO content_quest_poi
        (QuestID,id,ObjectiveIndex,MapID,WorldMapAreaId,Floor,Priority,Flags,VerifiedBuild)
        SELECT QuestID,Idx1,ObjectiveIndex,MapID,UiMapID,0,Priority,Flags,VerifiedBuild
        FROM tdb_quest_poi a WHERE QuestID IN ({quests}) AND BlobIndex=(SELECT MIN(BlobIndex)
        FROM tdb_quest_poi b WHERE b.QuestID=a.QuestID AND b.Idx1=a.Idx1)""")
    conn.execute(f"""INSERT INTO content_quest_poi_points (QuestID,Idx1,Idx2,X,Y,VerifiedBuild)
        SELECT QuestID,Idx1,Idx2,X,Y,VerifiedBuild FROM tdb_quest_poi_points WHERE QuestID IN ({quests})""")


def import_definitions(conn, receipt):
    definitions = select_definitions(conn, receipt)
    conn.execute("CREATE TEMP TABLE IF NOT EXISTS world_quest_selection (ID INTEGER PRIMARY KEY)")
    conn.execute("DELETE FROM world_quest_selection")
    conn.executemany("INSERT INTO world_quest_selection VALUES (?)", [(row[0],) for row in definitions])
    quests = "SELECT ID FROM world_quest_selection"
    with conn:
        conn.execute("""CREATE TABLE IF NOT EXISTS content_world_quest (
            quest_id INTEGER PRIMARY KEY, zone_id INTEGER NOT NULL, duration_seconds INTEGER NOT NULL CHECK(duration_seconds>0),
            managed_world_state_id INTEGER NOT NULL, source_template_json TEXT NOT NULL)""")
        for table, key in [("content_quest_template","ID"), ("content_quest_poi","QuestID"), ("content_quest_poi_points","QuestID")]:
            conn.execute(f"DELETE FROM {table} WHERE {key} IN ({quests})")
        conn.execute("DELETE FROM content_world_quest")
        insert_templates(conn, quests)
        insert_pins(conn, quests)
        statement = conn.execute(f"SELECT * FROM tdb_quest_template WHERE ID IN ({quests})")
        columns = [column[0] for column in statement.description]
        templates = {row[0]: json.dumps(dict(zip(columns,row))) for row in statement.fetchall()}
        conn.executemany("INSERT INTO content_world_quest VALUES (?,?,?,?,?)", [(*row,templates[row[0]]) for row in definitions])
    return [row[0] for row in definitions]


def copy_and_import(source, output, receipt):
    source = Path(source).resolve(strict=True)
    output = Path(output).resolve()
    if source == output:
        raise ValueError("output must be a new private database, not the source")
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("xb"):
        pass
    with closing(sqlite3.connect(f"{source.as_uri()}?mode=ro", uri=True)) as original:
        with closing(sqlite3.connect(output)) as private:
            original.backup(private)
            return import_definitions(private, receipt)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-db", required=True, type=Path)
    parser.add_argument("--output-db", required=True, type=Path)
    parser.add_argument("--duration-receipt", required=True, type=Path)
    args = parser.parse_args()
    receipt = json.loads(args.duration_receipt.read_text())
    imported = copy_and_import(args.source_db, args.output_db, receipt)
    print(json.dumps({"output": str(args.output_db), "imported": imported}))


if __name__ == "__main__":
    main()
