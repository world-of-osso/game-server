# Warlock pets

Permanent local demon summons enter the controlled-pet path: effect56's MiscValue supplies the creature entry, whose imported family supplies the bar. Imp688→416/family23 has SkillLine188 and automatic Firebolt3110. Summon/dismiss reuse existing client pet messages; hunter StableMaster operations add a separate protocol.

## Lifecycle

`hunter_pet::check_summon_pet` admits the seven locally authored permanent spell IDs and validates the creature/model before costs. `effect_summon_pet` recalls the same living pet, resending its existing bar; otherwise `summon_demon` loads the saved identity or creates one from the template/model at the owner's level. Explicit resummoning restores full living health; login instead preserves saved wounded health. `put_pet_out` supplies ownership, follow, replication, faction and `pet_control::init_pet_control`.

The existing `HunterPet { owner, slot }` component represents controlled ownership for both hunters and demons. A server-only `SummonedDemon` marker selects demon spell-power contribution and death removal. Warlocks use internal storage slot 0 of `StoredPetStable`; this is an adaptation of TC's unslotted SUMMON_PET record, not a hunter Call Pet slot offered to warlocks. The original Imp slice made no schema change. The later [combat-pet lifecycle](combat-pet-lifecycle.md) owns V2 redb health/spec/stabled-record migration.

Firebolt's imported SpellPower row costs 40 Energy (PowerType 3), not mana. `demon_spell_power` reads that cost; the admitted families15/16/17/23/29 share imported Energy costs; PowerTypeTable supplies Energy's full 100-point pool and 10/s regeneration. This adapts TC's power initialization to the actual imported retail spell, rather than substituting mana costs. Health uses the existing pet health calculation. `effects::demon_power_stats` applies TC's owner contribution: 15% spell bonus and 57% AP, using the server's generic owner spell power (school-specific fire/shadow bonuses are not modelled). Firebolt has zero base points and a 0.4 SP coefficient: 100 owner SP gives 6 damage. Bar/autocast, attack and follow reuse [[spell-casting]] and existing pet control.

Dismiss and a dead demon both call `remove_pet`: stop combat, despawn, save current=None and send PetClearSpells. The stored record remains summonable. Logout removes ownerless world entities without clearing current; login loads the permanent current demon. TC excludes positive-duration summons from current-pet load; timed guardians remain separate. The original slice restored full health/power. [Combat-pet lifecycle](combat-pet-lifecycle.md) now preserves saved health through reopened storage; power still reinitializes. Generated demon names, base pet attributes/resistances, school-specific owner bonuses and dead-owner temporary-unsummon handling remain unimplemented. Player flight uses the shared [temporary pet lifecycle](../../specs/pet-lifecycle.md).

## Expansion proof (2026-10-11)

Atf771781, seven warlock fixtures pass, including real-data688/697/712/691/30146/366222 summon→wound→redb reopen→command3 dismiss→reopen→full explicit resummon. Incubus365349 authors entry184600, absent from the active template view: explicit refusal, not invented content. Felhunter417's legacy expansion base-health entry is1; its legal0.5 wound demonstrates persistence without certifying correct stat scaling.

Atc4ba3ef the existing Fel Domination333889 aura108 modifiers pass a real cast fixture: normal Imp summon is incomplete after1s, modified summon completes within1s, then the next-summon charge disappears. No spell-mod implementation change.

Native serverf771781/protocol203acd3/client16fdd7f18 private54134 proves six summon/dismiss bars in Modern and Forever, plus server-confirmed native Log Out→current Felguard restored→menu dismiss→Log Out→no automatic restore in both skins. Imp world model renders. Voidwalker1132/Succubus4162 fall outside imported appearance coverage; Felhunter850/Felguard14255 lack model receipts. Their captures prove authoritative lifecycle/bar state, not body rendering. Durable logs: game-engine/data/diagnostics/pet-lifecycle-2026-10-11/.

## Live Firebolt investigation (2026-10-05)

The original attack fixture used a hostile kobold, fixed 100 owner spell power, a three-spell catalog, and no terrain or LOS. It now uses computed level-3 warlock stats, the full imported catalog, neutral Northshire Training Dummy 44548/display 3019 at its captured position, real map-0 terrain and LOS. The earlier seven `warlock_` targeted tests passed, including Firebolt damage and a dismissal fixture that artificially taught 2641. That fixture has since been corrected to use command 3 without granting a hunter spell. No failing reproduction or production AI fix was obtained.

The live evidence is not a complete spell trace: server admin combat history records melee events, while spell events go through `spell_cast::log_event` to the owner. The client's `godot/rust/src/ipc/combat.rs` reads only its `CombatEvent` deque (line 17), not the account's `CombatLogEvent` deque, and its formatter prints `spell=-` even when an event has a spell ID (line 73). A training dummy intentionally retains health after damage. The capture ledger also reports falling Energy; that is not proof that the Imp only melees. Reproduction remains blocked on a live pet-state/cast-rejection/owner-spell trace; absence from these IPC/admin captures alone does not identify an AI defect.

TC `PetAI::UpdateAI` (a352b1fa, lines 97-198) and AzerothCore's same routine both select enabled spells after casting/cooldown checks and validate autocast targets. The server's negative-spell path is exercised by the updated fixture; randomized spell selection and positive-spell support remain the existing documented gaps, not demonstrated causes of this report.

Evidence: `game-engine/data/diagnostics/servercheck-2026-10-05/retry3/` (original captures/ledger); `game-engine/data/diagnostics/impfirebolt-2026-10-05/live-stats.log` (seven passing tests at `c8da7a4`). See the implementation and test links in Sources below.

## Dismiss investigation (2026-10-05)

Retail's `UnitPopupPetDismissButtonMixin:OnClick` casts 2641 only for an abandonable hunter pet; otherwise it calls `PetDismiss()`. Its `CanShow` requires known 2641 only in the hunter case. Local Blizzard UI reference: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua:1037-1056`.

Imported DB2 `SkillLineAbility` row 35949 assigns spell 2641 to Hunter SkillLine 795, ClassMask 0, AcquireMethod 4. `SpellLevels` row 334 sets SpellLevel 5; `SkillRaceClassInfo` row 980 restricts line 795 to class mask 4 (hunter). Thus the production progression path should grant 2641 to hunters at level 5, not warlocks. A new real-data regression asserts hunter levels 4/5/10 and warlock levels 3/10; no live hunter was inspected.

TC a352b1fa `Player::PetSpellInitialize` only sends the pet bar/spells/cooldowns, not a player spell grant (`Player.cpp:20628-20662`). `WorldSession::HandlePetActionHelper`, `COMMAND_ABANDON`, dismisses summoned pets with `PET_SAVE_NOT_IN_SLOT` but deletes hunter pets (`PetHandler.cpp:217-238`). `HandlePetAbandon` is hunter deletion only (`:558-566`); it is not the summoned-demon dismiss path.

The original server `pet_control::command` omitted `COMMAND_ABANDON` (3), already defined in shared-protocol. The previous warlock fixture artificially taught 2641, hiding this missing command path. The fixture now omits it and sends `PetAction` with `pet_action_button(COMMAND_ABANDON, ACT_COMMAND)` through the owner connection; lifecycle assertions cover despawn, bar clear, reopened persistence and resummon. **RED reproduced:** at 841c3c3 the corrected fixture fails because the imp remains after command 3. A hunter command-3 regression likewise fails before the fix. The shared build helper's earlier pre-compilation failure is resolved upstream. **GREEN:** targeted `pet_control`, `warlock_pet` and `hunter_pet` tests pass 31/31 at 83a0c27, including reopened demon persistence and hunter slot deletion/bar clear. An earlier run's unrelated ordered-Bite assertion encountered a legitimate random Miss; Bite code was not changed. Client unit-menu tests pass 4/4 at 8a3bb57e (later comments-only commit does not change behavior). No live client dismissal proof was attempted.

`pet_control::command` now dispatches command 3 to `hunter_pet::abandon_or_dismiss_pet`: summoned demons retain their saved record; hunters delete the active stored slot. Both use existing `remove_pet` to stop combat, despawn, clear current, persist and clear the bar. Hunter spell 2641 remains a temporary dismissal. Client PetFrame Dismiss sends existing `PetAction { pet, action: pet_action_button(3, ACT_COMMAND), target: None, position: None }` for summoned pets; hunters cast known 2641. No wire-schema addition is needed.

## Starting spell

Imported SkillLineAbility 24651: SkillLine 849, Spell 688, class mask 256, AcquireMethod 2. Imported SpellLevels gives 688 level 3. Warlocks learn 688 at level 3 through the normal DB2 auto-learn path; there is no level-1 override (an earlier override came from a lead brief, not a user requirement, and was removed 2026-10-05).

## Sources

All TC sources pinned to a352b1fa:
- [Retail pet popup](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua): `UnitPopupPetDismissButtonMixin:CanShow/OnClick`; local snapshot lines 1037-1056.
- [PetHandler.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/PetHandler.cpp): `HandlePetActionHelper` command 3 and `HandlePetAbandon` have distinct summoned/hunter semantics.
- Imported `data/world.db`: `skill_line_ability` ID 35949, `spell_levels` ID 334, `skill_race_class_info` ID 980; [progression regression](../../../crates/server/src/class_progression_tests.rs).
- [SpellEffects.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp): `Spell::EffectSummonPet` (2656-2744): MiscValue entry, same-pet recall, current removal and `Player::SummonPet`.
- [Player.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Player/Player.cpp): `Player::SummonPet` (30458-30543): DB load or creation, owner's level/faction, full mana, `InitPetCreateSpells`, save current, bar initialization.
- [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::LoadPetFromDB` (205-394), especially current timed-summon exclusion (225-229), SUMMON_PET owner's level (287-292); `Pet::Update` (619-709), non-hunter corpse removal (629-635); `Guardian::InitStatsForLevel` (847-1099); `Pet::CreateBaseAtCreature` is the existing hunter conversion path, not demon creation.
- [SpellMgr.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellMgr.cpp): `SpellMgr::LoadPetLevelupSpellMap`; [Pet.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Pet/Pet.cpp): `Pet::InitLevelupSpellsForLevel`.
- [StatSystem.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Unit/StatSystem.cpp): `Guardian::UpdateAttackPowerAndDamage` (1358-1367): owner fire/shadow spell bonus × 0.15 and AP × 0.57.
- [TrinityCore PetAI.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/AI/CoreAI/PetAI.cpp), [AzerothCore PetAI.cpp](https://github.com/azerothcore/azerothcore-wotlk/blob/master/src/server/game/AI/CoreAI/PetAI.cpp): autocast selection and validation.
- [Owner spell event routing](../../../crates/server/src/spell_cast/mod.rs), [client IPC combat log](../../../../game-engine/godot/rust/src/ipc/combat.rs): separate spell/melee observation paths.
- [Server implementation](../../../crates/server/src/hunter_pet.rs), [shared pet control](../../../crates/server/src/pet_control.rs), [behavior tests](../../../crates/server/src/warlock_pet_tests.rs), [contract](../../specs/warlock-pets.md).

## See Also

- [[class-progression]] — known-spell grants
- [[persistence]] — mutable character storage
- [[spell-casting]] — Firebolt cast and damage pipeline
