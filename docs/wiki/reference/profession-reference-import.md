# Profession reference import

Verified source receipt: 2026-10-11. Nine local-CASC payloads were recovered from Retail build key `dcfc90fffd79ba00406ae46f5f657592`. [Contract](../../specs/profession-reference-import.md). This extends the available source corpus, not executable crafting coverage.

## Sources and staging interface

`python3 scripts/profession_data.py --check --source-dir /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-2026-10-11/db2`

Replace `--check` with `--db /absolute/private-world.db` for a lead-owned staging import. No default destination, extraction or downloads. Source receipts/DBD files live alongside that diagnostic directory. Pinned SHA-256 values are tracked in the script.

| Table | FDID | Layout | Raw / decoded rows |
|---|---:|---|---:|
| CraftingDifficulty | 4373536 | 7744A1A6 | 26 / 27 |
| CraftingDifficultyQuality | 4373537 | E2C031D4 | 91 / 91 |
| CraftingReagentQuality | 4382197 | BFED1EE1 | 1403 / 1403 |
| CraftingReagentEffect | 4541803 | 70597398 | 104 / 104 |
| Profession | 4508544 | 86FE926D | 15 / 15 |
| ProfTraitPathNode | 4542069 | 8622E546 | 915 / 915 |
| ProfTraitPerkNode | 4542084 | 741F7E26 | 5929 / 5929 |
| ProfTraitTree | 4542085 | D2C4C5BD | 138 / 138 |
| CraftingOrder | 4529122 | 05F8699D | 2047 / 2047 |

CraftingDifficulty includes a copy row; requiring decoded count equal to the raw record count would discard/reject authentic data. Row1 preserves reagent craft bonus25%, recraft6.25% and concentration curves77360/77361. Row2 of CraftingDifficultyQuality preserves tier2, QualityPercentage50 and difficulty1. These facts do not specify an operation algorithm.

## Exact unresolved semantics

WoWDBDefs layout7744A1A6 names `ConcentrationSkillCurveID` and `ConcentrationDifficultyCurveID` with question marks. Curve identities/points alone do not establish input normalization, composition, rounding, currency identity, regeneration or refund rules. CraftingDifficultyQuality marks `Order`, `QualityPercentage`, difficulty relation and `Field_10_0_0_44895_004` unresolved. CraftingReagentEffect calls its `ProfessionEffectID` interpretation a "pure guess". Preserve these fields; never treat guessed names as executable authority.

Retail `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeSchematicForm.lua:282-299` asks the native `C_TradeSkillUI.GetCraftingOperationInfo` for numeric outcomes. Generated `TradeSkillUITypesDocumentation.lua:170-191` describes base/bonus skill, difficulty, quality thresholds and concentration cost outputs, not their computation. TrinityCore a352b1fa `SpellEffects.cpp` maps effect288(CraftItem) and297(RecraftItem) to EffectNULL. The lead confirmed no local authentic evaluator or captured Retail operation inputs/outputs exists. Quality crafting stays explicitly unsupported, not approximated.

Pinned CraftingOrder rows join to89 default-difficulty effect24 spell rows in the read-only world.db corpus; Leatherworking413496→206434 is one source-backed fixed-output candidate. This is a source census, not proof of trainer acquisition, order service hosting or item-template readiness. Effect297 has25 default-difficulty rows with zero item/misc references; do not reinterpret CREATE_ITEM as recrafting or create replacement GUIDs without a sourced lineage/modification operation.

## Sources

- [Importer](../../../scripts/profession_data.py) — exact hashes/layouts/typed schema.
- [Behavioral tests](../../../scripts/tests/test_profession_data.py) — source and staging proof.
- Local wowdev/WoWDBDefs `definitions/*.dbd` at the receipt's layouts; downloaded reference files retained in the diagnostic directory, no game data downloaded.
- Local Retail AddOns `Blizzard_Professions*` and TrinityCore a352b1fa source receipts.

## See Also

- [Crafting reference import](crafting-reference-import.md) — existing seven-table corpus.
- [Professions](../systems/professions.md) — current fixed-output runtime.
