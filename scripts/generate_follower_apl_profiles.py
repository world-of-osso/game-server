#!/usr/bin/env python3
"""Write the pinned SimC profiles whose APLs production class followers run.

The same parser produces the DPS parity inputs, so a production follower and its
parity measurement read identical action lists, consumables and options.
Healers and Guardian already use tracked profiles under reference/healer-tank-top-logs.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from dps_parity import parse_profile

REVISION = "76f8118cf3f7f41dad20708e8328e1f4ab2deebc"
OUTPUT = Path(__file__).resolve().parents[1] / "crates/server-follower-apl/profiles"
SPECS = {
    62: "MID2_Mage_Arcane", 63: "MID2_Mage_Fire", 64: "MID2_Mage_Frost",
    66: "MID2_Paladin_Protection", 70: "MID2_Paladin_Retribution",
    71: "MID2_Warrior_Arms", 72: "MID2_Warrior_Fury", 73: "MID2_Warrior_Protection",
    102: "MID1_Druid_Balance", 103: "MID2_Druid_Feral",
    250: "MID2_Death_Knight_Blood", 251: "MID2_Death_Knight_Frost",
    252: "MID2_Death_Knight_Unholy", 253: "MID2_Hunter_Beast_Mastery",
    254: "MID2_Hunter_Marksmanship", 255: "MID2_Hunter_Survival", 258: "MID2_Priest_Shadow",
    259: "MID2_Rogue_Assassination", 260: "MID2_Rogue_Outlaw", 261: "MID2_Rogue_Subtlety",
    262: "MID2_Shaman_Elemental", 263: "MID2_Shaman_Enhancement",
    265: "MID2_Warlock_Affliction", 266: "MID2_Warlock_Demonology",
    267: "MID2_Warlock_Destruction", 268: "MID2_Monk_Brewmaster",
    269: "MID2_Monk_Windwalker", 577: "MID2_Demon_Hunter_Havoc",
    581: "MID2_Demon_Hunter_Vengeance", 1467: "MID1_Evoker_Devastation",
    1480: "MID2_Demon_Hunter_Devourer",
}


def build(spec, name, simc):
    relative = Path("profiles") / name.split("_", 1)[0] / f"{name}.simc"
    payload = (simc / relative).read_bytes()
    profile = parse_profile(payload.decode())
    profile.update({"spec_id": spec, "profile": str(relative),
                    "profile_sha256": hashlib.sha256(payload).hexdigest(),
                    "simc_revision": REVISION})
    return profile


def write(simc, output):
    revision = subprocess.run(["git", "-C", str(simc), "rev-parse", "HEAD"],
                              text=True, capture_output=True, check=True).stdout.strip()
    if revision != REVISION:
        raise ValueError(f"SimC checkout is {revision}, expected {REVISION}")
    output.mkdir(parents=True, exist_ok=True)
    for spec, name in SPECS.items():
        profile = build(spec, name, simc)
        (output / f"{spec}.profile.json").write_text(json.dumps(profile, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simc", type=Path, required=True, help="pinned SimC checkout")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    write(args.simc, args.output)


if __name__ == "__main__":
    main()
