#!/usr/bin/env python3
"""Import TDB1210 base-phase instance population into a COPY, never world.db.

python3 scripts/import_instance_population.py --db data/world.db.dungeon-population --report counts.json
Reads the pinned tdb_* reference tables already imported into the copy. No CDN,
synthetic rows, encounter scripts, raid admission or phase progression is added.
"""
import argparse
import json
import sqlite3
from pathlib import Path

import retail_content as retail

OFFSET = retail.RETAIL_GUID_OFFSET
MAPS = "SELECT map FROM tdb_instance_template"
ENTRIES = "SELECT DISTINCT id1 FROM temp.instance_creature"
# TrinityCore a352b1fa PhasingHandler::InitDbPhaseShift: ALWAYS_VISIBLE ignores phases.
BASE_PHASE = "terrainSwapMap=-1 AND (phaseUseFlags=1 OR " \
    "(phaseUseFlags=0 AND PhaseId IN (0,169) AND PhaseGroup=0))"


class ImportReport:
    def __init__(self):
        self.drops = []

    def drop(self, table, key, value, reason):
        self.drops.append(dict(table=table, key=key, value=value, reason=reason))


def select_instance_spawns(conn):
    conn.execute("CREATE TEMP TABLE retail_map AS SELECT MapID AS map_id, DifficultyID AS difficulty "
                 f"FROM map_difficulty WHERE MapID IN ({MAPS})")
    conn.execute("CREATE TEMP TABLE old_instance_guid AS SELECT guid FROM content_creature "
                 f"WHERE map IN ({MAPS})")
    for table in ("creature", "gameobject"):
        alias = ", s.id AS id1" if table == "creature" else ""
        conn.execute(f"CREATE TEMP TABLE instance_{table} AS SELECT s.*{alias} "
                     f"FROM tdb_{table} s WHERE map IN ({MAPS}) AND {BASE_PHASE} AND {retail.SPAWNS_ON_MAP}")
    conn.execute("CREATE TEMP TABLE retail_spawn AS SELECT guid AS tdb_guid, guid+? AS guid "
                 "FROM temp.instance_creature", (OFFSET,))
    conn.execute("CREATE TEMP TABLE retail_gameobject AS SELECT guid AS tdb_guid,id "
                 "FROM temp.instance_gameobject WHERE guid NOT IN "
                 "(SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry>0) OR guid IN "
                 "(SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry<0)")


def import_creatures(conn, report):
    retail.create_difficulty_table(conn, map_list=MAPS, base_phase=BASE_PHASE)
    conn.create_function("health_expansion", 1, retail.health_expansion, deterministic=True)
    conn.execute(f"CREATE TEMP TABLE kept_instance_loot AS SELECT entry,lootid,mingold,maxgold "
                 f"FROM content_creature_template WHERE entry IN ({ENTRIES}) AND lootid>={OFFSET}")
    conn.execute(f"CREATE TEMP TABLE kept_instance_difficulty_loot AS SELECT entry,DifficultyID,lootid,mingold,maxgold "
                 f"FROM content_creature_template_difficulty WHERE entry IN ({ENTRIES}) AND lootid>={OFFSET}")
    conn.execute(f"DELETE FROM content_creature_template WHERE entry IN ({ENTRIES})")
    retail.insert_tdb_templates(conn, ENTRIES, replace_legacy=True)
    # Keep every unrelated difficulty/loot row: the shared builder recreates this table.
    conn.execute(f"CREATE TEMP TABLE kept_difficulty AS SELECT * FROM content_creature_template_difficulty "
                 f"WHERE entry NOT IN ({ENTRIES})")
    retail.build_template_difficulties(conn, report)
    conn.execute("INSERT INTO content_creature_template_difficulty SELECT * FROM temp.kept_difficulty")
    for table, saved, match in (("content_creature_template", "kept_instance_loot", "t.entry=s.entry"),
                               ("content_creature_template_difficulty", "kept_instance_difficulty_loot",
                                "t.entry=s.entry AND t.DifficultyID=s.DifficultyID")):
        conn.execute(f"UPDATE {table} AS t SET lootid=s.lootid,mingold=s.mingold,maxgold=s.maxgold "
                     f"FROM temp.{saved} s WHERE {match}")
    for table, key in (("creature_template_model", "CreatureID"), ("creature_template_addon", "entry"),
                       ("creature_equip_template", "CreatureID")):
        conn.execute(f"DELETE FROM content_{table} WHERE {key} IN ({ENTRIES})")
    conn.execute(f"INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model "
                 f"WHERE CreatureID IN ({ENTRIES})")
    conn.execute(f"INSERT INTO content_creature_template_addon SELECT entry,{retail.ADDON_COLUMNS} "
                 f"FROM tdb_creature_template_addon WHERE entry IN ({ENTRIES})")
    conn.execute("INSERT INTO content_creature_equip_template SELECT CreatureID,ID,ItemID1,AppearanceModID1,"
                 f"ItemID2,AppearanceModID2,ItemID3,AppearanceModID3 FROM tdb_creature_equip_template WHERE CreatureID IN ({ENTRIES})")
    conn.execute(f"DELETE FROM content_creature WHERE map IN ({MAPS})")
    conn.execute(f"""INSERT INTO content_creature (guid,id1,map,zoneId,areaId,equipment_id,position_x,position_y,
        position_z,orientation,spawntimesecs,wander_distance,currentwaypoint,MovementType,unit_flags,ScriptName,
        VerifiedBuild,modelid,spawnDifficulties)
        SELECT guid+{OFFSET},id,map,zoneId,areaId,equipment_id,position_x,position_y,position_z,orientation,
        spawntimesecs,wander_distance,currentwaypoint,MovementType,COALESCE(unit_flags,0),ScriptName,VerifiedBuild,
        modelid,{retail.SPAWN_DIFFICULTIES} FROM temp.instance_creature
        WHERE id IN (SELECT entry FROM content_creature_template)""")


def import_creature_relationships(conn):
    affected = "SELECT guid FROM temp.old_instance_guid UNION SELECT guid FROM temp.retail_spawn"
    for table, key in (("creature_addon", "guid"), ("creature_formations", "memberGUID"), ("game_event_creature", "guid")):
        conn.execute(f"DELETE FROM content_{table} WHERE {key} IN ({affected})")
    conn.execute(f"INSERT INTO content_creature_addon SELECT r.guid,{retail.ADDON_COLUMNS} "
                 "FROM tdb_creature_addon a JOIN temp.retail_spawn r ON r.tdb_guid=a.guid "
                 "WHERE r.guid IN (SELECT guid FROM content_creature)")
    conn.execute("INSERT INTO content_creature_formations SELECT l.guid,m.guid,f.dist,f.angle,f.groupAI,f.point_1,f.point_2 "
                 "FROM tdb_creature_formations f JOIN temp.retail_spawn l ON l.tdb_guid=f.leaderGUID "
                 "JOIN temp.retail_spawn m ON m.tdb_guid=f.memberGUID")
    conn.execute("INSERT INTO content_game_event_creature SELECT e.eventEntry,r.guid FROM tdb_game_event_creature e "
                 "JOIN temp.retail_spawn r ON r.tdb_guid=e.guid")
    # Separate TDB path namespace: a real instance path collides with an AzerothCore path.
    conn.execute("CREATE TEMP TABLE instance_path AS SELECT path_id AS id FROM content_creature_addon "
                 "WHERE guid IN (SELECT guid FROM temp.retail_spawn) AND path_id<>0 UNION "
                 f"SELECT path_id FROM content_creature_template_addon WHERE entry IN ({ENTRIES}) AND path_id<>0")
    conn.execute(f"DELETE FROM content_waypoint_data WHERE id IN (SELECT id+{OFFSET} FROM temp.instance_path)")
    conn.execute(f"""INSERT INTO content_waypoint_data (id,point,position_x,position_y,position_z,orientation,delay,move_type)
        SELECT n.PathId+{OFFSET},n.NodeId,n.PositionX,n.PositionY,n.PositionZ,COALESCE(n.Orientation,0),n.Delay,p.MoveType
        FROM tdb_waypoint_path_node n JOIN tdb_waypoint_path p ON p.PathId=n.PathId
        WHERE n.PathId IN (SELECT id FROM temp.instance_path)""")
    conn.execute(f"UPDATE content_creature_addon SET path_id=path_id+{OFFSET} WHERE path_id<>0 "
                 "AND guid IN (SELECT guid FROM temp.retail_spawn)")
    conn.execute(f"UPDATE content_creature_template_addon SET path_id=path_id+{OFFSET} "
                 f"WHERE path_id<>0 AND entry IN ({ENTRIES})")


def import_gameobjects(conn, report):
    retail.build_gameobject_templates(conn, report, retain_missing_displays=True)
    conn.execute(f"DELETE FROM content_gameobject WHERE map IN ({MAPS})")
    conn.execute(f"""INSERT INTO content_gameobject (guid,id,map,zoneId,areaId,position_x,position_y,position_z,
        orientation,rotation0,rotation1,rotation2,rotation3,spawntimesecs,animprogress,state,ScriptName,VerifiedBuild,spawnDifficulties)
        SELECT g.guid+{OFFSET},g.id,g.map,g.zoneId,g.areaId,g.position_x,g.position_y,g.position_z,g.orientation,
        g.rotation0,g.rotation1,g.rotation2,g.rotation3,g.spawntimesecs,g.animprogress,g.state,g.ScriptName,g.VerifiedBuild,
        {retail.SPAWN_DIFFICULTIES} FROM tdb_gameobject g JOIN temp.retail_gameobject r ON r.tdb_guid=g.guid
        WHERE g.id IN (SELECT entry FROM content_gameobject_template)""")
    # Rebuild source-backed pools to translate the newly available spawn GUIDs too.
    retail.build_pools(conn, report)


def population_counts(conn):
    rows = []
    for (map_id,) in conn.execute(f"{MAPS} ORDER BY map"):
        row = {"map": map_id}
        for table, label in (("creature", "creatures"), ("gameobject", "gameobjects")):
            row[f"tdb_{label}"] = conn.execute(f"SELECT count(*) FROM tdb_{table} WHERE map=?", (map_id,)).fetchone()[0]
            row[f"after_{label}"] = conn.execute(f"SELECT count(*) FROM content_{table} WHERE map=?", (map_id,)).fetchone()[0]
            difficulty_match = "EXISTS (SELECT 1 FROM map_difficulty d WHERE d.MapID=s.map AND " \
                "','||replace(s.spawnDifficulties,' ','')||',' LIKE '%,'||d.DifficultyID||',%')"
            holiday = "" if table == "creature" else "AND (guid NOT IN " \
                "(SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry>0) OR guid IN " \
                "(SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry<0))"
            row[f"phase_excluded_{label}"] = conn.execute(f"SELECT count(*) FROM tdb_{table} "
                f"WHERE map=? AND NOT ({BASE_PHASE})", (map_id,)).fetchone()[0]
            row[f"difficulty_excluded_{label}"] = conn.execute(f"SELECT count(*) FROM tdb_{table} s "
                f"WHERE map=? AND {BASE_PHASE} AND NOT ({difficulty_match})", (map_id,)).fetchone()[0]
            eligible = f"map=? AND {BASE_PHASE} AND {difficulty_match} {holiday}"
            available = f"id IN (SELECT entry FROM tdb_{table}_template)"
            row[f"missing_{label}_templates"] = conn.execute(f"SELECT count(*) FROM tdb_{table} s "
                f"WHERE {eligible} AND NOT ({available})", (map_id,)).fetchone()[0]
            row[f"eligible_{label}"] = conn.execute(f"SELECT count(*) FROM tdb_{table} s "
                f"WHERE {eligible} AND {available}", (map_id,)).fetchone()[0]
        row["inactive_event_creatures"] = conn.execute("SELECT count(*) FROM content_creature c WHERE map=? "
            "AND EXISTS (SELECT 1 FROM content_game_event_creature e WHERE e.guid=c.guid) "
            "AND NOT EXISTS (SELECT 1 FROM content_game_event_creature e WHERE e.guid=c.guid AND e.eventEntry<0)",
            (map_id,)).fetchone()[0]
        row["model_bearing_gameobjects"] = conn.execute("SELECT count(*) FROM content_gameobject g "
            "JOIN content_gameobject_template t ON t.entry=g.id "
            "JOIN game_object_display_info d ON d.ID=t.displayId WHERE g.map=?", (map_id,)).fetchone()[0]
        rows.append(row)
    return rows


def apply_population(conn, report):
    """Apply inside the caller's transaction, also used by full content regeneration."""
    for name in ("retail_map", "retail_spawn", "retail_gameobject", "spawn_map_difficulty", "spawn_row", "tdb_difficulty"):
        conn.execute(f"DROP TABLE IF EXISTS temp.{name}")
    select_instance_spawns(conn)
    import_creatures(conn, report)
    import_creature_relationships(conn)
    import_gameobjects(conn, report)


def import_database(path):
    path = Path(path)
    if path.name == "world.db" or path.resolve().name == "world.db":
        raise ValueError("refusing destination named world.db; use a copy")
    if not path.is_file() or path.is_symlink() or path.stat().st_nlink != 1:
        raise ValueError("destination must be an existing, unlinked database copy")
    report = ImportReport()
    with sqlite3.connect(path) as conn:
        conn.execute("BEGIN IMMEDIATE")
        apply_population(conn, report)
        counts = population_counts(conn)
    return {"maps": counts, "drops": report.drops}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    result = import_database(args.db)
    args.report.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Imported {len(result['maps'])} instance maps; exclusions: {len(result['drops'])}; report: {args.report}")


if __name__ == "__main__":
    main()
