# Talent Starter Build

Server-owned built-in talent allocation from installed `TraitTreeLoadout`/`TraitTreeLoadoutEntry` rows. [Implementation](../wiki/systems/talents.md#starter-build).

## What it must do
- [x] Advertise availability only for the active specialization's authored build; never create a user-named Starter config.
- [x] Activate the eligible, affordable authored allocation (including deferring affordable future-level apex rows), update effective spells and publish authoritative selection/allocation snapshots.
- [x] Persist built-in selection across login and specialization switches; automatically allocate newly earned points on level changes.
- [x] Accepted manual talent commits leave Starter mode; rejected mutations preserve allocation and selection. Saved named configurations remain independent.

## How it works
- [Talent rules and persistence](../wiki/systems/talents.md#starter-build).

## Implementation inventory
- `crates/server/src/spell_info/trait_data.rs`: ordered authored loadouts.
- `crates/server/src/trait_config.rs`: affordable cumulative-rank construction and validation.
- `crates/server/src/class_progression.rs`: activation, snapshots, login/spec/level refresh and edit deactivation.
- `crates/server/src/persistence_progression.rs`: built-in selection uses the existing per-spec default allocation storage.

## Tests asserting this spec
- `crates/server/src/class_progression_loadout_tests.rs`: authoritative activation, level-up, relog and manual deactivation.
- `crates/server/src/trait_config_tests.rs`: real Paladin/Mage authored ranks and level budgets.

## Proof — 2026-10-15
Starter5 passes are scoped across unchanged allocator production code:4 passes at a3c95c40 (including every installed class build at10/11/70/80/90), plus the corrected exact Arms assertion atc5704b30. Arms apex110407 is absent at70/80, then has2/3/4 purchased ranks at81/84/90. Earlier22 talent/persistence regressions remain applicable outside the changed Starter allocator. Current scoped check and server build pass without warnings.

Real private UDP5881 with protocol8c90bfc and engine2eebb1ea4 passes both skins' level10→11→80 allocation, complete relog projections and confirmed edit deactivation. Evidence lives in engine `data/diagnostics/talent-import-export-2026-10-15/starter-*`; [client contract and captures](../../../game-engine/docs/specs/talent-loadouts.md#starter-proof--2026-10-15). No world/reference data edits or realm5000 operations.

## Known gaps (current cycle)
- [ ] Lead-owned coupled integration/landing gate; scoped source and both-skin native acceptance pass.

## Out of scope
New builds not authored in installed data, inferred hero picks, action-bar policies and unrelated talent rules.
