#!/usr/bin/env python3
"""Import authentic polygon tasks into a NEW private world.db copy.

Unknown durations use the explicit World of Osso policy; never Retail claims.
"""
import argparse
import csv
import json
from collections import defaultdict
import sqlite3
from contextlib import closing
from pathlib import Path

from retail_content import QUEST_COLUMNS, map_quest_objectives
from content_common import Report

WORLD_QUEST_MODIFIER = 0x100


def read_areas(directory):
    return {key: [dict((k, int(v)) for k, v in row.items())
                  for row in csv.DictReader((Path(directory) / filename).open())]
            for key, filename in [('blobs', 'QuestPOIBlob.csv'), ('points', 'QuestPOIPoint.csv')]}


def polygon_areas(poi):
    points = defaultdict(list)
    for point in sorted(poi['points'], key=lambda p: p['ID']):
        points[point['QuestPOIBlobID']].append(point)
    areas = defaultdict(list)
    for blob in poi['blobs']:
        vertices = points[blob['ID']]
        if len(vertices) != blob['NumPoints']:
            raise ValueError(f"blob {blob['ID']}: point count differs from DB2")
        if len({(p['X'], p['Y']) for p in vertices}) < 3:
            continue
        area = sum(a['X'] * b['Y'] - b['X'] * a['Y']
                   for a, b in zip(vertices, vertices[1:] + vertices[:1]))
        if area == 0:
            continue
        areas[blob['QuestID']].append((blob, vertices))
    return areas


def supported_objectives(conn, quest):
    rows = conn.execute('SELECT Type,ObjectID,Amount FROM tdb_quest_objectives WHERE QuestID=?', (quest,)).fetchall()
    counts = defaultdict(int)
    for kind, object_id, amount in rows:
        if kind not in (0, 1, 2, 7) or amount <= 0 or object_id <= 0:
            return False
        counts[kind] += 1
    return bool(rows) and counts[0] + counts[2] <= 4 and counts[1] <= 6 and counts[7] <= 1


def select_definitions(conn, receipt, areas, policy_duration):
    states = receipt["managed_world_states"]
    rows = conn.execute("""SELECT q.ID, q.QuestSortID, q.ManagedWorldStateID
        FROM tdb_quest_template q JOIN quest_info i ON i.ID=q.QuestInfoID
        JOIN quest_v2 v ON v.ID=q.ID WHERE (i.Modifiers & ?) != 0 ORDER BY q.ID""",
        (WORLD_QUEST_MODIFIER,)).fetchall()
    if not 0 < policy_duration <= 0xFFFFFFFF:
        raise ValueError('policy duration must be positive uint32')
    definitions = []
    for quest, zone, state_id in rows:
        if quest not in areas or not supported_objectives(conn, quest):
            continue
        state = states.get(str(state_id))
        duration = state['UpTimeSecs'] if state else policy_duration
        if state and (state['ID'] != state_id or not isinstance(duration, int) or not 0 < duration <= 0xFFFFFFFF):
            raise ValueError(f"quest {quest}: invalid authentic duration definition")
        definitions.append((quest, zone, duration, state_id, 'authentic' if state else 'policy'))
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


def import_definitions(conn, receipt, poi, policy_duration=86400):
    world_ids = {row[0] for row in conn.execute('SELECT q.ID FROM tdb_quest_template q JOIN quest_info i ON i.ID=q.QuestInfoID WHERE i.Modifiers & 256 != 0')}
    areas = polygon_areas(dict(poi, blobs=[blob for blob in poi['blobs'] if blob['QuestID'] in world_ids]))
    definitions = select_definitions(conn, receipt, areas, policy_duration)
    conn.execute("CREATE TEMP TABLE IF NOT EXISTS world_quest_selection (ID INTEGER PRIMARY KEY)")
    conn.execute("DELETE FROM world_quest_selection")
    conn.executemany("INSERT INTO world_quest_selection VALUES (?)", [(row[0],) for row in definitions])
    quests = "SELECT ID FROM world_quest_selection"
    with conn:
        conn.execute("""CREATE TABLE IF NOT EXISTS content_world_quest (
            quest_id INTEGER PRIMARY KEY, zone_id INTEGER NOT NULL, duration_seconds INTEGER NOT NULL CHECK(duration_seconds>0),
            managed_world_state_id INTEGER NOT NULL, source_template_json TEXT NOT NULL,
            duration_source TEXT NOT NULL)""")
        columns = {row[1] for row in conn.execute('PRAGMA table_info(content_world_quest)')}
        if 'duration_source' not in columns:
            conn.execute("ALTER TABLE content_world_quest ADD COLUMN duration_source TEXT NOT NULL DEFAULT 'authentic'")
        conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_area (quest_id INTEGER, blob_id INTEGER, map_id INTEGER, ui_map_id INTEGER, point_index INTEGER, x REAL, y REAL)')
        conn.execute('DELETE FROM content_world_quest_area')
        for quest, *_ in definitions:
            for blob, vertices in areas[quest]:
                conn.executemany('INSERT INTO content_world_quest_area VALUES (?,?,?,?,?,?,?)',
                                 [(quest, blob['ID'], blob['MapID'], blob['UiMapID'], n, p['X'], p['Y'])
                                  for n, p in enumerate(vertices)])
        for table, key in [("content_quest_template","ID"), ("content_quest_poi","QuestID"), ("content_quest_poi_points","QuestID")]:
            conn.execute(f"DELETE FROM {table} WHERE {key} IN ({quests})")
        conn.execute("DELETE FROM content_world_quest")
        insert_templates(conn, quests)
        insert_pins(conn, quests)
        statement = conn.execute(f"SELECT * FROM tdb_quest_template WHERE ID IN ({quests})")
        columns = [column[0] for column in statement.description]
        templates = {row[0]: json.dumps(dict(zip(columns,row))) for row in statement.fetchall()}
        conn.executemany('INSERT INTO content_world_quest (quest_id,zone_id,duration_seconds,managed_world_state_id,source_template_json,duration_source) VALUES (?,?,?,?,?,?)',
                         [(*row[:4],templates[row[0]],row[4]) for row in definitions])
    return [row[0] for row in definitions]


def copy_and_import(source, output, receipt, poi, policy_duration=86400):
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
            return import_definitions(private, receipt, poi, policy_duration)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-db", required=True, type=Path)
    parser.add_argument("--output-db", required=True, type=Path)
    parser.add_argument("--duration-receipt", required=True, type=Path)
    parser.add_argument('--poi-db2-dir', required=True, type=Path)
    parser.add_argument('--policy-duration', type=int, default=86400)
    args = parser.parse_args()
    receipt = json.loads(args.duration_receipt.read_text())
    imported = copy_and_import(args.source_db, args.output_db, receipt, read_areas(args.poi_db2_dir), args.policy_duration)
    print(json.dumps({"output": str(args.output_db), "imported": imported}))


if __name__ == "__main__":
    main()
