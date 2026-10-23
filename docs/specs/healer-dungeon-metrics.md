# Dungeon healing metrics

The standing healer benchmark replays recorded dungeon incoming hits in the headless simulator, not a SimC HPS reference. Implementation lives in `crates/server/src/dungeon_simulator/`; [coefficient source ledger](../reports/healer-coefficient-ledger.md).

## What it must do

- [ ] Record one actual Stockade pull sequence, seed, tick and incoming raw damage, with stable source/party-slot identity and hit/crit flags; replay the same intake for every supported healer.
- [ ] Capture before target mitigation, then replay through native target armor, resistance, absorbs and damage reductions. Missing native mitigation remains a named gap, never a post-mitigation fallback.
- [ ] Report JSON and Markdown party deaths, overheal percentage, ending mana, idle GCD equivalents and effective healing per combat second for specs65,256,257,264,270,105,1468; blocked specs retain null metrics and named gaps.
- [ ] Prove extraction with a concrete200-raw/150-effective/50-overheal3s fight and repeatability with identical intake.
- [ ] Keep core-heal coefficient and native per-cast amount checks against DB2 and TrinityCore, separately from dungeon-policy performance.

## How it works

- [Dungeon simulator](../wiki/systems/dungeon-simulator.md)
- [Coefficient ledger](../reports/healer-coefficient-ledger.md)

## Implementation inventory

- `crates/server/src/dungeon_simulator/healer_metrics.rs`: frame/event observations and metrics.
- `crates/server/src/spell_cast/mod.rs`: observation-only native cast/GCD readiness seam.
- `docs/reports/healer-coefficient-ledger.json`: exact DB2 rows and source evidence.

## Tests asserting this spec

- `crates/server/src/dungeon_simulator/healer_benchmark_tests.rs`: concrete fight extraction and repeatability.
- `crates/server/src/dungeon_simulator/healer_coefficient_tests.rs`: isolated native direct-heal probes for all seven specs.

## Known gaps (current cycle)

- [ ] NATIVE_HEALER_PROFILE_ABSENT:65,256,264,270,1468. Only native105 Crenna209072 and257 Henry215011 run without new actor/policy work.
- [ ] Build/test proof pending capacity admission; RED fixtures committed and pushed before implementation.

## Out of scope

- Healer decision-policy changes, invented follower profiles, player-healer rotation policy and simulator map/route edits: lead/other-agent ownership.
- SimC healer HPS matching: no profiles at76f8118.
