# Warlock pets

Summon Imp (688) enters the existing controlled-pet path: effect 56's MiscValue supplies creature 416, whose family 23 has SkillLine 188 and automatic level-1 Firebolt 3110. No client or shared-protocol change.

## Lifecycle

`hunter_pet::check_summon_pet` admits Summon Imp for warlocks and validates the creature/model before costs. `effect_summon_pet` recalls the same living pet, resending its existing bar; otherwise `summon_imp` loads the saved record or creates one from the template/model at the owner's level. `put_pet_out` supplies ownership, follow, replication, faction and `pet_control::init_pet_control`.

The existing `HunterPet { owner, slot }` component represents controlled ownership for both hunters and imps. A server-only `SummonedDemon` marker selects mana and death removal. Warlocks use internal storage slot 0 of `StoredPetStable`; this is an adaptation of TC's unslotted SUMMON_PET record, not a hunter Call Pet slot offered to warlocks. No persisted or wire schema change.

With pet_levelstats absent, mana takes `creature_classlevelstats.basemana` at the owner's level and creature class, without template ManaModifier (`Guardian::InitStatsForLevel`). Health uses the existing pet health calculation. Mana uses the existing creature regen timer. Bar/Firebolt autocast, attack and follow reuse [[spell-casting]] and existing pet control.

Dismiss and a dead demon both call `remove_pet`: stop combat, despawn, save current=None and send PetClearSpells. The stored record remains summonable. Logout removes ownerless world entities without clearing current; login loads a permanent current imp. TC excludes positive-duration summons from current-pet load; only permanent Summon Imp is implemented here. Full health/mana restoration follows existing hunter storage, not TC's saved current-pet health/mana. Generated demon names, exact pet stats/scaling and mounted/dead-owner temporary-unsummon handling remain unimplemented.

## Starting spell

Imported SkillLineAbility 24651: SkillLine 849, Spell 688, class mask 256, AcquireMethod 2. Imported SpellLevels gives 688 level 3. The task explicitly requires a level-1 starting kit, so `class_progression::compute_known_spells` adds only 688 for warlocks before deduplication; DB2 remains unchanged. This is an explicit World of Osso requirement, not a claim that TC/retail grants it at level 1.

## Sources

All TC sources pinned to a352b1fa:
- [SpellEffects.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp): `Spell::EffectSummonPet` (2656-2744): MiscValue entry, same-pet recall, current removal and `Player::SummonPet`.
- [Player.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Player/Player.cpp): `Player::SummonPet` (30458-30543): DB load or creation, owner's level/faction, full mana, `InitPetCreateSpells`, save current, bar initialization.
- [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::LoadPetFromDB` (205-394), especially current timed-summon exclusion (225-229), SUMMON_PET owner's level (287-292); `Pet::Update` (619-709), non-hunter corpse removal (629-635); `Guardian::InitStatsForLevel` (847-1099), creature base mana (915-921); `Pet::CreateBaseAtCreature` is the existing hunter conversion path, not demon creation.
- [SpellMgr.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellMgr.cpp): `SpellMgr::LoadPetLevelupSpellMap`; [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::InitLevelupSpellsForLevel`.
- [Server implementation](../../../crates/server/src/hunter_pet.rs), [shared pet control](../../../crates/server/src/pet_control.rs), [behavior tests](../../../crates/server/src/warlock_pet_tests.rs), [contract](../../specs/warlock-pets.md).

## See Also

- [[class-progression]] — known-spell grants
- [[persistence]] — mutable character storage
- [[spell-casting]] — Firebolt cast and damage pipeline
