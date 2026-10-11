#!/usr/bin/env python3
"""Promote authored base-phase battleground spawns without rebuilding content policy.

Only selected-map spawns, their creature addons and their template/model/equipment
rows are replaced. Other tables (including Zephras ownership, quests and tuning)
are untouched. Maps requiring event, pool, formation or waypoint catalogs are
rejected: those relationships need a separate scoped importer. No CSV/overlay
inputs are read. All writes are one transaction; an existing database is required.

  python3 scripts/promote-battlegrounds.py --db data/world.db --only-maps 2106
"""
import argparse
import sqlite3
from pathlib import Path

import retail_content as retail
from content_common import Report


ENTRIES = "SELECT DISTINCT id FROM temp.selected_creature"


def select_spawns(conn, map_ids):
    map_list = ",".join(str(int(m)) for m in sorted(set(map_ids)))
    for map_id in map_ids:
        row = conn.execute("SELECT InstanceType FROM map WHERE ID=?", (map_id,)).fetchone()
        if row != (retail.INSTANCE_TYPE_BATTLEGROUND,):
            raise ValueError(f"map {map_id} is not an authored battleground")
    retail.create_retail_map_table(conn, map_ids)
    for table in ("creature", "gameobject"):
        conn.execute(f"CREATE TEMP TABLE selected_{table} AS SELECT * FROM tdb_{table} "
                     f"WHERE map IN ({map_list}) AND {retail.BASE_PHASE}")
    conn.execute("CREATE TEMP TABLE retail_gameobject AS SELECT guid AS tdb_guid,id FROM temp.selected_gameobject")
    return map_list


def require_unlinked_spawns(conn):
    """Do not silently promote maps whose additional runtime catalogs are unsupported."""
    checks = [
        "SELECT 1 FROM tdb_creature_addon WHERE guid IN (SELECT guid FROM temp.selected_creature) AND PathId<>0",
        f"SELECT 1 FROM tdb_creature_template_addon WHERE entry IN ({ENTRIES}) AND PathId<>0",
        "SELECT 1 FROM tdb_creature_formations WHERE memberGUID IN (SELECT guid FROM temp.selected_creature) OR leaderGUID IN (SELECT guid FROM temp.selected_creature)",
    ]
    for table in ("creature", "gameobject"):
        checks.append(f"SELECT 1 FROM tdb_game_event_{table} WHERE guid IN (SELECT guid FROM temp.selected_{table})")
    checks.append("SELECT 1 FROM tdb_pool_members WHERE (type=0 AND spawnId IN (SELECT guid FROM temp.selected_creature)) OR (type=1 AND spawnId IN (SELECT guid FROM temp.selected_gameobject))")
    if any(conn.execute(query).fetchone() for query in checks):
        raise ValueError("battleground has event/pool/formation/waypoint dependencies; scoped promotion unsupported")


def promote_templates(conn, report, map_list):
    retail.create_difficulty_table(conn, map_list=map_list)
    conn.create_function("health_expansion", 1, retail.health_expansion, deterministic=True)
    conn.execute(f"DELETE FROM content_creature_template WHERE entry IN ({ENTRIES})")
    retail.insert_tdb_templates(conn, ENTRIES, replace_legacy=True)
    for table, key, columns in (
        ("creature_template_model", "CreatureID", "CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild"),
        ("creature_template_addon", "entry", f"entry,{retail.ADDON_COLUMNS}"),
        ("creature_equip_template", "CreatureID", "CreatureID,ID,ItemID1,AppearanceModID1,ItemID2,AppearanceModID2,ItemID3,AppearanceModID3"),
    ):
        conn.execute(f"DELETE FROM content_{table} WHERE {key} IN ({ENTRIES})")
        conn.execute(f"INSERT INTO content_{table} SELECT {columns} FROM tdb_{table} WHERE {key} IN ({ENTRIES})")
    retail.build_gameobject_templates(conn, report, retain_missing_displays=True)
    for table, key in (("creature", "id"), ("gameobject", "id")):
        missing = conn.execute(f"SELECT DISTINCT {key} FROM temp.selected_{table} WHERE {key} NOT IN "
                               f"(SELECT entry FROM content_{table}_template)").fetchall()
        if missing:
            raise ValueError(f"battleground {table} templates cannot be resolved: {missing}")


def promote_spawns(conn, map_list):
    for table in ("creature", "gameobject"):
        # Match the shared policy's explicit spawn projection; omitted legacy columns
        # take their schema defaults, not similarly named nullable Retail columns.
        if table == "creature":
            columns = "guid,id1,map,zoneId,areaId,equipment_id,position_x,position_y,position_z,orientation,spawntimesecs,wander_distance,currentwaypoint,MovementType,unit_flags,ScriptName,VerifiedBuild,modelid,spawnDifficulties".split(",")
        else:
            columns = "guid,id,map,zoneId,areaId,position_x,position_y,position_z,orientation,rotation0,rotation1,rotation2,rotation3,spawntimesecs,animprogress,state,ScriptName,VerifiedBuild,spawnDifficulties".split(",")
        expressions = {
            "guid": f"guid+{retail.RETAIL_GUID_OFFSET}",
            "id1": "id",
            "unit_flags": "COALESCE(unit_flags,0)",
            "spawnDifficulties": retail.SPAWN_DIFFICULTIES,
        }
        if table == "creature":
            expressions["modelid"] = "CASE WHEN modelid IN (SELECT ID FROM creature_display_info) THEN modelid ELSE 0 END"
        source = ",".join(expressions.get(column, f'"{column}"') for column in columns)
        target = ",".join(f'"{column}"' for column in columns)
        conn.execute(f"DELETE FROM content_{table} WHERE map IN ({map_list})")
        conn.execute(f"INSERT INTO content_{table} ({target}) SELECT {source} FROM temp.selected_{table}")
    conn.execute(f"DELETE FROM content_creature_addon WHERE guid IN "
                 f"(SELECT guid+{retail.RETAIL_GUID_OFFSET} FROM temp.selected_creature)")
    conn.execute(f"INSERT INTO content_creature_addon SELECT guid+{retail.RETAIL_GUID_OFFSET},{retail.ADDON_COLUMNS} "
                 "FROM tdb_creature_addon WHERE guid IN (SELECT guid FROM temp.selected_creature)")


def promote_database(path, map_ids):
    if not map_ids:
        raise ValueError("at least one battleground map is required")
    report = Report()
    with sqlite3.connect(f"{Path(path).resolve().as_uri()}?mode=rw", uri=True) as conn:
        conn.execute("BEGIN IMMEDIATE")
        map_list = select_spawns(conn, map_ids)
        require_unlinked_spawns(conn)
        promote_templates(conn, report, map_list)
        promote_spawns(conn, map_list)
        counts = {table: conn.execute(f"SELECT count(*) FROM content_{table} WHERE map IN ({map_list})").fetchone()[0]
                  for table in ("creature", "gameobject")}
    return counts, report


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--only-maps", type=int, nargs="+", required=True)
    args = parser.parse_args()
    counts, report = promote_database(args.db, args.only_maps)
    print(f"Promoted maps {args.only_maps}: {counts}")
    for table, count in sorted(report.counts.items()):
        print(f"  {table}: {count} policy diagnostics")


if __name__ == "__main__":
    main()
