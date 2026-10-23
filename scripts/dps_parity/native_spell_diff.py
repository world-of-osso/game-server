#!/usr/bin/env python3
"""Compare one native impact/tick to one printed SimC impact, without filling gaps."""
import argparse
import hashlib
import json
import math
from pathlib import Path

FORMULA_EVENTS = {"direct_amount", "periodic_amount"}


def numeric(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def select_impact(receipt, spell_id, index=0):
    events = receipt.get("events", [])
    positions = [i for i, event in enumerate(events)
                 if event.get("spell_id") == spell_id and event.get("event") in FORMULA_EVENTS]
    if index < 0 or index >= len(positions):
        raise ValueError(f"spell{spell_id}: impact index{index} absent ({len(positions)} formula records)")
    first = events[positions[index]]
    identity = (first.get("actor"), first.get("action"))
    stop = next((i for i in positions[index + 1:]
                 if (events[i].get("actor"), events[i].get("action")) == identity), len(events))
    selected = {"formula": first.get("factors", {}), "result": {}, "armor": {},
                "attribution": dict(first.get("factor_attribution", {})),
                "sources": dict(first.get("factor_sources", {})), "time": first.get("time")}
    for event in events[positions[index] + 1:stop]:
        if event.get("spell_id") != spell_id or event.get("action") != first.get("action"):
            continue
        if event.get("actor") is not None and event["actor"] != first.get("actor"):
            continue
        stage = event.get("event")
        if stage in {"result", "armor"}:
            # SimC prints pre/post result states; the last one is authoritative.
            selected[stage] = event.get("factors", {})
            selected["attribution"].update(event.get("factor_attribution", {}))
            selected["sources"].update(event.get("factor_sources", {}))
    return selected


def compare_factor(stage, name, native, oracle, tolerance):
    if native is None:
        status = "not_printed_native"
    elif oracle is None:
        status = "not_printed_oracle"
    else:
        if not math.isfinite(native) or not math.isfinite(oracle):
            raise ValueError(f"nonfinite {stage}.{name}")
        status = "match" if math.isclose(native, oracle, rel_tol=tolerance, abs_tol=tolerance) else "different"
    delta = (native - oracle) * 100 / abs(oracle) if native is not None and oracle not in (None, 0) else None
    return {"stage": stage, "factor": name, "native": native, "simc": oracle,
            "delta_pct": delta, "status": status}


def diff_receipts(native, oracle, spell_id, native_index=0, oracle_index=0, tolerance=1e-6):
    if tolerance < 0 or not math.isfinite(tolerance):
        raise ValueError("tolerance must be finite and nonnegative")
    left = select_impact(native, spell_id, native_index)
    right = select_impact(oracle, spell_id, oracle_index)
    rows = []
    for stage in ("formula", "result", "armor"):
        names = {key for values in (left[stage], right[stage]) for key, value in values.items() if numeric(value)}
        for name in sorted(names):
            a, b = left[stage].get(name), right[stage].get(name)
            row = compare_factor(stage, name, a if numeric(a) else None, b if numeric(b) else None, tolerance)
            row["native_attribution"] = left["attribution"].get(name)
            row["native_sources"] = left["sources"].get(name, [])
            rows.append(row)
    statuses = {row["status"] for row in rows}
    status = "different" if "different" in statuses else "incomplete" if statuses - {"match"} else "match"
    return {"schema_version": 1, "spell_id": spell_id, "status": status, "factors": rows,
            "native_impact": native_index, "oracle_impact": oracle_index,
            "native_time": left["time"], "simc_time": right["time"],
            "native_result": left["result"].get("result"), "simc_result": right["result"].get("result"),
            "limitations": ["Ordinal impact selection does not align buffs, procs or independent crit rolls; inspect both recorded states.",
                            "Unprinted factors remain unavailable, never zero; unattributed products do not invent aura sources."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", type=Path, required=True)
    parser.add_argument("--oracle", type=Path, required=True)
    parser.add_argument("--spell-id", type=int, required=True)
    parser.add_argument("--native-impact", type=int, default=0)
    parser.add_argument("--oracle-impact", type=int, default=0)
    parser.add_argument("--tolerance", type=float, default=1e-6)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        native, oracle = (json.loads(path.read_text()) for path in (args.native, args.oracle))
        result = diff_receipts(native, oracle, args.spell_id, args.native_impact, args.oracle_impact, args.tolerance)
        result["proof"] = {label: {"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                           for label, path in (("native", args.native), ("oracle", args.oracle))}
        output = json.dumps(result, indent=2, allow_nan=False) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(output)
        print(output, end="")
    except (OSError, ValueError, TypeError) as error:
        parser.exit(2, f"spell diff: {error}\n")
    parser.exit(0 if result["status"] == "match" else 1)


if __name__ == "__main__":
    main()
