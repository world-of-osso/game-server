"""Read-only AreaTable/UiMap census for content_policy's explicit hosted maps."""
import csv
from collections import defaultdict
from pathlib import Path


def root_area(areas, area):
    seen = set()
    while area in areas and areas[area][2]:
        if area in seen:
            raise ValueError(f"AreaTable parent cycle at {area}")
        seen.add(area)
        area = areas[area][2]
    return area if area in areas else 0


def read_zone_regions(csv_root, areas, maps):
    with (Path(csv_root) / "UiMap.csv").open(newline="", encoding="utf-8") as handle:
        ui = {int(r["ID"]): r for r in csv.DictReader(handle)
              if int(r["Type"]) == 3 and int(r["System"]) == 0}
    names = {(map_id, name): area for area, (name, map_id, parent) in areas.items() if parent == 0}
    regions = defaultdict(list)
    ui_maps = defaultdict(set)
    with (Path(csv_root) / "UiMapAssignment.csv").open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            map_id, ui_id = int(row["MapID"]), int(row["UiMapID"])
            if map_id not in maps or ui_id not in ui:
                continue
            area = root_area(areas, int(row["AreaID"]))
            if not area:
                area = names.get((map_id, ui[ui_id]["Name_lang"]), 0)
            if not area or areas[area][1] != map_id:
                continue
            ui_maps[(map_id, area)].add(ui_id)
            if int(row["WMODoodadPlacementID"]) or int(row["WMOGroupID"]):
                continue
            x0, y0, x1, y1 = (float(row[f"Region_{n}"]) for n in (0,1,3,4))
            xmin, xmax = sorted((x0, x1))
            ymin, ymax = sorted((y0, y1))
            if xmin < xmax and ymin < ymax:
                regions[map_id].append(((xmax-xmin)*(ymax-ymin), area, xmin, xmax, ymin, ymax))
    return {m: sorted(set(rows)) for m, rows in regions.items()}, ui_maps


def attribute_zone(areas, regions, map_id, zone, area, x, y):
    declared = root_area(areas, zone or area)
    if declared and areas[declared][1] == map_id:
        return declared, False
    for _, candidate, xmin, xmax, ymin, ymax in regions.get(map_id, ()):
        if xmin <= x <= xmax and ymin <= y <= ymax:
            return candidate, True
    return 0, False


def count_population(conn, areas, regions, maps, prefix, kind):
    table = f"{prefix}_{kind}"
    entry_column = "id1" if prefix == "content" and kind == "creature" else "id"
    columns = f"{entry_column},map,zoneId,areaId,position_x,position_y"
    counts, inferred, entries = defaultdict(int), defaultdict(int), defaultdict(set)
    for entry, map_id, zone, area, x, y in conn.execute(f"SELECT {columns} FROM {table}"):
        if map_id not in maps:
            continue
        zone, guessed = attribute_zone(areas, regions, map_id, zone, area, x, y)
        key = (map_id, zone)
        counts[key] += 1
        inferred[key] += int(guessed)
        entries[key].add(entry)
    return counts, inferred, entries


def count_zone_quests(conn, areas, maps, prefix):
    counts = defaultdict(set)
    for quest, sort in conn.execute(f"SELECT ID,QuestSortID FROM {prefix}_quest_template"):
        zone = root_area(areas, sort)
        if zone and areas[zone][1] in maps:
            counts[(areas[zone][1], zone)].add(quest)
    return counts


def load_relations(conn, prefix, kind, direction):
    rows = defaultdict(set)
    for entry, quest in conn.execute(f"SELECT id,quest FROM {prefix}_{kind}_quest{direction}"):
        rows[entry].add(quest)
    return rows


def zone_census(conn, maps, csv_root):
    """Raw counts, distinct quest/giver IDs and explicit coordinate-inference counts.

    No source rows are rewritten. Unattributable spawns stay in zone 0. Quest
    rank uses positive QuestSortID rolled up through AreaTable, not an inferred
    prerequisite graph or a claim that quest scripts are implemented.
    """
    maps = set(maps)
    areas = {r[0]: r[1:] for r in conn.execute("SELECT ID,AreaName_lang,ContinentID,ParentAreaID FROM area_table")}
    regions, ui_maps = read_zone_regions(csv_root, areas, maps)
    keys = {(map_id, area) for area, (_, map_id, parent) in areas.items() if map_id in maps and parent == 0}
    keys.update((m, 0) for m in maps)
    counts, quests, relations = {}, {}, {}
    for prefix in ("tdb", "content"):
        quests[prefix] = count_zone_quests(conn, areas, maps, prefix)
        for kind in ("creature", "gameobject"):
            counts[prefix, kind] = count_population(conn, areas, regions, maps, prefix, kind)
            keys.update(counts[prefix, kind][0])
            for direction in ("starter", "ender"):
                relations[prefix, kind, direction] = load_relations(conn, prefix, kind, direction)
    result = []
    for map_id, zone in keys:
        key = (map_id, zone)
        row = dict(map=map_id, zone=zone, name=areas[zone][0] if zone else "unassigned", ui_maps=sorted(ui_maps[key]))
        for prefix in ("tdb", "content"):
            row[f"{prefix}_quests"] = len(quests[prefix][key])
            for kind, label in (("creature", "creatures"), ("gameobject", "gameobjects")):
                population, inferred, entries = counts[prefix, kind]
                row[f"{prefix}_{label}"] = population[key]
                row[f"{prefix}_inferred_{label}"] = inferred[key]
            for direction, label in (("starter", "givers"), ("ender", "enders")):
                services, linked = set(), set()
                for kind in ("creature", "gameobject"):
                    for entry in counts[prefix, kind][2][key]:
                        related = relations[prefix, kind, direction].get(entry, set())
                        if related:
                            services.add((kind, entry))
                            linked.update(related)
                row[f"{prefix}_quest_{label}"] = len(services)
                row[f"{prefix}_{label}_quests"] = len(linked)
        result.append(row)
    return sorted(result, key=lambda r: (-r["tdb_quests"], r["map"], r["zone"]))
