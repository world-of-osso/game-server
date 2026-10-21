#!/usr/bin/env python3
"""Extract shipped SimC gear without substituting base item identities.

Manifest generation only; it does not claim equal gear or measured DPS.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

SPECS = {
    66: "Paladin_Protection", 73: "Warrior_Protection", 105: None, 257: None,
    262: "Shaman_Elemental", 63: "Mage_Fire", 253: "Hunter_Beast_Mastery",
    1467: "Evoker_Devastation", 269: "Monk_Windwalker", 266: "Warlock_Demonology",
    103: "Druid_Feral",
}
CLASSES = {"paladin", "warrior", "druid", "priest", "shaman", "mage", "hunter",
           "evoker", "monk", "warlock", "rogue", "death_knight", "demon_hunter"}
SLOTS = {"head", "neck", "shoulders", "back", "chest", "wrists", "hands", "waist",
         "legs", "feet", "finger1", "finger2", "trinket1", "trinket2", "main_hand", "off_hand"}
CONSUMABLES = {"potion", "flask", "food", "augmentation", "temporary_enchant"}


def parse_ids(value):
    return [int(part) for part in value.split("/")] if value else []


def parse_item(slot, value):
    _, *parts = value.split(",")
    options = dict(part.split("=", 1) for part in parts)
    if "id" not in options:
        raise ValueError(f"{slot} item has no id")
    return {
        "slot": slot, "item_id": int(options.pop("id")),
        "bonus_list": parse_ids(options.pop("bonus_id", "")),
        "item_level_override": int(options.pop("ilevel")) if "ilevel" in options else None,
        "enchant": int(options.pop("enchant_id", 0)),
        "gems": parse_ids(options.pop("gem_id", "")),
        "crafted_stats": parse_ids(options.pop("crafted_stats", "")),
        "options": options,
    }


def parse_profile(text):
    profile = {"equipment": [], "actions": [], "consumables": {}}
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        key, value = line.split("=", 1)
        if key in CLASSES:
            profile.update({"class": key, "name": value.strip('"')})
        elif key in SLOTS:
            profile["equipment"].append(parse_item(key, value))
        elif key in CONSUMABLES:
            profile["consumables"][key] = value
        elif key.startswith("actions"):
            profile["actions"].append(line)
        else:
            profile[key] = value
    return profile


def parse_selected_talents(debug_text):
    # Pinned player.cpp2872 emits exact trait identities during profile decoding.
    pattern = re.compile(r"adding (\S+) talent .*? \(node=(\d+) entry=(\d+) rank=(\d+)/(\d+)\)")
    selected = {}
    for match in pattern.finditer(debug_text):
        tree, node, entry, rank, maximum = match.groups()
        if not 0 < int(rank) <= int(maximum):
            raise ValueError(f"Invalid selected talent rank: {match.group()}")
        value = {"tree": tree, "node_id": int(node), "entry_id": int(entry), "ranks": int(rank)}
        key = (int(node), int(entry))
        if key in selected and selected[key] != value:
            raise ValueError(f"Conflicting selected talent records for node {node}, entry {entry}")
        selected[key] = value
    if not selected:
        raise ValueError("No selected talent metadata in the SimC debug output")
    return [selected[key] for key in sorted(selected)]


def sample_mean(value):
    return value["mean"] if isinstance(value, dict) else value


def flatten_actions(stats, actor):
    for action in stats:
        if action["type"] == "damage":
            damage = sample_mean(action.get("actual_amount", 0))
            hits = sample_mean(action.get("num_direct_results", 0))
            hits += sample_mean(action.get("num_tick_results", 0))
            yield {"actor": actor, "spell_id": action.get("id"), "name": action["name"],
                   "count": sample_mean(action["num_executes"]), "hits": hits,
                   "damage": damage, "average_hit": damage / hits if hits else 0}
        yield from flatten_actions(action.get("children", []), actor)


def normalize_simc(report):
    player, = report["sim"]["players"]
    collected = player["collected_data"]
    actions = list(flatten_actions(player["stats"], "player"))
    for pet, stats in player.get("stats_pets", {}).items():
        actions.extend(flatten_actions(stats, pet))
    total = sum(row["damage"] for row in actions)
    for row in actions:
        row["share_pct"] = row["damage"] / total * 100 if total else 0
    return {"dps": sample_mean(collected["dps"]),
            "hps": sample_mean(collected.get("hps", 0)),
            "stats": collected["buffed_stats"], "actions": actions,
            "buffs": {buff["name"]: buff.get("uptime", 0) for buff in player["buffs"]}}


def compare_stat_sheets(reference, server):
    """Compare identically-unitized attribute/stats maps; absent data is not zero."""
    return []


def build_simc_arguments(profile, mode, output):
    if mode not in {"single", "three"}:
        raise ValueError(f"Unknown parity scenario: {mode}")
    arguments = [profile["profile"], "fight_style=Patchwerk", "desired_targets=1",
                 "max_time=300", "fixed_time=1", "vary_combat_length=0",
                 "iterations=1000", "threads=1", "optimal_raid=0", "seed=70448",
                 f'target_level={int(profile["level"]) + 3}', "report_details=1",
                 f"json={output}.{mode}.json", f"output={output}.{mode}.txt"]
    if mode == "three":
        # Stock DungeonSlice forces360s/1main target; use reproducible3-target packs.
        arguments.append("raid_events=adds,count=2,count_range=0,first=0,cooldown=30,duration=30,cooldown_stddev=0,duration_stddev=0")
    return arguments


def write_manifests(simc, output):
    output.mkdir(parents=True, exist_ok=True)
    for spec, suffix in SPECS.items():
        if suffix is None:
            result = {"spec_id": spec, "gap": "No shipped current-tier healer profile; no HPS reference."}
        else:
            tier = "MID1" if spec == 1467 else "MID2"
            source = simc / "profiles" / tier / f"{tier}_{suffix}.simc"
            payload = source.read_bytes()
            result = parse_profile(payload.decode())
            result.update({"spec_id": spec, "profile": str(source), "reference_tier": tier,
                           "profile_sha256": hashlib.sha256(payload).hexdigest()})
            if spec == 1467:
                result["reference_gap"] = "SIMC_DEVASTATION_MID2_PROFILE_ABSENT"
        result.update({"simc_revision": "76f8118cf3f7f41dad20708e8328e1f4ab2deebc",
                       "simc_build": 70391, "server_build": 70448})
        (output / f"{spec}.profile.json").write_text(json.dumps(result, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simc", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    write_manifests(args.simc, args.output)


if __name__ == "__main__":
    main()
