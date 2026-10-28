#!/usr/bin/env python3
"""Generate native support-spec inputs from pinned MID2 gear and sourced talents."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from dps_parity import parse_profile, parse_selected_talents

MANIFEST = Path(__file__).with_name("mid2_support_manifest.json")
REVISION = "76f8118cf3f7f41dad20708e8328e1f4ab2deebc"


def generate(spec, rule, simc, wrapper, output):
    donor_path = simc / "profiles" / "MID2" / f"MID2_{rule['donor']}.simc"
    donor_text = donor_path.read_text()
    donor = parse_profile(donor_text)
    for item in donor["equipment"]:
        if item["slot"] in rule.get("item_replacements", {}):
            item["item_id"] = rule["item_replacements"][item["slot"]]
    profile = {
        "spec_id": int(spec), "class": rule["class"], "spec": rule["spec"],
        "name": f"MID2_Support_{spec}", "level": donor["level"], "race": rule.get("race", donor["race"]),
        "role": "tank" if spec == "104" else "heal", "position": "back",
        "equipment": donor["equipment"], "consumables": donor["consumables"],
        "actions": [], "talents": rule["talents"], "reference_tier": "MID2",
        "simc_revision": REVISION, "simc_build": 70391, "server_build": 70448,
        "gear_source": str(donor_path),
        "gear_source_sha256": hashlib.sha256(donor_text.encode()).hexdigest(),
        "talent_source": rule["source"],
    }
    metadata = output / f"{spec}.metadata.simc"
    metadata.write_text(
        f'{rule["class"]}="MID2_Support_{spec}"\n'
        f'level={profile["level"]}\nrace={profile["race"]}\nspec={rule["spec"]}\n'
        f'role={profile["role"]}\ntalents={rule["talents"]}\n'
        'target_level=90\nallow_experimental_specializations=1\nactions=wait,sec=0.1\n'
    )
    traits_path = output / f"{spec}.selected-traits-active.json"
    subprocess.run([str(wrapper), str(traits_path), str(metadata)], check=True)
    traits = [trait for trait in parse_selected_talents(traits_path.read_text())
              if trait["node_id"] not in rule.get("excluded_nodes", [])]
    traits_path.write_text(json.dumps(traits, indent=2) + "\n")
    (output / f"{spec}.profile.json").write_text(json.dumps(profile, indent=2) + "\n")
    metadata.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simc", type=Path, required=True, help="pinned SimC checkout")
    parser.add_argument("--wrapper", type=Path, required=True, help="simc_selected_traits executable")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--spec", action="append", help="generate only this spec; repeatable")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    specs = args.spec or list(manifest["specs"])
    revision = subprocess.run(["git", "-C", str(args.simc), "rev-parse", "HEAD"],
                              text=True, capture_output=True, check=True).stdout.strip()
    if revision != REVISION:
        raise ValueError(f"SimC checkout is {revision}, expected {REVISION}")
    args.output.mkdir(parents=True, exist_ok=True)
    for spec in specs:
        generate(spec, manifest["specs"][spec], args.simc, args.wrapper, args.output)
    (args.output / "mid2-support-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
