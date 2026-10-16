# Professions

Retail professions on the server: learning at profession trainers and crafting known recipes. How it works: [professions](../wiki/systems/professions.md).

## Requirements

- Trainers are TrinityCore TDB1210 trainers (`tdb_*` tables). Opening a trainer sends every service that fits the character's race and class, with its cost, required level/skill/abilities and state (Available, Unavailable, Known).
- A purchase succeeds only for an Available service of the open trainer, with enough copper and a free primary profession slot (2) when it adds a primary profession. It takes the cost and teaches the spell; refusals are `TrainerBuyFailed` (`NotEnoughMoney`, `Unavailable`) and change nothing.
- Learning a profession (264617 Tailoring) learns its base spell, its Classic line at rank 1 with the Retail tier cap (300), and the line's automatic recipes (2963 Bolt of Linen Cloth).
- A known recipe casts with its Retail cast time. Without every reagent, tool or bag room it fails with Retail error text and changes nothing. On completion it consumes the reagents, puts the created item in the bags and may raise the recipe's skill-up line by the TrinityCore difficulty chances.
- `CraftRecipe { casts }` repeats the cast; stopping or moving cancels the rest, and the first cast that cannot start ends the queue.
- Professions persist per character and reach the owner as `ProfessionSnapshot` on world entry and after every change.
- Spells learned through trainers, spell effects or online admin `learn-spell` also enter the player's live known-spell list and reach the owner's spellbook without relog through `SpellsLearned`. Repeated learning sends no duplicate delta. Persisted learned spells are included in `KnownSpellsSnapshot` on entry and remain known across level, specialization and talent re-evaluation.

- New characters start with empty bags and no money (the old 20 Linen / 10 Peacebloom / 10 Copper Ore / 10 gold seed is gone). Admin `grant-item <character> <item_id> <count>` stores items in the bags (content_item name, quality and sell price, stacked to the template's max) and sends an online owner the `InventoryDelta`; unknown items, unknown characters and full bags are refused unchanged.

## Quality reagent validation

- `CraftingOperationRequest` validates learned recipes, authored basic slot membership/quantities, aggregate owned Retail bag counts and fixed reagent requirements. It never consumes inventory, advances skill or creates an item.
- Runtime requires local Retail exports under `data/db2/active`: ModifiedCraftingSpellSlot, ModifiedCraftingReagentSlot, MCRSlotXMCRCategory, CraftingReagentQuality, Item and CraftingQuality. No network lookup or invented substitution is permitted.
- Quality remains `Err(CraftingOperationComposition)` (`CRAFTING_OPERATION_COMPOSITION`), never a default tier. Modifying/finishing eligibility and unevaluated slot conditions refuse explicitly. Pinned TC a352b1fa `SpellEffects.cpp` maps effects288/297 to EffectNULL; Retail SchematicForm delegates composition to native APIs.

Private acceptance fixture: ignored `profession::modern_crafting_fixture` tests create only the fixed 2026-10-16 diagnostics redb, grant concrete test bags/skills/knowledge and verify post-run reagent/skill conservation. Fixture grants are not acquisition evidence.

## Out of scope

Gathering nodes, Dragonflight+ quality/concentration/crafting orders/specializations, recrafting, unlearning, First Aid, reputation price discounts, gossip-menu-specific trainers.
