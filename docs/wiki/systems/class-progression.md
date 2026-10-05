# Class Progression

`ClassProgressionPlugin` (`crates/server/src/class_progression.rs`) owns each player's active specialization, trait config ([[talents]]), known spells and action bar. The rules are computed from [[spell-info]] `ClassSpells`. Race/class validation lives in `race_class.rs`. Casts do not check known spells yet: `knows_spell` is the hook for the cast pipeline.

## Race and class validation

- world.db has no `chr_races` or `CharBaseInfo` table. `race_class.rs` therefore holds the race→class table, copied from the client character-creation data (`game-engine src/scenes/char_create/data.rs` RACES).
- The table covers 31 races and classes 1–13. Monk 10 is available on all 29 non-Dracthyr races authored by local CharBaseInfo; DH 12 on Night Elf/Blood Elf/Void Elf (4/10/29); Evoker 13 only on Dracthyr (52/70). Gates were checked against locally extracted CharBaseInfo FDID 1343386 (WDC5 layout FEC46FB7).
- Kul Tiran 32, Earthen 84/85 and Haranir 86/91 expose only Monk in this bounded new-class slice. Their other authored classes are not enabled here.
- The same file holds each race's `ChrRaces.PlayableRaceBit` (`game-engine data/ChrRaces.csv`). DB2 `RaceMasks` use this bit.
- Character creation and `SelectCharacter` both call `validate_race_class` and reject a failing character. The errors are:
  - `Invalid race id N`
  - `Invalid class id N`
  - `Class C is not available for race R`
- The fallback race list and the dead `chr_races` query are removed.

## Default skill spells

Every default skill line gives its spells at creation and as levels rise. TrinityCore teaches them with `Player::LearnDefaultSkills` and `LearnSkillRewardedSpells` (Player.cpp:25275-25336, 25407-25457).

- **Lines:** `ClassInfo.default_skills` holds the `skill_race_class_info` rows with Availability 1 whose ClassMask allows the class. The class's own line is excluded because it is `learnable`. Rows are ordered by skill line, then row ID.
- **Row choice:** for each skill line, the first row whose `RaceMasks` allow the race and whose `MinLevel` is reached is used.
- **Spells:** the row's auto-learned abilities that match the race and are at or below the level. Examples:
  - Human Will to Survive 59752, Orc Blood Fury 20572, Haranir Thorn Bloom 1237885
  - weapon and armor passives: One-Handed Swords 201, Cloth 9078
  - languages: Language Orcish 669
  - GENERIC Auto Attack 6603
  - All Classes Recuperate 1231411
- **Skill value:** an AutomaticSkillRank ability also needs the line's starting skill value (`SkillRange::value`, `LearnDefaultSkill`):
  - 1 by default
  - 300 for languages
  - the maximum for `SKILL_FLAG_ALWAYS_MAX_VALUE` lines
  - `(level - 1) * 5` for a Death Knight
  
  Garrison Enchanting 161211 needs 35, so only a Death Knight at level 8 or higher learns it.
- The server keeps no skill values itself.

## New-character start levels and temporary locations

`race_class::start_level` follows Player::GetStartLevel (Player.cpp:24384-24398): 1, or 10 for an IsAlliedRace race (ChrRaces.Flags 0x80000) or Dracthyr; a Death Knight at least 8 (a Pandaren one 10); a Demon Hunter at least 8. The allied-race list is hardcoded from the 12.1.0.69933 ChrRaces export because the server loads no ChrRaces data.

`persistence_character::default_character_data` persists the level at creation. Demon Hunters use the existing race location. Dracthyr use Stormwind (map 0) or Orgrimmar (map 1) by faction. This workaround omits class intro quests/gear because starting zones were delayed; retire when class starting zones (Mardum, Forbidden Reach) are implemented. No class zones or intro content are added. Kul Tiran/Earthen/Haranir Monks retain the pre-existing Northshire placeholder placement; no new location fallback was added. Replace that placeholder when their race starts are authored. Haranir Alliance 86 has IsAlliedRace, so it starts at 10 although ChrRaces.StartingLevel says 1.

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
- **Create bar:** character creation stores the race/class `playercreateinfo_action` buttons ([[new-character-kit]]). A character without a progression record enters with an empty bar.
  - A new Orc Warrior has Slam 1464 (0), Attack 88163 (1), Blood Fury 20572 (9) and Recuperate 1231411 (10).
  - TDB's WotLK bonus-bar rows (buttons 72-119) move to the main bar for every class but Rogue and Druid, whose Stealth/Shadow Dance and Cat/Bear/Moonkin forms are the only Retail forms with a `SpellShapeshiftForm.BonusActionBar` (`scripts/retail_action_bars.py`).
  - Entering the world prunes buttons whose spells are not known; TrinityCore keeps them. [[new-character-kit]] lists the affected buttons.
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
- [class_progression_new_class_tests.rs](../../../crates/server/src/class_progression_new_class_tests.rs) — Monk/DH/Evoker start, level-up and spec grants
- [new-class spec](../../specs/new-classes.md) — bounded server contract and integration exclusions
- [race_class.rs](../../../crates/server/src/race_class.rs) — race/class table, race bits
- [persistence_progression.rs](../../../crates/server/src/persistence_progression.rs) — `character_progression` table

## See Also

- [[general-spell-grants]] — unresolved fresh-character content grants and candidate attribute evidence
- [[spell-info]] — `ClassSpells` learnables, specs, passive flags
- [[talents]] — trait trees, validation, `character_traits`
- [[new-character-kit]] — creation items and action buttons
- [[persistence]] — redb tables
