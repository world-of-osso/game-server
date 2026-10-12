"""Explicit-map TDB1210 open-world promotion; no full policy/overlay rebuild."""
from contextlib import closing
from collections import Counter
import json
from pathlib import Path
import sqlite3

import import_instance_population as population
import retail_content as retail
from zone_census import zone_census

# PhasingHandler::InitDbPhaseShift (TrinityCore a352b1fa): no conditional or
# inverse phases without a phase runtime. Only unflagged default phases here.
BASE_PHASE = "phaseUseFlags=0 AND PhaseId IN (0,169) AND PhaseGroup=0 AND terrainSwapMap=-1"


def validate_maps(conn, maps):
    maps = tuple(sorted(set(maps)))
    if not maps:
        raise ValueError("world-promote requires explicit --maps")
    for map_id in maps:
        row = conn.execute("SELECT InstanceType FROM map WHERE ID=?", (map_id,)).fetchone()
        if row != (0,):
            raise ValueError(f"map {map_id} is not an open-world Map.db2 row")
        if not conn.execute("SELECT 1 FROM map_difficulty WHERE MapID=? AND DifficultyID=0", (map_id,)).fetchone():
            raise ValueError(f"map {map_id} has no authored open-world difficulty 0")
    return maps


def preserve_rows(conn, table, where, keys):
    """Snapshot preexisting shared dependencies; restore by their actual keys."""
    saved = f"world_kept_{table}"
    conn.execute(f"CREATE TEMP TABLE {saved} AS SELECT * FROM {table} WHERE {where}")
    if keys is not None:
        conn.execute(f"CREATE INDEX temp.{saved}_keys ON {saved} ({','.join(keys)})")
    return table, saved, keys, where


def restore_rows(conn, snapshots):
    for table, saved, keys, where in snapshots:
        if keys is None:
            conn.execute(f"DELETE FROM {table} WHERE {where}")
        else:
            match = " AND ".join(f"s.{key}=t.{key}" for key in keys)
            conn.execute(f"DELETE FROM {table} AS t WHERE EXISTS (SELECT 1 FROM temp.{saved} s WHERE {match})")
        conn.execute(f"INSERT INTO {table} SELECT * FROM temp.{saved}")
        conn.execute(f"DROP TABLE temp.{saved}")


def protect_unrelated_dependencies(conn, map_list):
    for kind in ("creature", "gameobject"):
        conn.execute(f"CREATE TEMP TABLE world_owned_{kind}_guid AS SELECT guid+{population.OFFSET} AS guid FROM tdb_{kind} WHERE map IN ({map_list})")
        entry = "id1" if kind == "creature" else "id"
        legacy = f"""SELECT 1 FROM {kind} a WHERE a.guid=c.guid AND a.map=c.map
            AND a.{entry}=c.{entry} AND a.position_x=c.position_x
            AND a.position_y=c.position_y AND a.position_z=c.position_z"""
        conn.execute(f"""CREATE TEMP TABLE world_overlay_{kind} AS SELECT c.* FROM content_{kind} c
            WHERE map IN ({map_list}) AND guid NOT IN (SELECT guid FROM temp.world_owned_{kind}_guid)
              AND NOT EXISTS ({legacy})""")
    conn.execute(f"CREATE TEMP TABLE world_shared_creature AS SELECT DISTINCT id1 AS entry FROM content_creature WHERE map NOT IN ({map_list}) UNION SELECT id1 FROM temp.world_overlay_creature")
    conn.execute(f"CREATE TEMP TABLE world_shared_object AS SELECT DISTINCT id AS entry FROM content_gameobject WHERE map NOT IN ({map_list}) UNION SELECT id FROM temp.world_overlay_gameobject")
    shared = "SELECT entry FROM temp.world_shared_creature"
    objects = "SELECT entry FROM temp.world_shared_object"
    overlay = "SELECT guid FROM temp.world_overlay_creature"
    rules = [
        ("content_creature", f"guid IN ({overlay})", ("guid",)),
        ("content_gameobject", "guid IN (SELECT guid FROM temp.world_overlay_gameobject)", ("guid",)),
        ("content_creature_addon", f"guid IN ({overlay})", ("guid",)),
        ("content_creature_formations", f"leaderGUID IN ({overlay}) OR memberGUID IN ({overlay})", ("memberGUID",)),
        ("content_game_event_creature", f"guid IN ({overlay})", ("eventEntry", "guid")),
        ("content_creature_template", f"entry IN ({shared})", None),
        ("content_creature_template_difficulty", f"entry IN ({shared})", None),
        ("content_creature_template_model", f"CreatureID IN ({shared})", None),
        ("content_creature_template_addon", f"entry IN ({shared})", None),
        ("content_creature_equip_template", f"CreatureID IN ({shared})", None),
        ("content_gameobject_template", f"entry IN ({objects})", None),
        ("content_gameobject_template_addon", f"entry IN ({objects})", None),
        ("content_waypoint_data", "1", ("id", "point")),
        ("content_pool_template", "1", ("entry",)),
        ("content_game_event_pool", "1", ("pool_entry",)),
        ("content_pool_members", "NOT ((type=0 AND spawnId IN (SELECT guid FROM temp.world_owned_creature_guid)) OR (type=1 AND spawnId IN (SELECT guid FROM temp.world_owned_gameobject_guid)))", ("type", "spawnId")),
    ]
    return [preserve_rows(conn, *rule) for rule in rules]


def apply_population(conn, report, maps):
    maps = validate_maps(conn, maps)
    map_list = ",".join(str(m) for m in maps)
    snapshots = protect_unrelated_dependencies(conn, map_list)
    population.apply_population(conn, report, map_list=map_list, base_phase=BASE_PHASE)
    restore_rows(conn, snapshots)
    promote_zone_services(conn, report, map_list)
    conn.execute("DROP TABLE temp.world_shared_creature")
    conn.execute("DROP TABLE temp.world_shared_object")
    for kind in ("creature", "gameobject"):
        conn.execute(f"DROP TABLE temp.world_owned_{kind}_guid")
        conn.execute(f"DROP TABLE temp.world_overlay_{kind}")
    return maps


def promote_zone_services(conn, report, map_list):
    """Reuse Retail quest/vendor conversion without rebuilding existing quests/overlays."""
    conn.execute("CREATE TEMP TABLE retail_entry AS SELECT DISTINCT id AS entry FROM temp.instance_creature")
    retail.retail_relations(conn)
    conn.execute(f"""CREATE TEMP TABLE zone_quest_candidate AS
        SELECT DISTINCT quest AS ID FROM temp.retail_relation
        UNION SELECT ID FROM tdb_quest_template WHERE QuestSortID IN
            (SELECT ID FROM area_table WHERE ContinentID IN ({map_list}))""")
    conn.execute("""CREATE TEMP TABLE retail_quest AS SELECT q.ID FROM tdb_quest_template q
        JOIN temp.zone_quest_candidate z ON z.ID=q.ID
        WHERE q.ID IN (SELECT ID FROM quest_v2)
          AND q.ID NOT IN (SELECT ID FROM content_quest_template)
          AND NOT EXISTS (SELECT 1 FROM tdb_quest_objectives o WHERE o.QuestID=q.ID
              AND o.Type NOT IN (0,1,2,10))
          AND (SELECT count(*) FROM tdb_quest_objectives WHERE QuestID=q.ID AND Type IN(0,2))<=4
          AND (SELECT count(*) FROM tdb_quest_objectives WHERE QuestID=q.ID AND Type=1)<=6""")
    for (quest,) in conn.execute("""SELECT ID FROM temp.zone_quest_candidate
            WHERE ID NOT IN (SELECT ID FROM content_quest_template)
              AND ID NOT IN (SELECT ID FROM temp.retail_quest) ORDER BY ID""").fetchall():
        report.drop("tdb_quest_template", f"ID={quest}", quest,
                    "absent QuestV2/template or unsupported objective type/slot capacity; not promoted")
    quests = "SELECT ID FROM temp.retail_quest"
    conn.execute(f"CREATE TEMP TABLE azerothcore_quest AS SELECT * FROM content_quest_template WHERE ID IN ({quests})")
    retail.insert_quest_templates(conn, quests, report)
    retail.map_quest_objectives(conn, quests, report)
    retail.build_quest_children(conn, quests, report)
    retail.map_quest_exploration(conn, quests, report)
    # Add source relations only. Existing authored relations and quest decisions survive.
    for table in retail.RELATION_TABLES:
        conn.execute(f"""INSERT OR IGNORE INTO content_{table}(id,quest)
            SELECT id,quest FROM temp.retail_relation WHERE tbl=?
              AND quest IN (SELECT ID FROM content_quest_template)""", (table,))
    retail.build_vendors(conn, report)
    for table in ("retail_entry", "retail_gameobject_entry", "retail_relation", "zone_quest_candidate",
                  "retail_quest", "azerothcore_quest"):
        conn.execute(f"DROP TABLE temp.{table}")


def snapshot_content(conn):
    tables = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'content_%' ORDER BY name")]
    return {table: Counter(conn.execute(f'SELECT * FROM "{table}"')) for table in tables}


def diff_content(conn, before):
    result = {}
    for table, rows in before.items():
        after = Counter(conn.execute(f'SELECT * FROM "{table}"'))
        result[table] = dict(before=rows.total(), after=after.total(),
                             added=(after-rows).total(), removed=(rows-after).total())
    return result


def import_database(path, maps):
    path = population.validate_copy(path)
    report = population.ImportReport()
    with closing(sqlite3.connect(path)) as conn:
        with conn:
            conn.execute("BEGIN IMMEDIATE")
            before = snapshot_content(conn)
            maps = apply_population(conn, report, maps)
            row_diff = diff_content(conn, before)
            counts = {str(m): {t: conn.execute(f"SELECT count(*) FROM content_{t} WHERE map=?", (m,)).fetchone()[0]
                              for t in ("creature", "gameobject")} for m in maps}
    return {"target_maps": list(maps), "counts": counts, "row_diff": row_diff, "drops": report.drops}


def write_report(path, result):
    Path(path).write_text(json.dumps(result, indent=2) + "\n")
