"""Audit follower trash sources and prepare native SmartAI kit deltas, never publish.

Source precedence: Retail70448 DB2/TDB1210/TC a352b1fa, LittleWigs2777a9d,
then cached Wowhead NPC Abilities. No network access in this module.
"""

import argparse
import hashlib
import json
import re
import sqlite3
from collections import Counter
from pathlib import Path

SIMILAR_POLICY = "FOLLOWER_TRASH_SIMILAR_CREATURE_KIT"
TC = Path(
    "/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/src/server/scripts"
)
CACHE = Path("/home/osso/.cache/wowhead-cache")


def normalize(name):
    return " ".join(re.findall(r"[a-z0-9]+", name.lower()))


def read_lua_kits(root):
    """Associate explicit CAST events with registered NPC-name comment sections.

    Never interpret warning aura/damage IDs as casts. Ambiguous/unnamed sections
    remain unassigned instead of giving every mob every spell in a trash module.
    """
    kits = {}
    for path in sorted(root.glob("*/*/Trash.lua")):
        text = path.read_text()
        registration = re.search(r"mod:RegisterEnableMob\((.*?)\n\)", text, re.S)
        if not registration:
            continue
        names = {}
        for entry, name in re.findall(r"(\d+)\s*,?\s*--\s*([^\n]+)", registration[1]):
            names.setdefault(normalize(name), []).append(int(entry))
        enable = re.search(r"function mod:OnBossEnable\(\)(.*?)(?=\nend)", text, re.S)
        if not enable:
            continue
        section = None
        for line in enable[1].splitlines():
            heading = re.match(r"\s*--\s+([^-].*?)\s*$", line)
            if heading:
                section = normalize(heading[1])
            cast = re.search(
                r'self:Log\("SPELL_CAST_(?:START|SUCCESS)",\s*"[^"]+",(.*?)\)', line
            )
            if not cast or section not in names:
                continue
            spells = [int(s) for s in re.findall(r"\b\d+\b", cast[1])]
            for entry in names[section]:
                kit = kits.setdefault(entry, {"spells": [], "sources": []})
                kit["spells"] = sorted(set(kit["spells"] + spells))
                source = f"LittleWigs2777a9d:{path.relative_to(root)}"
                if source not in kit["sources"]:
                    kit["sources"].append(source)
    return kits


def read_wowhead_abilities(page):
    if page.startswith('"'):
        page = json.loads(page)
    section = re.search(
        r"new Listview\(\{template: 'spell', id: 'abilities',.*?data:\s*(.*?)\}\);",
        page,
        re.S,
    )
    if not section:
        return []
    return sorted({int(value) for value in re.findall(r'"id"\s*:\s*(\d+)', section[1])})


def infer_similar_kits(actors):
    """Single-hop, deterministic same-dungeon donors; no chains or zero-family matches."""
    donors = [
        actor
        for actor in actors
        if actor["spells"]
        and actor["classification"] not in ("none", "inferred")
        and not actor.get("excluded_reason")
    ]
    roles = {
        "mage",
        "caster",
        "arcanist",
        "sorcerer",
        "sorceress",
        "shaman",
        "healer",
        "priest",
        "warrior",
        "guard",
        "breaker",
        "lasher",
        "spider",
        "wolf",
        "hound",
    }
    for actor in actors:
        if actor["classification"] != "none" or actor.get("excluded_reason"):
            continue
        candidates = []
        for donor in donors:
            if not set(actor["maps"]) & set(donor["maps"]):
                continue
            same_name = normalize(actor["name"]) == normalize(donor["name"])
            same_model = bool(set(actor["models"]) & set(donor["models"]))
            same_family = actor["family"] != 0 and actor["family"] == donor["family"]
            words = set(normalize(actor["name"]).split()) & set(
                normalize(donor["name"]).split()
            )
            actor_words = normalize(actor["name"]).split()
            donor_words = normalize(donor["name"]).split()
            shared_prefix = (
                len(actor_words) > 1
                and len(donor_words) > 1
                and actor_words[0] == donor_words[0]
                and actor_words[0] not in {"the", "a", "an"}
            )
            same_stem = len(words) >= 2 or shared_prefix
            same_role = bool(words & roles)
            if same_name or same_model or same_family or same_stem or same_role:
                score = (
                    same_name,
                    same_model,
                    same_family,
                    same_stem,
                    same_role,
                    len(words),
                    -donor["entry"],
                )
                candidates.append((score, donor))
        if candidates:
            _, donor = max(candidates, key=lambda pair: pair[0])
            actor.update(
                classification="inferred",
                spells=donor["spells"].copy(),
                donor_entry=donor["entry"],
                policy=SIMILAR_POLICY,
                inference_basis=[
                    name
                    for name, yes in [
                        (
                            "same_name",
                            normalize(actor["name"]) == normalize(donor["name"]),
                        ),
                        (
                            "same_display",
                            bool(set(actor["models"]) & set(donor["models"])),
                        ),
                        (
                            "same_family",
                            actor["family"] != 0 and actor["family"] == donor["family"],
                        ),
                        (
                            "same_name_stem_or_role",
                            bool(
                                set(normalize(actor["name"]).split())
                                & set(normalize(donor["name"]).split())
                            ),
                        ),
                    ]
                    if yes
                ],
                sources=donor.get("sources", []).copy(),
            )


def cached_abilities(entry):
    url = f"https://www.wowhead.com/npc={entry}"
    body = CACHE / (hashlib.sha256(url.encode()).hexdigest() + ".html")
    if not body.exists():
        return None
    return dict(
        spells=read_wowhead_abilities(body.read_text()),
        url=url,
        cache=str(body),
        sha256=hashlib.sha256(body.read_bytes()).hexdigest(),
    )


def read_tc_combat_kit(path, script):
    """Read only literal Engage→event→cast→repeat lanes, never infer C++ clocks."""
    text = path.read_text()
    actor = re.search(
        r"(?:struct|class)\s+" + re.escape(script) + r"\s*:[^{]+\{(.*?)\n\};",
        text,
        re.S,
    )
    if actor is None:
        return None
    engage = re.search(r"void JustEngagedWith[^\{]*\{(.*?)\n    \}", actor[1], re.S)
    if engage is None:
        return None
    symbols = {
        name: int(value)
        for name, value in re.findall(r"\b(SPELL_\w+)\s*=\s*(\d+)", text)
    }
    initial = {
        event: int(value) * (1000 if unit == "s" else 1)
        for event, value, unit in re.findall(
            r"ScheduleEvent\((EVENT_\w+),\s*(\d+)(ms|s)\)", engage[1]
        )
    }
    lanes = re.findall(
        r"case\s+(EVENT_\w+):\s*DoCast(?:AOE|Self|Victim)\((SPELL_\w+)\);\s*_events.ScheduleEvent\(\1,\s*(\d+)(ms|s)\);",
        actor[1],
    )
    schedules = {
        str(symbols[spell]): dict(
            initial_ms=initial[event],
            repeat_ms=int(value) * (1000 if unit == "s" else 1),
        )
        for event, spell, value, unit in lanes
        if event in initial and spell in symbols
    }
    if not schedules:
        return None
    return dict(
        spells=sorted(int(spell) for spell in schedules), spell_schedules=schedules
    )


def source_script_files():
    result = {}
    for path in TC.rglob("*.cpp"):
        text = path.read_text()
        for name in re.findall(r"(?:struct|class)\s+(\w+)\s*[:{]", text):
            result[name] = str(path.relative_to(TC))
    return result


def census(world, inventory, littlewigs):
    maps = inventory["maps"]
    ids = ",".join(str(m["MapID"]) for m in maps)
    world.row_factory = sqlite3.Row
    actors = {}
    for table, columns in [
        ("creature", ["id1", "id2", "id3"]),
        ("content_creature", ["id1", "id2", "id3"]),
        ("tdb_creature", ["id"]),
    ]:
        for col in columns:
            for row in world.execute(
                f"SELECT {col} AS entry,map,COUNT(*) AS n FROM {table} WHERE map IN ({ids}) AND {col}>0 GROUP BY {col},map"
            ):
                actor = actors.setdefault(
                    row["entry"], dict(entry=row["entry"], maps=[], spawns={})
                )
                if row["map"] not in actor["maps"]:
                    actor["maps"].append(row["map"])
                actor["spawns"][f"{table}:{col}:{row['map']}"] = row["n"]
    bosses = {
        r[0]
        for r in world.execute(
            f"SELECT creature_entry FROM raid_encounter_creature WHERE map_id IN ({ids})"
        )
    }
    uncertain_encounters = set()
    for encounter in inventory["encounters"]:
        for journal_actor in encounter.get("journal_actors", []):
            uncertain_encounters.update(
                candidate["entry"]
                for candidate in journal_actor.get("source_map_model_candidates", [])
            )
        bosses.update(encounter["enabled_entries"])
        for actor in encounter["actors"]:
            if actor.get("bindings"):
                bosses.add(actor["entry"])
    lua = read_lua_kits(littlewigs)
    cpp = source_script_files()
    for actor in actors.values():
        entry = actor["entry"]
        template = world.execute(
            "SELECT entry,name,family,type,faction,npcflag,unit_flags,AIName,ScriptName FROM content_creature_template WHERE entry=?",
            (entry,),
        ).fetchone()
        if template is None:
            template = world.execute(
                "SELECT entry,name,family,type,faction,npcflag,unit_flags,AIName,ScriptName FROM creature_template WHERE entry=?",
                (entry,),
            ).fetchone()
        if template is None:
            template = world.execute(
                "SELECT entry,name,family,type,faction,npcflag,unit_flags,AIName,ScriptName FROM tdb_creature_template WHERE entry=?",
                (entry,),
            ).fetchone()
        if template is None:
            raise ValueError(f"spawned entry {entry} has no source template")
        actor.update(dict(template))
        actor["models"] = [
            r[0]
            for r in world.execute(
                "SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx",
                (entry,),
            )
        ]
        actor.update(classification="none", spells=[], sources=[])
        if entry in bosses:
            actor["excluded_reason"] = "boss_or_encounter_actor"
        elif entry in uncertain_encounters:
            actor["excluded_reason"] = "uncertain_encounter_candidate"
        elif (
            actor["type"] in (8, 10, 12, 13)
            or actor["npcflag"]
            or actor["faction"] in (35, 190)
        ):
            actor["excluded_reason"] = "noncombat_template_or_service"
        source = world.execute(
            "SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=?",
            (entry,),
        ).fetchone()
        actor["source_script"] = source["ScriptName"] if source else ""
        smart = [
            dict(r)
            for r in world.execute(
                "SELECT * FROM tdb_smart_scripts WHERE entryorguid=? AND source_type=0 ORDER BY id,link",
                (entry,),
            )
        ]
        bound = [
            r[0]
            for r in world.execute(
                'SELECT Spell FROM tdb_creature_template_spell WHERE CreatureID=? AND Spell>0 ORDER BY "Index"',
                (entry,),
            )
        ]
        auras = world.execute(
            "SELECT auras FROM tdb_creature_template_addon WHERE entry=?", (entry,)
        ).fetchone()
        actor["addon_auras"] = (
            [int(s) for s in (auras[0] or "").split()] if auras else []
        )
        if smart:
            actor["smart_events"] = smart
            actor["spells"] = sorted(
                {
                    r["action_param1"]
                    for r in smart
                    if r["action_type"] in (11, 85, 86) and r["action_param1"] > 0
                }
            )
            actor["sources"].append("TDB1210:smart_scripts")
        if bound:
            actor["spells"] = sorted(set(actor["spells"] + bound))
            actor["sources"].append("TDB1210:creature_template_spell")
        if actor["source_script"] in cpp:
            actor["sources"].append("TCa352b1fa:" + cpp[actor["source_script"]])
            tc_kit = read_tc_combat_kit(
                TC / cpp[actor["source_script"]], actor["source_script"]
            )
            if tc_kit:
                actor["spells"] = sorted(set(actor["spells"] + tc_kit["spells"]))
                actor["spell_schedules"] = tc_kit["spell_schedules"]
        if actor["addon_auras"]:
            actor["sources"].append(
                "TDB1210:creature_template_addon (passive auras only)"
            )
        if smart or bound or actor["source_script"] in cpp:
            actor["classification"] = "sourced-kit"
        elif entry in lua:
            actor.update(
                classification="Lua-only",
                spells=lua[entry]["spells"].copy(),
                sources=actor["sources"] + lua[entry]["sources"],
            )
        else:
            cached = cached_abilities(entry)
            if cached is not None:
                actor["wowhead"] = cached
                if cached["spells"]:
                    actor.update(
                        classification="Wowhead-only",
                        spells=cached["spells"],
                        sources=actor["sources"] + [cached["url"]],
                    )
        actor["maps"].sort()
    rows = sorted(actors.values(), key=lambda actor: actor["entry"])
    infer_similar_kits(rows)
    return dict(
        maps=maps,
        source_build=70448,
        tc_revision="a352b1fa",
        littlewigs_revision="2777a9d",
        policy=SIMILAR_POLICY,
        actors=rows,
        counts=dict(
            all_spawned_entries=len(rows),
            trash_entries=sum(not a.get("excluded_reason") for a in rows),
            all_classifications=dict(Counter(a["classification"] for a in rows)),
            trash_classifications=dict(
                Counter(
                    a["classification"] for a in rows if not a.get("excluded_reason")
                )
            ),
            excluded=dict(
                Counter(a["excluded_reason"] for a in rows if a.get("excluded_reason"))
            ),
        ),
    )


def write_census(report, output):
    output.mkdir(parents=True, exist_ok=True)
    (output / "census.json").write_text(json.dumps(report, indent=2) + "\n")
    lines = [
        "# Follower trash AI census — 2026-10-22",
        "",
        "Current canonical DB read-only; union legacy/content/TDB spawns, all positive entry slots.",
        "Classification describes source availability, NOT native execution certification.",
        "Boss/encounter actors and noncombat/service templates stay visible but are excluded from trash totals.",
        "",
        "```json",
        json.dumps(report["counts"], indent=2),
        "```",
        "",
        "Inference: `FOLLOWER_TRASH_SIMILAR_CREATURE_KIT`, single-hop same-dungeon donors; exact name, display, nonzero family, shared name stem, then shared role.",
        "Unfetched Wowhead pages remain explicitly unresolved; no absence claim from an unfetched page.",
        "",
        "| Entry | Name | Maps | Class | Spells | Donor | Boundary |",
        "|---|---|---|---|---|---|---|",
    ]
    for actor in report["actors"]:
        lines.append(
            f"| {actor['entry']} | {actor['name']} | {actor['maps']} | {actor['classification']} | {actor['spells']} | {actor.get('donor_entry', '')} | {actor.get('excluded_reason', '')} |"
        )
    lines.extend(["", "## Cached Wowhead citations"])
    for actor in report["actors"]:
        if actor.get("wowhead"):
            lines.append(
                f"- {actor['entry']}: {actor['wowhead']['url']} — `{actor['wowhead']['cache']}`, SHA256 `{actor['wowhead']['sha256']}`."
            )
    (output / "census.md").write_text("\n".join(lines) + "\n")


UNSOURCED_CADENCE_MS = 10000
CADENCE_POLICY = "FOLLOWER_TRASH_UNSOURCED_CADENCE_MS"


def spell_metadata(world, spell):
    # Follower205 resolves Normal1 then base0, matching native difficulty selection.
    misc = world.execute(
        "SELECT * FROM spell_misc WHERE SpellID=? AND DifficultyID IN (205,1,0) ORDER BY CASE DifficultyID WHEN 205 THEN 0 WHEN 1 THEN 1 ELSE 2 END LIMIT 1",
        (spell,),
    ).fetchone()
    if misc is None:
        return None
    cast = world.execute(
        "SELECT Base FROM spell_cast_times WHERE ID=?", (misc["CastingTimeIndex"],)
    ).fetchone()
    ranges = world.execute(
        "SELECT * FROM spell_range WHERE ID=?", (misc["RangeIndex"],)
    ).fetchone()
    cooldown = world.execute(
        "SELECT * FROM spell_cooldowns WHERE SpellID=? AND DifficultyID IN (205,1,0) ORDER BY CASE DifficultyID WHEN 205 THEN 0 WHEN 1 THEN 1 ELSE 2 END LIMIT 1",
        (spell,),
    ).fetchone()
    effects = [
        dict(row)
        for row in world.execute(
            "SELECT * FROM spell_effect WHERE SpellID=? AND DifficultyID IN (205,1,0) ORDER BY DifficultyID,EffectIndex",
            (spell,),
        )
    ]
    recovery = (
        max(cooldown["RecoveryTime"], cooldown["CategoryRecoveryTime"])
        if cooldown
        else 0
    )
    targets = {effect[f"ImplicitTarget_{i}"] for effect in effects for i in (0, 1)}
    # Explicit enemy-unit selectors need the current victim. Caster/area spells
    # retain their native implicit targeting; friendly explicit casts target self.
    target = 2 if targets & {6, 25, 77} else 1
    return dict(
        spell=spell,
        misc_id=misc["ID"],
        difficulty=misc["DifficultyID"],
        cast_time_ms=cast[0] if cast else 0,
        cast_time_index=misc["CastingTimeIndex"],
        range_index=misc["RangeIndex"],
        ranges=dict(ranges) if ranges else None,
        cooldown=dict(cooldown) if cooldown else None,
        effect_ids=[effect["ID"] for effect in effects],
        target_type=target,
        cadence_ms=recovery or UNSOURCED_CADENCE_MS,
        cadence_policy=None if recovery else CADENCE_POLICY,
    )


def prepare_projection(world, actors):
    """Stable source-key delta; authentic TDB rows are never a mutation target."""
    world.row_factory = sqlite3.Row
    schema = world.execute(
        "SELECT sql FROM sqlite_master WHERE name='tdb_smart_scripts'"
    ).fetchone()[0]
    schema = schema.replace(
        "CREATE TABLE tdb_smart_scripts",
        "CREATE TABLE IF NOT EXISTS content_smart_scripts",
        1,
    )
    statements = ["BEGIN IMMEDIATE;", schema + ";"]
    receipt = dict(
        casts=[], skipped=[], cadence_default={CADENCE_POLICY: UNSOURCED_CADENCE_MS}
    )
    for actor in actors:
        entry = actor["entry"]
        statements.append(
            f"DELETE FROM content_smart_scripts WHERE entryorguid={entry} AND source_type=0 AND comment GLOB 'FOLLOWER_TRASH_*';"
        )
        if actor.get("excluded_reason") or not actor["spells"]:
            continue
        template = world.execute(
            "SELECT AIName,ScriptName FROM content_creature_template WHERE entry=?",
            (entry,),
        ).fetchone()
        if template is None:
            receipt["skipped"].append(
                dict(entry=entry, reason="missing_runtime_template")
            )
            continue
        authored = world.execute(
            "SELECT 1 FROM tdb_smart_scripts WHERE entryorguid=? AND source_type=0 LIMIT 1",
            (entry,),
        ).fetchone()
        if (
            template["ScriptName"]
            or authored
            or template["AIName"] not in ("", "SmartAI")
        ):
            receipt["skipped"].append(dict(entry=entry, reason="existing_authored_ai"))
            continue
        admitted = []
        for spell in actor["spells"]:
            metadata = spell_metadata(world, spell)
            if metadata is None:
                receipt["skipped"].append(
                    dict(entry=entry, spell=spell, reason="missing_70448_spell_misc")
                )
                continue
            cadence = metadata["cadence_ms"]
            provenance = dict(
                classification=actor["classification"],
                sources=actor["sources"],
                donor_entry=actor.get("donor_entry"),
                policy=actor.get("policy"),
                cadence_policy=metadata["cadence_policy"],
            )
            comment = "FOLLOWER_TRASH_KIT " + json.dumps(
                provenance, sort_keys=True, separators=(",", ":")
            )
            columns = [
                "entryorguid",
                "source_type",
                "id",
                "link",
                "event_type",
                "event_chance",
                "event_param1",
                "event_param2",
                "event_param3",
                "event_param4",
                "action_type",
                "action_param1",
                "target_type",
                "comment",
            ]
            values = [
                entry,
                0,
                spell,
                0,
                0,
                100,
                cadence,
                cadence,
                cadence,
                cadence,
                11,
                spell,
                metadata["target_type"],
                comment,
            ]
            encoded = [
                world.execute("SELECT quote(?)", (value,)).fetchone()[0]
                for value in values
            ]
            statements.append(
                f"INSERT INTO content_smart_scripts ({','.join(columns)}) VALUES ({','.join(encoded)});"
            )
            admitted.append(spell)
            receipt["casts"].append({"entry": entry, **metadata, **provenance})
        if admitted:
            statements.append(
                f"UPDATE content_creature_template SET AIName='SmartAI' WHERE entry={entry} AND ScriptName='' AND AIName IN ('','SmartAI');"
            )
    statements.append("COMMIT;")
    return "\n".join(statements) + "\n", receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument(
        "--littlewigs", type=Path, default=Path("/home/osso/Repos/LittleWigs")
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--delta", type=Path, help="Write project SmartAI SQL; never apply it"
    )
    parser.add_argument(
        "--project-map", type=int, action="append", help="Limit SQL batch, not census"
    )
    args = parser.parse_args()
    with sqlite3.connect(args.world.resolve().as_uri() + "?mode=ro", uri=True) as world:
        world.execute("BEGIN")
        report = census(world, json.loads(args.inventory.read_text()), args.littlewigs)
        report["world_source"] = str(args.world.resolve())
        report["inventory_sha256"] = hashlib.sha256(
            args.inventory.read_bytes()
        ).hexdigest()
        if args.delta:
            actors = [
                actor
                for actor in report["actors"]
                if not args.project_map or set(actor["maps"]) & set(args.project_map)
            ]
            sql, receipt = prepare_projection(world, actors)
            args.delta.write_text(sql)
            args.delta.with_suffix(".receipt.json").write_text(
                json.dumps(receipt, indent=2) + "\n"
            )
    write_census(report, args.output)
    print(json.dumps(report["counts"], indent=2))


if __name__ == "__main__":
    main()
