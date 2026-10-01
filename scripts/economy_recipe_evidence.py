"""Offline, explicit published input evidence; DB2 independently checks listed costs.

load_published_evidence returns a SpellID-keyed mapping or raises ValueError.
crosscheck_published_inputs returns (None, sorted concrete pairs) on success or
(reason, []) when unsupported. Neither API fetches pages or changes the database.
Published provenance is not native-capture proof or evidence of slot requiredness.
"""

from collections import Counter
from datetime import datetime
import hashlib
import json
from pathlib import Path

from economy_crafting import require_crafting_tables
from published_reagents import parse_published_reagents

U32_MAX = 2**32 - 1
PUBLISHED_PROVENANCE = {
    "kind": "published-spell-page",
    "capture_method": "http-get",
    "timestamp_basis": "retrieval-time",
    "native_capture": False,
}


def is_positive_u32(value):
    return type(value) is int and 0 < value <= U32_MAX


def read_json(path):
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError) as error:
        raise ValueError(f"Cannot read evidence JSON {path}: {error}") from error


def same_json(left, right):
    """JSON equality without Python's True == 1 == 1.0 coercion."""
    return json.dumps(left, sort_keys=True) == json.dumps(right, sort_keys=True)


def reparse_cached_record(record, cache):
    if not isinstance(record, dict) or not is_positive_u32(record.get("spell_id")):
        raise ValueError("Published record requires a positive concrete spell_id")
    spell = record["spell_id"]
    page_path = cache / f"spell-{spell}.html"
    metadata = read_json(cache / f"spell-{spell}.json")
    try:
        raw = page_path.read_bytes()
    except OSError as error:
        raise ValueError(
            f"Cannot read raw published HTML {page_path}: {error}"
        ) from error
    source_url = f"https://www.wowhead.com/spell={spell}"
    if (
        not isinstance(metadata, dict)
        or set(metadata) != {"source_url", "sha256", "captured_utc", "provenance"}
        or metadata["source_url"] != source_url
        or metadata["sha256"] != hashlib.sha256(raw).hexdigest()
        or not same_json(metadata["provenance"], PUBLISHED_PROVENANCE)
    ):
        raise ValueError(
            f"Published cache source/hash/provenance mismatch for spell {spell}"
        )
    timestamp = metadata["captured_utc"]
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise ValueError(
            f"Published cache requires a UTC acquisition timestamp for spell {spell}"
        )
    try:
        datetime.fromisoformat(timestamp)
        parsed = parse_published_reagents(raw.decode("utf-8"), spell)
    except (UnicodeError, ValueError) as error:
        raise ValueError(
            f"Invalid published cache for spell {spell}: {error}"
        ) from error
    expected = {**parsed, **metadata}
    if not same_json(record, expected):
        raise ValueError(
            f"Edited published snapshot differs from raw cache for spell {spell}"
        )
    return expected


def load_published_evidence(snapshot_path, cache_dir):
    """Require the explicit Effect288 snapshot and its original HTML + metadata."""
    snapshot = read_json(snapshot_path)
    if (
        not isinstance(snapshot, dict)
        or set(snapshot) != {"schema_version", "effect", "skillup_line", "records"}
        or type(snapshot["schema_version"]) is not int
        or snapshot["schema_version"] != 1
        or type(snapshot["effect"]) is not int
        or snapshot["effect"] != 288
        or not is_positive_u32(snapshot["skillup_line"])
        or not isinstance(snapshot["records"], list)
    ):
        raise ValueError(
            "Expected schema_version 1, effect 288, positive skillup_line and records list"
        )
    records = {}
    for record in snapshot["records"]:
        parsed = reparse_cached_record(record, Path(cache_dir))
        spell = parsed["spell_id"]
        if spell in records:
            raise ValueError(f"duplicate published spell ID {spell}")
        records[spell] = parsed
    return records


def concrete_inputs(record):
    pairs = record.get("items")
    if not isinstance(pairs, list) or not pairs:
        return None
    ingredients = {}
    for pair in pairs:
        if (
            not isinstance(pair, (list, tuple))
            or len(pair) != 2
            or not all(is_positive_u32(value) for value in pair)
            or pair[0] in ingredients
        ):
            return None
        ingredients[pair[0]] = pair[1]
    return ingredients


def read_fixed_inputs(db, spell_id):
    """Retain existing fixed Source/ReCraftCount restrictions, even on empty rows."""
    cursor = db.execute("SELECT * FROM spell_reagents WHERE SpellID=?", (spell_id,))
    names = [column[0] for column in cursor.description]
    rows = cursor.fetchall()
    if len(rows) > 1:
        return "ambiguous_fixed_reagents", {}
    fixed = Counter()
    if not rows:
        return None, fixed
    row = dict(zip(names, rows[0]))
    for index in range(8):
        item, count = row[f"Reagent_{index}"], row[f"ReagentCount_{index}"]
        if (
            row[f"ReagentSource_{index}"] != 0
            or row[f"ReagentReCraftCount_{index}"] != 0
        ):
            return "special_reagents", {}
        if item == 0 and count == 0:
            continue
        if not is_positive_u32(item) or not is_positive_u32(count):
            return "special_reagents", {}
        fixed[item] += count
        if fixed[item] > U32_MAX:
            return "special_reagents", {}
    return None, fixed


def read_allowed_slots(db, spell_id):
    """Map item IDs through actual slot/category/reagent-definition relations."""
    return db.execute(
        """SELECT DISTINCT i.ID, s.ID, s.Slot, s.ReagentCount, r.ReagentSource
        FROM modified_crafting_spell_slot s
        JOIN modified_crafting_reagent_slot r ON r.ID=s.ModifiedCraftingReagentSlotID
        JOIN mcr_slot_x_mcr_category x
          ON x.ModifiedCraftingReagentSlotID=r.ID
        JOIN modified_crafting_category c ON c.ID=x.ModifiedCraftingCategoryID
        JOIN modified_crafting_reagent_item m ON m.ModifiedCraftingCategoryID=c.ID
        JOIN item_sparse i ON i.ModifiedCraftingReagentItemID=m.ID
        WHERE s.SpellID=?""",
        (spell_id,),
    ).fetchall()


def crosscheck_modified_inputs(ingredients, fixed, allowed):
    slots_by_item = {}
    for item, slot_id, slot, count, source in allowed:
        slots_by_item.setdefault(item, set()).add((slot_id, slot, count, source))
    selected = set()
    for item, count in sorted(ingredients.items()):
        if item in fixed:
            continue
        candidates = slots_by_item.get(item, set())
        if not candidates:
            return "unmapped_published_reagent"
        candidates = {slot for slot in candidates if slot[2] == count}
        if not candidates:
            return "incorrect_modified_published_count"
        if any(slot[3] != 0 for slot in candidates):
            return "special_modified_reagent_source"
        if len(candidates) != 1:
            return "ambiguous_published_reagent_slots"
        slot_id, slot, _, _ = next(iter(candidates))
        # Both row identity and spell-local slot identity must be unique.
        if ("id", slot_id) in selected or ("slot", slot) in selected:
            return "ambiguous_published_reagent_slots"
        selected.update({("id", slot_id), ("slot", slot)})
    return None


def crosscheck_published_inputs(db, spell_id, record):
    """Validate listed costs only; no inferred optionality or synthetic DB2 rows.

    Fixed inputs require exact aggregated counts. Every other pair must select
    exactly one allowed initial-craft slot, at most once. Unknown Source modes
    are unsupported for selected slots; ReCraftCount is not an initial cost.
    Ambiguous assignments are unsupported rather than solved combinatorially.
    The caller must obtain records through load_published_evidence first.
    """
    if (
        not is_positive_u32(spell_id)
        or not isinstance(record, dict)
        or type(record.get("spell_id")) is not int
        or record["spell_id"] != spell_id
        or record.get("eligible") is not True
        or record.get("unresolved") != []
    ):
        return "ineligible_published_reagents", []
    ingredients = concrete_inputs(record)
    if ingredients is None:
        return "invalid_published_reagents", []
    require_crafting_tables(db)
    reason, fixed = read_fixed_inputs(db, spell_id)
    if reason:
        return reason, []
    for item, count in fixed.items():
        if item not in ingredients:
            return "missing_fixed_published_reagent", []
        if ingredients[item] != count:
            return "incorrect_fixed_published_count", []
    reason = crosscheck_modified_inputs(
        ingredients, fixed, read_allowed_slots(db, spell_id)
    )
    if reason:
        return reason, []
    return None, sorted([item, count] for item, count in ingredients.items())
