# Pet control

The hunter pet's action bar and the orders and stances it gives: Attack, Follow, Stay, Move To; Passive, Defensive, Assist; and the pet melee they lead to. Taming, the stable and Call/Dismiss Pet: [hunter taming](hunter-taming.md).

References: TrinityCore a352b1fa `WorldSession::HandlePetAction`/`HandlePetActionHelper` (PetHandler.cpp:64-286), `CharmInfo::CharmInfo`/`InitPetActionBar`/`AddSpellToActionBar` (CharmInfo.cpp:26-62, 166-215), `CharmInfo::SaveStayPosition` (Unit.cpp:13306), `Player::PetSpellInitialize` (Player.cpp:22440-22477), `PetAI::UpdateAI`/`SelectNextTarget`/`CanAttack`/`DoAttack`/`HandleReturnMovement`/`NeedToStop` (PetAI.cpp), `SpellMgr::LoadPetLevelupSpellMap` (SpellMgr.cpp:2152-2199), `Pet::InitLevelupSpellsForLevel` (Pet.cpp:1494-1529), `Unit::SetOwnerGUID` (`SummonedBy`), `UNIT_FLAG_PET_IN_COMBAT` 0x800; Retail stance texts `PET_MODE_DEFENSIVE_TOOLTIP`, `PET_MODE_ASSIST_TOOLTIP`.

## What it must do

- [x] A pet out carries `UnitSummonedBy` (its owner), full Focus (`power_type` 100/100) and a `PetCharm`: Follow, Passive (`CharmInfo`'s constructor), and its bar.
- [x] The bar: Attack, Follow, Move To, four spell slots, Assist, Defensive, Passive. Spell slots take the family's level-up spells (CreatureFamily `SkillLine_0/1`, `AutomaticCharLevel`, spell level up to the pet's), highest level first; non-passive only; `ACT_ENABLED`/`ACT_DISABLED`/`ACT_PASSIVE` by `NO_AUTOCAST_AI`/`AUTOCAST_OFF_BY_DEFAULT`. A level 10 wolf: Dash, Bite, Growl.
- [x] The owner gets `PetSpells` (bar, command and react state) when the pet comes out and after every order; `PetClearSpells` when it is dismissed.
- [x] `PetAction` from a mounted owner, for another unit or a dead pet is ignored; spell buttons do nothing yet.
- [x] Attack: a target the owner may attack; the pet chases and auto-attacks it (`UNIT_FLAG_PET_IN_COMBAT` while it goes after a target), even when passive.
- [x] Follow: stop attacking, follow the owner; attack nothing until back with it.
- [x] Stay: hold where it stands; attack only what is in melee reach unless ordered.
- [x] Move To: run to the ground point and hold there (TrinityCore would follow again).
- [x] Passive: stop attacking; never attack unordered. Defensive: attack what auto-attacks the owner or the pet. Assist: also the owner's victim (TrinityCore ignores `REACT_ASSIST`; its Defensive also takes the owner's victim).
- [x] A victim that dies or is out of reach (owner 90 yd away) ends the attack; the pet picks its next target or returns (to its owner, or to its held spot).
- [ ] Not modelled: casting pet spells and autocast, Abandon, the stance kept in the stable (`character_pet.ReactState`), attacks on damage events instead of polling attackers, the chase angle behind the victim, crowd-controlled targets, pet death.

## Tests

- `crates/server/src/pet_control_tests.rs` (world.db with `creature_family`): Diseased Young Wolf 299 called by a level 10 hunter gets the bar, Focus and owner; Attack/Follow against a Kobold Vermin; Stay; Move To; Passive, Defensive (owner or pet attacked, owner's own victim left alone), Assist; Passive stops an ordered attack.
