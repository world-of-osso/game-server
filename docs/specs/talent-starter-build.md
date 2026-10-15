# Talent Starter Build

Server-owned built-in talent allocation from installed `TraitTreeLoadout`/`TraitTreeLoadoutEntry` rows. [Implementation](../wiki/systems/talents.md#starter-build).

## What it must do
- [ ] Advertise availability only for the active specialization's authored build; never create a user-named Starter config.
- [ ] Activate the eligible, affordable authored allocation (including deferring affordable future-level apex rows), update effective spells and publish authoritative selection/allocation snapshots.
- [ ] Persist built-in selection across login and specialization switches; automatically allocate newly earned points on level changes.
- [ ] Accepted manual talent commits leave Starter mode; rejected mutations preserve allocation and selection. Saved named configurations remain independent.

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

## Known gaps (current cycle)
- [ ] Coupled protocol/server/engine proof and both-skin native acceptance pending.

## Out of scope
New builds not authored in installed data, inferred hero picks, action-bar policies and unrelated talent rules.
