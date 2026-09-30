#!/usr/bin/env python3
"""Read-only Retail/TSM economy catalog importer. No third-party dependencies."""

import argparse
from collections import Counter, defaultdict
from contextlib import closing
import csv
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation, ROUND_HALF_UP
from email.utils import parsedate_to_datetime
import json
import math
from pathlib import Path
import random
import sqlite3
import sys
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SOURCES = {
    "items": "https://public-data.tradeskillmaster.com/retail/us/region/items.csv",
    "commodities": "https://public-data.tradeskillmaster.com/retail/us/commodities.csv",
}
# Provisional integration-test tuning, NOT established economy policy.
DEFAULT_TUNING = {
    "cycle_secs": 60,
    "gatherer_listing_limit": 8,
    "gatherer_stack_max": 20,
    "gatherer_capital": 1000000,
    "customer_budget": 1000000,
    "customer_quantity": 5,
    "capital_limit": 1000000,
    "capacity": 100,
    "min_price_bps": 8000,
    "max_price_bps": 12000,
}
I32_MAX = 2**31 - 1
U32_MAX = 2**32 - 1
U64_MAX = 2**64 - 1
PRIMARY_SUBCLASSES = {5, 6, 7, 9}  # Cloth, leather, metal/stone, herbs.


def retry_delay(header, attempt):
    delay = 2**attempt
    if header:
        try:
            delay = float(header)
        except ValueError:
            delay = (
                parsedate_to_datetime(header) - datetime.now(timezone.utc)
            ).total_seconds()
    if delay > 120:
        raise ValueError("Retry-After exceeds bounded 120 second wait")
    return max(0, delay) + random.uniform(0, 0.25)


def fetch_snapshot(url, destination):
    """Four attempts; only transient GET failures retried; replace after full read."""
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "game-server-economy-catalog/1.0", "Accept": "text/csv"},
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                payload = response.read()
            destination.parent.mkdir(parents=True, exist_ok=True)
            pending = destination.with_suffix(".csv.part")
            pending.write_bytes(payload)
            pending.replace(destination)
            return
        except (urllib.error.URLError, TimeoutError) as exc:
            transient = not isinstance(exc, urllib.error.HTTPError) or exc.code in {
                408,
                429,
                500,
                502,
                503,
                504,
            }
            header = (
                exc.headers.get("Retry-After")
                if isinstance(exc, urllib.error.HTTPError)
                else None
            )
            if isinstance(exc, urllib.error.HTTPError):
                exc.close()
            if not transient or attempt == 3:
                raise
            delay = retry_delay(header, attempt)
            print(
                f"Retry {attempt + 1}/3 {url}: {exc}; wait {delay:.2f}s",
                file=sys.stderr,
            )
            time.sleep(delay)


def read_csv(path, required):
    with path.open(encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        header = reader.fieldnames or []
        missing = set(required) - set(header)
        if missing:
            raise ValueError(
                f"{path}: missing CSV columns {sorted(missing)}; observed {header}"
            )
        return header, list(reader)


def parse_decimal(value):
    number = Decimal(value)
    if not number.is_finite():
        raise ValueError("non-finite number")
    return number


def load_markets(items_path, commodities_path, tuning):
    header, items = read_csv(
        items_path, ["itemId", "marketValue", "saleRate", "soldPerDay", "updatedAt"]
    )
    commodity_header, commodities = read_csv(
        commodities_path, ["itemId", "marketValue", "updatedAt"]
    )
    prices = {}
    for row in commodities:
        item_id = int(row["itemId"])
        if item_id in prices:
            raise ValueError(f"duplicate commodity itemId {item_id}")
        prices[item_id] = row
    markets, provenance, excluded, seen = {}, {}, [], set()
    for row in items:
        item_id = int(row["itemId"])
        if item_id in seen:
            raise ValueError(f"duplicate region itemId {item_id}")
        seen.add(item_id)
        try:
            price_row = prices.get(item_id, row)
            anchor = parse_decimal(price_row["marketValue"])
            rate = parse_decimal(row["saleRate"])
            sold = parse_decimal(row["soldPerDay"])
            if (
                not 0 < item_id <= U32_MAX
                or anchor != int(anchor)
                or not 0 < anchor <= U64_MAX
            ):
                raise ValueError("invalid item ID or copper price")
            if not 0 <= rate <= 1 or sold < 0:
                raise ValueError("invalid saleRate or soldPerDay")
            anchor = int(anchor)
            maximum = (anchor * tuning["max_price_bps"] + 9999) // 10000
            if maximum > U64_MAX:
                raise ValueError("price bound exceeds u64")
            markets[item_id] = {
                "item_id": item_id,
                "anchor_price": anchor,
                "min_price": max(1, anchor * tuning["min_price_bps"] // 10000),
                "max_price": maximum,
                "popularity_bps": int(
                    (rate * 10000).quantize(Decimal(1), rounding=ROUND_HALF_UP)
                ),
            }
            provenance[str(item_id)] = {
                "price_source": "commodities" if item_id in prices else "items",
                "market_value": price_row["marketValue"],
                "price_updated_at": price_row["updatedAt"],
                "popularity_source": "items",
                "sale_rate": row["saleRate"],
                "sold_per_day": row["soldPerDay"],
                "popularity_updated_at": row["updatedAt"],
            }
        except (ValueError, InvalidOperation, TypeError) as exc:
            excluded.append({"item_id": item_id, "reason": str(exc)})
    return markets, {
        "snapshots": {
            "items": {
                "path": str(items_path),
                "header": header,
                "rows": len(items),
                "updated_at": sorted({row["updatedAt"] for row in items}),
            },
            "commodities": {
                "path": str(commodities_path),
                "header": commodity_header,
                "rows": len(commodities),
                "updated_at": sorted({row["updatedAt"] for row in commodities}),
            },
        },
        "market_provenance": provenance,
        "excluded_market_rows": excluded,
        "commodities_without_popularity": sorted(set(prices) - seen),
    }


def load_tuning(path):
    tuning = dict(DEFAULT_TUNING)
    if path:
        overrides = json.loads(path.read_text())
        if not isinstance(overrides, dict) or set(overrides) - set(tuning):
            raise ValueError("tuning must be an object containing only documented keys")
        tuning.update(overrides)
    for key, value in tuning.items():
        maximum = (
            U64_MAX
            if key
            in {"cycle_secs", "customer_budget", "gatherer_capital", "capital_limit"}
            else U32_MAX
        )
        if type(value) is not int or not 0 < value <= maximum:
            raise ValueError(f"{key} must be a positive integer <= {maximum}")
    if not tuning["min_price_bps"] <= 10000 <= tuning["max_price_bps"]:
        raise ValueError("price bounds must bracket 10000 bps")
    return tuning


def round_output_count(points):
    """Match profession.rs: raw base points rounded, then clamped to at least one."""
    return math.floor(max(1, points) + 0.5)


def recipe_failure(effects, reagent, content, markets, output):
    if len(effects) != 1 or effects[0]["Effect"] != 24:
        return "multiple_or_special_effects", []
    effect = effects[0]
    points = effect["EffectBasePointsF"]
    varying = [
        "Variance",
        "Coefficient",
        "EffectRealPointsPerLevel",
        "EffectPointsPerResource",
        "ResourceCoefficient",
        "ScalingClass",
    ]
    if (
        points is None
        or not math.isfinite(points)
        or round_output_count(points) > U32_MAX
        or any(effect[k] != 0 for k in varying)
        or effect["GroupSizeBasePointsCoefficient"] != 1
    ):
        return "non_deterministic_output", []
    if reagent is None:
        return "missing_reagents", []
    ingredients = Counter()
    for index in range(8):
        item, count = reagent[f"Reagent_{index}"], reagent[f"ReagentCount_{index}"]
        if reagent[f"ReagentSource_{index}"] or reagent[f"ReagentReCraftCount_{index}"]:
            return "special_reagents", []
        if item == 0 and count == 0:
            continue
        if (
            item is None
            or count is None
            or not 0 < item <= U32_MAX
            or not 0 < count <= U32_MAX
        ):
            return "special_reagents", []
        ingredients[item] += count
        if ingredients[item] > U32_MAX:
            return "special_reagents", []
    if not ingredients:
        return "missing_reagents", []
    referenced = {output, *ingredients}
    if not referenced <= content.keys():
        return "missing_content_item", []
    if not referenced <= markets.keys():
        return "missing_supported_market", []
    return None, sorted([item, count] for item, count in ingredients.items())


def inspect_recipes(db, markets):
    content = {
        row["ID"]: row
        for row in db.execute(
            "SELECT ID,ClassID,SubclassID,Name,Quality,RequiredLevel,SellPrice FROM content_item"
        )
    }
    expansions = {
        row["ID"]: row["ExpansionID"]
        for row in db.execute("SELECT ID,ExpansionID FROM item_sparse")
    }
    parents = {
        row["ID"]: row["ParentSkillLineID"] or row["ID"]
        for row in db.execute("SELECT ID,ParentSkillLineID FROM skill_line")
    }
    professions = defaultdict(set)
    for row in db.execute("SELECT Spell,SkillLine FROM skill_line_ability"):
        if row["SkillLine"] in parents:
            professions[row["Spell"]].add(parents[row["SkillLine"]])
    effects = defaultdict(list)
    for row in db.execute(
        "SELECT * FROM spell_effect WHERE DifficultyID=0 AND Effect!=0"
    ):
        effects[row["SpellID"]].append(row)
    reagents = {
        row["SpellID"]: row for row in db.execute("SELECT * FROM spell_reagents")
    }
    crafted = {
        row[0]
        for row in db.execute("""
        SELECT DISTINCT e.EffectItemType FROM spell_effect e
        JOIN skill_line_ability a ON a.Spell=e.SpellID WHERE e.Effect IN (24,157)
    """)
    }
    eligible, excluded, coverage = defaultdict(list), [], defaultdict(Counter)
    output_spells, mapped_spells, unmapped = defaultdict(set), defaultdict(set), []
    for spell, rows in sorted(effects.items()):
        for output in sorted(
            {row["EffectItemType"] for row in rows if row["Effect"] == 24}
        ):
            expansion = expansions.get(output)
            output_spells[str(expansion)].add(spell)
            if spell in professions:
                mapped_spells[str(expansion)].add(spell)
            else:
                unmapped.append(
                    {
                        "spell_id": spell,
                        "output_item_id": output,
                        "expansion": expansion,
                    }
                )
    for spell, rows in sorted(effects.items()):
        outputs = [row["EffectItemType"] for row in rows if row["Effect"] == 24]
        if not outputs or spell not in professions:
            continue
        output = outputs[0]
        expansion = expansions.get(output)
        reason, ingredients = recipe_failure(
            rows, reagents.get(spell), content, markets, output
        )
        if expansion is None or not 0 <= expansion <= I32_MAX:
            reason = "missing_output_expansion"
        for parent in sorted(professions[spell]):
            key = f"{expansion}:{parent}"
            coverage[key]["candidates"] += 1
            if reason:
                coverage[key]["excluded"] += 1
                excluded.append(
                    {
                        "spell_id": spell,
                        "output_item_id": output,
                        "expansion": expansion,
                        "parent_profession": parent,
                        "reason": reason,
                    }
                )
                continue
            coverage[key]["eligible"] += 1
            eligible[(expansion, parent)].append(
                {
                    "spell_id": spell,
                    "expansion": expansion,
                    "output_item_id": output,
                    "output_count": round_output_count(rows[0]["EffectBasePointsF"]),
                    "ingredients": ingredients,
                }
            )
    return (
        eligible,
        content,
        crafted,
        {
            "unmapped_output_spells": unmapped,
            "output_spell_coverage": {
                key: {
                    "create_item_spells": len(spells),
                    "profession_mapped": len(mapped_spells[key]),
                    "missing_profession_mapping": len(spells - mapped_spells[key]),
                }
                for key, spells in sorted(output_spells.items())
            },
            "excluded_recipes": excluded,
            "coverage": {
                key: {
                    "candidates": value["candidates"],
                    "eligible": value["eligible"],
                    "excluded": value["excluded"],
                }
                for key, value in sorted(coverage.items())
            },
        },
    )


def inspect_assignment(db, assignment):
    entry, expansion, parent = assignment
    template = db.execute(
        "SELECT name,npcflag FROM content_creature_template WHERE entry=?", (entry,)
    ).fetchone()
    spawns = db.execute(
        "SELECT COUNT(*) FROM content_creature WHERE id1=? OR id2=? OR id3=?",
        (entry, entry, entry),
    ).fetchone()[0]
    profession = db.execute(
        "SELECT ParentSkillLineID FROM skill_line WHERE ID=?", (parent,)
    ).fetchone()
    error = None
    if not template or not template["npcflag"] & 128:
        error = "merchant_not_vendor"
    elif not spawns:
        error = "merchant_not_spawned"
    elif not profession or profession[0] != 0:
        error = "not_parent_profession"
    return {
        "entry": entry,
        "expansion": expansion,
        "parent_profession": parent,
        "name": template["name"] if template else None,
        "spawn_count": spawns,
        "error": error,
    }


def build_catalog(db, markets, assignments, tuning):
    eligible, content, crafted, report = inspect_recipes(db, markets)
    report["assignments"] = [
        inspect_assignment(db, assignment) for assignment in assignments
    ]
    for assignment in report["assignments"]:
        count = len(
            eligible[(assignment["expansion"], assignment["parent_profession"])]
        )
        assignment["eligible_recipe_count"] = count
        assignment["warning"] = "no_supported_recipes" if count == 0 else None
    merchants, referenced, ingredients = [], set(), set()
    for entry, expansion, parent in assignments:
        recipes = eligible[(expansion, parent)]
        merchants.append(
            {
                "entry": entry,
                "expansion": expansion,
                "capital_limit": tuning["capital_limit"],
                "capacity": tuning["capacity"],
                "recipes": recipes,
            }
        )
        for recipe in recipes:
            referenced.add(recipe["output_item_id"])
            for item, _ in recipe["ingredients"]:
                referenced.add(item)
                ingredients.add(item)
    gatherables = sorted(
        item
        for item in ingredients
        if item not in crafted
        and content[item]["ClassID"] == 7
        and content[item]["SubclassID"] in PRIMARY_SUBCLASSES
    )
    report["gathering_item_ids"] = gatherables
    report["non_gathering_ingredient_ids"] = sorted(ingredients - set(gatherables))
    report["secondary_crafting_item_ids"] = sorted(ingredients & crafted)
    report["vendor_or_other_supply_item_ids"] = sorted(
        ingredients - crafted - set(gatherables)
    )
    config_keys = [
        "cycle_secs",
        "gatherer_listing_limit",
        "gatherer_stack_max",
        "gatherer_capital",
        "customer_budget",
        "customer_quantity",
    ]
    config = {key: tuning[key] for key in config_keys}
    metadata = {}
    for item in sorted(referenced):
        row = content[item]
        if not isinstance(row["Name"], str) or not row["Name"].strip():
            raise ValueError(f"content_item {item}: missing name")
        bounds = {"Quality": 255, "RequiredLevel": 65535, "SellPrice": U32_MAX}
        for key, maximum in bounds.items():
            if type(row[key]) is not int or not 0 <= row[key] <= maximum:
                raise ValueError(
                    f"content_item {item}: invalid {key} for runtime metadata"
                )
        metadata[str(item)] = {
            "name": row["Name"],
            "quality": row["Quality"],
            "required_level": row["RequiredLevel"],
            "vendor_sell_price": row["SellPrice"],
        }
    config.update(
        gatherables=gatherables,
        items=metadata,
        markets={str(item): markets[item] for item in sorted(referenced)},
        merchants=merchants,
    )
    return config, report


def parse_assignment(value):
    try:
        parts = tuple(int(part) for part in value.split(":"))
        if (
            len(parts) != 3
            or any(not 0 <= part <= U32_MAX for part in parts)
            or not parts[0]
            or not parts[2]
            or parts[1] > I32_MAX
        ):
            raise ValueError()
        return parts
    except ValueError as exc:
        raise argparse.ArgumentTypeError(
            "merchant must be entry:expansion:parentProfession; Midnight expansion=11"
        ) from exc


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=ROOT / "data/world.db")
    parser.add_argument("--snapshot-dir", type=Path, default=ROOT / "data/economy")
    parser.add_argument(
        "--fetch",
        action="store_true",
        help="fetch public US Retail snapshots without authentication",
    )
    parser.add_argument("--items", type=Path)
    parser.add_argument("--commodities", type=Path)
    parser.add_argument(
        "--merchant", action="append", required=True, type=parse_assignment
    )
    parser.add_argument(
        "--tuning", type=Path, help="JSON overrides of provisional numeric tuning"
    )
    parser.add_argument(
        "--output", type=Path, default=ROOT / "data/economy/config.json"
    )
    parser.add_argument(
        "--report", type=Path, default=ROOT / "data/economy/report.json"
    )
    args = parser.parse_args()
    try:
        if len({assignment[0] for assignment in args.merchant}) != len(args.merchant):
            raise ValueError(
                "duplicate merchant entry; one explicit assignment per merchant"
            )
        tuning = load_tuning(args.tuning)
        paths = {
            "items": args.items or args.snapshot_dir / "items.csv",
            "commodities": args.commodities or args.snapshot_dir / "commodities.csv",
        }
        if args.fetch:
            for name, path in paths.items():
                if Path("/tmp") in path.resolve().parents:
                    raise ValueError(
                        "snapshots must be persistent; do not download into /tmp"
                    )
                fetch_snapshot(SOURCES[name], path)
        markets, snapshot_report = load_markets(
            paths["items"], paths["commodities"], tuning
        )
        with closing(
            sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True)
        ) as db:
            db.row_factory = sqlite3.Row
            config, report = build_catalog(db, markets, args.merchant, tuning)
        snapshot_report["market_provenance"] = {
            item: snapshot_report["market_provenance"][item]
            for item in config["markets"]
        }
        report.update(snapshot_report)
        report["tuning"] = tuning
        report["database"] = str(args.db.resolve())
        write_json(args.report, report)
        failures = [
            assignment for assignment in report["assignments"] if assignment["error"]
        ]
        if failures:
            raise ValueError(
                f"invalid merchant assignments: {failures}; see {args.report}; config not written"
            )
        for assignment in report["assignments"]:
            if assignment["warning"]:
                print(
                    f"economy catalog: merchant {assignment['entry']}: {assignment['warning']}; "
                    "no production recipes configured",
                    file=sys.stderr,
                )
        write_json(args.output, config)
        print(
            f"Wrote {args.output}: {len(config['markets'])} markets, {len(config['gatherables'])} gatherables, "
            f"{sum(len(m['recipes']) for m in config['merchants'])} recipes; report {args.report}"
        )
        return 0
    except (OSError, ValueError, sqlite3.Error, InvalidOperation) as exc:
        print(f"economy catalog: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
