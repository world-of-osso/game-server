# Monk, Demon Hunter and Evoker — server slice

Server creation, progression and six basic spells for classes 10, 12 and 13. Implementation details: [class progression](../wiki/systems/class-progression.md) and [spell casting](../wiki/systems/spell-casting.md).

## What it must do

- [ ] Follow the [installed race/class eligibility contract](race-class-eligibility.md), replacing the copied Retail matrix while retaining source rejection and separately authored Forever gates. Existing class/start-level controls remain required.
- [x] Persist starting levels as Player::GetStartLevel (Player.cpp:24384-24398): 1, or 10 for an IsAlliedRace race (ChrRaces.Flags 0x80000) or Dracthyr; a Death Knight at least 8 (a Pandaren one 10); a Demon Hunter at least 8: ordinary-race Monk 1, Night Elf/Blood Elf DH 8, allied-race classes (Haranir Alliance included) 10, Dracthyr 10, Death Knight 8.
- [x] Load class skill lines, authored specializations and level/spec learnable spells; deliver known-spell and action-bar snapshots and level-up deltas through existing progression.
- [x] Tiger Palm and Blackout Kick deal damage and use authored Energy/Chi costs; Windwalker Tiger Palm gains the Chi amount from its passive spell modifier.
- [x] Demon's Bite generates Fury and deals damage; Chaos Strike spends 40 Fury and triggers both damage hits, or fails without sufficient Fury.
- [x] Living Flame casts damage on enemies and healing on allies; Azure Strike hits its explicit enemy plus at most one other baseline enemy.

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

- [x] A Dracthyr's visage form (`CharacterAppearance.visage`, shared-protocol branch `visage`) is stored with its appearance. Records written before the field load with none (`LegacyCharacterDataV6`), the barber shop keeps it, and creation refuses a visage form for any other race.

## Known gaps (current cycle)

- [x] Every default skill line teaches its spells (`LearnDefaultSkills`): the `Racial - *` lines (Haranir 2930, Earthen 2895, Dracthyr 2808, …), the weapons, armor, languages, GENERIC and All Classes lines (wiki new-character-kit and class-progression). Racial stat modifiers follow SimC `__gt_race_stats`, which has zeros for Dracthyr, Earthen and Haranir.
- [x] Creation gives the Retail CharacterLoadout kit and playercreateinfo_action buttons (wiki new-character-kit); `crates/server/src/player_create_info_tests.rs` asserts the Night Elf Demon Hunter and Dracthyr Evoker kits.

- [x] Live over UDP, 2026-09-30, private server on 5100: game-engine `godot/tests/new_class_live.gd` created a Human Monk, a Night Elf Demon Hunter and an Alliance Dracthyr Evoker through the Godot creation screens. They entered at level 1 in Northshire, level 8 in Shadowglen and level 10 at the Stormwind flight master. Each was teleported to a Northshire Training Dummy, where Tiger Palm, Demon's Bite (25 Fury) and Living Flame (180 damage) hit. Captures are in game-engine `data/diagnostics/newclasses-2026-09-30/`.
- [x] Havoc's SpecializationSpells OverridesSpellID replace the baselines (TrinityCore `Player::LearnSpecializationSpells` → `AddOverrideSpell`). Demon's Bite 344859, Chaos Strike 344862 and Fel Rush 344865 are unlearned at 10, and their bar buttons move to 162243/162794/195072. The same rule now replaces Holy Paladin's Judgment 20271, Shield of the Righteous 53600 and Cleanse Toxins 440013.
- [x] Energizing Flame 400006 refunds its effect-0 percent (80) of Living Flame's mana cost on an enemy hit. Azure Strike picks its other target at random (`RandomResize`).
- [x] Kul Tiran, Earthen and Haranir accept their CharBaseInfo classes, not only Monk. They start at the faction capital (Stormwind/Orgrimmar) as a workaround until their race starts exist. 
- [ ] Full class rotations, talent/proc parity and extra specialization effects are not proven by six basic spell tests.

## Out of scope

- Starting zones: user delayed them. Temporary race starts and faction capitals omit class intro quests/gear; retire when class starting zones (Mardum, Forbidden Reach) are implemented.
- Powers/runes, Essence regeneration, spec/form pool selection and resource replication: powers agent owns them.
- Engine UI, class art and DH/Dracthyr customization: separate engine slice.
