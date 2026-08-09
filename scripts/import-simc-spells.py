#!/usr/bin/env python3
"""Convert SimC spell data to SQL overlay for our spell_bonus_data table.

Reads SimC sc_spell_data.inc and sc_spell_lists.inc to extract spell IDs,
names, and coefficients. Outputs SQL INSERT statements that can be loaded
into world.db to overlay retail values on top of AzerothCore baseline.

Usage:
    python3 scripts/import-simc-spells.py [simc_dir] > data/simc_overlay.sql

    simc_dir defaults to ~/Repos/simc/engine/dbc/generated
"""

import re
import sys
from pathlib import Path

SIMC_DIR = (
    Path(sys.argv[1])
    if len(sys.argv) > 1
    else Path.home() / "Repos/simc/engine/dbc/generated"
)

SPELL_DATA_FILE = SIMC_DIR / "sc_spell_data.inc"

# Pattern to extract: { "Name", spell_id, school_mask, sp_coeff, ap_coeff, ... }
# Fields: name(0), id(1), school(2), sp_coeff(3), ap_coeff(4), ...
SPELL_RE = re.compile(
    r'\{\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*\d+\s*,\s*([0-9.]+)\s*,\s*([0-9.]+)\s*,\s*([0-9.]+)'
)


def parse_spell_data(path: Path) -> list[dict]:
    """Parse sc_spell_data.inc and extract spell ID + coefficients."""
    spells = []
    text = path.read_text(encoding="utf-8", errors="replace")

    for match in SPELL_RE.finditer(text):
        name = match.group(1)
        spell_id = int(match.group(2))
        sp_coeff = float(match.group(3))
        ap_coeff = float(match.group(4))
        # Field 5 is dot_coeff in some versions
        dot_coeff = float(match.group(5))

        # Skip entries with no coefficients
        if sp_coeff == 0.0 and ap_coeff == 0.0 and dot_coeff == 0.0:
            continue

        spells.append(
            {
                "id": spell_id,
                "name": name,
                "sp_direct": sp_coeff,
                "sp_dot": dot_coeff,
                "ap_direct": ap_coeff,
                "ap_dot": 0.0,
            }
        )

    return spells


def emit_sql(spells: list[dict]) -> str:
    """Generate SQL INSERT/REPLACE statements for spell_bonus_data."""
    lines = [
        "-- SimC spell coefficient overlay",
        f"-- Source: {SPELL_DATA_FILE}",
        f"-- {len(spells)} spells with non-zero coefficients",
        "",
        "BEGIN TRANSACTION;",
    ]

    for s in spells:
        lines.append(
            f"INSERT OR REPLACE INTO spell_bonus_data "
            f"(entry, direct_bonus, dot_bonus, ap_bonus, ap_dot_bonus, comments) "
            f"VALUES ({s['id']}, {s['sp_direct']}, {s['sp_dot']}, "
            f"{s['ap_direct']}, {s['ap_dot']}, "
            f"'{s['name'].replace(chr(39), chr(39) + chr(39))}');"
        )

    lines.append("COMMIT;")
    return "\n".join(lines)


def main():
    if not SPELL_DATA_FILE.exists():
        print(f"ERROR: {SPELL_DATA_FILE} not found", file=sys.stderr)
        sys.exit(1)

    spells = parse_spell_data(SPELL_DATA_FILE)
    print(f"-- Parsed {len(spells)} spells with coefficients", file=sys.stderr)
    print(emit_sql(spells))


if __name__ == "__main__":
    main()
