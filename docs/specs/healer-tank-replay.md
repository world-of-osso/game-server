# Healer and tank replay

Replay recorded incoming damage in the native server combat simulation to compare healer and tank performance on the same timeline.

## What it must do

- [ ] Load incoming hits with time, damage school, raw amount and target role. Reject invalid timelines rather than silently changing them.
- [ ] Prefer SimulationCraft tank-dummy output as the timeline source; also accept recorded World of Osso runs. Preserve the distinction between swings, strikes and bleeds. SimC direct damage amounts are source rolls before hit outcomes and must not be reported as post-crit damage.
- [ ] Use `/home/osso/Projects/world-of-osso/game-server/data/world.db` for every measurement. Use level-90 MID2 profile gear and selected talents for max-level measurements; Stockade is a smoke test.
- [ ] Replay hits against a dummy party through native combat, with Holy Paladin (65) as the first healer and Protection Paladin (66) as the first tank. Extend the same harness to every healer and tank specialization and their spells. The initial Holy Paladin benchmark uses its native learned Holy Light and Flash of Light; this is not complete spell coverage.
- [ ] For each healer report effective HPS, raw HPS, overheal percentage, per-spell HPS, mana over time, and time to first party death. Sample mana at one-second intervals. Use the recorded combat duration for HPS and report no death as null.
- [ ] For each tank report raw and mitigated damage taken and mitigation percentage, self-healing and consumed absorbs per second, time to death without a healer, active-mitigation uptime, and largest five-second damage spike. Raw incoming damage includes avoided attacks; mitigation includes avoidance. Stop the tank measurement at death; report no death as null. For the initial Protection Paladin, active mitigation is Shield of the Righteous.
- [ ] Print report tables with `scripts/healer_tank_replay.py`.

## How it works

- [Replay system](../wiki/systems/healer-tank-replay.md)

## Implementation inventory

- `crates/server-simulation/src/healer_tank_timeline.rs`: recorded incoming-hit timeline loader.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC tank-dummy debug trace reader.
- `crates/server-combat/src/followers/simulator/healer_profiles.rs`: benchmark-only healer configuration and initial Holy Paladin decisions.
- `crates/server-simulation/src/dungeon_simulator/healer_metrics.rs` and `healer_replay.rs`: native healing receipts and recorded-timeline metrics.
- `crates/server-simulation/src/dungeon_simulator/tank_replay.rs`: isolated native tank replay and metrics.
- `scripts/healer_tank_replay.py`: SimC-dummy and native recorded-intake report tables.

## Tests asserting this spec

- `crates/server-simulation/src/healer_tank_timeline.rs`: concrete timeline loader test.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC dummy raw damage trace test.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_replay_tests.rs`: native Holy Paladin replay of recorded incoming damage.
- `crates/server-simulation/src/dungeon_simulator/tank_replay_tests.rs`: native Protection Paladin replay without a healer.
- `crates/server-simulation/src/dungeon_simulator/replay_cli_tests.rs`: CLI output from a native tank replay.

## Known gaps (current cycle)

- [ ] Connect SimC tank-dummy trace input to the healer replay.
- [ ] Register native tank APL adapters. SimC tank replay requires a registered literal parity driver and explicit active-mitigation auras; unregistered tanks fail rather than use the smoke-test policy. Protection 66 executes the pinned MID2 literal APL; Holy Bulwark absorbs, Shining Light–driven Word of Glory and Divine Guidance are exercised under incoming damage. Other native spell effects still require qualification.
- [ ] Support replay durations other than 300 seconds with matching literal-APL encounter timing.
- [x] Publish native victim-side melee miss, dodge and parry proc events with their hit outcomes; explicit native or sourced script masks determine which auras can react.
- [ ] SimC's default `fake_solidarity` adds allied Reflection of Radiance opportunities that the solo native tank replay does not reproduce. Its 35 Holy Bulwark–sourced Grand Crusader procs include those simulated allied effects; their separate share is not observable from that run.
- [x] Keep no-healer tank replay as the default. An optional SimC dummy comparison applies the dummy's 75,000 external heal every five seconds and reports per-spell casts, effective self-healing, consumed absorbs and aura uptime separately.
- [ ] Extend coverage beyond the initial two specializations.

## Out of scope

- Deployment; this is a local simulation harness.
