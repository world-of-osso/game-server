#!/usr/bin/env python3
"""Capture per-page Forever Zephras references; no server database writes.

Sequential HTTPS captures stop on 403/429. Saved pages are immutable within a
scrape directory. Unknown fields remain null; mapper locations are observations,
not authoritative spawns. Run --capture, then --normalize (optionally --bounds
and --terrain). Browser-rendered source can be captured with --source FILE
--kind quest|npc|zone|object --id ID after a blocked HTTPS request.
"""

import argparse
import html
import json
import math
import re
import struct
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ZONE = 16593
MAP = 2991
UI_MAP = 2665
SEEDS = [92471, 92461, 92470, 92598, 93836, 93089, 93739, 92644, 94414, 96638]
ROOT = Path(__file__).resolve().parents[1] / "data/reference/wowhead"
UA = "Mozilla/5.0 (compatible; game-server reference capture)"


def read_json_at(source, start):
    literal = source[start:].lstrip()
    if literal.startswith(("{", "[")):
        literal = balanced(literal, 0)
        # Protect quoted strings while quoting the bare keys in Wowhead loot rows.
        tokens = r'"(?:\\.|[^"\\])*"|\b([A-Za-z_$][\w$]*)(?=\s*:)'
        literal = re.sub(tokens, lambda m: json.dumps(m[1]) if m[1] else m[0], literal)
    return json.JSONDecoder().raw_decode(literal)[0]


def balanced(source, start):
    """Read one JS object, without executing page JavaScript."""
    depth, quote, escaped = 0, None, False
    for end in range(start, len(source)):
        char = source[end]
        if quote:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char in "\"'`":
            quote = char
        elif char in "{[":
            depth += 1
        elif char in "}]":
            depth -= 1
            if depth == 0:
                return source[start : end + 1]
    raise ValueError("unterminated page data object")


def plain(markup):
    markup = re.sub(r"<script\b[^>]*>.*?</script>", "", markup, flags=re.S)
    markup = re.sub(r"<br\s*/?>|</(?:p|li|tr)>", "\n", markup)
    return html.unescape(re.sub(r"<[^>]+>", "", markup)).strip()


def parse_page(source, kind, entry):
    if "Just a moment" in source or "cf-chl-" in source:
        raise ValueError("Cloudflare challenge, not a database page")
    views = []
    for match in re.finditer(r"new Listview\(\s*(\{)", source):
        raw = balanced(source, match.start(1))
        field = re.search(r'(?:\bdata|"data")\s*:\s*', raw)
        if not field:
            continue
        template = re.search(r'(?:\btemplate|"template")\s*:\s*[\"\']([^\"\']+)', raw)
        name = re.search(r'(?:\bid|"id")\s*:\s*[\"\']([^\"\']+)', raw)
        if (
            template
            and template[1] in ("npc", "quest", "object", "item", "spell")
            and name
        ):
            views.append(
                {
                    "template": template[1],
                    "id": name[1],
                    "data": read_json_at(raw, field.end()),
                }
            )
    gatherer = []
    for match in re.finditer(r"WH\.Gatherer\.addData\((\d+),\s*(\d+),\s*", source):
        gatherer.append(
            {
                "type": int(match[1]),
                "environment": int(match[2]),
                "data": read_json_at(source, match.end()),
            }
        )
    entity = {}
    match = re.search(r"\$\.extend\(g_\w+\[" + str(entry) + r"\],\s*", source)
    if match:
        entity = read_json_at(source, match.end())
    mapper = {}
    match = re.search(r"(?:var\s+)?g_mapperData\s*=\s*", source)
    if match:
        mapper = read_json_at(source, match.end())
    objective_mapper = {}
    match = re.search(r"new Mapper\(\s*", source)
    if match:
        objective_mapper = read_json_at(source, match.end())
    text = {}
    heading = re.search(r"<h1\b[^>]*>.*?</h1>", source, re.S)
    if heading and kind == "quest":
        tail = source[heading.end() :]
        text["objectives"] = plain(re.split(r"<h2\b", tail)[0])
        for section in re.finditer(
            r"<h2\b[^>]*>(.*?)</h2>(.*?)(?=<h2\b|$)", tail, re.S
        ):
            title = plain(section[1]).lower()
            if title in ("description", "progress", "completion"):
                text[title] = plain(section[2])
    series = []
    match = re.search(r'<table class="series">(.*?)</table>', source, re.S)
    if match:
        for row in re.findall(r"<tr>(.*?)</tr>", match[1], re.S):
            ids = [int(v) for v in re.findall(r"/forever/quest=(\d+)", row)]
            if "<b>" in row:
                ids.append(entry)
            series.append(ids)
    displays = sorted(
        {
            int(v)
            for v in re.findall(
                r'(?:displayId(?:&quot;|\")?\s*[:=]\s*|data-mv-display-id=")(\d+)',
                source,
            )
        }
    )
    raw_scripts = [
        s
        for s in re.findall(r"<script\b[^>]*>(.*?)</script>", source, re.S)
        if any(
            key in s
            for key in (
                "Gatherer.addData",
                "$.extend(g_",
                "g_mapperData",
                "new Mapper",
                "WH.markup.printHtml",
            )
        )
    ]
    if not entity and not views and not gatherer:
        raise ValueError(f"{kind} {entry}: no database payload found")
    return {
        "kind": kind,
        "id": entry,
        "entity": entity,
        "listviews": views,
        "gatherer": gatherer,
        "mapper": mapper,
        "objective_mapper": objective_mapper,
        "displayIds": displays,
        "text": text,
        "series_rows": series,
        "raw_scripts": raw_scripts,
        "html": source,
    }


def zone_ids(page, kind):
    return sorted(
        {
            row["id"]
            for view in page["listviews"]
            if view["template"] == kind
            for row in view["data"]
            if kind == "quest" or ZONE in row.get("location", [])
        }
    )


def mapper_points(page):
    for zone, info in page["objective_mapper"].get("objectives", {}).items():
        for level in info.get("levels", []):
            for point in level:
                yield int(zone), point


def race_ids(mask):
    if mask is None:
        return None
    # ChrRaces RaceBit 32/33; not RaceID - 1 for Forever races.
    return [95 + bit - 32 for bit in (32, 33) if int(mask) & (1 << bit)]


def normalize_quest(page):
    row = page["entity"]
    starters, enders, objectives = set(), set(), []
    for zone, point in mapper_points(page):
        if point.get("point") in ("start", "end") and point.get("type") == 1:
            (starters if point["point"] == "start" else enders).add(point["id"])
        elif point.get("id"):
            objectives.append(
                {
                    "type": point.get("type"),
                    "target_id": point["id"],
                    "count": point.get("count"),
                    "name": point.get("name"),
                    "zone": zone,
                    "source": "mapper observation; not necessarily objective credit",
                }
            )
    # Objective tables carry item/NPC IDs and counts when mapper does not.
    objective_html = re.split(r"<h2\b[^>]*>Description", page["html"])[0]
    for match in re.finditer(r"<tr\b[^>]*>(.*?)</tr>", objective_html, re.S):
        target = re.search(r"/forever/(npc|item|object)=(\d+)", match[1])
        count = re.search(r"\((\d+)\)|(?:x|×)\s*(\d+)", plain(match[1]))
        if target and count:
            objectives.append(
                {
                    "type": {"npc": "kill", "item": "collect", "object": "interact"}[
                        target[1]
                    ],
                    "target_id": int(target[2]),
                    "count": int(count[1] or count[2]),
                    "name": plain(match[1]),
                    "source": "objective table",
                }
            )
    series = page["series_rows"]
    slot = next((i for i, ids in enumerate(series) if page["id"] in ids), None)
    return {
        "id": page["id"],
        "name": row.get("name"),
        "level": row.get("level"),
        "reqlevel": row.get("reqlevel"),
        "side": row.get("side"),
        "races": race_ids(row.get("reqrace", row.get("race"))),
        "race_mask": row.get("reqrace", row.get("race")),
        "classes": row.get("reqclass"),
        "starter_npc_ids": sorted(starters),
        "ender_npc_ids": sorted(enders),
        "objectives": objectives,
        "text": page["text"],
        "rewards": {
            "xp": row.get("xp"),
            "money": row.get("money"),
            "items": row.get("itemrewards"),
            "choice_items": row.get("itemchoices"),
            "spells": row.get("rewardspells", row.get("rewardspell")),
            "reputation": row.get("reprewards"),
        },
        "series": {
            "rows": series,
            "ids": [v for ids in series for v in ids],
            "order": slot + 1 if slot is not None else None,
            "prev": series[slot - 1] if slot is not None and slot else [],
            "next": series[slot + 1]
            if slot is not None and slot + 1 < len(series)
            else [],
        },
        "category": row.get("category"),
        "source": page.get("url"),
    }


def normalize_npc(page):
    row = page["entity"]
    models = set(page["displayIds"])
    for data in page["gatherer"]:
        if data["type"] == 1:
            model = data["data"].get(str(page["id"]), {}).get("model")
            if model:
                models.add(model)
    coords = []
    for zone, entries in page["mapper"].items():
        if isinstance(entries, list):
            for entry in entries:
                for point in entry.get("coords", []):
                    coords.append(
                        {
                            "zone": int(zone),
                            "uiMap": entry.get("uiMapId"),
                            "percent": point[:2],
                        }
                    )

    def quests(view_id):
        return sorted(
            {
                r["id"]
                for v in page["listviews"]
                if v["template"] == "quest" and v["id"] == view_id
                for r in v["data"]
                if r.get("category") == ZONE
            }
        )

    view_ids = {v["id"] for v in page["listviews"]}
    return {
        "id": page["id"],
        "name": row.get("name"),
        "title": row.get("tag"),
        "level": {"min": row.get("minlevel"), "max": row.get("maxlevel")},
        "react": row.get("react"),
        "displayIds": sorted(models),
        "coords": coords,
        "vendor": True if "sells" in view_ids else None,
        "trainer": True if "teaches" in view_ids else None,
        "starts_quests": quests("starts"),
        "ends_quests": quests("ends"),
        "source": page.get("url"),
    }


def percent_to_world(percent, bounds):
    u, v = (float(value) / 100 for value in percent)
    lo, hi = bounds["ui_min"], bounds["ui_max"]
    if not lo[0] <= u <= hi[0] or not lo[1] <= v <= hi[1]:
        raise ValueError(f"percent coordinate outside assignment: {percent}")
    low, high = bounds["region_min"], bounds["region_max"]
    return (
        high[0] - (v - lo[1]) / (hi[1] - lo[1]) * (high[0] - low[0]),
        high[1] - (u - lo[0]) / (hi[0] - lo[0]) * (high[1] - low[1]),
    )


def chunk_payloads(data):
    offset = 0
    while offset + 8 <= len(data):
        tag = data[offset : offset + 4][::-1]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        if offset + 8 + size > len(data):
            raise ValueError(f"truncated {tag!r} at {offset}")
        yield tag, data[offset + 8 : offset + 8 + size]
        offset += 8 + size


def interpolate_fan(tl, tr, bl, br, center, col, row):
    if row <= col and row <= 1 - col:
        return tl * (1 - col - row) + tr * (col - row) + center * 2 * row
    if col >= row and col >= 1 - row:
        return tr * (col - row) + br * (col + row - 1) + center * 2 * (1 - col)
    if row >= col and row >= 1 - col:
        return bl * (row - col) + br * (col + row - 1) + center * 2 * (1 - row)
    return tl * (1 - col - row) + bl * (row - col) + center * 2 * col


def sample_adt(data, x, y):
    """Port of server terrain_height.rs MCVT fan sampling; holes return unknown.

    No WMO floors, liquids, facing or phase are inferred here.
    """
    chunk_size = (1600 / 3) / 16
    unit = chunk_size / 8
    for tag, payload in chunk_payloads(data):
        if tag != b"MCNK":
            continue
        cx, cy, base = struct.unpack_from("<3f", payload, 0x68)
        if round(cx / chunk_size) != math.ceil(x / chunk_size) or round(
            cy / chunk_size
        ) != math.ceil(y / chunk_size):
            continue
        rf, cf = max(0, min(8, (cx - x) / unit)), max(0, min(8, (cy - y) / unit))
        r, c = min(7, int(rf)), min(7, int(cf))
        flags = struct.unpack_from("<I", payload)[0]
        if flags & 0x10000:
            holes = struct.unpack_from("<Q", payload, 0x14)[0]
            hole = holes >> (r * 8 + c) & 1
        else:
            holes = struct.unpack_from("<I", payload, 0x3C)[0] & 0xFFFF
            hole = holes >> ((r // 2) * 4 + c // 2) & 1
        if hole:
            return None
        sub = dict(chunk_payloads(payload[128:]))
        if b"MCVT" not in sub:
            raise ValueError("MCNK missing MCVT")
        h = struct.unpack_from("<145f", sub[b"MCVT"])
        return base + interpolate_fan(
            h[r * 17 + c],
            h[r * 17 + c + 1],
            h[(r + 1) * 17 + c],
            h[(r + 1) * 17 + c + 1],
            h[r * 17 + 9 + c],
            cf - c,
            rf - r,
        )
    return None


class Terrain:
    def __init__(self, root):
        self.root = root
        maid = dict(chunk_payloads((root / "7198644.wdt").read_bytes()))[b"MAID"]
        self.tiles = {
            i: struct.unpack_from("<I", maid, i * 32)[0] for i in range(len(maid) // 32)
        }
        self.cache = {}

    def position(self, x, y):
        # WDT MAID tile index is tile_y*64 + tile_x; axes match server GroundMap.
        tx, ty = math.floor(32 - y / (1600 / 3)), math.floor(32 - x / (1600 / 3))
        fdid = self.tiles.get(ty * 64 + tx) if 0 <= tx < 64 and 0 <= ty < 64 else None
        path = self.root / f"{fdid}.adt"
        if not fdid or not path.is_file():
            return {
                "x": x,
                "y": y,
                "z": None,
                "tile": [tx, ty],
                "root_fdid": fdid,
                "z_status": "missing-root",
            }
        if fdid not in self.cache:
            self.cache[fdid] = path.read_bytes()
        z = sample_adt(self.cache[fdid], x, y)
        return {
            "x": x,
            "y": y,
            "z": z,
            "tile": [tx, ty],
            "root_fdid": fdid,
            "z_status": "terrain" if z is not None else "hole-or-outside",
        }


def save_page(root, page, url):
    page.update(
        url=url,
        fetched_at=datetime.now(timezone.utc).isoformat(),
        provenance=root.name,
        reconstructed=True,
    )
    path = root / page["kind"] / f"{page['id']}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(page, ensure_ascii=False, indent=2) + "\n")
    return page


class Capture:
    def __init__(self, root):
        self.root = root
        self.last = 0
        self.fetched = []
        self.cached = []
        self.failed = []

    def page(self, kind, entry):
        path = self.root / kind / f"{entry}.json"
        if path.exists():
            self.cached.append([kind, entry])
            return json.loads(path.read_text())
        url = f"https://www.wowhead.com/forever/{kind}={entry}"
        time.sleep(max(0, 1.5 - (time.monotonic() - self.last)))
        self.last = time.monotonic()
        try:
            request = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(request, timeout=40) as response:
                source = response.read().decode("utf-8")
            page = save_page(self.root, parse_page(source, kind, entry), url)
        except (urllib.error.HTTPError, urllib.error.URLError, ValueError) as error:
            self.failed.append(
                {"kind": kind, "id": entry, "url": url, "error": str(error)}
            )
            self.report()
            raise SystemExit(f"Stopped capture: {url}: {error}") from error
        self.fetched.append([kind, entry])
        print(f"{kind} {entry}: captured", flush=True)
        return page

    def report(self):
        self.root.mkdir(parents=True, exist_ok=True)
        (self.root / "capture-report.json").write_text(
            json.dumps(
                {"fetched": self.fetched, "cached": self.cached, "failed": self.failed},
                indent=2,
            )
            + "\n"
        )

    def run(self):
        zone = self.page("zone", ZONE)
        allowed = set(zone_ids(zone, "quest"))
        # Seeds are inspected, but foreign category pages are never followed.
        pending = sorted(allowed | set(SEEDS))
        seen, npc_ids, object_ids = set(), set(zone_ids(zone, "npc")), set()
        while pending:
            entry = pending.pop(0)
            if entry in seen:
                continue
            seen.add(entry)
            page = self.page("quest", entry)
            if entry not in allowed and not any(
                z == ZONE for z, _ in mapper_points(page)
            ):
                continue
            quest = normalize_quest(page)
            npc_ids.update(quest["starter_npc_ids"] + quest["ender_npc_ids"])
            for zone_id, point in mapper_points(page):
                if zone_id == ZONE and point.get("type") == 1:
                    npc_ids.add(point["id"])
                elif zone_id == ZONE and point.get("type") == 2:
                    object_ids.add(point["id"])
            pending.extend(
                q for q in quest["series"]["ids"] if q in allowed and q not in seen
            )
        for entry in sorted(npc_ids):
            page = self.page("npc", entry)
            for quest in (
                normalize_npc(page)["starts_quests"]
                + normalize_npc(page)["ends_quests"]
            ):
                if quest not in seen and quest in allowed:
                    self.page("quest", quest)
                    seen.add(quest)
        for entry in sorted(object_ids):
            self.page("object", entry)
        self.report()


def normalize(root, bounds=None, terrain=None):
    quests, npcs, objects = [], [], []
    zone_page = json.loads((root / "zone" / f"{ZONE}.json").read_text())
    allowed = set(zone_ids(zone_page, "quest"))
    for path in sorted((root / "quest").glob("*.json")):
        page = json.loads(path.read_text())
        if page["id"] in allowed or any(z == ZONE for z, _ in mapper_points(page)):
            quests.append(normalize_quest(page))
    for path in sorted((root / "npc").glob("*.json")):
        npc = normalize_npc(json.loads(path.read_text()))
        # Starter pages can lack their own coords; preserve quest mapper observations.
        known = {(c["zone"], tuple(c["percent"])) for c in npc["coords"]}
        for qp in sorted((root / "quest").glob("*.json")):
            for zone, point in mapper_points(json.loads(qp.read_text())):
                if (
                    zone == ZONE
                    and point.get("type") == 1
                    and point.get("id") == npc["id"]
                ):
                    for pair in point.get("coords", []):
                        if (zone, tuple(pair[:2])) not in known:
                            npc["coords"].append(
                                {
                                    "zone": zone,
                                    "uiMap": None,
                                    "percent": pair[:2],
                                    "source": "quest mapper",
                                }
                            )
                            known.add((zone, tuple(pair[:2])))
        if bounds:
            for point in npc["coords"]:
                if point["zone"] == ZONE:
                    x, y = percent_to_world(point["percent"], bounds)
                    point["world"] = (
                        terrain.position(x, y)
                        if terrain
                        else {"x": x, "y": y, "z": None, "z_status": "not-sampled"}
                    )
                    point["calibrated_uiMap"] = UI_MAP
        npcs.append(npc)
    for path in sorted((root / "object").glob("*.json")):
        page = json.loads(path.read_text())
        objects.append(
            {
                "id": page["id"],
                "entity": page["entity"],
                "mapper": page["mapper"],
                "source": page.get("url"),
            }
        )
    content = {
        "provenance": root.name,
        "reconstructed": True,
        "zone": ZONE,
        "map": MAP,
        "uiMap": UI_MAP,
        "bounds": bounds,
        "quests": quests,
        "npcs": npcs,
        "objects": objects,
        "gaps": [
            "No scripted events, phasing or facing reconstructed.",
            "Unknown metadata stays null; mapper points do not prove objective credit.",
            "Wowhead uiMap 2521 is retained verbatim; zone 16593 maps to client uiMap 2665.",
            "Terrain z is not a WMO floor or liquid surface.",
        ],
    }
    (root / "zephras-content.json").write_text(
        json.dumps(content, ensure_ascii=False, indent=2) + "\n"
    )
    print(f"Normalized {len(quests)} quests, {len(npcs)} NPCs, {len(objects)} objects")
    return content


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT / f"forever-zephras-{datetime.now(timezone.utc):%Y%m%d}",
    )
    parser.add_argument("--capture", action="store_true")
    parser.add_argument("--normalize", action="store_true")
    parser.add_argument("--source", type=Path)
    parser.add_argument("--kind", choices=["quest", "npc", "zone", "object"])
    parser.add_argument("--id", type=int)
    parser.add_argument("--bounds", type=Path)
    parser.add_argument("--terrain", type=Path)
    args = parser.parse_args()
    if args.source:
        if not args.kind or args.id is None:
            parser.error("--source requires --kind and --id")
        save_page(
            args.root,
            parse_page(args.source.read_text(), args.kind, args.id),
            f"https://www.wowhead.com/forever/{args.kind}={args.id}",
        )
    if args.capture:
        Capture(args.root).run()
    if args.normalize:
        bounds = json.loads(args.bounds.read_text()) if args.bounds else None
        normalize(args.root, bounds, Terrain(args.terrain) if args.terrain else None)
    if not (args.source or args.capture or args.normalize):
        parser.error("select --capture, --source or --normalize")


if __name__ == "__main__":
    main()
