#!/usr/bin/env python3
"""Generate the normal-dungeon follower profile roster from native and pinned SimC data."""
import argparse
import hashlib
import json
import sqlite3
import subprocess
from pathlib import Path

from dps_parity import parse_profile


def normalized(value):
    return "".join(c for c in value.lower() if c.isalnum())


def generate(db, simc, parity):
    classes = {normalized(name): ident for ident, name in db.execute("SELECT ID, Filename FROM chr_classes")}
    specs = {(class_id, normalized(name)): (ident, role) for ident, name, class_id, role in db.execute("SELECT ID, Name_lang, ClassID, Role FROM chr_specialization WHERE Name_lang <> 'Initial'")}
    races = {}
    for ident, name in db.execute("SELECT ID, Name_lang FROM chr_races ORDER BY ID"):
        races.setdefault(normalized(name), ident)
    profiles = {}
    # The shortest filename selects the default profile, not a named hero variant.
    files = [path for tier in ("MID2", "MID1") for path in sorted(
        (simc / "profiles" / tier).glob("*.simc"), key=lambda p: (len(p.name), p.name)
    )]
    for path in files:
        payload = path.read_bytes()
        profile = parse_profile(payload.decode())
        class_id = classes.get(normalized(profile.get("class", "")))
        spec = specs.get((class_id, normalized(profile.get("spec", ""))))
        race = races.get(normalized(profile.get("race", "")))
        if not spec or not race or not profile.get("talents"):
            continue
        spec_id, role = spec
        profiles.setdefault(spec_id, {"spec_id": spec_id, "class_id": class_id, "role": role, "race": race, "talents": profile["talents"], "race_source": "RACE_FROM_SIMC_PROFILE", "source": f"{path.parent.name}/{path.name}", "sha256": hashlib.sha256(payload).hexdigest()})
    for path in sorted(parity.glob("*.profile.json")):
        payload = path.read_bytes()
        profile = json.loads(payload)
        class_id = classes.get(normalized(profile.get("class", "")))
        spec = specs.get((class_id, normalized(profile.get("spec", ""))))
        race = races.get(normalized(profile.get("race", "")))
        if not spec or not race or not profile.get("talents") or spec[0] != profile.get("spec_id"):
            continue
        spec_id, role = spec
        row = {"spec_id": spec_id, "class_id": class_id, "role": role, "race": race, "talents": profile["talents"], "race_source": "RACE_FROM_PARITY_PROFILE", "source": path.name, "sha256": hashlib.sha256(payload).hexdigest()}
        # The pinned SimC tree has only the obsolete MID1 Guardian loadout.
        # The tracked MID2 parity profile is a current, native-decodable build.
        if spec_id == 104 and profile.get("reference_tier") == "MID2":
            profiles[spec_id] = row
        else:
            profiles.setdefault(spec_id, row)
    return [profiles[key] for key in sorted(profiles)]


def apply_mid2_overrides(profiles, directory):
    if not directory.is_dir():
        raise ValueError(f'MID2 follower overrides directory not found: {directory}')
    by_spec = {profile['spec_id']: profile for profile in profiles}
    for path in sorted(directory.glob('*.json')):
        payload = path.read_bytes()
        source = json.loads(payload)
        profile = by_spec[source['spec_id']]
        if source['class_id'] != profile['class_id'] or source['reference_tier'] != 'MID2':
            raise ValueError(f'invalid MID2 follower profile {path}')
        profile['talents'] = source['talents']
        profile['source'] = f'class-follower-mid2/{path.name}'
        profile['sha256'] = hashlib.sha256(payload).hexdigest()
    return profiles


def decode_profiles(profiles, decoder, db2):
    completed = subprocess.run(
        [str(decoder), str(db2)], input=json.dumps(profiles), text=True,
        capture_output=True, check=True,
    )
    results = {row['spec_id']: row for row in json.loads(completed.stdout)}
    if len(results) != len(profiles) or set(results) != {row['spec_id'] for row in profiles}:
        raise ValueError('decoder did not return exactly one result per spec')
    for profile in profiles:
        result = results[profile['spec_id']]
        profile['talents_sha256'] = hashlib.sha256(profile['talents'].encode()).hexdigest()
        profile['talent_entries'] = result['entries']
        profile['rejection'] = result['error']
    return profiles


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--simc", type=Path, required=True)
    parser.add_argument("--parity", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--decoder", type=Path, required=True)
    parser.add_argument("--db2", type=Path, required=True)
    parser.add_argument("--mid2-overrides", type=Path, default=Path(__file__).resolve().parent / 'reference/class-follower-mid2')
    args = parser.parse_args()
    with sqlite3.connect(f"file:{args.world_db}?mode=ro", uri=True) as db:
        profiles = generate(db, args.simc, args.parity)
    profiles = apply_mid2_overrides(profiles, args.mid2_overrides)
    profiles = decode_profiles(profiles, args.decoder, args.db2)
    args.output.write_text(json.dumps(profiles, indent=2) + "\n")


if __name__ == "__main__":
    main()
