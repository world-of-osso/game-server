# Healer and tank replay

Replay recorded incoming damage in the native server combat simulation to compare healer and tank performance on the same timeline.

## What it must do

- [ ] Load incoming hits with time, damage school, raw amount and target role. Reject invalid timelines rather than silently changing them.
- [ ] Prefer SimulationCraft tank-dummy output as the timeline source; also accept recorded World of Osso runs. Preserve the distinction between swings, strikes and bleeds. SimC direct damage amounts are source rolls before hit outcomes and must not be reported as post-crit damage.
- [ ] Use `/home/osso/Projects/world-of-osso/game-server/data/world.db` for every measurement. Use level-90 MID2 profile gear and selected talents for max-level measurements; Stockade is a smoke test. The replay party is single-faction; the healer adopts the tank's faction template while retaining its profile race and stats.
- [ ] Replay hits against a dummy party through native combat, with Holy Paladin (65) as the first healer and Protection Paladin (66) as the first tank. Extend the same harness to every healer and tank specialization and their spells. The initial Holy Paladin benchmark uses its native learned Holy Light and Flash of Light; this is not complete spell coverage.
- [ ] For each healer report effective HPS, raw HPS, overheal percentage, per-spell HPS, mana over time, and time to first party death. Sample mana at one-second intervals. Use the recorded combat duration for HPS and report no death as null.
- [ ] For each tank report raw and mitigated damage taken and mitigation percentage, self-healing and consumed absorbs per second, time to death without a healer, active-mitigation uptime, and largest five-second damage spike. Raw incoming damage includes avoided attacks; mitigation includes avoidance. Damage taken and spike include self-sourced periodic damage from tank mechanics. Stop the tank measurement at death; report no death as null. For the initial Protection Paladin, active mitigation is Shield of the Righteous.
- [ ] Print report tables with `scripts/healer_tank_replay.py`.
- [ ] Mistweaver Mana Tea uses native-description-derived spending, stack cap, channel consumption and mana restoration. The pinned SimC source has no Mistweaver Mana Tea model.

## How it works

- [Replay system](../wiki/systems/healer-tank-replay.md)

## Implementation inventory

- `crates/server-simulation/src/healer_tank_timeline.rs`: recorded incoming-hit timeline loader.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC tank-dummy debug trace reader.
- `crates/server-combat/src/followers/simulator/healer_profiles.rs`: benchmark-only healer configuration and initial Holy Paladin decisions.
- `crates/server-simulation/src/dungeon_simulator/healer_metrics.rs` and `healer_replay.rs`: native healing receipts and recorded-timeline metrics.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_simc.rs`: Holy Paladin triage policy for the two-player SimC dummy replay; this is not a complete Holy spell policy.
- `crates/server-simulation/src/dungeon_simulator/tank_replay.rs`: isolated native tank replay and metrics.
- `scripts/healer_tank_replay.py`: SimC-dummy and native recorded-intake report tables.

## Tests asserting this spec

- `crates/server-simulation/src/healer_tank_timeline.rs`: concrete timeline loader test.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC dummy raw damage trace test.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_replay_tests.rs`: native Holy Paladin replay of recorded incoming damage.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_simc_tests.rs`: MID2 Holy Paladin healing a native MID2 Protection tank on the SimC timeline.
- `crates/server-simulation/src/dungeon_simulator/tank_replay_tests.rs`: native Protection Paladin replay without a healer.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/brewmaster_stagger_tests.rs`: Brewmaster incoming damage, Stagger ticks and Purifying Brew.
- `crates/server-simulation/src/dungeon_simulator/replay_cli_tests.rs`: CLI output from a native tank replay.

## Known gaps (current cycle)

- [ ] Blood 250: SimC `76f8118` applies Hemostasis twice to Death Strike healing (parsed action modifier and `death_strike_heal_t::action_multiplier`). Its debug trace shows `1.08²` for one 8% stack, about 8% more than one application. The source does not identify this as an in-game behavior; native gameplay applies it once. Blood Shield inherits the raw-heal difference.

- [x] Guardian Ursoc's Fury grants the native cumulative 372505 absorb from actual Thrash and Maul-family damage, including Thrash ticks, capped at 30% max health. Source: `sc_druid.cpp5148–5188,5600–5604,5912–5920`.
- [ ] Connect SimC tank-dummy trace input to the healer replay.
- [ ] Register native tank APL adapters. Guardian 104 uses the pinned default Guardian APL and reports Ironfur, Frenzied Regeneration, Barkskin and Rage of the Sleeper as active mitigation. SimC tank replay requires a registered literal parity driver and explicit active-mitigation auras; unregistered tanks fail rather than use the smoke-test policy. Protection 66 executes the pinned MID2 literal APL; Holy Bulwark absorbs, Shining Light–driven Word of Glory, Divine Guidance and Grand Crusader's Avenger's Shield reset and Crusader's Judgment recharge are exercised under incoming damage. Other native spell effects and Grand Crusader sources still require qualification.
- [ ] Support replay durations other than 300 seconds with matching literal-APL encounter timing.
- [x] Publish native victim-side melee miss, dodge and parry proc events with their hit outcomes; explicit native or sourced script masks determine which auras can react.
- [ ] SimC's default `fake_solidarity` adds allied Reflection of Radiance opportunities that the solo native tank replay does not reproduce. Its 35 Holy Bulwark–sourced Grand Crusader procs include those simulated allied effects; their separate share is not observable from that run.
- [x] Report adapter-registered named resource counters and target debuff uptime. Vengeance 581 records successful fragment generation, actual fragment consumption, and the owned Fiery Brand and Sigil of Flame debuffs.
- [x] Keep no-healer tank replay as the default. An optional SimC dummy comparison applies the dummy's 75,000 external heal every five seconds and the trace's native Bloodlust at its recorded start time; it reports the buff, per-spell casts, effective self-healing, consumed absorbs and aura uptime separately.
- [ ] Extend coverage beyond the initial two specializations.

## Out of scope

- Deployment; this is a local simulation harness.
