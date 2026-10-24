# Fire literal parity Fixture

NativePlayer63 qualification uses pinnedSimC76f8118 Fire action rows, retaining native numeric data/admission and the dungeon kit. [Shared native machinery](../wiki/systems/simc-apl.md).

## What it must do

- [ ] Interpret exact88rows/8lists with pinned ordering, expressions and variable semantics for explicit parityFixture only.
- [ ] Execute ordinary native Arcane Intellect→Mirror Image→Pyroblast precombat in source order before combat decisions, retaining actual Player identity, spellbook, selected traits and gear.
- [ ] Use shared native target/readiness/execution, including declared off-GCD/while-casting/item options; no Fire-only executor or unsupported-operand defaults.
- [ ] Read actual native Fire-specific conditions and delayed hits; preserve nativeDB2/TC authority, no grants/tuning/synthetic timers.
- [ ] Produce300sST/3T current-canonical measurements with source-view proof and accepted reference-fixed.three adds qualification; missing measurements are not zero.

## How it works

- [Shared evaluator and native Fixture machinery](../wiki/systems/simc-apl.md).

## Implementation inventory

- `crates/server-simulation/src/dungeon_simulator/dps_parity/fire_rotation/76f8118.simc` — pinned literal source data.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/fire_rotation/` — staged source/bindings adapter using shared Driver, Prepared, Equipment and native facts. Unsupported facts remain errors, not defaults.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/mod.rs` and `preparation.rs` — staged Player63 Fixture caller; shared poll preserves Waiting/Selected/Finished while actual App frames complete cast/GCD state. Combat clock starts after Finished; no flight-completion claim.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/literal_rotation/` — shared Driver, Prepared and Equipment; spec-local execution is out of scope.

## Tests asserting this spec

- `fire_rotation/tests.rs`: native90 recipient Firestarter90%/Scorch30% boundaries, explicit second-recipient selection, ordinary Pyroblast hardcast→launched travel→impact and overlapping native Pyroblast/Fireball travel counts. These RED candidates are not proof until the actual native run; no pending queue injection.

- `dps_parity/fire_literal_tests.rs`: actual native Player cast-order boundary; not yet executed.
- Existing `dps_parity_native_bis_dummy_records_300s`: prior native Player ST/3T stopped at SBA12616condition11 before combat. The staged caller avoids that SBA path but has not run; native/equipment fact gaps remain explicit, with no DPS proof.

## Known gaps (current cycle)

- [ ] Qualify staged Fire adapter/hookup after shared ordered-precombat/options and configured-duration dependencies land, preserving all literal rows. Landed rt1086 Equipment is integrated from master5d39f490a; set_bonus/equipped/trinket facts all delegate to it. Full caller runtime qualification remains open.
- [x] Qualify Fire-local native HP conditions and actual Hot Streak pending-event count; [scoped native RED/GREEN proof](../handoffs/fire-local-facts-proof-ledger.md).
- [ ] Consume/qualify delegated generic duration/react/cast/history and equipment fields in the full caller.
- [ ] Fresh combined runtime measurements and per-spell comparison after source/admission proof.

## Out of scope

Dungeon-kit changes, source/trait/stat grants, coefficient tuning, invented timers and a Fire-only executor. Shared Program/target, temporal/react/history, equipment/tier and execution/precombat extensions remain with their assigned owners.
