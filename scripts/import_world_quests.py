#!/usr/bin/env python3
"""Import authentic polygon tasks into a NEW private world.db copy.

Unknown durations use the explicit World of Osso policy; never Retail claims.
"""
import argparse
import csv
import json
import math
from collections import defaultdict
import sqlite3
from contextlib import closing
from pathlib import Path

from retail_content import QUEST_COLUMNS

WORLD_QUEST_MODIFIER = 0x100


def read_areas(directory):
    # All five exports come from the same installed build. Separate pinned binary
    # inputs silently retained older CreatureLabel rows after active CSV updates.
    tables = [('blobs', 'QuestPOIBlob.csv'), ('points', 'QuestPOIPoint.csv'),
              ('xp', 'QuestXP.csv'), ('labels', 'CreatureLabel.csv'),
              ('factions', 'QuestFactionReward.csv')]
    poi = {}
    for key, filename in tables:
        with (Path(directory) / filename).open() as stream:
            poi[key] = [{column: int(value) for column, value in row.items()}
                        for row in csv.DictReader(stream)]
    poi['creature_labels'] = [[row['ID'], row['LabelID'], row['CreatureDifficultyID']]
                              for row in poi.pop('labels')]
    poi['faction_rewards'] = {row['ID']: [row[f'Difficulty_{n}'] for n in range(10)]
                              for row in poi.pop('factions')}
    return poi


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


def insert_creature_labels(conn, poi):
    # CreatureLabel's parent is the TDB difficulty row's CreatureDifficultyID,
    # not its difficulty mode or KillCredit aliases (pinned Player::KilledMonster).
    conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_creature_label (label_id INTEGER,creature_entry INTEGER,difficulty_id INTEGER,PRIMARY KEY(label_id,creature_entry,difficulty_id))')
    conn.execute('DELETE FROM content_world_quest_creature_label')
    difficulties = defaultdict(list)
    for entry, mode, db2_id in conn.execute('SELECT Entry,DifficultyID,CreatureDifficultyID FROM tdb_creature_template_difficulty WHERE CreatureDifficultyID>0'):
        difficulties[db2_id].append((entry,mode))
    labels = defaultdict(list)
    for _, label, difficulty in poi['creature_labels']:
        labels[label].append(difficulty)
    for label, parents in labels.items():
        if any(parent not in difficulties for parent in parents):
            continue
        memberships = {(label,entry,mode) for parent in parents for entry,mode in difficulties[parent]}
        conn.executemany('INSERT INTO content_world_quest_creature_label VALUES (?,?,?)',sorted(memberships))


def insert_criterion_bindings(conn):
    # All three polygon task trees have exactly one unconditional, amount-one
    # leaf. Its completion bit belongs in the task's persisted objective count,
    # not lifetime achievement progress. Other tree shapes stay unrepresented.
    conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_criterion (root_id INTEGER PRIMARY KEY,criterion_id INTEGER,kind INTEGER,asset INTEGER)')
    conn.execute('DELETE FROM content_world_quest_criterion')
    roots = conn.execute('SELECT DISTINCT ObjectID FROM tdb_quest_objectives WHERE Type=14').fetchall()
    for (root,) in roots:
        tree = conn.execute('SELECT Operator,Amount,CriteriaID,Flags FROM criteria_tree WHERE ID=?',(root,)).fetchone()
        children = conn.execute('SELECT ID,Operator,Amount,CriteriaID,Flags FROM criteria_tree WHERE Parent=?',(root,)).fetchall()
        if tree != (4,0,0,0) or len(children) != 1:
            continue
        leaf, operator, amount, criterion, flags = children[0]
        if operator != 0 or amount != 1 or flags != 0 or not criterion:
            continue
        if conn.execute('SELECT EXISTS(SELECT 1 FROM criteria_tree WHERE Parent=?)',(leaf,)).fetchone()[0]:
            continue
        row = conn.execute('SELECT Type,Asset FROM criteria WHERE ID=? AND ModifierTreeId=0 AND StartEvent=0 AND StartAsset=0 AND StartTimer=0 AND FailEvent=0 AND FailAsset=0 AND EligibilityWorldStateID=0 AND EligibilityWorldStateValue=0 AND Flags=32',(criterion,)).fetchone()
        if row and row[0] in (0,69) and row[1] > 0:
            conn.execute('INSERT INTO content_world_quest_criterion VALUES(?,?,?,?)',(root,criterion,*row))


def supported_objectives(conn, quest):
    rows = conn.execute('SELECT Type,ObjectID,Amount,Flags,ProgressBarWeight FROM tdb_quest_objectives WHERE QuestID=?', (quest,)).fetchall()
    for kind, object_id, amount, flags, weight in rows:
        if kind not in (0, 1, 2, 3, 9, 14, 15, 21) or amount <= 0 or (kind not in (9, 15) and object_id <= 0):
            return False
        if kind == 14 and (amount != 1 or not conn.execute('SELECT EXISTS(SELECT 1 FROM content_world_quest_criterion WHERE root_id=?)',(object_id,)).fetchone()[0]):
            return False
        if kind == 21 and not conn.execute('SELECT EXISTS(SELECT 1 FROM content_world_quest_creature_label WHERE label_id=?)',(object_id,)).fetchone()[0]:
            return False
        if not math.isfinite(weight) or weight < 0:
            raise ValueError(f'quest {quest}: invalid authenticated objective weight')
    if any(row[0] == 15 for row in rows) and not any(row[3] & 64 and row[4] > 0 for row in rows):
        return False
    return bool(rows)


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
    conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_objective (quest_id INTEGER, objective_id INTEGER, ordinal INTEGER, kind INTEGER, object_id INTEGER, amount INTEGER, flags INTEGER, weight REAL, description TEXT, flags2 INTEGER)')
    conn.execute('DELETE FROM content_world_quest_objective')
    rows = conn.execute(f'''SELECT QuestID,ID,Type,ObjectID,Amount,Flags,ProgressBarWeight,Description,Flags2
        FROM tdb_quest_objectives WHERE QuestID IN ({quests}) ORDER BY QuestID,"Order",StorageIndex,ID''').fetchall()
    ordinals = defaultdict(int)
    for quest, objective, kind, object_id, amount, flags, weight, description, flags2 in rows:
        conn.execute('INSERT INTO content_world_quest_objective VALUES (?,?,?,?,?,?,?,?,?,?)',
                     (quest,objective,ordinals[quest],kind,object_id,amount,flags,weight,description or '',flags2))
        ordinals[quest] += 1


def insert_pins(conn, quests):
    conn.execute(f"""INSERT INTO content_quest_poi
        (QuestID,id,ObjectiveIndex,MapID,WorldMapAreaId,Floor,Priority,Flags,VerifiedBuild)
        SELECT QuestID,Idx1,ObjectiveIndex,MapID,UiMapID,0,Priority,Flags,VerifiedBuild
        FROM tdb_quest_poi a WHERE QuestID IN ({quests}) AND BlobIndex=(SELECT MIN(BlobIndex)
        FROM tdb_quest_poi b WHERE b.QuestID=a.QuestID AND b.Idx1=a.Idx1)""")
    conn.execute(f"""INSERT INTO content_quest_poi_points (QuestID,Idx1,Idx2,X,Y,VerifiedBuild)
        SELECT QuestID,Idx1,Idx2,X,Y,VerifiedBuild FROM tdb_quest_poi_points WHERE QuestID IN ({quests})""")


def insert_source_rewards(conn, templates, poi):
    conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_reward (quest_id INTEGER, kind TEXT, reward_id INTEGER, amount INTEGER, cap INTEGER)')
    conn.execute('DELETE FROM content_world_quest_reward')
    conn.execute('CREATE TABLE IF NOT EXISTS content_world_quest_xp (level INTEGER, difficulty INTEGER, amount INTEGER)')
    conn.execute('DELETE FROM content_world_quest_xp')
    conn.executemany('INSERT INTO content_world_quest_xp VALUES (?,?,?)',
                     [(row['ID'], n, row[f'Difficulty_{n}']) for row in poi['xp'] for n in range(10)])
    for quest, raw in templates.items():
        source = json.loads(raw)
        for n in range(1, 5):
            currency, amount = source.get(f'RewardCurrencyID{n}', 0), source.get(f'RewardCurrencyQty{n}', 0)
            if currency and amount:
                conn.execute('INSERT INTO content_world_quest_reward VALUES (?,?,?,?,?)', (quest, 'currency', currency, amount, 0))
        for n in range(1, 6):
            faction = source.get(f'RewardFactionID{n}', 0)
            if not faction:
                continue
            override = source.get(f'RewardFactionOverride{n}', 0)
            index = source.get(f'RewardFactionValue{n}', 0)
            amount = int(override / 100) if override else poi['faction_rewards'][2 if index < 0 else 1][abs(index)]
            if faction and amount:
                conn.execute('INSERT INTO content_world_quest_reward VALUES (?,?,?,?,?)',
                             (quest, 'reputation', faction, amount, source[f'RewardFactionCapIn{n}']))


def import_definitions(conn, receipt, poi, policy_duration=86400):
    world_ids = {row[0] for row in conn.execute('SELECT q.ID FROM tdb_quest_template q JOIN quest_info i ON i.ID=q.QuestInfoID WHERE i.Modifiers & 256 != 0')}
    areas = polygon_areas(dict(poi, blobs=[blob for blob in poi['blobs'] if blob['QuestID'] in world_ids]))
    insert_creature_labels(conn, poi)
    insert_criterion_bindings(conn)
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
        # Rebuild the derived area table; old XY-only exports must not supply fake Z0.
        conn.execute('DROP TABLE IF EXISTS content_world_quest_area')
        conn.execute('CREATE TABLE content_world_quest_area (quest_id INTEGER, blob_id INTEGER, map_id INTEGER, ui_map_id INTEGER, point_index INTEGER, x REAL, y REAL, z REAL NOT NULL)')
        for quest, *_ in definitions:
            for blob, vertices in areas[quest]:
                conn.executemany('INSERT INTO content_world_quest_area VALUES (?,?,?,?,?,?,?,?)',
                                 [(quest, blob['ID'], blob['MapID'], blob['UiMapID'], n, p['X'], p['Y'], p['Z'])
                                  for n, p in enumerate(vertices)])
        for table, key in [("content_quest_template","ID"), ("content_quest_poi","QuestID"), ("content_quest_poi_points","QuestID")]:
            conn.execute(f"DELETE FROM {table} WHERE {key} IN ({quests})")
        conn.execute("DELETE FROM content_world_quest")
        insert_templates(conn, quests)
        insert_pins(conn, quests)
        statement = conn.execute(f"SELECT * FROM tdb_quest_template WHERE ID IN ({quests})")
        columns = [column[0] for column in statement.description]
        templates = {row[0]: json.dumps(dict(zip(columns,row))) for row in statement.fetchall()}
        insert_source_rewards(conn, templates, poi)
        for table in ('quest_template_addon', 'quest_offer_reward', 'quest_request_items'):
            installed = conn.execute("SELECT count(*) FROM sqlite_master WHERE name=?", ('tdb_' + table,)).fetchone()[0]
            if not installed:
                continue
            target = [row[1] for row in conn.execute(f'PRAGMA table_info(content_{table})')]
            source = {row[1] for row in conn.execute(f'PRAGMA table_info(tdb_{table})')}
            columns = ','.join(name for name in target if name in source)
            conn.execute(f'DELETE FROM content_{table} WHERE ID IN ({quests})')
            conn.execute(f'INSERT INTO content_{table} ({columns}) SELECT {columns} FROM tdb_{table} WHERE ID IN ({quests})')
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


def census(conn, poi):
    world_ids = {row[0] for row in conn.execute('SELECT q.ID FROM tdb_quest_template q JOIN quest_info i ON i.ID=q.QuestInfoID WHERE i.Modifiers &256 !=0')}
    blobs = [blob for blob in poi['blobs'] if blob['QuestID'] in world_ids]
    blob_ids = {blob['QuestID'] for blob in blobs}
    area_ids = set(polygon_areas(dict(poi, blobs=blobs)))
    admitted = {row[0] for row in conn.execute('SELECT quest_id FROM content_world_quest')}
    durations = dict(conn.execute('SELECT duration_source,count(*) FROM content_world_quest GROUP BY duration_source'))
    raw = [json.loads(row[0]) for row in conn.execute('SELECT source_template_json FROM content_world_quest')]
    no_rewards = sum(not (row['RewardXPDifficulty'] or row['RewardMoneyDifficulty']
                         or any(row.get(f'RewardItem{n}', 0) for n in range(1,5))
                         or conn.execute('SELECT count(*) FROM content_world_quest_reward WHERE quest_id=?', (row['ID'],)).fetchone()[0]) for row in raw)
    return dict(total=len(world_ids), no_poi_blob=len(world_ids-blob_ids), point_only=len(blob_ids-area_ids),
                polygon_quests=len(area_ids), unsupported_objectives=len(area_ids-admitted), admitted=len(admitted),
                authentic_durations=durations.get('authentic',0), policy_durations=durations.get('policy',0), no_reward_data=no_rewards)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-db", required=True, type=Path)
    parser.add_argument("--output-db", required=True, type=Path)
    parser.add_argument("--duration-receipt", required=True, type=Path)
    parser.add_argument('--poi-db2-dir', required=True, type=Path)
    parser.add_argument('--policy-duration', type=int, default=86400)
    args = parser.parse_args()
    receipt = json.loads(args.duration_receipt.read_text())
    poi = read_areas(args.poi_db2_dir)
    imported = copy_and_import(args.source_db, args.output_db, receipt, poi, args.policy_duration)
    with closing(sqlite3.connect(f'{args.output_db.resolve().as_uri()}?mode=ro', uri=True)) as conn:
        counts = census(conn, poi)
    print(json.dumps({'output': str(args.output_db), 'imported': imported, 'census': counts}))


if __name__ == "__main__":
    main()
