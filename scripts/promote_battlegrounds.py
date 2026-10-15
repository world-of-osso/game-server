#!/usr/bin/env python3
"""Lead-owned, explicit-map TDB1210 battleground/arena population promotion.

No full content policy, loot, quest, economy, SmartAI or Wowhead input. Existing
non-target placements, shared templates and overlays remain unchanged. Run orb
and Silvershard summon importers afterward to add scripted runtime dependencies.
"""
import argparse
from contextlib import closing
import json
from pathlib import Path
import sqlite3

import import_instance_population as population
import retail_content as retail
import world_population


def validate_maps(conn, maps):
    maps = tuple(sorted(set(maps)))
    if not maps:
        raise ValueError("promotion requires explicit --maps")
    for map_id in maps:
        row = conn.execute("SELECT InstanceType FROM map WHERE ID=?", (map_id,)).fetchone()
        if row not in ((3,), (4,)):
            raise ValueError(f"map {map_id} is not a battleground or arena Map.db2 row")
        if not conn.execute("SELECT 1 FROM map_difficulty WHERE MapID=? AND DifficultyID=0", (map_id,)).fetchone():
            raise ValueError(f"map {map_id} has no authored difficulty 0")
    return maps


def import_creatures(conn, map_list):
    entries = population.ENTRIES
    retail.create_difficulty_table(conn, map_list=map_list, base_phase=population.BASE_PHASE)
    conn.create_function("health_expansion", 1, retail.health_expansion, deterministic=True)
    conn.execute(f"DELETE FROM content_creature_template WHERE entry IN ({entries})")
    retail.insert_tdb_templates(conn, entries, replace_legacy=True)
    for table, key in (("creature_template_model", "CreatureID"),
                       ("creature_template_addon", "entry"),
                       ("creature_equip_template", "CreatureID")):
        conn.execute(f"DELETE FROM content_{table} WHERE {key} IN ({entries})")
    conn.execute(f"INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID IN ({entries})")
    conn.execute(f"INSERT INTO content_creature_template_addon SELECT entry,{retail.ADDON_COLUMNS} FROM tdb_creature_template_addon WHERE entry IN ({entries})")
    conn.execute(f"INSERT INTO content_creature_equip_template SELECT CreatureID,ID,ItemID1,AppearanceModID1,ItemID2,AppearanceModID2,ItemID3,AppearanceModID3 FROM tdb_creature_equip_template WHERE CreatureID IN ({entries})")
    conn.execute(f"DELETE FROM content_creature WHERE map IN ({map_list})")
    conn.execute(f"""INSERT INTO content_creature (guid,id1,map,zoneId,areaId,equipment_id,position_x,position_y,
        position_z,orientation,spawntimesecs,wander_distance,currentwaypoint,MovementType,unit_flags,ScriptName,
        VerifiedBuild,modelid,spawnDifficulties)
        SELECT guid+{population.OFFSET},id,map,zoneId,areaId,equipment_id,position_x,position_y,position_z,orientation,
        spawntimesecs,wander_distance,currentwaypoint,MovementType,COALESCE(unit_flags,0),ScriptName,VerifiedBuild,
        modelid,'0' FROM temp.instance_creature WHERE id IN (SELECT entry FROM content_creature_template)""")
    population.import_creature_relationships(conn)


def import_isle_demolisher_spellclick(conn):
    source = conn.execute(
        "SELECT npc_entry,spell_id,cast_flags,user_type FROM tdb_npc_spellclick_spells WHERE npc_entry=34775"
    ).fetchall()
    if not source:
        raise ValueError("missing authored Isle Demolisher34775 spellclick")
    conn.execute("""CREATE TABLE IF NOT EXISTS content_npc_spellclick_spells
        (npc_entry INTEGER,spell_id INTEGER,cast_flags INTEGER,user_type INTEGER,
         PRIMARY KEY(npc_entry,spell_id))""")
    conn.execute("DELETE FROM content_npc_spellclick_spells WHERE npc_entry=34775")
    conn.executemany("INSERT INTO content_npc_spellclick_spells VALUES (?,?,?,?)", source)


def apply_population(conn, maps):
    maps = validate_maps(conn, maps)
    map_list = ",".join(map(str, maps))
    snapshots = world_population.protect_unrelated_dependencies(conn, map_list)
    report = population.ImportReport()
    population.select_instance_spawns(conn, map_list=map_list)
    import_creatures(conn, map_list)
    population.import_gameobjects(conn, report, map_list=map_list, rebuild_pools=False)
    if 628 in maps:
        import_isle_demolisher_spellclick(conn)
    world_population.restore_rows(conn, snapshots)
    counts = {str(m): {t: conn.execute(f"SELECT count(*) FROM content_{t} WHERE map=?", (m,)).fetchone()[0]
                       for t in ("creature", "gameobject")} for m in maps}
    return {"target_maps": list(maps), "counts": counts, "drops": report.drops}


def import_database(path, maps):
    # Unlike copy-only investigation tools, this command is intentionally usable
    # by the lead on canonical. Agents run it exclusively on diagnostic copies.
    with closing(sqlite3.connect(Path(path).resolve().as_uri() + "?mode=rw", uri=True)) as conn:
        with conn:
            conn.execute("BEGIN IMMEDIATE")
            return apply_population(conn, maps)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--maps", required=True, help="comma-separated explicit Map.db2 battleground/arena IDs")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    result = import_database(args.db, [int(m) for m in args.maps.split(",")])
    text = json.dumps(result, indent=2) + "\n"
    if args.report:
        args.report.write_text(text)
    print(text, end="")


if __name__ == "__main__":
    main()
