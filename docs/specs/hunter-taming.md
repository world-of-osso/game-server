# Hunter taming

Which creature a hunter can tame with Tame Beast (1515). The cast flow, pet persistence, pet AI and UI are separate work.

References: TrinityCore a352b1fa `spell_hun_tame_beast::CheckCast` (spell_hunter.cpp:1266-1321), `CreatureTemplate::IsTameable`/`IsExotic` (CreatureData.h:547-559), `Player::CanTameExoticPets` (Player.h:2498, `SPELL_AURA_ALLOW_TAME_PET_TYPE` 146), `CREATURE_TYPE_FLAG_TAMEABLE` 0x1 / `TAMEABLE_EXOTIC` 0x10000 (SharedDefines.h:5132,5148); error text from Retail GlobalStrings (`SPELL_FAILED_HIGHLEVEL`, `SPELL_FAILED_BAD_TARGETS`, `PETTAME_TOOMANY`, `PETTAME_CREATUREALREADYOWNED` …).

## What it must do

- [x] `tame::can_tame` checks in CheckCast order: target level for the hunter above the hunter's level → `SPELL_FAILED_HIGHLEVEL`; not tameable → `SPELL_FAILED_BAD_TARGETS`; a pet out → `SPELL_FAILED_ALREADY_HAVE_SUMMON`; no free active slot (5) or the free slot's Call Pet spell (883, 83242-83245) unknown → `PETTAME_TOOMANY` (skipped before the hunter's first pet stable); a charm → `SPELL_FAILED_ALREADY_HAVE_CHARM`; target has an owner → `PETTAME_CREATUREALREADYOWNED`.
- [x] Tameable: Beast, family nonzero, `TAMEABLE` type flag; `TAMEABLE_EXOTIC` ones only with the exotic-taming aura. Diseased Young Wolf 299 tameable, Devilsaur 6498 exotic, Razorbeak Skylord 2659 (no flag) and Hogger 448 (Humanoid) not.
- [x] Type, family and type flags are Retail's: AzerothCore templates TrinityCore also has take TrinityCore's `type`, `family` and difficulty `TypeFlags` (`retail_content.take_retail_taming_columns`). Old Blanchy 582 is a tameable Beast, not a Critter.
- [ ] Generic target checks (dead, range, line of sight, tapped) belong to the Tame Beast cast flow (`SpellInfo::CheckTarget`).
- [ ] TrinityCore reports exotic-without-aura as `SPELL_FAILED_BAD_TARGETS`, not `PETTAME_CANTCONTROLEXOTIC`; kept as TrinityCore does.

## Tests

- `crates/server/src/tame_tests.rs` (world.db templates 299, 6498, 2659, 448, 582).
