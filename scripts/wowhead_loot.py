"""Wowhead drop data as creature loot where TrinityCore TDB1210 has none (retail_content.build_loot).

A scrape is a directory data/reference/wowhead/<name>-<YYYYMMDD>/ of <creature entry>.json files, each the
`drops` Listview embedded in that npc= page (browser-cli). Per drop row: item `id`, `quality`, `classs`,
`stack` [min, max] and `modes` {"<difficulty>": {count, outof}}. The rows are kept verbatim per difficulty in
wowhead_creature_drop with the scrape name as provenance; refreshing is a new scrape directory.
"""

import json
import os
from pathlib import Path

DEFAULT_ROOT = Path(__file__).resolve().parent.parent / "data/reference/wowhead"


def wowhead_root():
    return Path(os.environ.get("WOWHEAD_DIR", DEFAULT_ROOT))


def drop_rows(scrape, entry, page):
    """(entry, item, difficulty, count, outof, stack_min, stack_max, quality, classs, scrape) of one npc page."""
    for drop in page["drops"]:
        for mode, rate in drop["modes"].items():
            if mode == "mode":
                continue
            low, high = drop["stack"]
            yield (entry, drop["id"], int(mode), rate["count"], rate["outof"], low, high, drop["quality"],
                   drop["classs"], scrape)


def load_wowhead_drops(conn, root):
    """wowhead_creature_drop from every scrape under `root` (none when `root` does not exist)."""
    conn.execute("DROP TABLE IF EXISTS wowhead_creature_drop")
    conn.execute("""CREATE TABLE wowhead_creature_drop (entry INTEGER NOT NULL, item INTEGER NOT NULL,
        difficulty INTEGER NOT NULL, count INTEGER NOT NULL, outof INTEGER NOT NULL, stack_min INTEGER NOT NULL,
        stack_max INTEGER NOT NULL, quality INTEGER NOT NULL, classs INTEGER NOT NULL, scrape TEXT NOT NULL,
        PRIMARY KEY (entry, item, difficulty))""")
    # Flat numeric NPC pages are the loot format; calibration/content reports are not.
    pages = sorted(root.glob("*/[0-9]*.json")) if root.is_dir() else []
    scraped = {}
    for path in pages:
        entry = int(path.stem)
        if entry in scraped:
            raise SystemExit(f"Wowhead creature {entry} is in two scrapes: {scraped[entry]} and {path.parent.name}")
        scraped[entry] = path.parent.name
        page = json.loads(path.read_text(encoding="utf-8"))
        conn.executemany("INSERT INTO wowhead_creature_drop VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                         drop_rows(path.parent.name, entry, page))
