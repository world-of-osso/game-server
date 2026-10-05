# Fresh-character General spell grants

Verified 2026-10-05 on server base `ea3d28d`. No server change: available data does not establish a prerequisite rule that also preserves Throw. The strongest candidate is `SpellMisc.Attributes_13 & 0x10000`, but its meaning and Throw's fresh-character eligibility remain unproven. Do not substitute a spell denylist.

## Evidence scope

Read-only queries used `data/world.db` and canonical engine `data/SpellMisc.csv`. Historical engine capture `data/diagnostics/spellbook-2026-10-04/spellbook-forever.log` contains 41 distinct `SpellBookItem<ID>` entries: eight ordinary entries (6603, 20598, 59752, 76271, 76294, 79738, 458619, 1231411), 31 content abilities on skill 183, Throw 385265, and Heart Essence 296208. Capture is not a Retail screenshot or fresh-paladin test.

The 31 content entries include Garrison Ability 161691, Covenant Ability 313347, Sanity Restoration Orb 314955, Pocopoc 360078, Exit Lorewalking 467538, Return to the Story 466325, View Next Adventure 1236329 and G-99 Breakneck 1215279. All comparisons below use actual DB rows, not inferred spell names as a production filter.

## Candidate results

| Candidate | Observed evidence | Result |
|---|---|---|
| SkillLineAbility flags/category/ranks | All 31 content rows: `Flags=0`, `TradeSkillCategoryID=0`, `MinSkillLineRank=1`, both trivial ranks 0. Auto Attack row 3999, Armor Skills row 26677 and Weapon Skills row 26678 have the same values. | No separation. Recuperate's Flags=128 does not cover other ordinary spells. |
| Conditions | SkillLineAbility has no PlayerConditionID column. No local SpellLearnSpell table/CSV. SpellMisc.ShowFutureSpellPlayerConditionID=0 and ContentTuningID=0 on all 31 content spells and the ordinary skill-183 entries. No `tdb_conditions.SourceEntry` matches any of the 31 IDs. | No available learn-condition link. Pinned TrinityCore SpellLearnSpellEntry also has no PlayerConditionID. |
| Class/race masks | Content rows have ClassMask 0 or -1 and both RaceMasks 0 or -1; all allow a human paladin. Ordinary Auto Attack ClassMask=16383, armor/weapon ClassMask=2. Throw has ClassMask=0 and both RaceMasks=0, identical to many content rows. | Would discard Throw, not establish a prerequisite. |
| Expansion/Covenant/Garrison | No per-ability prerequisite columns in SkillLineAbility, Spell or ChrClasses. SkillLine has expansion display-string IDs, shared by every row on that line. PlayerCondition has expansion/Covenant fields but these spells have no nonzero condition link. SkillRaceClassInfo row 5 grants skill 183: Availability=1, MinLevel=0, ClassMask=16383, RaceMasks=(-1,-1). | No linked prerequisite in available data. |
| Existing hide attributes | 94 skill-183 AcquireMethod=2 rows; 89 have DifficultyID=0 SpellMisc records, 59 survive ATTR0 0x80 / ATTR4 0x8000 / learned ATTR8 0x2000, before class filtering. The 31 captured content abilities survive these bits. | Client hide flags do not explain these grants. |
| Attributes_13 bit 16 | All 31 content abilities have 0x10000; all eight ordinary captured entries clear it. The bit also marks Throw 385265. Across skill 183 AcquireMethod=2, 34 spells set it: those 31, Throw, Boost 1215078 and Honk 1223284 (last two already hidden). Bitwise AND across all 17 attributes of the 31 content spells leaves only this bit. | Strong candidate, not a verified Retail learn gate: TrinityCore calls it `SPELL_ATTR13_UNK16`. |

Client CSV comparison: all 41 captured spell IDs have DifficultyID=0 records; 32 set bit 16 (31 content + Throw). Attributes_13 and Attributes_8 agree with SQLite on all 41, zero mismatches.

Heart Essence is not skill 183: duplicate SkillLineAbility rows 40581/40584 put 296208 on skill 2727 (All Classes), AcquireMethod=2, Flags=0, MinSkillLineRank=1, ClassMask=0, RaceMasks=(0,0). Its Attributes_13=1, not 0x10000, and current CSV/SQLite Attributes_8=0x2000 hides it once learned. Its presence in the historical capture is not proof Retail shows it at level 1. Recuperate 1231411 is also on skill 2727; removing that skill would lose it.

## Reproduce the strongest candidate

Against read-only world.db:

```sql
SELECT a.ID, a.Spell, n.Name_lang, a.ClassMask,
       a.RaceMasks_0, a.RaceMasks_1, m.Attributes_13
FROM skill_line_ability a
JOIN spell_name n ON n.ID = a.Spell
JOIN spell_misc m ON m.SpellID = a.Spell AND m.DifficultyID = 0
WHERE a.SkillLine = 183 AND a.AcquireMethod = 2
  AND (m.Attributes_13 & 65536) != 0
ORDER BY a.ID;
```

Expected: 34 rows including Throw. Selecting the same fields for Auto Attack 6603, paladin Armor Skills 76271 and Weapon Skills 76294 yields bit clear.

## Decision and proof

Server `default_skill_spells` currently teaches these rows through auto-acquisition, skill rank, level and race checks (`class_progression.rs:224-253`). No proven additional gate was found that meets the requested preservation contract. Stopped before implementation as instructed. No Rust edits, builds, behavioral tests or live runs; evidence is read-only data analysis, not acceptance proof.

To proceed, establish Throw 385265's Retail eligibility and the meaning of Attributes_13 bit 16, or obtain linked Retail prerequisite data. Unknown bit correlation alone is not proof that learned content spells should be permanently suppressed.

## Sources

- Server `data/world.db`: SkillLineAbility, SpellMisc, SpellLevels, SkillRaceClassInfo, PlayerCondition and tdb_conditions queries, 2026-10-05.
- Engine `/home/osso-test/Projects/world-of-osso/game-engine/data/SpellMisc.csv`: DifficultyID=0 comparisons for all 41 captured IDs.
- Engine `data/diagnostics/spellbook-2026-10-04/spellbook-forever.log` and `/home/osso-test/.worktrees/handoff-spellbook.md`: historical IDs and client-hide commit `3d9c720b`.
- [TrinityCore a352b1fa SharedDefines.h](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Miscellaneous/SharedDefines.h): SpellAttr13 enum, `SPELL_ATTR13_UNK16 = 0x00010000`.
- [TrinityCore a352b1fa DB2Structure.h](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/DataStores/DB2Structure.h): SkillLineAbilityEntry and SpellLearnSpellEntry field definitions.
- [class_progression.rs](../../../crates/server/src/class_progression.rs):224-253 — current default-skill grant boundary.

## See Also

- [[class-progression]] — default skills and known-spell generation
- [[spell-info]] — raw spell attributes and class catalogs
