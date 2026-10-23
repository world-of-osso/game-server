# Dungeon healing metrics

The standing healer benchmark replays recorded dungeon incoming hits in the headless simulator, not a SimC HPS reference. Implementation lives in `crates/server/src/dungeon_simulator/`; [coefficient source ledger](../reports/healer-coefficient-ledger.md).

## What it must do

- [ ] Record one actual Stockade pull sequence, seed, tick and incoming raw damage, with stable source/party-slot identity and hit/crit flags; replay the same intake for every supported healer.
- [ ] Preserve recorded source native levels and per-spawn tuning deltas across healer-dependent initialization RNG; native tuning/identity drift is a named gap rather than an invented stat sheet.
- [ ] Capture before target mitigation, then replay through native target armor, resistance, absorbs and damage reductions. Preserve trigger-depth proc eligibility and separate damage-shield owner/bonus-caster identity. Missing native mitigation or caster identity remains a named gap, never a post-mitigation fallback.
- [ ] Report JSON and Markdown party deaths, overheal percentage, ending mana, idle GCD equivalents and effective healing per combat second for specs65,256,257,264,270,105,1468; blocked specs retain null metrics and named gaps.
- [ ] Use the frozen intake's recorded combat window for eHPS and combined effective healing-plus-absorb throughput, even after early wipes; retain native observed combat seconds separately.
- [ ] Attribute only actually consumed native healer-cast shield points to absorbed damage, excluding applied/unconsumed capacity and combat-log double counting; report absorbed damage and combined throughput in their own columns.
- [ ] Retain the native damage-meter consumed-point convention, not normalized prevented HP. Record each hit's native level-scaling factor and unit/non-unit counts; retain ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING for non-unit cases and report their share of consumed points. If all captured hits are unit scale, explicitly mark the gap moot with the count. Do not add a wider prevented-HP observer.
- [ ] Prove extraction with a concrete200-raw/150-effective/50-overheal3s fight and repeatability with identical intake.
- [ ] Keep core-heal coefficient and native per-cast amount checks against DB2 and TrinityCore, separately from dungeon-policy performance.

## How it works

- [Dungeon healing metrics](../wiki/systems/healer-dungeon-metrics.md)
- [Coefficient ledger](../reports/healer-coefficient-ledger.md)

## Implementation inventory

- `crates/server/src/dungeon_simulator/{healer_intake,healer_metrics,healer_replay,healer_report}.rs`: stable raw receipts, frame/event metrics, native replay and JSON/MD artifacts.
- `crates/server/src/{combat.rs,spell_cast/effects.rs}`: narrow native source-amount capture and recipient mitigation replay seams.
- `crates/server/src/followers/simulator.rs`: explicit native healer-profile selection; no policy changes.
- `crates/server/src/spell_cast/mod.rs`: observation-only native cast/GCD readiness seam.
- `crates/server/src/level_scaling.rs`: test-only accessor for the native damage-to-HP level factor.
- `docs/reports/healer-coefficient-ledger.json`: exact DB2 rows and source evidence.

## Tests asserting this spec

- `crates/server/src/dungeon_simulator/healer_benchmark_tests.rs`: concrete fight extraction and repeatability.
- `crates/server/src/dungeon_simulator/healer_intake_tests.rs`: raw-before-absorb capture, same-input native determinism and recipient reduction/armor/shield controls.
- `crates/server/src/dungeon_simulator/healer_report.rs`: explicit ignored actual Stockade capture and recorded-input report drivers.
- `crates/server/src/dungeon_simulator/healer_coefficient_tests.rs`: isolated native direct-heal probes for all seven specs.

## Known gaps (current cycle)

- [ ] NATIVE_HEALER_PROFILE_ABSENT:65,256,264,270,1468. Only native105 Crenna209072 and257 Henry215011 run without new actor/policy work.
- [ ] Build/test proof pending capacity admission; RED fixtures committed and pushed before implementation.

## Out of scope

- Healer decision-policy changes, invented follower profiles, player-healer rotation policy and simulator map/route edits: lead/other-agent ownership.
- SimC healer HPS matching: no profiles at76f8118.
