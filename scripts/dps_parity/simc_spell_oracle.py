#!/usr/bin/env python3
"""Pinned SimC single-action factor oracle. Python 3 standard library only."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time

REVISION = "76f8118cf3f7f41dad20708e8328e1f4ab2deebc"
SIMC_ROOT = Path("/home/osso/.worktrees/simc-76f8118")
BINARY = SIMC_ROOT / "engine/simc"
RUNNER = Path("/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run")
DEFAULT_OUTPUT = Path("/var/tmp/osso/simc-spell-oracle")
REFERENCE_ROOT = Path("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/dps-parity-2026-10-21")
# Only short-probe controls and report destinations differ from the reference.
PROBE_OPTION_KEYS = {"desired_targets", "max_time", "iterations", "threads", "deterministic", "log", "debug", "output", "json", "json2", "html", "xml", "csv"}
ACTION = re.compile(r"Action '([^']+)' \((\d+)\)")
AMOUNT = re.compile(r"([\d.]+) (?:Player|Pet|Enemy) '(.+?)' (direct|tick) amount for Action '([^']+)' \((\d+)\)(.*)")
RESULT = re.compile(r"([\d.]+) (\S+) (\S+) ([^:]+): (.*\bresult=.*)")
STARTUP = re.compile(r"^[\d.]+ (.+?) \((\d+)\) eff#(\d+) modifying (.+?) \((\d+)\) (\S+)(.*)")
EFFECT = re.compile(r"action-effects: Action '([^']+)' \((\d+)\) (.+?) modified by (.+?) (per consumed stack of buff|per stack of buff|with buff|from) (.+?) \((\d+)#(\d+)\)")
ARMOR = re.compile(r"Action '([^']+)' \((\d+)\).* reduced by ([\d.]+)% from armor \(armor=([\d.]+), armor coeff=([\d.]+)\)")
KIND = {"direct_damage": "direct", "direct damage": "direct", "periodic_damage": "periodic", "tick damage": "periodic", "crit bonus multiplier": "crit_bonus", "auto attack": "auto_attack", "flat cost": "cost", "cost percent": "cost"}


def scalar(value):
    value = value.strip().rstrip(",")
    try:
        return float(value) if any(c in value.lower() for c in ".e") else int(value)
    except ValueError:
        return value


def factors(text):
    return {m[1]: scalar(m[2]) for m in re.finditer(r"\b(\w+)=((?:\{[^}]*\})|[^\s,]+)", text)}


def modifier_value(text):
    match = re.match(r"(?:by )?([-+\d.]+)(%|\*mastery)?", text.strip())
    return (scalar(match[1]), "percent" if match[2] == "%" else "mastery_coefficient" if match[2] else "printed_value") if match else (None, "not_printed")


def parse_modifier(line, number):
    actor_match = re.match(r"^[\d.]+ (.+?) \((\d+)\) eff#(\d+) modifying Player '(.+?)' (\S+)(.*)", line)
    if actor_match:
        source, source_id, effect, actor, kind, tail = actor_match.groups()
        value, unit = modifier_value(tail)
        return {"action": None, "spell_id": None, "actor": actor, "scope": "actor", "source_name": source, "source_spell_id": int(source_id), "effect_index": int(effect), "kind": KIND.get(kind, kind), "value": value, "unit": unit, "application": "passive", "line": number, "raw": line}
    match = EFFECT.search(line)
    if match:
        name, spell, kind, value, mode, source, source_id, effect = match.groups()
        override = "(value override)" in value
        dynamic = "value function" in value or "with condition" in value
        value, unit = modifier_value(value)
        application = {"per stack of buff": "per_buff_stack", "per consumed stack of buff": "per_consumed_buff_stack", "with buff": "buff_present"}.get(mode, "mastery_scaled" if unit == "mastery_coefficient" else "passive")
        if dynamic:
            application = "conditional_or_dynamic"
    else:
        match = STARTUP.search(line)
        if not match:
            return None
        source, source_id, effect, name, spell, kind, tail = match.groups()
        value, unit = modifier_value(tail)
        application = "passive"
        override = False
    return {"printed_kind": kind, "value_override": override, "action": name, "spell_id": int(spell), "source_name": source, "source_spell_id": int(source_id), "effect_index": int(effect), "kind": KIND.get(kind, kind), "value": value, "unit": unit, "application": application, "line": number, "raw": line}


def observed(events, key):
    values = {e["factors"][key] for e in events if key in e["factors"]}
    return sorted(values, key=str) if values else None


def summarize(events, modifiers):
    summaries = {}
    for spell_id in sorted({e["spell_id"] for e in events if e["spell_id"] is not None}):
        rows = [e for e in events if e["spell_id"] == spell_id]
        mods = [m for m in modifiers if m["spell_id"] == spell_id]
        passive = {}
        for kind in ("direct", "periodic", "auto_attack"):
            sources = {}
            for m in mods:
                if m["application"] == "passive" and m["kind"] == kind and m["unit"] == "percent":
                    sources[(m["source_spell_id"], m["effect_index"], m["value"])] = m
            itemised = list(sources.values())
            passive[kind] = {"product": math.prod(1 + m["value"] / 100 for m in itemised) if itemised else None, "sources": itemised, "scope": "printed passive percentage sources only; not a reconstruction of dynamic mult"}
        summary = {"names": sorted({e["action"] for e in rows}), "formula_records": sum(e["event"].endswith("_amount") for e in rows), "result_records": sum(e["event"] == "result" for e in rows), "passive_multipliers": passive}
        mapping = {"ap_coefficient": "a_mod", "sp_coefficient": "s_mod", "base": "base", "weapon": "weapon", "weapon_multiplier": "w_mult", "crit_chance": "crit", "target_crit_chance": "tgt_crit", "crit_bonus": "crit_bonus", "versatility": "versatility", "armor_mitigation_fraction": "armor_fraction"}
        summary.update({label: observed(rows, key) for label, key in mapping.items()})
        summary["target_multipliers"] = {k: observed(rows, k) for k in ("tgt_da_mul", "tgt_ta_mul", "tgt_mitg_da_mul", "tgt_mitg_ta_mul", "target_armor")}
        summary["dynamic_multipliers"] = {k: observed(rows, k) for k in ("mult", "da_mul", "ta_mul", "ply_mul", "vs_mul", "per_mul", "tick_mult", "rta_mult")}
        summaries[str(spell_id)] = summary
    return summaries


def parse_log(text):
    events, modifiers, unparsed = [], [], []
    identities = {}
    lines = text.splitlines()
    for line in lines:
        for name, spell in ACTION.findall(line):
            identities.setdefault(name, set()).add(int(spell))
    for number, line in enumerate(lines, 1):
        modifier = parse_modifier(line, number)
        if modifier:
            modifiers.append(modifier)
        match = AMOUNT.search(line)
        if match:
            stamp, actor, kind, name, spell, tail = match.groups()
            events.append({"event": "direct_amount" if kind == "direct" else "periodic_amount", "time": float(stamp), "actor": actor, "action": name, "spell_id": int(spell), "factors": factors(tail), "line": number, "raw": line})
            continue
        match = ARMOR.search(line)
        if match:
            name, spell, reduction, armor, coefficient = match.groups()
            events.append({"event": "armor", "time": float(line.split()[0]), "action": name, "spell_id": int(spell), "factors": {"armor_fraction": float(reduction) / 100, "armor": float(armor), "armor_coefficient": float(coefficient)}, "line": number, "raw": line})
            continue
        match = RESULT.fullmatch(line)
        if match:
            stamp, actor, name, target, tail = match.groups()
            ids = sorted(identities.get(name, []))
            events.append({"event": "result", "time": float(stamp), "actor": actor, "action": name, "target": target, "spell_id": ids[0] if len(ids) == 1 else None, "spell_id_candidates": ids, "factors": factors(tail), "line": number, "raw": line})
        elif any(token in line for token in (" amount for Action", " result=", "action-effects:", " eff#")) and not modifier:
            unparsed.append({"line": number, "raw": line})
    return {"schema_version": 1, "modifiers": modifiers, "events": events, "summary": summarize(events, modifiers), "unparsed_relevant_lines": unparsed, "limitations": ["All observed actors/children/procs retained; co-occurrence does not establish parent-child causality.", "Result logs contain multiple calculation stages, not independent casts. Use formula records for cast/tick counts; retain line ordering.", "Missing printed fields are null, never inferred; passive products exclude buff stacks and mastery-scaled effects.", "Spell IDs come from printed Action records; ambiguous result identities remain null with candidates."]}


def build_profile(source, actions, precombat_actions=None):
    precombat_actions = precombat_actions or []
    if not actions or any(not re.fullmatch(r"[a-zA-Z0-9_]+(?:,[^\r\n]*)?", action) for action in actions + precombat_actions):
        raise ValueError("at least one valid SimC action is required; no newlines")
    lines = [line for line in source.splitlines() if not re.match(r"\s*actions(?:[.+=\s]|$)", line)]
    if precombat_actions:
        lines += ["actions.precombat=" + precombat_actions[0]]
        lines += ["actions.precombat+=/" + action for action in precombat_actions[1:]]
    return "\n".join(lines) + "\n\nactions=" + actions[0] + "\n" + "".join("actions+=/" + action + "\n" for action in actions[1:])


def resolve_profile(value):
    path = Path(value).expanduser()
    if path.is_file():
        return path.resolve()
    name = re.sub(r"[^a-z0-9]", "", value.lower().removeprefix("mid2_"))
    suffix = "_" + re.sub(r"[ -]", "_", value.lower())
    matches = [p for p in (SIMC_ROOT / "profiles/MID2").glob("*.simc") if re.sub(r"[^a-z0-9]", "", p.stem.lower().removeprefix("mid2_")) == name or p.stem.lower().endswith(suffix)]
    if len(matches) != 1:
        raise ValueError(f"profile {value!r} not found or ambiguous; use full MID2 profile name or path")
    return matches[0]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def reference_options(proof):
    command = proof.get("command")
    if not isinstance(command, list) or not command:
        raise ValueError("reference proof must contain its recorded command")
    return [option for option in command if "=" in option and option.split("=", 1)[0] not in PROBE_OPTION_KEYS]


def resolve_reference_proof(profile, explicit=None):
    profile = Path(profile).resolve()
    candidates = [Path(explicit).resolve()] if explicit else sorted(REFERENCE_ROOT.rglob("*.single.proof.json"))
    matches = []
    for path in candidates:
        proof = json.loads(path.read_text())
        profiles = [Path(arg).resolve() for arg in proof.get("command", []) if arg.endswith(".simc")]
        if profiles == [profile]:
            matches.append((path, reference_options(proof)))
    if not matches:
        raise ValueError(f"no matching reference proof for {profile}; supply --reference-proof")
    if len({tuple(sorted(options)) for _, options in matches}) != 1:
        raise ValueError(f"conflicting reference options for {profile}; supply --reference-proof")
    return matches[0][0]


def run_oracle(profile, actions, output, targets=1, duration=30, reference_proof=None, precombat_actions=None):
    if targets < 1 or not math.isfinite(duration) or duration <= 0:
        raise ValueError("targets and time must be positive")
    profile = resolve_profile(str(profile))
    reference_path = resolve_reference_proof(profile, reference_proof)
    reference = json.loads(reference_path.read_text())
    inherited = reference_options(reference)
    output = Path(output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    generated, log, proof_path = (output.with_suffix(s) for s in (".simc", ".txt", ".proof.json"))
    generated.write_text(build_profile(profile.read_text(), actions, precombat_actions))
    overrides = [f"desired_targets={targets}", f"max_time={duration}", "iterations=1", "threads=1", "deterministic=1", "log=1", "debug=1", f"output={log}"]
    command = [str(RUNNER), "simc-oracle", str(BINARY), str(generated), *inherited, *overrides]
    proof = {"precombat_actions": precombat_actions or [], "reference_proof": str(reference_path), "reference_proof_sha256": sha256(reference_path), "reference_command": reference["command"], "inherited_options": inherited, "probe_overrides": overrides, "command": command, "binary_sha256": sha256(BINARY), "profile": str(profile), "profile_sha256": sha256(profile), "generated_profile_sha256": sha256(generated), "revision": REVISION, "actions": actions, "targets": targets, "time": duration, "started_unix": time.time(), "environment": {"TMPDIR": "/var/tmp/osso"}}
    proof_path.write_text(json.dumps(proof, indent=2) + "\n")
    try:
        with output.with_suffix(".console.txt").open("w") as console:
            result = subprocess.run(command, stdout=console, stderr=subprocess.STDOUT, env={**os.environ, "TMPDIR": "/var/tmp/osso"}, check=False, timeout=180)
        proof["exit_code"] = result.returncode
    except (OSError, subprocess.TimeoutExpired) as exc:
        proof.update(exit_code=None, error=f"{type(exc).__name__}: {exc}")
        raise
    finally:
        proof["elapsed_seconds"] = time.time() - proof["started_unix"]
        proof_path.write_text(json.dumps(proof, indent=2) + "\n")
    if result.returncode:
        raise RuntimeError(f"SimC exited {result.returncode}; see {output.with_suffix('.console.txt')}")
    data = parse_log(log.read_text())
    proof["output_sha256"] = sha256(log)
    proof_path.write_text(json.dumps(proof, indent=2) + "\n")
    data["proof"] = proof
    data["artifacts"] = {"profile": str(generated), "log": str(log), "proof": str(proof_path)}
    output.write_text(json.dumps(data, indent=2) + "\n")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", required=True, help="path or MID2 profile/spec name, e.g. Warrior_Arms")
    parser.add_argument("--action", action="append", required=True, help="repeat for resource generators or proc drivers; order is APL priority")
    parser.add_argument("--precombat-action", action="append", help="explicit sourced one-time setup action; repeat in precombat order")
    parser.add_argument("--reference-proof", type=Path, help="reference command proof; default discovers matching single-target profile proof")
    parser.add_argument("--targets", type=int, default=1)
    parser.add_argument("--time", type=float, default=30)
    parser.add_argument("--output", type=Path, help="JSON path; adjacent .simc, .txt and .proof.json retained")
    args = parser.parse_args()
    output = args.output or DEFAULT_OUTPUT / f"{time.time_ns()}-{args.action[0].split(',')[0]}.json"
    try:
        data = run_oracle(args.profile, args.action, output, args.targets, args.time, args.reference_proof, args.precombat_action)
    except (ValueError, RuntimeError, OSError, subprocess.TimeoutExpired) as exc:
        parser.exit(1, f"oracle: {exc}\n")
    print(json.dumps({"output": str(output), "summary": data["summary"]}, indent=2))


if __name__ == "__main__":
    main()
