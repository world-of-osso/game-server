# Warlock pets

Summon Imp (688) enters the existing controlled-pet path: effect 56's MiscValue supplies creature 416, whose family 23 has SkillLine 188 and automatic level-1 Firebolt 3110. No client or shared-protocol change.

## Lifecycle

`hunter_pet::check_summon_pet` admits Summon Imp for warlocks and validates the creature/model before costs. `effect_summon_pet` recalls the same living pet, resending its existing bar; otherwise `summon_imp` loads the saved record or creates one from the template/model at the owner's level. `put_pet_out` supplies ownership, follow, replication, faction and `pet_control::init_pet_control`.

The existing `HunterPet { owner, slot }` component represents controlled ownership for both hunters and imps. A server-only `SummonedDemon` marker selects demon spell-power contribution and death removal. Warlocks use internal storage slot 0 of `StoredPetStable`; this is an adaptation of TC's unslotted SUMMON_PET record, not a hunter Call Pet slot offered to warlocks. No persisted or wire schema change.

Firebolt's imported SpellPower row costs 40 Energy (PowerType 3), not mana. `imp_spell_power` reads that cost; PowerTypeTable supplies Energy's full 100-point pool and 10/s regeneration. This adapts TC's power initialization to the actual imported retail spell, rather than substituting mana costs. Health uses the existing pet health calculation. `effects::demon_power_stats` applies TC's owner contribution: 15% spell bonus and 57% AP, using the server's generic owner spell power (school-specific fire/shadow bonuses are not modelled). Firebolt has zero base points and a 0.4 SP coefficient: 100 owner SP gives 6 damage. Bar/autocast, attack and follow reuse [[spell-casting]] and existing pet control.

Dismiss and a dead demon both call `remove_pet`: stop combat, despawn, save current=None and send PetClearSpells. The stored record remains summonable. Logout removes ownerless world entities without clearing current; login loads a permanent current imp. TC excludes positive-duration summons from current-pet load; only permanent Summon Imp is implemented here. Full health/power restoration follows existing hunter storage, not TC's saved current-pet health/mana. Generated demon names, base pet attributes/resistances, school-specific owner bonuses and mounted/dead-owner temporary-unsummon handling remain unimplemented.

## Starting spell

Imported SkillLineAbility 24651: SkillLine 849, Spell 688, class mask 256, AcquireMethod 2. Imported SpellLevels gives 688 level 3. Warlocks learn 688 at level 3 through the normal DB2 auto-learn path; there is no level-1 override (an earlier override came from a lead brief, not a user requirement, and was removed 2026-10-05).

## Sources

All TC sources pinned to a352b1fa:
- [SpellEffects.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp): `Spell::EffectSummonPet` (2656-2744): MiscValue entry, same-pet recall, current removal and `Player::SummonPet`.
- [Player.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Player/Player.cpp): `Player::SummonPet` (30458-30543): DB load or creation, owner's level/faction, full mana, `InitPetCreateSpells`, save current, bar initialization.
- [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::LoadPetFromDB` (205-394), especially current timed-summon exclusion (225-229), SUMMON_PET owner's level (287-292); `Pet::Update` (619-709), non-hunter corpse removal (629-635); `Guardian::InitStatsForLevel` (847-1099); `Pet::CreateBaseAtCreature` is the existing hunter conversion path, not demon creation.
- [SpellMgr.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellMgr.cpp): `SpellMgr::LoadPetLevelupSpellMap`; [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::InitLevelupSpellsForLevel`.
- [StatSystem.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Unit/StatSystem.cpp): `Guardian::UpdateAttackPowerAndDamage` (1358-1367): owner fire/shadow spell bonus × 0.15 and AP × 0.57.
- [Server implementation](../../../crates/server/src/hunter_pet.rs), [shared pet control](../../../crates/server/src/pet_control.rs), [behavior tests](../../../crates/server/src/warlock_pet_tests.rs), [contract](../../specs/warlock-pets.md).

## See Also

- [[class-progression]] — known-spell grants
- [[persistence]] — mutable character storage
- [[spell-casting]] — Firebolt cast and damage pipeline
