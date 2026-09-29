# Class Progression

`ClassProgressionPlugin` (`crates/server/src/class_progression.rs`) owns each player's active specialization, trait config ([[talents]]), known spells and action bar. The rules are computed from [[spell-info]] `ClassSpells`. Race/class validation lives in `race_class.rs`. Casts do not check known spells yet: `knows_spell` is the hook for the cast pipeline.

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
2. The active spec's `specialization_spells`, in `DisplayOrder`, whose `spell_levels.SpellLevel` is at most the level (TrinityCore `Player::LearnSpecializationSpells`). Arms at 10 has Mastery: Master of Arms 1258398 but not Seasoned Soldier 279423 (11) or Plate Specialization 86101 (27); Holy at 10 lacks Beacon of Light 53563 (16).
3. The spec's mastery spells.
4. Talent spells of the active spec's trait config, including granted nodes ([[talents]]). A Retribution level 20 Paladin knows the granted Hammer of Wrath 1241288 and Greater Judgment 231663; switching to Holy swaps them for Lay on Hands 633 and Divine Steed 190784.

Duplicates are dropped. A talent whose `TraitDefinition.OverridesSpellID` is set removes that baseline spell ([[talents]]): Retribution's Final Verdict 383328 replaces Templar's Verdict 85256. Passives stay in the list with no flag; the client decides from its catalog. `LearnableSpell.passive` and `SpecSpell.passive` are `spell_misc.Attributes_0 & 0x40`, and mastery spells count as passive.

The list is stored in the `KnownSpells` component. Cast validation should call `class_progression::knows_spell(&Query<&KnownSpells>, player, spell_id)`.

## Action bars

- Slots are 0..=119.
- `SetActionButton` with `Spell(id)` requires a known spell. `Item`/`Macro` refs are stored as given, and `None` clears the slot.
  - Accepted requests are persisted and get no reply.
  - Rejected requests are answered with the unchanged `ActionBarSnapshot`.
- **Default bar:** when a character has no progression record, its first 12 known non-passive spells go on slots 0–11 in learn order.
  - Known spells come only from the class skill line, the spec and talents, so profession and riding spells never qualify.
  - Class mounts are skipped: learnables with a `spell_effect.EffectAura` 78 (SPELL_AURA_MOUNTED) effect (`LearnableSpell.mount`). A new Human Paladin knows Summon Warhorse 13819 and Summon Charger 23214 but its bar is only Crusader Strike 35395 on slot 0.
  - Other utility spells without a mount aura still qualify, for example Sense Undead 5502 at level 10.
  - Saved bars of existing characters are not changed.
  - A new Orc Warrior gets Slam 1464 on slot 0 and Attack 88163 on slot 1.
- **Overrides:** a button of a spell that an applied talent overrides now holds the talent spell. When the talent is removed and its spell is no longer known, the button goes back to the baseline spell. Both send `ActionBarSnapshot` and persist the bar. The rewrite also runs on enter world.
- A spell button whose spell stops being known (spec change or lower level) is removed. The server then sends `ActionBarSnapshot` and persists the bar.
- **Pushed spells:** each newly learned non-passive, non-mount spell (level-up, admin `set-level`, spec change, talent commit) goes to the first empty slot 0–11 in learn order unless already on the bar, as Retail `SPELL_PUSHED_TO_ACTIONBAR`; a full main bar takes none. A level 1 Warrior at level 2 gets Charge 100 on slot 2; switching Retribution to Holy fills the freed slots with Holy's actives.

## Messages (TalentChannel)

| When | Sent |
|---|---|
| First frame the player has a `PlayerLink` connection | `SpecializationChanged` and `TraitConfigSnapshot` (only if the class has spec data), `KnownSpellsSnapshot`, `ActionBarSnapshot` |
| Level change (`Changed<Level>`) or accepted `SetSpecialization` | `SpecializationChanged` if the spec changed, then `SpellsUnlearned` and `SpellsLearned` (each only when non-empty), then `ActionBarSnapshot` if the bar was rewritten or pruned, then `TraitConfigSnapshot` if the spec, config or point budget changed |
| `CommitTraitConfig` | `TraitCommitResult`; when accepted, then the spell deltas and `TraitConfigSnapshot` ([[talents]]) |

## Level

- `CharacterData.level` in redb is the source of truth.
- `Level(u8)` is spawned from it and replicated. Periodic saves never write the live `Level` back.
- Levels change through XP ([[experience]]) or admin IPC `set-level <character> <level>`:
  - It validates 1..=90 (`spell_info::MAX_LEVEL`) and saves `CharacterData`.
  - It queues `set_live_level`, which updates the online player's `Level` and, if present, `UnitLevel`.
  - `sync_level_changes` then sends the deltas.

## Persistence

- redb table `character_progression`: `character_id` (u64) → bitcode `ProgressionData { spec_id: u32, action_bar: Vec<(u8, ActionRefData)> }`.
- The table is created in `create_tables`, and its row is removed with the character.
- Trait purchases per spec live in `character_traits` ([[talents]]).
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
- [[talents]] — trait trees, validation, `character_traits`
- [[persistence]] — redb tables
