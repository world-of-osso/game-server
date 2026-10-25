# Shared parity frontal-pack proof ledger

## Scope

Approved harness-only correction from master `a8857beac1b97aa9ffd36423126bc3415ccb137f`. The three-target reference assumes three geometrically eligible opponents. Preserve native31146, target identities/lifetimes, the primary's1.5yd distance, spell cones/radii and all gameplay. No parity gating tests, APL, gear, powers, coefficients or native producers change. The common constructor applies to every class and every replacement wave; this does not count as a gameplay fix.

## Cause and change

Old WoW-space positions `[1.5,index,0]` put the second/third opponents at33.69°/53.13° from the stationary melee actor. Native117418 has60° full ConeDegrees. TC's strict close boundary admits the second at1.803yd but not the third at2.5yd; gameplay correctly excludes the third.

The common constructor alternates lateral half-yard steps:0,−0.5,+0.5,−1,+1,…; the measured three-target pack is within±18.435° and1.582yd. The primary remains unchanged; higher-count native fixtures remain supported without a three-entry indexing cap. No Monk/class/spell switch is added.

## Proof

|Phase|Exact source|Command/scope|Result|
|---|---|---|---|
|RED|`1496b0e0d4f4229120da7d06af95aab7c1b52bdd`|`cargo test -p server-simulation --lib -j2 --no-run` through admitted agent-run|PASS441.33s, zero warnings|
|RED|same|`literal_native_frontal_pack_ --ignored --nocapture --test-threads=1` through frozen native wrapper|Two genuine REDs12.83s: actual FoF reaches2/3 real identities; common fixture rejects WW off-arc target. Mage geometry control passes before the WW assertion. No fabricated power/aura/stat/target data.|
|GREEN|Pending|Same native cases after only shared constructor change|Pending|

Evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ww-parity-2026-10-23/`. Compiler: `frontal-pack-red-build.{log,proof.json}`. Native: `native-frontal-pack-red/` retains currentcanonical4442/private SQLite backup, frozen269 and63 inputs, binary hash and runtime receipt. `frontal-pack-proof-ledger.json` records command scope. Native RED binary SHA256 `f635682f7d2a3669fb53bcec9b84b3c0a82b3d10aadd5df9553cea283c9cff6a`.

After GREEN, a new unpublished measurement-only descendant of frozen `085cd8e9d` receives only the harness commits. Run3T300s once with a fresh canonical backup; retain the prior ST receipt unchanged. Do not merge unrelated current-master gameplay into that controlled comparison. Measurement and lead integration gates remain open.

## Sources

- [DPS parity contract](../specs/dps-parity.md).
- Shared constructor: `crates/server-simulation/src/dungeon_simulator/dps_parity/mod.rs::spawn_dummy`; native regressions in `windwalker_rotation/tests.rs`.
- Current native117418 SpellTargetRestrictions.ConeDegrees60; TC a352b1fa Unit.cpp712–720 and Spell.cpp9476 preserve the correct close-boundary exception. [Simulator wiki](../wiki/systems/dungeon-simulator.md#frontal-parity-pack--2026-10-24).
