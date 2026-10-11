"""Explicit-map TDB1210 open-world promotion; no full policy/overlay rebuild."""
from contextlib import closing
import json
from pathlib import Path
import sqlite3

import import_instance_population as population

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
    conn.execute(f"CREATE TEMP TABLE world_shared_creature AS SELECT DISTINCT id1 AS entry FROM content_creature WHERE map NOT IN ({map_list})")
    conn.execute(f"CREATE TEMP TABLE world_shared_object AS SELECT DISTINCT id AS entry FROM content_gameobject WHERE map NOT IN ({map_list})")
    shared = "SELECT entry FROM temp.world_shared_creature"
    objects = "SELECT entry FROM temp.world_shared_object"
    rules = [
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
        ("content_pool_members", f"NOT ((type=0 AND spawnId IN (SELECT guid FROM content_creature WHERE map IN ({map_list}))) OR (type=1 AND spawnId IN (SELECT guid FROM content_gameobject WHERE map IN ({map_list}))))", ("type", "spawnId")),
    ]
    return [preserve_rows(conn, *rule) for rule in rules]


def apply_population(conn, report, maps):
    maps = validate_maps(conn, maps)
    map_list = ",".join(str(m) for m in maps)
    snapshots = protect_unrelated_dependencies(conn, map_list)
    population.apply_population(conn, report, map_list=map_list, base_phase=BASE_PHASE)
    restore_rows(conn, snapshots)
    conn.execute("DROP TABLE temp.world_shared_creature")
    conn.execute("DROP TABLE temp.world_shared_object")
    return maps


def import_database(path, maps):
    path = population.validate_copy(path)
    report = population.ImportReport()
    with closing(sqlite3.connect(path)) as conn:
        with conn:
            conn.execute("BEGIN IMMEDIATE")
            maps = apply_population(conn, report, maps)
            counts = {str(m): {t: conn.execute(f"SELECT count(*) FROM content_{t} WHERE map=?", (m,)).fetchone()[0]
                              for t in ("creature", "gameobject")} for m in maps}
    return {"target_maps": list(maps), "counts": counts, "drops": report.drops}


def write_report(path, result):
    Path(path).write_text(json.dumps(result, indent=2) + "\n")
