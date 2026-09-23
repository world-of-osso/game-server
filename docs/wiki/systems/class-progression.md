# Class Progression

`ClassProgressionPlugin` (`crates/server/src/class_progression.rs`) owns each player's active specialization, known spells and action bar. The rules are computed from [[spell-info]] `ClassSpells`. Race/class validation lives in `race_class.rs`. Casts do not check known spells yet: `knows_spell` is the hook for the cast pipeline.

## Race and class validation

- world.db has no `chr_races` or `CharBaseInfo` table. `race_class.rs` therefore holds the race→class table, copied from the client character-creation data (`game-engine src/scenes/char_create/data.rs` RACES).
- The table covers 24 races (Pandaren 24/25/26 included) and classes 1–9 and 11. Dracthyr, Earthen and Haranir are not in the table.
- The same file holds each race's `ChrRaces.PlayableRaceBit` (`game-engine data/ChrRaces.csv`). DB2 `RaceMasks` use this bit.
- Character creation and `SelectCharacter` both call `validate_race_class` and reject a failing character. The errors are:
  - `Invalid race id N`
  - `Invalid class id N`
  - `Class C is not available for race R`
- The fallback race list and the dead `chr_races` query are removed.

## Specialization

- **Below level 10** (`SPEC_UNLOCK_LEVEL`), the active spec is the class "Initial" spec: the `chr_specialization` row with `OrderIndex` 4, such as 1451 Initial Paladin or 1446 Initial Warrior.
- **At level 10 and above**, the active spec is the stored spec if it is a non-Initial spec of the class. Otherwise it is the first non-Initial spec by `OrderIndex`. Paladin at 10 gets Holy 65.
- Dropping below level 10 switches back to the Initial spec. The earlier choice is not remembered.
- `SetSpecialization` is accepted only for a non-Initial spec of the player's class, at level 10 or higher.
  - A rejected request is answered with `SpecializationChanged` carrying the current spec.
  - An accepted request persists the spec, sends `SpecializationChanged` and then the spell deltas.

## Known spells

The known list is built in learn order:

1. `ClassSpells` learnables with `AcquireMethod` 1, 2 or 4 (auto-learned).
   - `AcquireMethod` 0 (trainer/other) is excluded, for example Contemplation 121183 and Consecration 81297.
   - `required_level` must be at most the level.
   - `RaceMasks` must be zero or include the race bit. Summon Warhorse 13819 is Human/Dwarf/Dark Iron; Thalassian Warhorse 34769 is Blood Elf.
2. The active spec's `specialization_spells`, in `DisplayOrder`.
3. The spec's mastery spells.

Duplicates are dropped. Passives stay in the list with no flag; the client decides from its catalog. `LearnableSpell.passive` and `SpecSpell.passive` are `spell_misc.Attributes_0 & 0x40`, and mastery spells count as passive.

The list is stored in the `KnownSpells` component. Cast validation should call `class_progression::knows_spell(&Query<&KnownSpells>, player, spell_id)`.

## Action bars

- Slots are 0..=119.
- `SetActionButton` with `Spell(id)` requires a known spell. `Item`/`Macro` refs are stored as given, and `None` clears the slot.
  - Accepted requests are persisted and get no reply.
  - Rejected requests are answered with the unchanged `ActionBarSnapshot`.
- **Default bar:** when a character has no progression record, its first 12 known non-passive spells go on slots 0–11 in learn order. A new Orc Warrior gets Slam 1464 on slot 0 and Attack 88163 on slot 1.
- A spell button whose spell stops being known (spec change or lower level) is removed. The server then sends `ActionBarSnapshot` and persists the bar.

## Messages (TalentChannel)

| When | Sent |
|---|---|
| First frame the player has a `PlayerLink` connection | `SpecializationChanged` (only if the class has spec data), `KnownSpellsSnapshot`, `ActionBarSnapshot` |
| Level change (`Changed<Level>`) or accepted `SetSpecialization` | `SpecializationChanged` if the spec changed, then `SpellsUnlearned` and `SpellsLearned` (each only when non-empty), then `ActionBarSnapshot` if the bar was pruned |

## Level

- `CharacterData.level` in redb is the source of truth.
- `Level(u8)` is spawned from it and replicated. Periodic saves never write the live `Level` back.
- There is no XP gain system. Level changes go through admin IPC `set-level <character> <level>`:
  - It validates 1..=90 (`spell_info::MAX_LEVEL`) and saves `CharacterData`.
  - It queues `set_live_level`, which updates the online player's `Level` and, if present, `UnitLevel`.
  - `sync_level_changes` then sends the deltas.

## Persistence

- redb table `character_progression`: `character_id` (u64) → bitcode `ProgressionData { spec_id: u32, action_bar: Vec<(u8, ActionRefData)> }`.
- The table is created in `create_tables`, and its row is removed with the character.
- It is written when:
  - the first world entry finds no record
  - the resolved spec differs from the stored spec
  - the bar is pruned
  - a spec change is accepted
  - a `SetActionButton` is accepted

## Sources

- [class_progression.rs](../../../crates/server/src/class_progression.rs) — components, rules, systems
- [class_progression_tests.rs](../../../crates/server/src/class_progression_tests.rs) — real world.db + temp redb tests
- [race_class.rs](../../../crates/server/src/race_class.rs) — race/class table, race bits
- [persistence_progression.rs](../../../crates/server/src/persistence_progression.rs) — `character_progression` table

## See Also

- [[spell-info]] — `ClassSpells` learnables, specs, passive flags
- [[persistence]] — redb tables
