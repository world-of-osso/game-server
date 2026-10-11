# Warlock pets

Locally authored permanent demon summons use the server's existing pet ownership, bar, movement and combat systems. Implementation: `crates/server/src/hunter_pet.rs`; [mechanics and references](../wiki/systems/warlock-pets.md).

## What it must do

- [x] A new level-1 warlock knows Summon Imp; other classes do not receive it.
- [x] Casting 688 creates an imp from effect 56's MiscValue next to its owner, with the owner's faction, a creature model and Firebolt's imported power pool (Energy).
- [x] The owner receives the existing pet bar with family-derived Firebolt (3110) autocast enabled.
- [x] Attack produces Firebolt damage on the ordered target with TC's 15% owner spell-power contribution; Follow stops attacking and moves back to the owner.
- [x] Recasting the same summon recalls the existing imp instead of duplicating it.
- [x] PetAction command 3 dismisses the imp without requiring hunter spell 2641; dismissal and death clear the owner's bar and do not restore it at relog. The saved demon remains explicitly summonable. The same command abandons/deletes a hunter pet's saved slot (TC `PetHandler.cpp:217-238`).
- [x] Summon Imp688, Voidwalker697, Succubus712, Felhunter691, Felguard30146 and Sayaad366222 create the creature authored by their effect56. Incubus365349 is admitted only when its authored template/model exists; absent local content is an explicit refusal.
- [x] Explicitly summoning a dismissed demon after relog restores its saved identity as a living demon. Current-pet relog preserves wounded health instead.
- [x] A permanent current imp returns at login through persisted pet storage and restores its bar.
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

Timed guardians, generated demon names, imported pet_levelstats, base pet attributes/resistances and school-specific owner bonuses, saved mana and autocast toggles, map transfers and dead/mounted-owner temporary-unsummon rules. Character-scoped health persistence is defined by [combat pet lifecycle](pet-lifecycle.md).
