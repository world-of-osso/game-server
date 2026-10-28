# Healer and tank replay

Replay recorded incoming damage in the native server combat simulation to compare healer and tank performance on the same timeline.

## What it must do

- [ ] Load incoming hits with time, damage school, raw amount and target role. Reject invalid timelines rather than silently changing them.
- [ ] Prefer SimulationCraft tank-dummy output as the timeline source; also accept recorded World of Osso runs.
- [ ] Use `/home/osso/Projects/world-of-osso/game-server/data/world.db` for every measurement.
- [ ] Replay hits against a dummy party through native combat, with Holy Paladin (65) as the first healer and Protection Paladin (66) as the first tank. Extend the same harness to every healer and tank specialization and their spells.
- [ ] For each healer report effective HPS, raw HPS, overheal percentage, per-spell HPS, mana over time, and time to first party death.
- [ ] For each tank report raw and mitigated damage taken and mitigation percentage, self-healing and absorbs per second, time to death without a healer, active-mitigation uptime, and largest short-window damage spike.
- [ ] Print report tables with `scripts/healer_tank_replay.py`.

## How it works

- [Replay system](../wiki/systems/healer-tank-replay.md)

## Implementation inventory

- `crates/server-simulation/src/healer_tank_timeline.rs`: recorded incoming-hit timeline loader.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC tank-dummy debug trace reader.

## Tests asserting this spec

- `crates/server-simulation/src/healer_tank_timeline.rs`: concrete timeline loader test.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC dummy raw damage trace test.

## Known gaps (current cycle)

- [ ] Import source timelines, run native replay, and produce healer and tank reports.
- [ ] Extend coverage beyond the initial two specializations.

## Out of scope

- Deployment; this is a local simulation harness.
