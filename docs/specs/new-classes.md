# Monk, Demon Hunter and Evoker — server slice

Server creation, progression and six basic spells for classes 10, 12 and 13. Implementation details: [[../wiki/systems/class-progression]] and [spell casting](../wiki/systems/spell-casting.md).

## What it must do

- [ ] Accept authored race/class gates: Monk on existing non-Dracthyr races, Demon Hunter on Night Elf/Blood Elf/Void Elf, Evoker on Alliance/Horde Dracthyr; reject unsupported pairs without persisting a character.
- [ ] Persist new-class starting levels as max(ChrClasses.StartingLevel, ChrRaces.StartingLevel): ordinary-race Monk 1, Night Elf/Blood Elf DH 8, allied-race Monk/DH 10, Dracthyr Evoker 10.
- [ ] Load class skill lines, authored specializations and level/spec learnable spells; deliver known-spell and action-bar snapshots and level-up deltas through existing progression.
- [ ] Tiger Palm and Blackout Kick deal damage and use authored Energy/Chi costs; Windwalker Tiger Palm gains the Chi amount from its passive spell modifier.
- [ ] Demon's Bite generates Fury and deals damage; Chaos Strike spends 40 Fury and triggers both damage hits, or fails without sufficient Fury.
- [ ] Living Flame casts damage on enemies and healing on allies; Azure Strike hits its explicit enemy plus at most one other baseline enemy.

## How it works

- [Class progression](../wiki/systems/class-progression.md)
- [Spell casting](../wiki/systems/spell-casting.md)

## Implementation inventory

- `crates/server/src/race_class.rs`: allowed pairs, playable race bits, new-character levels.
- `crates/server/src/persistence_character.rs`: creation persistence and temporary locations.
- `crates/server/src/spell_info/class_data.rs`: class skill-line catalog; existing data loaders supply specs/spells.
- `crates/server/src/spell_cast/{mod,scripts,effects}.rs`: effect hooks and modified ENERGIZE amounts.

## Tests asserting this spec

- `crates/server/src/race_class.rs` tests: allowed/refused pairs, start levels, race masks.
- `crates/server/src/persistence_accounts_tests.rs`: creation and persisted levels/maps; rejection has no roster side effects.
- `crates/server/src/class_progression_new_class_tests.rs`: real DB2-derived progression and owner-message snapshots/deltas.
- `crates/server/src/spell_cast/new_class_tests.rs`: real spell rows, Bevy effects and HostClient message delivery.

## Known gaps (current cycle)

- [ ] Live creation/casting through Godot and actual UDP remain main integration work; HostClient tests do not prove them.
- [ ] Full class rotations, talent/proc parity and extra specialization effects are not proven by six basic spell tests.

## Out of scope

- Starting zones: user delayed them. Temporary race starts and faction capitals omit class intro quests/gear; retire when class starting zones (Mardum, Forbidden Reach) are implemented.
- Powers/runes, Essence regeneration, spec/form pool selection and resource replication: powers agent owns them.
- Engine UI, class art and DH/Dracthyr customization: separate engine slice.
- Death Knight starting levels: unchanged outside this slice.
