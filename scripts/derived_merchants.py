"""Publish explicit source-backed derived placements; never manufacture native TDB rows.

The caller owns the transaction and prepares temp.tdb_difficulty,
content_tuning_level and the health_expansion SQLite function via content_policy.
Only content templates, models, spawns and vendor rows are written here.
"""

import json
import math
from pathlib import Path
from urllib.parse import urlsplit

import retail_content

MERCHANT_FIELDS = {
    "entry",
    "name",
    "minlevel",
    "maxlevel",
    "display_id",
    "guid",
    "map",
    "area_id",
    "position_x",
    "position_y",
    "position_z",
    "orientation",
    "source_url",
    "placement_kind",
    "source_reaction",
    "derived_faction_template",
    "faction_policy",
    "vendors",
}
FACTION_POLICY_FIELDS = {
    "source_reaction",
    "derived_faction_template",
    "faction_policy",
}
FACTION_POLICY = "derived friendly-both mapping from Wowhead React A/H q2 green; not a Retail faction capture"
SPAWN_PROVENANCE_PREFIX = "derived placement; local policies: orientation=0, stationary, schema-default respawn; "
VENDOR_FIELDS = {
    "slot",
    "item",
    "buy_price",
    "buy_count",
    "maxcount",
    "incrtime",
    "extended_cost",
}
MODEL_COLUMNS = (
    "CreatureID, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild"
)
VENDOR_COLUMNS = "entry, slot, item, maxcount, incrtime, ExtendedCost"
SPAWN_COLUMNS = (
    "guid",
    "id1",
    "map",
    "areaId",
    "position_x",
    "position_y",
    "position_z",
    "orientation",
    "modelid",
    "MovementType",
    "wander_distance",
    "Comment",
)


def require_fields(record, fields, label, optional=()):
    if not isinstance(record, dict):
        raise TypeError(f"{label}: expected an object with explicit evidence")
    missing, unsupported = (
        fields - record.keys(),
        record.keys() - fields - set(optional),
    )
    if missing or unsupported:
        raise ValueError(
            f"{label}: missing fields {sorted(missing)}; unsupported fields {sorted(unsupported)}"
        )


def require_integer(value, field, minimum):
    if type(value) is not int or not minimum <= value <= 2**63 - 1:
        raise ValueError(f"{field}: expected integer >= {minimum} within SQLite range")


def validate_vendor(vendor):
    require_fields(vendor, VENDOR_FIELDS, "vendor")
    for field in VENDOR_FIELDS:
        require_integer(
            vendor[field], field, 1 if field in ("item", "buy_count") else 0
        )
    for field in ("maxcount", "incrtime", "extended_cost"):
        if vendor[field] != 0:
            raise ValueError(
                f"{field}: only unlimited stock with no restock interval or currency cost is supported"
            )


def validate_merchant(record):
    require_fields(record, MERCHANT_FIELDS, "merchant", optional=("provenance",))
    if "provenance" in record:
        if not isinstance(record["provenance"], dict):
            raise ValueError("provenance: expected an object")
        try:
            json.dumps(record["provenance"], allow_nan=False)
        except (TypeError, ValueError) as error:
            raise ValueError(f"provenance: invalid JSON metadata: {error}") from error
    for field in (
        "entry",
        "display_id",
        "guid",
        "minlevel",
        "maxlevel",
        "map",
        "area_id",
    ):
        require_integer(record[field], field, 0 if field == "map" else 1)
    if not record["minlevel"] <= record["maxlevel"] <= retail_content.MAX_LEVEL:
        raise ValueError("minlevel/maxlevel: invalid observed level range")
    if not isinstance(record["name"], str) or not record["name"].strip():
        raise ValueError("name: missing identity evidence")
    if record["placement_kind"] != "derived":
        raise ValueError("placement_kind: only derived placement is supported")
    for field in ("position_x", "position_y", "position_z", "orientation"):
        value = record[field]
        if type(value) not in (int, float) or not math.isfinite(value):
            raise ValueError(f"{field}: expected finite numeric coordinate")
    if record["orientation"] != 0:
        raise ValueError("orientation: local derived placement policy requires 0")
    source = record["source_url"]
    if (
        not isinstance(source, str)
        or urlsplit(source).scheme not in ("http", "https")
        or not urlsplit(source).netloc
    ):
        raise ValueError("source_url: missing HTTP(S) provenance")
    validate_faction_policy(record)
    vendors = record["vendors"]
    if not isinstance(vendors, list) or not vendors:
        raise ValueError("vendors: explicit nonempty vendor evidence required")
    slots, items = set(), set()
    for vendor in vendors:
        validate_vendor(vendor)
        if vendor["slot"] in slots or vendor["item"] in items:
            raise ValueError("vendors: duplicate slot or item")
        slots.add(vendor["slot"])
        items.add(vendor["item"])


def validate_faction_policy(record):
    if record["source_reaction"] != [1, 1] or any(
        type(value) is not int for value in record["source_reaction"]
    ):
        raise ValueError(
            "source_reaction: only explicitly approved Wowhead [1,1] is supported"
        )
    if (
        type(record["derived_faction_template"]) is not int
        or record["derived_faction_template"] != 35
    ):
        raise ValueError(
            "derived_faction_template: only approved mapping 35 is supported"
        )
    if record["faction_policy"] != FACTION_POLICY:
        raise ValueError(
            "faction_policy: must label approved runtime mapping as derived"
        )
    if record["entry"] != 243531 or record["name"] != "Zaralda":
        raise ValueError("entry/name: faction mapping approved only for Zaralda 243531")
    if record["source_url"] != "https://www.wowhead.com/npc=243531/zaralda":
        raise ValueError("source_url: faction mapping requires approved Zaralda source")


def validate_faction_template(conn):
    # Verify the existing DB2 row, never synthesize an unknown-faction fallback.
    columns = (
        ("Faction", "Flags", "FactionGroup", "FriendGroup", "EnemyGroup")
        + tuple(f"Enemies_{index}" for index in range(8))
        + tuple(f"Friend_{index}" for index in range(8))
    )
    row = conn.execute(
        f"SELECT {', '.join(columns)} FROM faction_template WHERE ID = 35"
    ).fetchone()
    expected = (31, 0, 0, 1, 0) + (0,) * 8 + (31,) + (0,) * 7
    if row is None or tuple(row) != expected:
        raise ValueError(
            "faction_template 35: missing or mismatched friendly-player-group/no-enemy mapping"
        )


def validate_manifest(merchants):
    entries, guids = set(), set()
    for record in merchants:
        validate_merchant(record)
        if record["entry"] in entries or record["guid"] in guids:
            raise ValueError("manifest: duplicate entry or guid")
        entries.add(record["entry"])
        guids.add(record["guid"])


def reject_json_constant(value):
    raise ValueError(f"manifest: nonfinite JSON constant {value}")


def load_manifest(path):
    """Read a required UTF-8 JSON list; [] explicitly disables derived placements.

    Structural/evidence-field validation runs here; database source validation
    runs in apply_derived_merchants before any writes.
    """
    records = json.loads(
        Path(path).read_text(encoding="utf-8"), parse_constant=reject_json_constant
    )
    if not isinstance(records, list):
        raise TypeError("manifest: expected a JSON list of merchants")
    validate_manifest(records)
    return records


def spawn_values(record):
    provenance = SPAWN_PROVENANCE_PREFIX + json.dumps(
        record, sort_keys=True, separators=(",", ":"), allow_nan=False
    )
    return (
        record["guid"],
        record["entry"],
        record["map"],
        record["area_id"],
        record["position_x"],
        record["position_y"],
        record["position_z"],
        0,
        record["display_id"],
        0,
        0,
        provenance,
    )


def validate_spawn(conn, record):
    guid = record["guid"]
    for table in ("creature", "tdb_creature"):
        if conn.execute(f"SELECT 1 FROM {table} WHERE guid = ?", (guid,)).fetchone():
            raise ValueError(f"guid {guid}: collision with native {table} spawn")
    existing = conn.execute(
        f"SELECT {', '.join(SPAWN_COLUMNS)} FROM content_creature WHERE guid = ?",
        (guid,),
    ).fetchone()
    if existing is None or tuple(existing) == spawn_values(record):
        return
    if tuple(existing)[:-1] == spawn_values(record)[:-1] and matches_prior_provenance(
        existing[-1], record
    ):
        return
    raise ValueError(f"guid {guid}: collision with unrelated or changed content spawn")


def matches_prior_provenance(comment, record):
    if not isinstance(comment, str) or not comment.startswith(SPAWN_PROVENANCE_PREFIX):
        return False
    try:
        prior = json.loads(
            comment[len(SPAWN_PROVENANCE_PREFIX) :], parse_constant=reject_json_constant
        )
    except (TypeError, ValueError):
        return False
    # Only the old approved record lacking these new fields may be upgraded.
    # Placement, identity, vendors and all prior provenance must remain exact.
    approved_prior = {
        key: value for key, value in record.items() if key not in FACTION_POLICY_FIELDS
    }
    return prior == approved_prior


def validate_identity(conn, record):
    entry = record["entry"]
    source = conn.execute(
        "SELECT name, VerifiedBuild FROM tdb_creature_template WHERE entry = ?",
        (entry,),
    ).fetchone()
    if source is None or source[0] != record["name"]:
        raise ValueError(f"entry {entry} name: missing or mismatched TDB identity")
    difficulty = conn.execute(
        "SELECT * FROM tdb_creature_template_difficulty WHERE Entry = ? AND DifficultyID = 0",
        (entry,),
    ).fetchone()
    selected = conn.execute(
        "SELECT * FROM temp.tdb_difficulty WHERE Entry = ?", (entry,)
    ).fetchone()
    if difficulty is None or selected is None or tuple(difficulty) != tuple(selected):
        raise ValueError(
            f"entry {entry}: missing or mismatched genuine difficulty-0 source context"
        )
    if conn.execute(
        "SELECT 1 FROM creature_template WHERE entry = ?", (entry,)
    ).fetchone():
        raise ValueError(
            f"entry {entry}: raw creature_template collision prevents authentic TDB transform"
        )
    existing = conn.execute(
        "SELECT name, VerifiedBuild FROM content_creature_template WHERE entry = ?",
        (entry,),
    ).fetchone()
    if existing is not None and tuple(existing) != tuple(source):
        raise ValueError(f"entry {entry}: unrelated content template identity/build")
    models = conn.execute(
        f"SELECT {MODEL_COLUMNS} FROM tdb_creature_template_model WHERE CreatureID = ? ORDER BY Idx",
        (entry,),
    ).fetchall()
    if not any(model[2] == record["display_id"] for model in models):
        raise ValueError(f"entry {entry} display_id: missing matching TDB model source")
    for model in models:
        if not conn.execute(
            "SELECT 1 FROM creature_display_info WHERE ID = ?", (model[2],)
        ).fetchone():
            raise ValueError(
                f"entry {entry} display_id {model[2]}: missing client display evidence"
            )
    existing_models = conn.execute(
        f"SELECT {MODEL_COLUMNS} FROM content_creature_template_model WHERE CreatureID = ? ORDER BY Idx",
        (entry,),
    ).fetchall()
    if any(
        tuple(model) not in [tuple(source_model) for source_model in models]
        for model in existing_models
    ):
        raise ValueError(f"entry {entry}: unrelated content model rows")
    return models


def vendor_values(entry, vendor):
    return (entry, vendor["slot"], vendor["item"], 0, 0, 0)


def validate_vendor_sources(conn, record):
    entry = record["entry"]
    for vendor in record["vendors"]:
        item = vendor["item"]
        expected = (vendor["buy_price"], vendor["buy_count"])
        for table in ("item_sparse", "content_item"):
            row = conn.execute(
                f"SELECT BuyPrice, VendorStackCount FROM {table} WHERE ID = ?", (item,)
            ).fetchone()
            if row is None:
                raise ValueError(f"item {item}: missing {table} price/bundle evidence")
            if tuple(row) != expected:
                raise ValueError(
                    f"item {item}: buy_price/buy_count do not match {table} BuyPrice/VendorStackCount"
                )
        existing = conn.execute(
            f"SELECT {VENDOR_COLUMNS} FROM content_npc_vendor WHERE entry = ? AND (item = ? OR slot = ?)",
            (entry, item, vendor["slot"]),
        ).fetchall()
        if any(tuple(row) != vendor_values(entry, vendor) for row in existing):
            raise ValueError(
                f"entry {entry} item {item}: unrelated content vendor row or slot collision"
            )


def insert_merchant(conn, record, models):
    entry = record["entry"]
    # entry has already been validated as an integer; this helper accepts a SQL query.
    retail_content.insert_tdb_templates(conn, str(entry))
    conn.execute(
        "UPDATE content_creature_template SET npcflag = npcflag | 128, minlevel = ?, maxlevel = ?, faction = ? WHERE entry = ?",
        (
            record["minlevel"],
            record["maxlevel"],
            record["derived_faction_template"],
            entry,
        ),
    )
    for model in models:
        if not conn.execute(
            "SELECT 1 FROM content_creature_template_model WHERE CreatureID = ? AND Idx = ?",
            (entry, model[1]),
        ).fetchone():
            conn.execute(
                f"INSERT INTO content_creature_template_model ({MODEL_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?)",
                tuple(model),
            )
    if not conn.execute(
        "SELECT 1 FROM content_creature WHERE guid = ?", (record["guid"],)
    ).fetchone():
        conn.execute(
            f"INSERT INTO content_creature ({', '.join(SPAWN_COLUMNS)}) VALUES ({', '.join('?' for _ in SPAWN_COLUMNS)})",
            spawn_values(record),
        )
    else:
        conn.execute(
            "UPDATE content_creature SET Comment = ? WHERE guid = ? AND Comment != ?",
            (spawn_values(record)[-1], record["guid"], spawn_values(record)[-1]),
        )
    for vendor in record["vendors"]:
        if not conn.execute(
            "SELECT 1 FROM content_npc_vendor WHERE entry = ? AND item = ? AND ExtendedCost = 0",
            (entry, vendor["item"]),
        ).fetchone():
            conn.execute(
                f"INSERT INTO content_npc_vendor ({VENDOR_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?)",
                vendor_values(entry, vendor),
            )


def apply_derived_merchants(conn, merchants):
    """Validate all records then publish in caller's active transaction; return record count.

    Does not begin, commit or catch SQL errors. Caller must roll back on failure.
    Reapplication of identical records is supported; conflicting rows are rejected.
    """
    if not conn.in_transaction:
        raise ValueError("derived merchants require an active caller transaction")
    records = list(merchants)
    validate_manifest(records)
    source_models = []
    for record in records:
        validate_faction_template(conn)
        models = validate_identity(conn, record)
        validate_vendor_sources(conn, record)
        validate_spawn(conn, record)
        source_models.append(models)
    for record, models in zip(records, source_models):
        insert_merchant(conn, record, models)
    return len(records)
