#!/usr/bin/env python3
"""Extract per-spec formula quirks from SimC class_modules.

Scans class module C++ files for:
- composite_*_multiplier overrides (damage/target/action multipliers)
- effectN().percent() references (spell data coefficient usage)
- action_multiplier() overrides (per-ability special scaling)

Outputs a JSON file documenting spec-specific formula quirks that
aren't captured in raw DBC/spell_data extracts.

Usage:
    python3 scripts/extract-simc-quirks.py [simc_dir] > data/simc_quirks.json
"""

import json
import re
import sys
from pathlib import Path

SIMC_DIR = (
    Path(sys.argv[1])
    if len(sys.argv) > 1
    else Path.home() / "Repos/simc/engine/class_modules"
)

# Patterns to detect formula quirks
MULTIPLIER_RE = re.compile(
    r"(composite_\w*multiplier|action_multiplier|action_ta_multiplier)\s*\("
)
EFFECT_RE = re.compile(
    r"(\w+)->effectN\(\s*(\d+)\s*\)\.(percent|base_value|resource)\(\)"
)
DA_OVERRIDE_RE = re.compile(r"(da_multiplier|ta_multiplier)\s*\*=\s*([^;]+);")
SPELL_REF_RE = re.compile(r"(talents?\.\w+\.\w+|spec\.\w+|buff\.\w+)")


def extract_quirks(path: Path) -> list[dict]:
    """Extract formula quirks from a single class module file."""
    quirks = []
    class_name = path.stem.replace("sc_", "")
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

    current_func = None
    for i, line in enumerate(lines):
        # Track function context
        if MULTIPLIER_RE.search(line) and "override" in line:
            current_func = MULTIPLIER_RE.search(line).group(1)

        # Find effectN references with percent/base_value
        for m in EFFECT_RE.finditer(line):
            spell_ref = m.group(1)
            effect_n = int(m.group(2))
            accessor = m.group(3)
            quirks.append(
                {
                    "class": class_name,
                    "file": path.name,
                    "line": i + 1,
                    "type": "effect_reference",
                    "spell_ref": spell_ref,
                    "effect_index": effect_n,
                    "accessor": accessor,
                    "context": current_func or "unknown",
                    "code": line.strip()[:120],
                }
            )

        # Find direct multiplier modifications
        for m in DA_OVERRIDE_RE.finditer(line):
            quirks.append(
                {
                    "class": class_name,
                    "file": path.name,
                    "line": i + 1,
                    "type": "multiplier_override",
                    "field": m.group(1),
                    "expression": m.group(2).strip()[:80],
                    "context": current_func or "unknown",
                    "code": line.strip()[:120],
                }
            )

    return quirks


def main():
    if not SIMC_DIR.exists():
        print(f"ERROR: SimC class_modules not found at {SIMC_DIR}", file=sys.stderr)
        sys.exit(1)

    all_quirks = []
    cpp_files = sorted(SIMC_DIR.glob("sc_*.cpp"))

    for path in cpp_files:
        quirks = extract_quirks(path)
        all_quirks.extend(quirks)

    # Summary by class
    classes = {}
    for q in all_quirks:
        cls = q["class"]
        classes.setdefault(cls, 0)
        classes[cls] += 1

    summary = {
        "total_quirks": len(all_quirks),
        "per_class": classes,
        "quirks": all_quirks,
    }

    print(
        f"Extracted {len(all_quirks)} formula quirks from {len(cpp_files)} class files",
        file=sys.stderr,
    )
    for cls, count in sorted(classes.items()):
        print(f"  {cls}: {count}", file=sys.stderr)

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
