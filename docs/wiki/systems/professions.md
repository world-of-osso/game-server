# Professions

For the seven-table staging reference importer and modern recipe resolver handoff, see [[crafting-reference-import]]. For merchant-bot use of recipe data, see [[bot-economy]]. Existing player crafting below does not prove the economy importer or expansion-specific merchant coverage.

Professions in `crates/server/src/profession.rs`, with `profession/data.rs` (world.db data) and `profession/trainer.rs` (trainer frames). The spec is [professions](../../specs/professions.md). Opening the trainer role is covered by [npc-interaction](npc-interaction.md); crafting casts go through [spell-casting](spell-casting.md).

## Native Leatherworking proof — verified2026-10-11

`profession_tests::modern_crafting_leatherworking_trainer_cast_reagents_skillup_and_persistence` exercises actual Simon Tanner/TDB trainer56, profession264578, repeated Light Leather2881 casts, recipe2153 purchase, pants2303 result, authored skill advancement and redb/snapshot persistence. No synthetic recipe/formula or world.db writes. The [native client proof](../../../../game-engine/docs/wiki/systems/native-professions-crafting.md) owns both-skin UDP5770 artifacts and current quality-operation gaps. Client automatic-learning thresholds must not override a recipe the authoritative trainer taught.

See [[gathering]] for the gathering-nodes branch's authored resource loop and explicit acceptance boundary.

## Model

Retail splits a profession into a parent `SkillLine` (197 Tailoring) and one child line per expansion (2540 Classic Tailoring, `ParentSkillLineID` 197, `ParentTierIndex` 4). `PlayerProfessions { lines, spells }` holds each learned line (`skill_line`, `step`, `rank`, `max_rank`) and every spell learned through professions. It is saved (redb `profession_state`, bitcode) and sent as `ProfessionSnapshot` whenever it changes (`save_and_send_professions`, `Changed<PlayerProfessions>`), which includes entering the world. The AzerothCore-era `professions` table (placeholder Mining/Blacksmithing) is deleted by `create_tables`.

## Character-owned recipe Favorites

`profession/favorites.rs` stores characterID→spellID pairs in redb `character_recipe_favorites`; existing bitcode `PlayerProfessions { lines, spells }` records are unchanged. The new table is initialized with the other persistence tables. `SetRecipeFavorite` takes its owner from the authenticated `PlayerLink`/`ClientId`, checks current `PlayerProfessions::knows`, then commits only the preference. Unknown-spell refusals log and return unchanged owner state.

The existing owner snapshot adds `favorites`, loaded from this table and intersected with current known spells on world entry, profession changes and preference replies. No client-only preference cache grants authority. The [spec](../../specs/professions.md#recipe-favorites) owns requirements/source lines; fresh server/client reopen acceptance remains pending.

## Data (`ProfessionData`, loaded at startup)

| Source | Use |
|---|---|
| `skill_line` | name, `CategoryID` (11 primary profession), parent line and tier index |
| `skill_race_class_info` + `tdb_skill_tiers` | tier caps: `SkillTiers.Value[step-1]` (Classic tier 336 = 300). Retail 12.x ships no SkillTiers DB2; the caps are TrinityCore server data |
| `skill_line_ability` | recipes and automatic abilities, keyed by `SkillupSkillLineID` (or `SkillLine`), trivial ranks, `NumSkillUps` |
| `spell_reagents`, `spell_totems`, `item_sparse.TotemCategoryID`, `totem_category` | reagents and tools |
| `tdb_trainer`, `tdb_trainer_spell`, `tdb_creature_trainer` | TrinityCore TDB1210 Retail trainers (Georgio Bolero 1346 → trainer 163) |

TDB1210 trainers teach the profession spell (264617 Tailoring) and expansion lines (264623 Cataclysm Tailoring), not the AzerothCore Apprentice/Journeyman/Expert/Artisan rank spells, so those do not exist here.

## Learning (TrinityCore master a352b1fa)

- `LEARN_SPELL` (36, `Spell::EffectLearnSpell`) learns its trigger spell; learning a spell applies its `SKILL` (118) effects (`Player::AddSpell` spell_learn_skill): the line at step `BasePoints` with the tier cap.
- `SKILL_STEP` (44, `Spell::EffectSkill`) raises a line to a tier: rank `max(rank, 1)`, cap from the tier.
- Setting a line enables its parent at `ParentTierIndex` (`Player::SetSkill`) and learns its automatic abilities (`LearnSkillRewardedSpells`: AcquireMethod 1/2/4, AutomaticSkillRank only from `MinSkillLineRank`, race/class masks). The parent of a Classic line gets step 4 and cap `tier[3]`, which is 0 for tier 462 (TrinityCore does the same).
- Primary limit 2 (`MaxPrimaryTradeSkill`): a spell that adds a new `CategoryID` 11 parent line is refused when two are known.

## Trainer

`send_trainer_lists` answers `Changed<ActiveInteraction>` with role `Trainer` with `TrainerList`: every trainer spell that fits race/class, state per `Trainer::GetSpellState` (Known, Unavailable for skill/ability/level, Known when every `LEARN_SPELL` target is known). `TrainerBuySpell` for the open trainer checks state and primary limit (`CanTeachSpell`), takes `MoneyCost` in one `InventoryModel` save (`NotEnoughMoney`), casts spells that have a `LEARN_SPELL` effect (`cast_triggered`) and learns others, then sends the list again. Creatures with several `tdb_creature_trainer` rows (gossip-menu-specific trainers, 21 creatures) are refused with a warning.

## Crafting

[Fixed-output orders](crafting-orders.md) reuse this cast pipeline, with customer escrow and atomic result/commission mail instead of crafter-bag output. This does not add quality-crafting or recrafting.

`CraftRecipe { spell_id, casts }` starts a normal player cast (`spell_cast::cast_for_player`: known-spell check includes profession spells, cast bar from `SpellCastTimes`). The remaining casts sit in `CraftQueue`; `continue_craft_queues` starts the next one after the cast ends, and `cancel_cast` (stop, movement, death) drops the queue. A cast that cannot start answers `CastFailed` and ends the queue.

- `check_cast_items` (`Spell::CheckItems`) runs at cast start and completion: every reagent in the bags (`Reagents`, "Missing reagent: X"), every tool item or a bag item whose totem category is compatible (`IsTotemCategoryCompatibleWith`: same type, mask covers; `Totems`, "Requires X"), and room for every created item (`InventoryFull`).
- `take_reagents` (`Spell::TakeReagents`) removes the reagents after the costs are spent.
- `CREATE_ITEM` (24) stores `BasePoints` of `EffectItemType` in the bags (name/quality/sell price from `VendorCatalog.goods`) and rolls `UpdateCraftSkill`: orange below `TrivialLow` 100%, yellow 75%, green from the midpoint 25%, grey at `TrivialHigh` 0% (`SkillChance.*` defaults), +`NumSkillUps` capped at the line's max.
- Recipes with other effects (enchanting, item targets) log an unhandled effect; recipes with unsupported implicit targets are refused with `NotSupported`.
