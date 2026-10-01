"""Explicit offline item-page skinning evidence, never runtime loot or native proof.

Select a cache directory containing item-<ID>.json acquisition records paired with
item-<ID>.html. Unselected HTML is ignored. Each record requires source_url,
sha256, captured_utc and PUBLISHED_PROVENANCE. No fetch or database writes occur.
Only the exact NPC skinned-from Listview's raw JSON observations prove a source;
this does not prove that those NPCs exist in the local world or implement Skinning.
"""

from datetime import datetime
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re

U32_MAX = 2**32 - 1
PUBLISHED_PROVENANCE = {
    "kind": "published-item-skinning-page",
    "capture_method": "http-get",
    "timestamp_basis": "retrieval-time",
    "native_capture": False,
}
# Skip JS strings/comments while locating actual Listview declarations. Never run JS.
JS_DECLARATIONS = re.compile(
    r"(?P<skip>'(?:\\.|[^'\\])*'|\"(?:\\.|[^\"\\])*\"|`(?:\\.|[^`\\])*`|//[^\n]*|/\*.*?\*/)"
    r"|(?P<listview>\bnew\s+Listview\s*\(\s*\{)",
    re.DOTALL,
)
SKIN_HEADER = re.compile(
    r"\A\s*template\s*:\s*(['\"])npc\1\s*,\s*id\s*:\s*(['\"])skinned-from\2\s*,"
)
SKIN_ID = re.compile(r"(?:\A|[,\n])\s*id\s*:\s*(['\"])skinned-from\1\s*[,\n]")


def positive_integer(value, maximum=U32_MAX):
    return type(value) is int and 0 < value <= maximum


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key {key}")
        result[key] = value
    return result


def reject_nonfinite(value):
    raise ValueError(f"Non-JSON numeric constant {value}")


DECODER = json.JSONDecoder(
    object_pairs_hook=unique_object, parse_constant=reject_nonfinite
)


class PageScripts(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.scripts = []
        self.current = None
        self.identity = None

    def handle_starttag(self, tag, attrs):
        if tag != "script":
            return
        attributes = dict(attrs)
        self.current = (attributes, [])
        if attributes.get("id") == "data.page.info":
            if (
                self.identity is not None
                or attributes.get("type") != "application/json"
                or len(attributes) != len(attrs)
            ):
                raise ValueError("Duplicate or non-JSON item page identity")
            self.identity = self.current

    def handle_data(self, data):
        if self.current is not None:
            self.current[1].append(data)

    def handle_endtag(self, tag):
        if tag == "script" and self.current is not None:
            self.scripts.append(self.current)
            self.current = None


def extract_skinning_rows(script):
    """Recognize the published config header, then decode only its data JSON."""
    rows = []
    for declaration in JS_DECLARATIONS.finditer(script):
        if declaration.lastgroup != "listview":
            continue
        closing = re.search(
            r"^\s*\}\s*\)\s*;", script[declaration.end() :], re.MULTILINE
        )
        if closing is None:
            continue
        block = script[declaration.end() : declaration.end() + closing.start()]
        if not SKIN_ID.search(block):
            continue
        if not SKIN_HEADER.match(block):
            raise ValueError(
                "Skinning source must be the exact NPC skinned-from Listview"
            )
        data = list(re.finditer(r"^\s*data\s*:\s*", block, re.MULTILINE))
        if len(data) != 1:
            raise ValueError("Skinning Listview requires one raw JSON data array")
        payload = block[data[0].end() :]
        records, end = DECODER.raw_decode(payload)
        if not re.fullmatch(r"\s*,?\s*", payload[end:]) or not isinstance(
            records, list
        ):
            raise ValueError("Skinning data must be a raw JSON array, not JavaScript")
        rows.append(records)
    return rows


def parse_published_skinning(html, item_id):
    """Return concrete NPC observations; unrelated sold/drop/zone tabs do not count."""
    if not positive_integer(item_id):
        raise ValueError("Item ID must be a positive u32")
    parser = PageScripts()
    parser.feed(html)
    parser.close()
    if parser.identity is None or parser.identity not in parser.scripts:
        raise ValueError("Missing complete data.page.info item identity")
    identity = json.loads("".join(parser.identity[1]), object_pairs_hook=unique_object)
    if (
        not isinstance(identity, dict)
        or type(identity.get("entityType")) is not int
        or identity["entityType"] != 3
        or type(identity.get("entityId")) is not int
        or identity["entityId"] != item_id
    ):
        raise ValueError(f"Wrong item page identity for {item_id}")
    blocks = []
    for attributes, parts in parser.scripts:
        if attributes.get("type", "text/javascript") in {
            "text/javascript",
            "application/javascript",
        }:
            blocks.extend(extract_skinning_rows("".join(parts)))
    if len(blocks) != 1 or not blocks[0]:
        raise ValueError("Expected one nonempty NPC skinned-from source block")
    seen = set()
    for row in blocks[0]:
        if (
            not isinstance(row, dict)
            or not positive_integer(row.get("id"))
            or not positive_integer(row.get("count"))
            or not positive_integer(row.get("outof"))
            or row["count"] > row["outof"]
            or row["id"] in seen
        ):
            raise ValueError(
                "Skinning NPC requires unique positive u32 ID and positive count <= outof"
            )
        seen.add(row["id"])
    return {"item_id": item_id, "npcs": blocks[0]}


def load_cached_item(metadata_path):
    item_id = int(metadata_path.stem.removeprefix("item-"))
    try:
        metadata = json.loads(
            metadata_path.read_text(encoding="utf-8"), object_pairs_hook=unique_object
        )
        raw = metadata_path.with_suffix(".html").read_bytes()
    except (OSError, UnicodeError, ValueError) as error:
        raise ValueError(
            f"Cannot read published skinning cache {metadata_path}: {error}"
        ) from error
    if (
        not isinstance(metadata, dict)
        or set(metadata) != {"source_url", "sha256", "captured_utc", "provenance"}
        or metadata["source_url"] != f"https://www.wowhead.com/item={item_id}"
        or metadata["sha256"] != hashlib.sha256(raw).hexdigest()
        or json.dumps(metadata["provenance"], sort_keys=True)
        != json.dumps(PUBLISHED_PROVENANCE, sort_keys=True)
    ):
        raise ValueError(
            f"Published skinning source/hash/provenance mismatch for item {item_id}"
        )
    timestamp = metadata["captured_utc"]
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise ValueError(
            f"Published skinning requires a UTC acquisition timestamp for item {item_id}"
        )
    try:
        datetime.fromisoformat(timestamp)
        parsed = parse_published_skinning(raw.decode("utf-8"), item_id)
    except (UnicodeError, ValueError) as error:
        raise ValueError(
            f"Invalid published skinning cache for item {item_id}: {error}"
        ) from error
    return {**parsed, **metadata}


def load_published_skinning_evidence(cache_dir):
    """Metadata files explicitly select paired raw pages; nothing is fetched/inferred."""
    cache = Path(cache_dir)
    if not cache.is_dir():
        raise ValueError(f"Missing published skinning cache directory {cache}")
    paths = sorted(cache.glob("item-*.json"))
    if not paths:
        raise ValueError(f"No selected item acquisition metadata in {cache}")
    evidence = {}
    for path in paths:
        if not re.fullmatch(r"item-[1-9][0-9]*\.json", path.name):
            raise ValueError(f"Invalid selected skinning metadata filename {path.name}")
        record = load_cached_item(path)
        evidence[record["item_id"]] = record
    return evidence
