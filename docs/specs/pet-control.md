# Pet control

The controlled pet's action bar and the orders and stances it gives: Attack, Follow, Stay, Move To; Passive, Defensive, Assist; and the pet melee they lead to. Warlock Summon Imp reuses these systems: [warlock pets](warlock-pets.md). Taming, the stable and Call/Dismiss Pet: [hunter taming](hunter-taming.md).

References: TrinityCore a352b1fa `WorldSession::HandlePetAction`/`HandlePetActionHelper` (PetHandler.cpp:64-391), `HandlePetSpellAutocastOpcode` (PetHandler.cpp:628-676), `PetAI::UpdateAI` autocast (PetAI.cpp:105-212), `Pet::LoadPetFromDB`/`SavePetToDB`/`FillPetInfo` (`ReactState`, Pet.cpp:325, 521, 551), `CanHaveGlobalCooldown` (Spell.cpp:9162), `Guardian::UpdateAttackPowerAndDamage` (StatSystem.cpp:1324-1345), `CharmInfo::CharmInfo`/`InitPetActionBar`/`AddSpellToActionBar` (CharmInfo.cpp:26-62, 166-215), `CharmInfo::SaveStayPosition` (Unit.cpp:13306), `Player::PetSpellInitialize` (Player.cpp:22440-22477), `PetAI::UpdateAI`/`SelectNextTarget`/`CanAttack`/`DoAttack`/`HandleReturnMovement`/`NeedToStop` (PetAI.cpp), `SpellMgr::LoadPetLevelupSpellMap` (SpellMgr.cpp:2152-2199), `Pet::InitLevelupSpellsForLevel` (Pet.cpp:1494-1529), `Unit::SetOwnerGUID` (`SummonedBy`), `UNIT_FLAG_PET_IN_COMBAT` 0x800; Retail stance texts `PET_MODE_DEFENSIVE_TOOLTIP`, `PET_MODE_ASSIST_TOOLTIP`.

## What it must do

- [x] A pet out carries `UnitSummonedBy` (its owner), full Focus (`power_type` 100/100) and a `PetCharm`: Follow, Passive (`CharmInfo`'s constructor), and its bar.
- [x] Hunter-pet Focus regenerates in spawn-anchored four-second ticks of 24, multiplied by its Focus regeneration aura multiplier and truncated to whole units, clamped to maximum. The timer keeps advancing while full; spending does not reset it. This is the hunter-pet path, not the player's `PowerType` continuous rate. Player and demon regeneration remain unchanged (TC a352b1fa `CreatureData.h:423`, `Pet.cpp:64,671-687`, `Creature.cpp:964-998`; normal world Focus rate 1).
- [x] The bar: Attack, Follow, Move To, four spell slots, Assist, Defensive, Passive. Spell slots take the family's level-up spells (CreatureFamily `SkillLine_0/1`, `AutomaticCharLevel`, spell level up to the pet's), highest level first; non-passive only; `ACT_ENABLED`/`ACT_DISABLED`/`ACT_PASSIVE` by `NO_AUTOCAST_AI`/`AUTOCAST_OFF_BY_DEFAULT`. A level 10 wolf: Dash, Bite, Growl.
- [x] The owner gets `PetSpells` (bar, command and react state) when the pet comes out and after every order; `PetClearSpells` when it is dismissed.
- [x] `PetAction` from a mounted owner, for another unit or a dead pet is ignored.
- [x] Command 3 (`COMMAND_ABANDON`) removes the owner's pet and clears its bar/current state. Hunter pets lose their saved slot; summoned demons retain their record for an explicit summon, never relog restoration (TC `PetHandler.cpp:217-238`).
- [x] Spell buttons (`ACT_ENABLED`/`ACT_DISABLED`/`ACT_PASSIVE`): a spell on the pet's bar, not passive and without an enemy area target, is cast by the pet through the spell pipeline (`creature_cast`: cost, cooldown, range, sight) at the given unit; the pet then goes after that unit if its owner may attack it (`_AttackStart`), even when passive (`IsCommandAttack`). A failed cast clears the attack order. A level 10 wolf's Bite (17253, 25 Focus, 0.75 × AP) hits the target.
- [x] A hunter pet's attack power is its owner's × 0.22 (`Guardian::UpdateAttackPowerAndDamage`; the pet's own `2 × Strength - 20` needs `pet_levelstats`, not imported). Pets have the global cooldown (`CanHaveGlobalCooldown`: a creature with a `CharmInfo`). A pet's combat log events go to its owner.
- [x] `PetSpellAutocast` (`CMSG_PET_SPELL_AUTOCAST`) sets a learned autocastable spell (not `ACT_PASSIVE`) `ACT_ENABLED` or `ACT_DISABLED` on the owner's living pet; the owner gets the bar again.
- [x] Autocast: a pet not casting, with a victim it may attack, casts the first `ACT_ENABLED` negative spell on its bar that the victim takes (cooldown, Focus, range). A wolf attacking a kobold Bites it; with Bite's autocast off it only swings.
- [x] The stance is kept with the pet in its stable slot (redb `pet_stables`, `StoredPet.react_state`) whenever it changes and restored when the pet comes out (Call Pet, login). A newly tamed pet is Passive (`CharmInfo::CharmInfo`; TrinityCore's tamed pet keeps it). Stables saved before this load Passive (`character_pet.ReactState` default 0).
- [x] A pet (`UNIT_FLAG_PLAYER_CONTROLLED`) takes the player side of the faction attack rules (`GetAffectingPlayer`): it attacks neutral creatures as its owner may, and they fight it back.
- [x] Attack: a target the owner may attack; the pet chases and auto-attacks it (`UNIT_FLAG_PET_IN_COMBAT` while it goes after a target), even when passive.
- [x] Follow: stop attacking, follow the owner; attack nothing until back with it.
- [x] Stay: hold where it stands; attack only what is in melee reach unless ordered.
- [x] Move To: run to the ground point and hold there (TrinityCore would follow again).
- [x] Passive: stop attacking; never attack unordered. Defensive: attack what auto-attacks the owner or the pet. Assist: also the owner's victim (TrinityCore ignores `REACT_ASSIST`; its Defensive also takes the owner's victim).
- [x] A victim that dies or is out of reach (owner 90 yd away) ends the attack; the pet picks its next target or returns (to its owner, or to its held spot).
- [ ] Retail's default stance for a newly tamed pet is unverified (no Blizzard source found); TrinityCore's Passive is kept.
- [ ] Not modelled: autocast of positive spells (Dash) and TrinityCore's random pick among castable autocast spells, autocast states kept in the stable, `SMSG_PET_CAST_FAILED`, attacks on damage events instead of polling attackers, the chase angle behind the victim, crowd-controlled targets, hunter pet death (Summon Imp death removal is covered by [warlock pets](warlock-pets.md)).

## Tests

- `crates/server/src/pet_control_tests.rs` (world.db with `creature_family`): Diseased Young Wolf 299 called by a level 10 hunter gets the bar, Focus and owner; Attack/Follow against a Kobold Vermin; Stay; Move To; Passive, Defensive (owner or pet attacked, owner's own victim left alone), Assist; Passive stops an ordered attack; Defensive survives a relog; Bite ordered from the bar damages the kobold and costs 25 Focus; an attacking wolf autocasts Bite, not with its autocast off, again once back on.
- `crates/server/src/hunter_pet_tests.rs`: a tamed pet is stored Passive; a stable saved without stances loads Passive.
