# Healer and tank replay

Replay recorded incoming damage in the native server combat simulation to compare healer and tank performance on the same timeline.

## What it must do

- [ ] Load incoming hits with time, damage school, raw amount and target role. Reject invalid timelines rather than silently changing them. An optional `--party-trace` replays SimC raid-event source rolls onto the five native party slots alongside the tank trace.
- [ ] Prefer SimulationCraft tank-dummy output as the timeline source; also accept recorded World of Osso runs. Preserve the distinction between swings, strikes and bleeds. SimC direct damage amounts are source rolls before hit outcomes and must not be reported as post-crit damage.
- [ ] Use `/home/osso/Projects/world-of-osso/game-server/data/world.db` for every measurement. Use level-90 MID2 profile gear and selected talents for max-level measurements; Stockade is a smoke test. The replay party is single-faction; the healer adopts the tank's faction template before setup advances the world, while retaining its profile race and stats.
- [ ] Replay hits against a dummy party through native combat, with Holy Paladin (65) as the first healer and Protection Paladin (66) as the first tank. Extend the same harness to every healer and tank specialization and their spells. The initial Holy Paladin benchmark uses its native learned Holy Light and Flash of Light; this is not complete spell coverage.
- [ ] For each healer report effective HPS, raw HPS, overheal percentage, per-spell HPS, mana over time, time to first party death, and idle-GCD percentage. Idle-GCD percentage is the fraction of fight time sampled at 50 ms when the living healer has no active cast or channel and no active GCD, after the policy has acted; dead time is not idle. Sample mana at one-second intervals. Use the recorded combat duration for HPS and report no death as null.
- [ ] For each tank report raw and mitigated damage taken and mitigation percentage, self-healing and consumed absorbs per second, time to death without a healer, active-mitigation uptime, and largest five-second damage spike. Raw incoming damage includes avoided attacks; mitigation includes avoidance. Damage taken and spike include self-sourced periodic damage from tank mechanics. Stop the tank measurement at death; report no death as null. For the initial Protection Paladin, active mitigation is Shield of the Righteous.
- [ ] In SimC dummy comparison mode, apply and report the trace-declared Mark of the Wild, Power Word: Fortitude, Battle Shout and Skyfury raid buffs and the target's Chaos Brand, Mystic Touch and Hunter's Mark debuffs. Do not add them to raid-free DPS parity or no-healer replay.
- [ ] Mistweaver Mana Tea uses native-description-derived spending, stack cap, channel consumption and mana restoration. The pinned SimC source has no Mistweaver Mana Tea model.
- [ ] Print report tables with `scripts/healer_tank_replay.py`. SimC healer reports include frame-sampled minimum healer mana, minimum tank health including immediately after each incoming hit, and tank damage receipts per replay second.
- [ ] `--all` runs all seven healers on the Protection Paladin trace and six tanks on their respective 300-second seed-70448 SimC dummy traces, with both externally healed and no-healer tank runs. Run replays in parallel and print healer and tank summary tables; the healer table includes idle-GCD percentage. Write per-spell details in dated output files.
- [ ] See [Restoration Shaman (264)](healer-tank-replay/264.md) for its replay policy and data gaps.

## Specializations

- [Restoration Druid 105](healer-tank-replay/105.md)
- [Preservation Evoker 1468](healer-tank-replay/1468.md)

## Specializations

- [Brewmaster (268)](healer-tank-replay/268.md)

## Specialization notes

- [Guardian Druid (104)](healer-tank-replay/104.md)

## SimC party-pressure trace

The second canonical healer input is `game-engine/data/diagnostics/healer-tank-replay/66.simc-party-pulse-seed70448.debug.log`. It adds a 20,000 Shadow pulse every 12 seconds from 8 seconds through 296 seconds. This is a reproducible synthetic dungeon-pressure scenario, not a measured dungeon encounter. SimC `76f8118` generates the five recipient source rolls with the MID2 Protection profile and four named copies; the native replay maps them to its tank, healer and three DPS followers. The original tank-only trace still supplies the tank's melee and bleed rolls.

Generate it with the pinned `engine/simc`, the MID2 Protection profile, `copy=party_healer copy=party_damage_1 copy=party_damage_2 copy=party_damage_3 tank_dummy=Tank_Dummy_Dungeon iterations=1 threads=1 max_time=300 fixed_time=1 vary_combat_length=0 seed=70448 debug=1 'raid_events+=/damage,first=8,cooldown=12,amount=20000,type=shadow'` and the canonical output path. The copied SimC actors supply event slots, not reference healer or DPS behavior.

## How it works

- [Replay system](../wiki/systems/healer-tank-replay.md)

## Tank per-spell reporting

The SimC tank report includes raw and effective self-healing and outgoing periodic damage tick counts per spell. Its fixed-time target health uses the same encounter clock as the DPS parity fixture, so execute-phase abilities use the projected target health.

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

## Specializations

- [Protection Paladin (66)](healer-tank-replay/66.md)
- [Protection Warrior (73)](healer-tank-replay/73.md)
- [Discipline Priest (256)](healer-tank-replay/256.md)
- [Holy Priest (257)](healer-tank-replay/257.md)

## Known gaps (current cycle)

- [Blood Death Knight 250](healer-tank-replay/250.md): spec-specific replay notes and known reference differences.

- [x] Guardian Ursoc's Fury grants the native cumulative 372505 absorb from actual Thrash and Maul-family damage, including Thrash ticks, capped at 30% max health. Source: `sc_druid.cpp5148–5188,5600–5604,5912–5920`.
- [ ] Connect SimC tank-dummy trace input to the healer replay.
- [ ] Register native tank APL adapters. Guardian 104 uses the pinned default Guardian APL and reports Ironfur, Frenzied Regeneration, Barkskin and Rage of the Sleeper as active mitigation. SimC tank replay requires a registered literal parity driver and explicit active-mitigation auras; unregistered tanks fail rather than use the smoke-test policy.
- [ ] Support replay durations other than 300 seconds with matching literal-APL encounter timing.
- [x] Publish native victim-side melee miss, dodge and parry proc events with their hit outcomes; explicit native or sourced script masks determine which auras can react.
- [x] Report adapter-registered named resource counters and target debuff uptime. Vengeance 581 records successful fragment generation, actual fragment consumption, and the owned Fiery Brand and Sigil of Flame debuffs.
- [x] Report attempted and effective tank Energize combat-log amounts by spell. Events do not identify the resource type; a spell that grants multiple resources combines them, and changes without an Energize event are not included.
- [x] `--seeds N` runs consecutive native tank seeds starting at `--seed` (default 70448) against the same SimC trace and reports mean and min/max casts, aura uptimes and mitigation, plus death counts. Missing spell entries count as zero; unavailable mitigation is N/A. `--all` saves each tank seed's report separately and runs each healer once with seed 70448.
- [x] SimC tank JSON reports split damage taken into dummy swings, strikes, bleeds and self-periodic damage.
- [x] Keep no-healer tank replay as the default. An optional SimC dummy comparison applies the dummy's 75,000 external heal every five seconds and the trace's native Bloodlust at its recorded start time; it reports the buff, per-spell casts, effective self-healing, consumed absorbs and aura uptime separately.
- [ ] Extend coverage beyond the initial two specializations.

## Out of scope

- Deployment; this is a local simulation harness.
