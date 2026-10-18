#!/usr/bin/env python3
"""Overlay labelled Wowhead Forever Zephras reconstruction into server-read content tables.

Requires calibrated normalized capture; never fetches pages, changes player storage,
substitutes displays, converts interactions to kills, or invents scripted completion.
Caller owns the transaction. CLI requires an existing scratch database.
"""

import argparse
import hashlib
import json
import math
import re
import sqlite3
from collections import Counter
from contextlib import closing
from pathlib import Path

import zephras_creatures

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTENT = (
    ROOT / "data/reference/wowhead/forever-zephras-20261004/zephras-content.json"
)
MAP_ID = 2991
ZONE_ID = 16593
FACTION_BASE = 90_000_000
SPAWN_BASE = 3_000_000_000
# Captured A/H reactions map to these verified Forever Faction IDs, not template IDs.
SKYBORNE_FACTIONS = (2795, 2794)
DDL = (
    """CREATE TABLE IF NOT EXISTS zephras_row_source (
        table_name TEXT NOT NULL, row_key TEXT NOT NULL, scrape TEXT NOT NULL,
        source_url TEXT NOT NULL, kind TEXT NOT NULL, source_json TEXT NOT NULL,
        PRIMARY KEY(table_name,row_key))""",
    """CREATE TABLE IF NOT EXISTS zephras_quest_meta (
        quest_id INTEGER PRIMARY KEY, reward_xp INTEGER,
        completion_blocked INTEGER NOT NULL, race_mask_playable INTEGER NOT NULL)""",
)


def load_input(path=DEFAULT_CONTENT, *, source_dir):
    content = json.loads(path.read_text())
    if content.get("map") != MAP_ID or content.get("reconstructed") is not True:
        raise ValueError(f"{path}: expected labelled map-2991 reconstruction")
    for group in ("npcs", "quests"):
        ids = [record["id"] for record in content[group]]
        if len(ids) != len(set(ids)):
            raise ValueError(f"{path}: duplicate {group} IDs")
    catalog, decoded = zephras_creatures.load_catalog(source_dir)
    decoded["normalized_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    return content, catalog, decoded


def clear_overlay(db):
    for ddl in DDL:
        db.execute(ddl)
    for table, encoded in db.execute(
        "SELECT table_name,row_key FROM zephras_row_source WHERE kind='reconstructed'"
    ).fetchall():
        keys = json.loads(encoded)
        if not re.fullmatch(
            r"(?:content_\w+|faction_template|zephras_quest_meta)", table
        ):
            raise ValueError(f"invalid provenance table {table!r}")
        if any(not re.fullmatch(r"\w+", key) for key in keys):
            raise ValueError(f"invalid provenance key {keys!r}")
        where = " AND ".join(f'"{key}"=?' for key in keys)
        db.execute(f'DELETE FROM "{table}" WHERE {where}', list(keys.values()))
    db.execute("DELETE FROM zephras_row_source WHERE kind='reconstructed'")


class OverlayWriter:
    def __init__(self, db, content):
        self.db = db
        self.scrape = content["provenance"]
        self.counts = Counter()

    def insert(self, table, values, keys, source):
        row_key = json.dumps({key: values[key] for key in keys}, sort_keys=True)
        where = " AND ".join(f'"{key}"=?' for key in keys)
        if self.db.execute(
            f'SELECT 1 FROM "{table}" WHERE {where}', [values[key] for key in keys]
        ).fetchone():
            raise ValueError(f"non-overlay collision in {table}: {row_key}")
        if self.db.execute(
            "SELECT 1 FROM zephras_row_source WHERE table_name=? AND row_key=?",
            (table, row_key),
        ).fetchone():
            raise ValueError(
                f"non-overlay collision in zephras_row_source: {table} {row_key}"
            )
        columns = ",".join(f'"{name}"' for name in values)
        placeholders = ",".join("?" for _ in values)
        self.db.execute(
            f'INSERT INTO "{table}" ({columns}) VALUES ({placeholders})',
            list(values.values()),
        )
        self.db.execute(
            "INSERT INTO zephras_row_source VALUES (?,?,?,?,?,?)",
            (
                table,
                row_key,
                self.scrape,
                source["source"],
                "reconstructed",
                json.dumps(source, sort_keys=True),
            ),
        )
        self.counts[table] += 1


def faction_row(react):
    # Missing reactions use the approved neutral-identity/friendly-to-Skyborne policy.
    pair = [1 if value is None else value for value in (react or [None, None])]
    if len(pair) != 2 or any(value not in (-1, 0, 1) for value in pair):
        raise ValueError(f"unsupported Wowhead A/H reactions: {react!r}")
    entry = FACTION_BASE + (pair[0] + 1) * 3 + pair[1] + 1
    values = {
        "ID": entry,
        "Faction": 0,
        "Flags": 0,
        "FactionGroup": 0,
        "FriendGroup": 0,
        "EnemyGroup": 0,
    }
    friends = [
        faction for faction, reaction in zip(SKYBORNE_FACTIONS, pair) if reaction == 1
    ]
    enemies = [
        faction for faction, reaction in zip(SKYBORNE_FACTIONS, pair) if reaction == -1
    ]
    values.update({f"Friend_{i}": faction for i, faction in enumerate(friends)})
    values.update({f"Enemies_{i}": faction for i, faction in enumerate(enemies)})
    return values


def npc_models(npc):
    # Captured Forever NPC pages are the appearance source; no client-table substitution.
    return [
        {"display_id": display, "probability": 1.0, "scale": 1.0}
        for display in npc["displayIds"]
    ]


def npc_template(npc, catalog, faction):
    level = npc["level"] or {}
    minimum, maximum = level.get("min"), level.get("max")
    if minimum is None or maximum is None or not 1 <= minimum <= maximum <= 255:
        return None
    record = catalog.get(npc["id"], {})
    return {
        "entry": npc["id"],
        "name": npc["name"],
        "subname": npc["title"],
        "minlevel": minimum,
        "maxlevel": maximum,
        "faction": faction,
        "npcflag": 2 if npc["starts_quests"] or npc["ends_quests"] else 0,
        "rank": record.get("classification", 0),
        "type": record.get("type", 0),
        "family": record.get("family", 0),
        # Neither normalized NPC pages nor the decoded Creature catalog carries these stats.
        "unit_class": 1,
        "HealthModifier": 1.0,
    }


def import_npc_spawns(writer, npc, report):
    if len(npc["coords"]) >= 1000 or not 0 < npc["id"] < 1_000_000:
        raise ValueError(f"NPC {npc['id']}: deterministic spawn GUID range exceeded")
    seen = set()
    for index, observation in enumerate(npc["coords"]):
        world = observation.get("world", {})
        if world.get("z_status") != "terrain" or world.get("z") is None:
            report["skipped_positions"][world.get("z_status", "unknown")] += 1
            continue
        point = tuple(world[key] for key in ("x", "y", "z"))
        if not all(math.isfinite(value) for value in point):
            raise ValueError(f"NPC {npc['id']}: non-finite position {point}")
        if point in seen:
            report["duplicate_positions"] += 1
            continue
        seen.add(point)
        values = {
            "guid": SPAWN_BASE + npc["id"] * 1000 + index,
            "id1": npc["id"],
            "map": MAP_ID,
            "zoneId": ZONE_ID,
            "position_x": point[0],
            "position_y": point[1],
            "position_z": point[2],
            "orientation": 0,
            "Comment": "Wowhead Forever reconstructed mapper observation; facing unknown",
        }
        source = {**observation, "source": observation.get("source", npc["source"])}
        writer.insert("content_creature", values, ["guid"], source)


def import_npcs(writer, content, catalog, report):
    factions = set()
    for npc in sorted(content["npcs"], key=lambda npc: npc["id"]):
        faction = faction_row(npc["react"])
        template = npc_template(npc, catalog, faction["ID"])
        if template is None:
            report["skipped_npcs"].append(npc["id"])
            report["positions_without_template"] += len(npc["coords"])
            continue
        if faction["ID"] not in factions:
            writer.insert("faction_template", faction, ["ID"], npc)
            factions.add(faction["ID"])
        source = {
            **npc,
            "stat_estimate": {
                "fields": {
                    "unit_class": template["unit_class"],
                    "HealthModifier": template["HealthModifier"],
                },
                "basis": "User-approved coarse estimate from normal-rank exp-0 quest-giver analogues; not authored NPC stats",
                "reference_ids": [713, 714, 23904],
                "health_curve": {
                    "table": "creature_classlevelstats",
                    "class": template["unit_class"],
                    "column": "basehp0",
                    "level_column": "minlevel",
                    "modifier": template["HealthModifier"],
                    "reference_base_hp": {"1": 42, "5": 102},
                },
            },
        }
        writer.insert("content_creature_template", template, ["entry"], source)
        models = npc_models(npc)
        if not models:
            report["unresolved_display_npcs"].append(npc["id"])
        for index, model in enumerate(models):
            writer.insert(
                "content_creature_template_model",
                {
                    "CreatureID": npc["id"],
                    "Idx": index,
                    "CreatureDisplayID": model["display_id"],
                    "Probability": model["probability"],
                    "DisplayScale": model["scale"],
                },
                ["CreatureID", "Idx"],
                npc,
            )
        import_npc_spawns(writer, npc, report)


def retail_reward_ids(db):
    items = {
        row[0]
        for row in db.execute(
            "SELECT s.ID FROM item_sparse s JOIN content_item c ON c.ID=s.ID"
        )
    }
    spells = {row[0] for row in db.execute("SELECT ID FROM spell_name")}
    return items, spells


def reward_fields(quest, item_ids, spell_ids, report):
    fields = {}
    rewards = quest["rewards"]
    for source, id_column, count_column, limit in (
        ("items", "RewardItem", "RewardAmount", 4),
        ("choice_items", "RewardChoiceItemID", "RewardChoiceItemQuantity", 6),
    ):
        accepted = []
        for item, count in rewards.get(source) or []:
            if item not in item_ids:
                report["dropped_rewards"].append(
                    {
                        "quest": quest["id"],
                        "kind": "item",
                        "id": item,
                        "reason": "absent from retail/runtime item tables",
                    }
                )
            elif len(accepted) >= limit:
                report["dropped_rewards"].append(
                    {
                        "quest": quest["id"],
                        "kind": "item",
                        "id": item,
                        "reason": "runtime reward slot limit",
                    }
                )
            else:
                accepted.append((item, count))
        for index, (item, count) in enumerate(accepted, 1):
            fields[f"{id_column}{index}"] = item
            fields[f"{count_column}{index}"] = count
    for spell in rewards.get("spells") or []:
        if spell not in spell_ids or "RewardSpell" in fields:
            report["dropped_rewards"].append(
                {
                    "quest": quest["id"],
                    "kind": "spell",
                    "id": spell,
                    "reason": "absent from retail spell table or runtime spell slot limit",
                }
            )
        else:
            fields["RewardSpell"] = spell
    if rewards.get("money") is not None:
        fields["RewardMoney"] = rewards["money"]
    return fields


def objective_fields(quest, report):
    fields, counts = {}, Counter()
    blocked = False
    for objective in quest["objectives"]:
        kind = (objective["type"], objective["target_kind"])
        if kind == ("kill", "npc"):
            counts["npc"] += 1
            index = counts["npc"]
            if index <= 4:
                fields[f"RequiredNpcOrGo{index}"] = objective["target_id"]
                fields[f"RequiredNpcOrGoCount{index}"] = objective["count"]
                fields[f"ObjectiveText{index}"] = objective["name"]
                continue
        elif kind == ("collect", "item"):
            counts["item"] += 1
            index = counts["item"]
            if index <= 6:
                fields[f"RequiredItemId{index}"] = objective["target_id"]
                fields[f"RequiredItemCount{index}"] = objective["count"]
                continue
        blocked = True
        report["unsupported_objectives"].append({"quest": quest["id"], **objective})
    # A counted prose-only objective is not an empty, immediately complete quest.
    if not quest["objectives"] and re.search(
        r"\(\d+\)", quest["text"].get("objectives") or ""
    ):
        blocked = True
        report["unsupported_objectives"].append(
            {
                "quest": quest["id"],
                "type": "prose-only",
                "text": quest["text"]["objectives"],
            }
        )
    return fields, blocked


def quest_fields(quest, item_ids, spell_ids, report):
    fields = {
        "ID": quest["id"],
        "LogTitle": quest["name"],
        "QuestLevel": quest["level"],
        "MinLevel": quest["reqlevel"],
        "QuestSortID": quest["category"],
        "AllowableRaces": quest["race_mask"],
        "LogDescription": quest["text"].get("objectives"),
        "QuestDescription": quest["text"].get("description"),
    }
    fields.update(reward_fields(quest, item_ids, spell_ids, report))
    objectives, blocked = objective_fields(quest, report)
    fields.update(objectives)
    return fields, blocked


def import_quest_links(writer, quest, templates, report):
    for kind in ("starter", "ender"):
        for npc in sorted(set(quest[f"{kind}_npc_ids"])):
            if npc not in templates:
                report["unresolved_quest_links"].append(
                    {"quest": quest["id"], "npc": npc, "role": kind}
                )
                continue
            writer.insert(
                f"content_creature_quest{kind}",
                {"id": npc, "quest": quest["id"]},
                ["id", "quest"],
                quest,
            )


def import_quests(writer, content, report):
    item_ids, spell_ids = retail_reward_ids(writer.db)
    templates = {
        row[0]
        for row in writer.db.execute("SELECT entry FROM content_creature_template")
    }
    captured = {quest["id"] for quest in content["quests"] if quest["page_captured"]}
    for quest in sorted(content["quests"], key=lambda quest: quest["id"]):
        if (
            not quest["page_captured"]
            or quest["level"] is None
            or quest["reqlevel"] is None
        ):
            report["skipped_quests"].append(quest["id"])
            continue
        fields, blocked = quest_fields(quest, item_ids, spell_ids, report)
        writer.insert("content_quest_template", fields, ["ID"], quest)
        previous, following = quest["series"]["prev"], quest["series"]["next"]
        addon = {"ID": quest["id"], "AllowableClasses": quest["class_mask"]}
        if len(previous) == 1 and previous[0] in captured:
            addon["PrevQuestID"] = previous[0]
        if len(following) == 1 and following[0] in captured:
            addon["NextQuestID"] = following[0]
        writer.insert("content_quest_template_addon", addon, ["ID"], quest)
        writer.insert(
            "content_quest_offer_reward",
            {"ID": quest["id"], "RewardText": quest["text"].get("completion")},
            ["ID"],
            quest,
        )
        writer.insert(
            "zephras_quest_meta",
            {
                "quest_id": quest["id"],
                "reward_xp": quest["rewards"].get("xp"),
                "completion_blocked": int(blocked),
                "race_mask_playable": 1,
            },
            ["quest_id"],
            quest,
        )
        if blocked:
            report["completion_blocked_quests"].append(quest["id"])
        import_quest_links(writer, quest, templates, report)


def apply_overlay(db, content, catalog, decoded):
    """Caller-owned transaction; retain source JSON alongside every reconstructed row."""
    clear_overlay(db)
    report = {
        "source": content["provenance"],
        "kind": "reconstructed",
        "decoded": decoded,
        "rows": {},
        "skipped_positions": Counter(),
        "duplicate_positions": 0,
        "positions_without_template": 0,
        "skipped_npcs": [],
        "skipped_quests": [],
        "unresolved_quest_links": [],
        "dropped_rewards": [],
        "unsupported_objectives": [],
        "completion_blocked_quests": [],
        "unresolved_display_npcs": [],
    }
    writer = OverlayWriter(db, content)
    import_npcs(writer, content, catalog, report)
    import_quests(writer, content, report)
    report["rows"] = dict(sorted(writer.counts.items()))
    report["skipped_positions"] = dict(sorted(report["skipped_positions"].items()))
    report["display_coverage"] = {
        "captured_npcs": len(content["npcs"]),
        "db2_npcs": sum(npc["id"] in catalog for npc in content["npcs"]),
        "wowhead_npcs": sum(bool(npc["displayIds"]) for npc in content["npcs"]),
    }
    # Full captured catalog, including NPCs omitted for unknown levels/terrain or referenced elsewhere.
    report["engine_display_ids"] = sorted(
        {display for npc in content["npcs"] for display in npc["displayIds"]}
    )
    report["spawn_display_ids"] = sorted(
        {
            row[0]
            for row in db.execute(
                "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID IN "
                "(SELECT id1 FROM content_creature WHERE map=2991)"
            )
        }
    )
    report["unsupported_objects"] = [obj["id"] for obj in content.get("objects", [])]
    report["gaps"] = [
        "NPC facing unknown (zero stored); mapper observations are not authored spawns",
        "No scripted events, phasing, loot, vendor inventories or gossip reconstructed",
        "Reputation rewards retained in provenance only; runtime grant unsupported",
        "Retained RewardSpell IDs have no native reward-spell grant handler",
        "Quest faction-side restrictions retained in provenance; native quest rules have no side field. Source race masks are unchanged",
        "Unknown creature type/classification use schema zero, not a guessed classification",
        "Client CDI rows and model assets are provisioned separately; no rendered-client proof",
    ]
    return report


def write_report(path, report):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    display_path = path.with_name("engine-needed-display-ids.txt")
    display_path.write_text(
        "".join(f"{display}\n" for display in report["engine_display_ids"])
    )
    return display_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--content", type=Path, default=DEFAULT_CONTENT)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--db", type=Path)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content, catalog, decoded = load_input(args.content, source_dir=args.source_dir)
    if args.check:
        return
    if args.db is None or args.report is None:
        parser.error("--db and --report are required for import")
    with closing(sqlite3.connect(f"file:{args.db}?mode=rw", uri=True)) as db, db:
        db.execute("BEGIN")
        report = apply_overlay(db, content, catalog, decoded)
    display_path = write_report(args.report, report)
    print(
        json.dumps(
            {
                key: report[key]
                for key in ("rows", "display_coverage", "skipped_positions")
            },
            indent=2,
        )
    )
    print(f"report: {args.report}; engine displays: {display_path}")


if __name__ == "__main__":
    main()
