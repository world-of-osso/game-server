# Hunter taming

Which creature a hunter can tame with Tame Beast (1515), the pet it becomes, its stable, Call Pet 1-5, Dismiss Pet and the pet following its owner. Pet attack, stances, commands and the pet action bar: [pet control](pet-control.md).

References: TrinityCore a352b1fa `spell_hun_tame_beast::CheckCast` (spell_hunter.cpp:1266-1321), `Spell::EffectTameCreature`/`EffectSummonPet`/`EffectDismissPet` (SpellEffects.cpp:2606-2728, 3586-3598), `Spell::CheckCast` SUMMON_PET/DISMISS_PET (Spell.cpp:6454-6538), `Unit::InitTamedPet` (Unit.cpp:11180-11215), `Pet::CreateBaseAtCreature` (Pet.cpp:768-799), `FollowMovementGenerator` (FollowMovementGenerator.cpp:76-188), `CreatureTemplate::IsTameable`/`IsExotic` (CreatureData.h:547-559), `Player::CanTameExoticPets` (Player.h:2498, `SPELL_AURA_ALLOW_TAME_PET_TYPE` 146), `CREATURE_TYPE_FLAG_TAMEABLE` 0x1 / `TAMEABLE_EXOTIC` 0x10000 (SharedDefines.h:5132,5148); error text from Retail GlobalStrings (`SPELL_FAILED_HIGHLEVEL`, `SPELL_FAILED_BAD_TARGETS`, `PETTAME_TOOMANY`, `PETTAME_CREATUREALREADYOWNED` …).

## What it must do

- [x] `tame::can_tame` checks in CheckCast order: target level for the hunter above the hunter's level → `SPELL_FAILED_HIGHLEVEL`; not tameable → `SPELL_FAILED_BAD_TARGETS`; a pet out → `SPELL_FAILED_ALREADY_HAVE_SUMMON`; no free active slot (5) or the free slot's Call Pet spell (883, 83242-83245) unknown → `PETTAME_TOOMANY` (skipped before the hunter's first pet stable); a charm → `SPELL_FAILED_ALREADY_HAVE_CHARM`; target has an owner → `PETTAME_CREATUREALREADYOWNED`.
- [x] Tameable: Beast, family nonzero, `TAMEABLE` type flag; `TAMEABLE_EXOTIC` ones only with the exotic-taming aura. Diseased Young Wolf 299 tameable, Devilsaur 6498 exotic, Razorbeak Skylord 2659 (no flag) and Hogger 448 (Humanoid) not.
- [x] Type, family and type flags are Retail's: AzerothCore templates TrinityCore also has take TrinityCore's `type`, `family` and difficulty `TypeFlags` (`retail_content.take_retail_taming_columns`). Old Blanchy 582 is a tameable Beast, not a Critter.
- [x] Generic target checks (alive, enemy, creature type, range, line of sight) run before `can_tame`, as `Spell::CheckCast` runs its script after them (Spell.cpp:6119); a target that is no creature fails `SPELL_FAILED_BAD_IMPLICIT_TARGETS`. Tapped-by-others is not checked.
- [x] Tame Beast channels 6 s: APPLY_AURA DUMMY on the beast, PERIODIC_TRIGGER_SPELL (6 s) on the hunter casting 13481, whose TAMECREATURE (55) hits the channel target (TARGET_UNIT_CHANNEL_TARGET 77). A channel that runs its full duration lands its auras' ticks due by their expiry before its auras are removed (`auras::channel_completed`); an interrupted channel does not.
- [x] `EffectTameCreature`: a hunter without a pet out; the target becomes a pet of its template, model and name at the hunter's level in the first free active slot, out at once, full health (ExpectedStat CreatureHealth × HealthModifier at that level; `pet_levelstats` is not imported), the hunter's faction, `UNIT_FLAG_PLAYER_CONTROLLED`. The creature despawns (`DespawnOrUnsummon`: no corpse, respawns after its spawn delay).
- [x] Name: the creature's name. TrinityCore names it after its `CreatureFamily`, which is not imported.
- [x] The stable persists in redb `pet_stables` (slot entry, display, name, level, family, stance; which slot is out). The pet out at logout returns at login (`Player::LoadPet`); a pet whose owner leaves the world leaves with it.
- [x] Call Pet N (883, 83242-83245, SUMMON_PET with the slot in its value) summons slot N at the owner's combat reach ahead; with a pet out it brings that pet there instead (`EffectSummonPet`). An empty slot or absent character pet storage fails with "You do not have a pet to summon" before CastSuccess. Missing storage is not permission to report a successful no-op summon; this applies equally when the player has selected a hostile target. No pet is fabricated or granted.
- [x] Dismiss Pet (2641, 3 s, DISMISS_PET at TARGET_UNIT_PET 5) removes the pet from the world and keeps it in its slot; without a pet out it fails `SPELL_FAILED_NO_PET`.
- [x] The pet follows its owner (`FollowMovementGenerator`, `PET_FOLLOW_DIST` 1, `PET_FOLLOW_ANGLE` π ± π/4) at the owner's speed × the catch-up factor 1-1.3, straight paths on the floor.
- [ ] Tame failure texts other than "Invalid target" travel as the server's `detail` (no `CastFailReason` for them yet).
- [ ] Not modelled: pet death and Revive Pet, taking the pet through map transfers, stable slots beyond the five active ones, charms, `SMSG_PET_NEWLY_TAMED`.
- [ ] TrinityCore reports exotic-without-aura as `SPELL_FAILED_BAD_TARGETS`, not `PETTAME_CANTCONTROLEXOTIC`; kept as TrinityCore does.

## Tests

- `crates/server/src/tame_tests.rs` (world.db templates 299, 6498, 2659, 448, 582).
- `crates/server/src/hunter_pet_tests.rs`: a level 10 hunter tames Diseased Young Wolf 299 through the real channel; Hogger 448 is refused; the stable survives a reload; Dismiss/Call Pet; empty-slot and no-pet failures; the pet runs after its owner.
