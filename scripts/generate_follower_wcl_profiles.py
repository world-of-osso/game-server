#!/usr/bin/env python3
"""Generate Mythic+ follower profiles from selected WCL rankings.

Each spec and key band tries its ranked characters in order and keeps the first
qualified one. Every logged item keeps its slot, enchant, gems and other bonuses,
but its item-level bonuses are replaced by the cap rank of the highest native
upgrade track its actual key reaches (GEAR_ITEMS_FROM_WCL_ILVL_FROM_BAND_TRACK);
original levels are recorded. A character qualifies when every item takes that
track and its WCL talents pass the engine decoder (granted ranks subtracted) and
strict native validation. Rejected characters are recorded with their reasons.
"""
import argparse
import hashlib
import json
import sqlite3
import statistics
import subprocess
from pathlib import Path


BANDS = {
    "2-5": (2, 5),
    "6-9": (6, 9),
    "10-14": (10, 14),
    "15-19": (15, 19),
    "20+": (20, None),
}
# MythicPlusSeason120 (Midnight Season 2) StartTimeEvent; Kings' Rest loot tree5953
# overrides only at this event.
SEASON_TIME_EVENT = 1552
# Ruby Life Pools, the ranked encounter 112521.
RUBY_LIFE_POOLS_MAP = 2521
TALENTS_SOURCE = "TALENTS_FROM_WCL"
# The Blizzard profile race WCL caches in Character.gameData, used only when the
# class follower roster has no SimC or parity race for the spec.
WCL_RACE_SOURCE = "RACE_FROM_WCL_CHARACTER"
GEAR_SOURCE = "GEAR_ITEMS_FROM_WCL_ILVL_FROM_BAND_TRACK"


def normalize(value):
    return "".join(
        character.lower()
        for character in value
        if character.isascii() and character.isalnum()
    )


def read_spec_catalog(path):
    uri = path.resolve().as_uri() + "?mode=ro"
    with sqlite3.connect(uri, uri=True) as database:
        rows = database.execute(
            "SELECT c.ID, c.Name_lang, s.ID, s.Name_lang "
            "FROM chr_classes c JOIN chr_specialization s ON s.ClassID=c.ID"
        )
        catalog = {}
        for class_id, class_name, spec_id, spec_name in rows:
            key = (normalize(class_name), normalize(spec_name))
            if key in catalog:
                raise ValueError(f"ambiguous native specialization {class_name}/{spec_name}")
            catalog[key] = (class_id, spec_id)
        return catalog


def read_races(class_profiles):
    """Native race and its provenance label per spec, from the class follower roster."""
    return {
        profile["spec_id"]: {"id": profile["race"], "source": profile["race_source"]}
        for profile in class_profiles
    }


def race_for(races, spec_id, character):
    if spec_id in races:
        return races[spec_id]
    if character.get("race"):
        return {"id": character["race"]["id"], "source": WCL_RACE_SOURCE}
    return None


def collect_candidates(selected, catalog):
    candidates = []
    for key, group in sorted(selected.items()):
        spec, band = key.rsplit("/", 1)
        low, high = BANDS[band]
        for character in group["characters"]:
            level = character["hardModeLevel"]
            identity = (normalize(character["class"]), normalize(character["spec"]))
            if identity not in catalog:
                raise ValueError(f"{key}: native class/spec {identity} is missing")
            if level < low or (high is not None and level > high):
                raise ValueError(f"{key}: actual key {level} is outside its band")
            report = character["report"]
            if not report.get("code") or not report.get("fightID"):
                raise ValueError(f"{key}: report provenance is missing")
            class_id, spec_id = catalog[identity]
            candidates.append((key, spec, band, class_id, spec_id, character))
    return candidates


def judge(candidates, races, decode, validate):
    """Rejection reasons per candidate; the decoder's purchases for qualified talents."""
    decoded = decode([
        {
            "spec_id": spec_id,
            "class_id": class_id,
            "entries": [[talent["talentID"], talent["points"]] for talent in character["talents"]],
        }
        for _, _, _, class_id, spec_id, character in candidates
    ])
    verdicts = validate([
        {
            "key": key,
            "class_id": class_id,
            "spec_id": spec_id,
            "actual_key": character["hardModeLevel"],
            "gear": character["gear"],
            "purchases": result["purchases"] or [],
        }
        for (key, _, _, class_id, spec_id, character), result in zip(candidates, decoded)
    ])
    judged = []
    for (_, _, _, _, spec_id, character), result, verdict in zip(candidates, decoded, verdicts):
        reasons = list(verdict["rejections"])
        if result["error"]:
            reasons.append(f"talents: engine decode: {result['error']}")
        if race_for(races, spec_id, character) is None:
            reasons.append("race: unavailable in SimC, parity profiles and WCL character data")
        judged.append((reasons, result["purchases"], verdict["items"]))
    return judged


def band_gear(gear, band_items):
    """Logged gear with each validated item's bonuses and level replaced; shirt, tabard
    and empty slots, which the validator skips, stay as logged."""
    pending = list(band_items)
    rebased = []
    for item in gear:
        if not pending or item["id"] != pending[0]["id"]:
            rebased.append(item)
            continue
        band_item = pending.pop(0)
        rebased.append({
            **item,
            "bonusIDs": [str(bonus) for bonus in band_item["bonus_ids"]],
            "itemLevel": band_item["item_level"],
            "sourceBonusIDs": item.get("bonusIDs", []),
            "sourceItemLevel": item.get("itemLevel"),
            "sourceNativeItemLevel": band_item["source_item_level"],
        })
    if pending:
        raise ValueError(f"validated item {pending[0]['id']} is not in the logged gear")
    return rebased


def describe(character):
    return {
        "character": character["name"],
        "server": character["server"],
        "report": character["report"],
        "actual_key": character["hardModeLevel"],
    }


def generate(source_bytes, catalog, races, decode, validate):
    selected = json.loads(source_bytes)
    candidates = collect_candidates(selected, catalog)
    judged = judge(candidates, races, decode, validate) if candidates else []
    profiles, rejected = {}, {}
    for candidate, (reasons, purchases, band_items) in zip(candidates, judged):
        key, spec, band, class_id, spec_id, character = candidate
        if reasons:
            rejected.setdefault(key, []).append({**describe(character), "reasons": reasons})
            continue
        if key in profiles:
            continue
        profiles[key] = {
            "spec": spec,
            "class_id": class_id,
            "spec_id": spec_id,
            "key_band": band,
            **describe(character),
            "race": race_for(races, spec_id, character),
            "gear": band_gear(character["gear"], band_items),
            "gear_source": GEAR_SOURCE,
            "median_item_level": statistics.median(item["item_level"] for item in band_items),
            "source_median_item_level": statistics.median(
                item["source_item_level"] for item in band_items
            ),
            "talents": character["talents"],
            "talents_source": TALENTS_SOURCE,
            "talent_purchases": purchases,
        }
    catalog_rows = sorted(
        (class_name, spec_name, *ids)
        for (class_name, spec_name), ids in catalog.items()
    )
    catalog_bytes = json.dumps(catalog_rows, separators=(",", ":")).encode()
    return {
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "native_spec_catalog": "chr_classes and chr_specialization",
        "native_spec_catalog_sha256": hashlib.sha256(catalog_bytes).hexdigest(),
        "source": "Warcraft Logs characterRankings, Midnight Season 2, Ruby Life Pools encounter 112521",
        "qualification": (
            f"{GEAR_SOURCE}: item-level bonuses replaced by the cap rank of the highest"
            f" time-event-{SEASON_TIME_EVENT} upgrade track each actual key reaches, resolved"
            f" through native ItemBonus; {TALENTS_SOURCE} decoded by the engine and validated"
            " strictly at level 90"
        ),
        "profiles": profiles,
        "rejected": rejected,
    }


def run_json(command, payload):
    result = subprocess.run(
        command, input=json.dumps(payload), capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(f"{command[0]} failed: {result.stderr.strip()}")
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("selected", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--world-db", type=Path, required=True, help="native database, opened read-only"
    )
    parser.add_argument("--db2", type=Path, required=True, help="native DB2 CSV directory")
    parser.add_argument(
        "--class-profiles", type=Path, required=True, help="class-follower-profiles.json"
    )
    parser.add_argument(
        "--decoder", type=Path, required=True, help="game-engine decode_follower_talents"
    )
    parser.add_argument(
        "--validator", type=Path, required=True, help="server-combat follower-profile-validate"
    )
    args = parser.parse_args()
    validator = [
        str(args.validator), str(args.world_db), str(args.db2),
        str(SEASON_TIME_EVENT), str(RUBY_LIFE_POOLS_MAP),
    ]
    result = generate(
        args.selected.read_bytes(),
        read_spec_catalog(args.world_db),
        read_races(json.loads(args.class_profiles.read_text())),
        lambda profiles: run_json([str(args.decoder), str(args.db2)], profiles),
        lambda candidates: run_json(validator, candidates),
    )
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
