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

from economy_crafting import require_crafting_tables

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
# Explicit raw conversion exception; direct loot is still required.
RAW_CONVERSION_ITEMS = {2318}


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


def output_count_failure(effects):
    if len(effects) != 1 or effects[0]["Effect"] not in (24, 288):
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
    return None, []


def fixed_recipe_inputs(reagent):
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
    return None, sorted([item, count] for item, count in ingredients.items())


def ingredient_failure(ingredients, content, markets, output, sources):
    ingredients = dict(ingredients)
    referenced = {output, *ingredients}
    if not referenced <= content.keys():
        return "missing_content_item", []
    market_inputs = {
        item for item in ingredients if sources[item]["route"] != "vendor_supplier"
    }
    if not {output, *market_inputs} <= markets.keys():
        return "missing_supported_market", []
    unsupported = [
        item for item in ingredients if sources[item]["route"] == "unsupported"
    ]
    if unsupported:
        return "unsupported_ingredient_source", sorted(
            [item, count] for item, count in ingredients.items()
        )
    if markets[output]["max_price"] > U32_MAX:
        return "merchant_output_price_exceeds_u32", []
    return None, sorted([item, count] for item, count in ingredients.items())


def recipe_failure(effects, reagent, content, markets, output, sources):
    reason, ingredients = output_count_failure(effects)
    if reason is None:
        reason, ingredients = fixed_recipe_inputs(reagent)
    if reason is None:
        reason, ingredients = ingredient_failure(
            ingredients, content, markets, output, sources
        )
    return reason, ingredients


def published_recipe_inputs(db, candidate, record):
    """Crosscheck selected external evidence; never clear output failures."""
    from economy_recipe_evidence import crosscheck_published_inputs

    candidate["reagent_provenance"] = {
        key: record[key]
        for key in ("source_url", "sha256", "captured_utc", "provenance")
    }
    reason, ingredients = crosscheck_published_inputs(db, candidate["spell_id"], record)
    candidate["reagent_resolution_reason"] = reason
    if reason is None:
        reason = candidate["output_resolution_reason"]
    return reason, ingredients


def expand_vendor_rows(raw, entry, seen):
    """Match MerchantCatalog reference order and stock filtering before slot indexing."""
    if entry in seen:
        return []
    seen.add(entry)
    result = []
    for row in raw.get(entry, []):
        if row["item"] < 0:
            result.extend(expand_vendor_rows(raw, -row["item"], seen))
        elif (row["maxcount"] == 0) == (row["incrtime"] == 0):
            result.append(row)
    return result


def vendor_row_failure(row, goods):
    if (row["maxcount"] == 0) != (row["incrtime"] == 0):
        return "invalid_stock_restock"
    if any(
        type(row[key]) is not int or not 0 <= row[key] <= U32_MAX
        for key in ("entry", "slot", "maxcount", "incrtime")
    ):
        return "invalid_vendor_amount"
    price, count = goods["BuyPrice"], goods["VendorStackCount"]
    if type(price) is not int or not 0 <= price <= U32_MAX:
        return "invalid_buy_price"
    if type(count) is not int or not 0 < count <= U32_MAX:
        return "invalid_buy_count"
    return None


def inspect_suppliers(db, content, ingredient_ids):
    raw, suppliers, rejected = defaultdict(list), defaultdict(list), []
    rows = list(db.execute("SELECT * FROM content_npc_vendor ORDER BY entry,slot,item"))
    for row in rows:
        if row["ExtendedCost"] == 0:
            raw[row["entry"]].append(row)
        elif row["item"] in ingredient_ids:
            rejected.append(
                {
                    "item_id": row["item"],
                    "source_entry": row["entry"],
                    "database_slot": row["slot"],
                    "reason": "extended_cost",
                }
            )
    templates = {
        row["entry"]: row["npcflag"]
        for row in db.execute("SELECT entry,npcflag FROM content_creature_template")
    }
    spawned = {
        row[0]
        for row in db.execute("""
        SELECT id1 FROM content_creature creature
        WHERE NOT EXISTS (SELECT 1 FROM content_game_event_creature event WHERE event.guid=creature.guid)
        OR EXISTS (SELECT 1 FROM content_game_event_creature event
                   WHERE event.guid=creature.guid AND event.eventEntry<0)
    """)
    }
    for entry in sorted(raw):
        # Filter missing goods and truncate exactly where the runtime catalog does.
        expanded = [
            row
            for row in expand_vendor_rows(raw, entry, set())
            if row["item"] in content
        ]
        for slot, row in enumerate(expanded):
            item = row["item"]
            if item not in ingredient_ids:
                continue
            provenance = {
                "vendor_entry": entry,
                "source_entry": row["entry"],
                "database_slot": row["slot"],
                "item_id": item,
                "buy_price": content[item]["BuyPrice"],
                "extended_cost": row["ExtendedCost"],
                "buy_count": content[item]["VendorStackCount"],
                "max_count": row["maxcount"],
                "restock_secs": row["incrtime"],
            }
            reason = vendor_row_failure(row, content[item])
            if not templates.get(entry, 0) & 128:
                reason = "not_supported_vendor"
            elif entry not in spawned:
                reason = "vendor_not_spawned"
            elif slot >= 150:
                reason = "catalog_slot_limit"
            elif reason is None and row["maxcount"] > 0:
                reason = "limited_stock"
            if reason:
                rejected.append({**provenance, "reason": reason})
                continue
            goods = content[item]
            suppliers[item].append(
                {
                    **provenance,
                    "supply": {
                        "vendor_entry": entry,
                        "slot": slot,
                        "purchase_price": goods["BuyPrice"],
                        "buy_count": goods["VendorStackCount"],
                        "max_count": row["maxcount"],
                        "restock_secs": row["incrtime"],
                    },
                }
            )
    for row in rows:
        if row["ExtendedCost"] == 0 and row["item"] in ingredient_ids:
            reason = vendor_row_failure(row, content[row["item"]])
            if reason == "invalid_stock_restock":
                rejected.append(
                    {
                        "item_id": row["item"],
                        "source_entry": row["entry"],
                        "database_slot": row["slot"],
                        "reason": reason,
                    }
                )
    return (
        suppliers,
        rejected,
        {row["item"] for row in rows if row["item"] in ingredient_ids},
    )


def inspect_sources(db, content, crafted, ingredient_ids):
    direct_loot, producers = defaultdict(list), defaultdict(set)
    for table in (
        "content_creature_loot_template",
        "content_gameobject_loot_template",
        "content_item_loot_template",
    ):
        for row in db.execute(
            f"SELECT Entry,Item,Reference,Chance,QuestRequired,MinCount,MaxCount FROM {table}"
        ):
            if (
                row["Item"] in ingredient_ids
                and row["Reference"] == 0
                and row["QuestRequired"] == 0
                and row["Chance"] >= 0
                and 0 < row["MinCount"] <= row["MaxCount"] <= U32_MAX
            ):
                direct_loot[row["Item"]].append({"table": table, **dict(row)})
    for (
        row
    ) in db.execute("""SELECT DISTINCT e.EffectItemType,e.SpellID FROM spell_effect e
        JOIN skill_line_ability a ON a.Spell=e.SpellID WHERE e.Effect IN (24,157)"""):
        producers[row[0]].add(row[1])
    for row in db.execute("""
        SELECT DISTINCT c.CraftedItemID,e.SpellID FROM spell_effect e
        JOIN skill_line_ability a ON a.Spell=e.SpellID
        JOIN crafting_data c ON c.ID=e.EffectMiscValue_0
        WHERE e.Effect=288 AND e.DifficultyID=0
          AND c.Type=0 AND c.CraftedTreasureID=0 AND c.CraftedItemID>0
    """):
        producers[row[0]].add(row[1])
    suppliers, rejected, vendor_items = inspect_suppliers(db, content, ingredient_ids)
    sources = {}
    for item in sorted(ingredient_ids):
        goods = content[item]
        raw_gathered = (
            bool(direct_loot[item])
            and goods["ClassID"] == 7
            and goods["SubclassID"] in PRIMARY_SUBCLASSES
        )
        source = {
            "direct_loot": direct_loot[item],
            "crafted_spell_ids": sorted(producers[item]),
            "vendor_candidates": suppliers[item],
        }
        if raw_gathered and (item not in crafted or item in RAW_CONVERSION_ITEMS):
            source["route"] = "gathered_auction"
        elif item in crafted:
            source["route"] = "crafted_auction"
        elif suppliers[item] and not direct_loot[item]:
            source.update(route="vendor_supplier", supplier=suppliers[item][0])
        elif item in vendor_items and not direct_loot[item]:
            source.update(route="unsupported", reason="no_supported_vendor_supplier")
        else:
            source.update(
                route="market_auction", reason="not_a_proven_raw_gathered_resource"
            )
        sources[item] = source
    return sources, rejected


def modern_crafting_references(db):
    """Index genuine output references and uninterpreted modified slots."""
    require_crafting_tables(db)
    data = {row["ID"]: dict(row) for row in db.execute("SELECT * FROM crafting_data")}
    quality, slots = defaultdict(list), defaultdict(list)
    for row in db.execute(
        "SELECT CraftingDataID,ItemID FROM crafting_data_item_quality ORDER BY ItemID"
    ):
        quality[row["CraftingDataID"]].append(row["ItemID"])
    for row in db.execute("""
        SELECT s.*,r.ReagentType,r.ReagentSource,r.Flags
        FROM modified_crafting_spell_slot s
        LEFT JOIN modified_crafting_reagent_slot r ON r.ID=s.ModifiedCraftingReagentSlotID
        ORDER BY s.SpellID,s.Slot,s.ID
    """):
        slots[row["SpellID"]].append(dict(row))
    return data, quality, slots


def resolve_modern_output(effect, references):
    """Resolve only Type0 direct items; keep quality/slot evidence unresolved."""
    data, quality, slots = references
    crafting_id = effect["EffectMiscValue_0"]
    crafting = data.get(crafting_id)
    reason, output = None, None
    if crafting is None:
        reason = "missing_crafting_data"
    elif crafting["Type"] != 0:
        reason = "unresolved_crafting_data_type"
    elif crafting["CraftedTreasureID"] != 0:
        reason = "unresolved_crafted_treasure"
    elif crafting["CraftedItemID"] == 0:
        reason = "unresolved_quality_output"
    elif not 0 < crafting["CraftedItemID"] <= U32_MAX:
        reason = "invalid_crafted_item"
    else:
        output = crafting["CraftedItemID"]
    modified_slots = slots[effect["SpellID"]]
    output_reason = reason
    if modified_slots:
        reason = "unresolved_modified_reagent_requirements"
    return {
        "spell_id": effect["SpellID"],
        "effect": effect["Effect"],
        "crafting_data_id": crafting_id,
        "crafting_data": crafting,
        "quality_item_ids": quality[crafting_id],
        "output_resolution_reason": output_reason,
        "output_item_id": output,
        "modified_slots": modified_slots,
        "reason": reason,
    }


def inspect_recipes(db, markets, published_records=None):
    references = modern_crafting_references(db)
    content = {
        row["ID"]: row
        for row in db.execute(
            "SELECT ID,ClassID,SubclassID,Name,Quality,RequiredLevel,SellPrice,BuyPrice,VendorStackCount FROM content_item"
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
    professions, abilities = defaultdict(set), defaultdict(list)
    for row in db.execute(
        "SELECT DISTINCT Spell,SkillLine,SkillupSkillLineID FROM skill_line_ability ORDER BY Spell,SkillLine,SkillupSkillLineID"
    ):
        abilities[row["Spell"]].append(
            {
                "skill_line_id": row["SkillLine"],
                "skillup_skill_line_id": row["SkillupSkillLineID"],
            }
        )
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
    modern, candidates = [], defaultdict(list)
    for spell, rows in sorted(effects.items()):
        legacy = [row["EffectItemType"] for row in rows if row["Effect"] == 24]
        if legacy:
            candidates[spell].append({"output_item_id": legacy[0], "reason": None})
        for row in rows:
            if row["Effect"] != 288:
                continue
            candidate = resolve_modern_output(row, references)
            candidate["skill_line_abilities"] = abilities[spell]
            candidate["expansion"] = expansions.get(candidate["output_item_id"])
            modern.append(candidate)
            candidates[spell].append(candidate)
            if candidate["output_item_id"] is not None and spell in professions:
                crafted.add(candidate["output_item_id"])
    ingredient_ids = {
        row[f"Reagent_{index}"]
        for row in reagents.values()
        for index in range(8)
        if row[f"Reagent_{index}"] in content
    }
    if published_records is not None:
        ingredient_ids.update(
            item
            for record in published_records.values()
            for item, _ in record["items"]
            if item in content
        )
    sources, unsupported_vendor_rows = inspect_sources(
        db, content, crafted, ingredient_ids
    )
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
    for spell, spell_candidates in sorted(candidates.items()):
        for candidate in spell_candidates:
            is_modern = "crafting_data_id" in candidate
            output = candidate["output_item_id"]
            expansion = expansions.get(output)
            if spell not in professions:
                if is_modern:
                    if candidate["reason"] is None:
                        candidate["reason"] = "missing_profession_mapping"
                    unmapped.append(dict(candidate))
                continue
            reason, ingredients = candidate["reason"], []
            record = (
                published_records.get(spell) if published_records is not None else None
            )
            if is_modern and record is not None:
                reason, ingredients = published_recipe_inputs(db, candidate, record)
                if reason is None:
                    reason, _ = output_count_failure(effects[spell])
                if reason is None:
                    reason, ingredients = ingredient_failure(
                        ingredients, content, markets, output, sources
                    )
            elif reason is None:
                reason, ingredients = recipe_failure(
                    effects[spell],
                    reagents.get(spell),
                    content,
                    markets,
                    output,
                    sources,
                )
            if expansion is None or not 0 <= expansion <= I32_MAX:
                if not is_modern or candidate["reason"] is None:
                    reason = "missing_output_expansion"
            if is_modern:
                candidate["reason"] = reason
            for parent in sorted(professions[spell]):
                key = f"{expansion}:{parent}"
                coverage[key]["candidates"] += 1
                if reason:
                    coverage[key]["excluded"] += 1
                    price_details = (
                        {
                            "output_market": markets[output],
                            "vendor_price_limit": U32_MAX,
                        }
                        if reason == "merchant_output_price_exceeds_u32"
                        else {}
                    )
                    excluded.append(
                        {
                            **(candidate if is_modern else {}),
                            "spell_id": spell,
                            "output_item_id": output,
                            "expansion": expansion,
                            "parent_profession": parent,
                            "reason": reason,
                            **(
                                {"ingredients": ingredients}
                                if reason == "unsupported_ingredient_source"
                                else {}
                            ),
                            **price_details,
                        }
                    )
                    continue
                coverage[key]["eligible"] += 1
                eligible[(expansion, parent)].append(
                    {
                        "spell_id": spell,
                        "expansion": expansion,
                        "output_item_id": output,
                        "output_count": round_output_count(
                            effects[spell][0]["EffectBasePointsF"]
                        ),
                        "ingredients": ingredients,
                    }
                )
    return (
        eligible,
        content,
        sources,
        {
            "unsupported_vendor_rows": unsupported_vendor_rows,
            "modern_recipes": modern,
            "modern_skillup_coverage": modern_skillup_coverage(modern, professions),
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


def modern_skillup_coverage(recipes, professions):
    """Count discovered modern spells independently from content/market eligibility."""
    groups = defaultdict(lambda: {"spells": set(), "outputs": defaultdict(set)})
    for recipe in recipes:
        spell = recipe["spell_id"]
        for ability in recipe["skill_line_abilities"]:
            for parent in professions[spell]:
                key = f"{ability['skillup_skill_line_id']}:{parent}"
                groups[key]["spells"].add(spell)
                if recipe["output_item_id"] is not None:
                    groups[key]["outputs"][str(recipe["expansion"])].add(spell)
    return {
        key: {
            "craft_item_spells": len(group["spells"]),
            "direct_output_expansions": {
                expansion: len(spells)
                for expansion, spells in sorted(group["outputs"].items())
            },
        }
        for key, group in sorted(groups.items())
    }


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


def build_catalog(db, markets, assignments, tuning, published_records=None):
    eligible, content, sources, report = inspect_recipes(db, markets, published_records)
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
    source_ids = ingredients | {
        item
        for row in report["excluded_recipes"]
        if row["reason"] == "unsupported_ingredient_source"
        and (row["expansion"], row["parent_profession"])
        in {(expansion, parent) for _, expansion, parent in assignments}
        for item, _ in row["ingredients"]
    }
    report["ingredient_sources"] = {
        str(item): sources[item] for item in sorted(source_ids)
    }
    report["unsupported_vendor_rows"] = [
        row for row in report["unsupported_vendor_rows"] if row["item_id"] in source_ids
    ]
    gatherables = sorted(
        item for item in ingredients if sources[item]["route"] == "gathered_auction"
    )
    vendor_supplies = {
        str(item): sources[item]["supplier"]["supply"]
        for item in sorted(ingredients)
        if sources[item]["route"] == "vendor_supplier"
    }
    report["gathering_item_ids"] = gatherables
    report["non_gathering_ingredient_ids"] = sorted(ingredients - set(gatherables))
    report["secondary_crafting_item_ids"] = sorted(
        item for item in ingredients if sources[item]["route"] == "crafted_auction"
    )
    report["vendor_or_other_supply_item_ids"] = sorted(
        item
        for item in ingredients
        if sources[item]["route"] not in {"gathered_auction", "crafted_auction"}
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
        vendor_supplies=vendor_supplies,
        items=metadata,
        markets={
            str(item): markets[item] for item in sorted(referenced) if item in markets
        },
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
        "--published-reagents",
        type=Path,
        help="explicit published required-reagent snapshot; never fetched automatically",
    )
    parser.add_argument(
        "--published-reagent-cache",
        type=Path,
        help="cached HTML and acquisition metadata validating the selected snapshot",
    )
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
        if (args.published_reagents is None) != (args.published_reagent_cache is None):
            raise ValueError(
                "--published-reagents and --published-reagent-cache must be paired"
            )
        published_records = None
        if args.published_reagents is not None:
            from economy_recipe_evidence import load_published_evidence

            published_records = load_published_evidence(
                args.published_reagents, args.published_reagent_cache
            )
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
            config, report = build_catalog(
                db, markets, args.merchant, tuning, published_records
            )
        reported_market_ids = set(config["markets"]) | {
            str(row["output_item_id"])
            for row in report["excluded_recipes"]
            if row["reason"] == "merchant_output_price_exceeds_u32"
        }
        snapshot_report["market_provenance"] = {
            item: snapshot_report["market_provenance"][item]
            for item in sorted(reported_market_ids)
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
