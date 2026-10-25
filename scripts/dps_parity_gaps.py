#!/usr/bin/env python3
"""Rank missing DPS per spell across all specs from the latest measured train results.

For each spec, the newest merge-train measurement (`/home/osso/.worktrees/train/results`)
gives the native `<spec>.native.<n>.json`. Its per-spell damage is compared with the
pinned SimC reference `<spec>.reference.{single,three}.normalized.json`. Per-spell DPS is
`share_pct × total DPS`, summed over actors (player, pets, guardians), matched by spell ID.
Spells missing in several specs (trinkets, consumables, shared engine rules) are grouped,
because one shared fix closes all of them.
"""
import argparse
import glob
import json
import os
from pathlib import Path

RESULTS = Path("/home/osso/.worktrees/train/results")
REFERENCES = Path("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/"
                  "dps-parity-2026-10-21/all-spec-expanded-references")
TARGETS = {1: "single", 3: "three"}


def latest_outputs():
    """spec -> output dir of the newest result with a successful measurement."""
    best = {}
    for path in RESULTS.glob("*.json"):
        entry = json.loads(path.read_text())
        if not any("NATIVE_DPS" in line for line in entry.get("measurements") or []):
            continue
        for measure in entry["measure"]:
            report = Path(measure["output"]) / f"{measure['spec']}.native.1.json"
            if report.exists():
                stamp = report.stat().st_mtime
                if measure["spec"] not in best or stamp > best[measure["spec"]][0]:
                    best[measure["spec"]] = (stamp, Path(measure["output"]), entry["status"])
    return {spec: (output, status) for spec, (_, output, status) in best.items()}


def per_spell(actions, dps):
    """spell_id -> (dps, name) summed over actors."""
    spells = {}
    for action in actions:
        # Native melee swings carry no spell ID; SimC reports main/off-hand autos as 0/1.
        spell = action.get("spell_id") or 0
        if spell == 1:
            spell = 0
        value, name = spells.get(spell, (0.0, action.get("name")))
        spells[spell] = (value + action.get("share_pct", 0.0) * dps / 100, name or action.get("name"))
    return spells


def compare(spec, output, count):
    native_path = output / f"{spec}.native.{count}.json"
    reference_path = REFERENCES / f"{spec}.reference.{TARGETS[count]}.normalized.json"
    if not native_path.exists() or not reference_path.exists():
        return None
    native = json.loads(native_path.read_text())["combat"]
    reference = json.loads(reference_path.read_text())
    ours = per_spell(native["actions"], native["dps"])
    theirs = per_spell(reference["actions"], reference["dps"])
    rows = []
    for spell, (ref_dps, name) in theirs.items():
        our_dps = ours.get(spell, (0.0, None))[0]
        rows.append({"spell": spell, "name": name, "ref": ref_dps, "native": our_dps, "gap": ref_dps - our_dps})
    return {"native_dps": native["dps"], "ref_dps": reference["dps"], "rows": rows,
            # Native-only damage the reference does not have (possible overshoot or ID mismatch).
            "extra": sorted(((ours[s][0], s) for s in ours if s not in theirs), reverse=True)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--top", type=int, default=8)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    lines, shared = [], {}
    for spec, (output, status) in sorted(latest_outputs().items(), key=lambda item: int(item[0])):
        for count in (1, 3):
            result = compare(spec, output, count)
            if result is None:
                continue
            pct = 100 * result["native_dps"] / result["ref_dps"]
            lines.append(f"\n## {spec} {TARGETS[count]}: {result['native_dps']:,.0f} / {result['ref_dps']:,.0f} "
                         f"= {pct:.1f}%  ({status[:28]}, {output})")
            lines.append("| spell | name | ref DPS | native DPS | missing |")
            lines.append("|---|---|---:|---:|---:|")
            for row in sorted(result["rows"], key=lambda r: -r["gap"])[:args.top]:
                lines.append(f"| {row['spell']} | {row['name']} | {row['ref']:,.0f} | {row['native']:,.0f} | {row['gap']:,.0f} |")
                if row["native"] < 0.05 * row["ref"] and row["ref"] > 1000:
                    shared.setdefault((row["spell"], row["name"]), []).append((spec, count, row["gap"]))
            if result["extra"][:3]:
                extra = ", ".join(f"{s} {d:,.0f}" for d, s in result["extra"][:3])
                lines.append(f"\nnative-only spells (top): {extra}")
    multi = sorted(((sum(g for *_, g in v), k, v) for k, v in shared.items() if len({s for s, *_ in v}) > 1),
                   reverse=True)
    head = ["# DPS parity gaps by spell", "", "## Missing in several specs (one shared fix each)", "",
            "| spell | name | total missing DPS | specs |", "|---|---|---:|---|"]
    head += [f"| {spell} | {name} | {total:,.0f} | {', '.join(f'{s}/{c}T' for s, c, _ in v)} |"
             for total, (spell, name), v in multi]
    text = "\n".join(head + lines) + "\n"
    if args.output:
        args.output.write_text(text)
    print(text)


if __name__ == "__main__":
    main()
