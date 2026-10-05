# Warlock pets

Summon Imp (688) uses the server's existing pet ownership, bar, movement and combat systems. Implementation: `crates/server/src/hunter_pet.rs`; [mechanics and references](../wiki/systems/warlock-pets.md).

## What it must do

- [ ] A new level-1 warlock knows Summon Imp; other classes do not receive it.
- [ ] Casting 688 creates an imp from effect 56's MiscValue next to its owner, with the owner's faction, a creature model and Firebolt's imported power pool (Energy).
- [ ] The owner receives the existing pet bar with family-derived Firebolt (3110) autocast enabled.
- [ ] Attack produces Firebolt damage on the ordered target with TC's 15% owner spell-power contribution; Follow stops attacking and moves back to the owner.
- [ ] Recasting the same summon recalls the existing imp instead of duplicating it.
- [ ] Dismiss and death remove the imp and clear the owner's bar. Neither restores it at relog; another explicit summon remains possible.
- [ ] A permanent current imp returns at login through persisted pet storage and restores its bar.
- [ ] Existing owner/pet replication carries the imp without wire changes (live proof belongs to team-lead).

## How it works

- [Warlock pet lifecycle, storage adaptation and TC references](../wiki/systems/warlock-pets.md)
- [Shared pet commands and autocast](pet-control.md)

## Implementation inventory

- `crates/server/src/hunter_pet.rs`: effect-56 creation, shared ownership/storage/spawn, data-derived power and death removal.
- `crates/server/src/pet_control.rs`: existing family spell loader, bar, messages, AI and recall bar resend.
- `crates/server/src/class_progression.rs`: scoped level-1 starting-kit grant.
- `crates/server/src/spell_cast/effects.rs`: demon owner spell-power contribution (separate from hunter AP).

## Tests asserting this spec

- `crates/server/src/warlock_pet_tests.rs`: real DB2 casting, message delivery, AI damage/movement and reopened redb lifecycle.
- `crates/server/src/class_progression_tests.rs`: level-1 warlock grant and non-warlock exclusion.

## Known gaps (current cycle)

- [ ] Live client PetFrame/bar and wire-level replication proof remains with team-lead.

## Out of scope

Other demons (Voidwalker, Succubus, Felhunter, Felguard), timed guardians, generated demon names, imported pet_levelstats, base pet attributes/resistances and school-specific owner bonuses, saved health/mana and autocast toggles, map transfers and dead/mounted-owner temporary-unsummon rules. Existing hunter persistence restores full health/power; this task reuses it rather than changing its format.
