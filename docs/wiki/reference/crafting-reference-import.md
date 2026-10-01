# Crafting Reference Import

The bounded importer loads seven pinned build 12.1.0.69933 tables into an explicit staging SQLite path. It does not modify economy catalog/content policy or import into live `world.db` automatically. Contract: [crafting reference import](../../specs/crafting-reference-import.md).

## Interface

```text
python scripts/crafting_data.py --db /absolute/staging.db --source-dir /absolute/data/reference/midnight-20261001
```

Python interface: `import_data(db, source_dir)` returns the same seven-element provenance list printed as JSON by the CLI. No extraction, downloads, source writes or implicit path selection. Existing unrelated tables remain untouched. All source parsing/validation completes before opening the destination; seven DROP/CREATE/INSERT operations share one explicit `BEGIN IMMEDIATE` transaction. SQLite failures roll back schema and rows. Connections close explicitly.

## Source and layout pins

| SQLite table | Source | Rows | Layout |
|---|---|---:|---|
| `crafting_data` | local CASC `4545611.db2` | 2785 | `C21C854F` |
| `crafting_data_item_quality` | local CASC `4545612.db2` | 1687 | `DE11A051` |
| `modified_crafting_spell_slot` | local CASC `3390956.db2` | 12769 | `00739B08` |
| `modified_crafting_reagent_slot` | WagoCSV `ModifiedCraftingReagentSlot.csv` | 393 | exact ordered CSV header |
| `modified_crafting_category` | WagoCSV `ModifiedCraftingCategory.csv` | 598 | exact ordered CSV header |
| `modified_crafting_reagent_item` | WagoCSV `ModifiedCraftingReagentItem.csv` | 623 | exact ordered CSV header |
| `mcr_slot_x_mcr_category` | WagoCSV `MCRSlotXMCRCategory.csv` | 910 | exact ordered CSV header |

Source SHA-256 pins live in `TABLES` in the importer. CSV files do not carry build or binary layout headers: their content hashes bind the supplied snapshot, and JSON `layout` reports the exact column-name array, not a fabricated binary layout hash. DB2 JSON `layout` is the eight-digit header hash. Each report entry also carries `table`, `source`, `source_kind`, `build`, `rows`, `sha256`. No extra database metadata table is written.

## Schema and decoder mapping

[`crafting_schema.sql`](../../../scripts/crafting_schema.sql) is the ordered column/type contract, including every unknown field. SQLite column names retain source spelling; only table names are snake_case. DBD blocks for build 12.1.0.69933 establish binary order/types. The three supplied CSV DBDs establish their corresponding columns; the MCR join CSV uses its exact supplied header.

CraftingData preserves these sixteen fields, in order:

```text
ID, Type, CraftingDifficultyID, CraftedItemID, ItemBonusTreeID,
CraftingDifficulty, Field_10_0_0_44649_005, CraftSkillBonusPercent,
ReCraftSkillBonusPercent, InspirationSkillBonusPercent,
Field_10_0_0_44649_009, Field_10_0_0_45141_011,
FirstCraftFlagQuestID, FirstCraftTreasureID,
FirstCraftPlayerDataFlagCharacterID, CraftedTreasureID
```

The six fields between CraftingDifficulty and FirstCraftFlagQuestID are DBD floats. Raw uint32 bits become exact float32 values stored as SQLite REAL, not numeric casts. Quality rows expose `ID, ItemID, CraftingDataID`, with non-inline ID and relationship parent from the existing decoder.

ModifiedCraftingSpellSlot DBD declares seven columns:

```text
ID, SpellID, Slot, ModifiedCraftingReagentSlotID,
Field_9_0_1_35679_003, ReagentCount, ReagentReCraftCount
```

Its decoder returns eight values: inline `$relation$SpellID` plus an appended relationship parent. Require exactly eight, compare value 7 with declared SpellID at value 1, then remove only value 7. Example source row: `15388,1237486,0,421,0,140,80,1237486`. Other widths or mismatched parents fail; no arbitrary column dropping. Encrypted sections fail through the existing decoder; hash pins also prevent zero-filled substitutes. CSV numbers, row widths, finite REALs, positive unique IDs and source counts are validated without skipping rows.

## Resolver handoff

Reference data only; no resolver changes. User-supplied downstream constraints: CraftItem effect 288 maps `EffectMiscValue0` to `crafting_data.ID`; SkillupSkillLineID 2915 contains 105 modern recipes, not an absence claim. Resolver must distinguish direct `CraftedItemID` from quality variants via `crafting_data_item_quality.CraftingDataID`. Spell-slot joins use `modified_crafting_spell_slot.SpellID`, then `ModifiedCraftingReagentSlotID`; category links use `mcr_slot_x_mcr_category`, with reagent definitions in `modified_crafting_reagent_item`. This importer does not interpret effect 288 or assert the 105-recipe count.

## Proof ledger

- RED: real-source CLI staging test failed because `scripts/crafting_data.py` did not exist.
- Development GREEN: seven bounded tests passed in 6.739s. Python 3.14 connection-lifetime warnings prompted explicit close changes; that run is not final proof for the revised source.
- Final command: `python scripts/tests/test_crafting_data.py -v`, scoped to this importer. Real-source test creates a temporary staging DB, verifies exact counts and real joins, and removes it afterward. Final receipt is reported against the committed revision, not a broader build/suite claim.

## Sources

- [Importer](../../../scripts/crafting_data.py) — pinned provenance, validation and atomic replacement.
- [Schema](../../../scripts/crafting_schema.sql) — exact column contract.
- [Tests](../../../scripts/tests/test_crafting_data.py) — behavioral fixtures and staging integration.
- [Existing decoder](../../../scripts/db2_casc.py) — raw WDC5/relationship/float decoding.

## See Also

- [[professions]] — current runtime crafting, separate from this import.
- [[economy-catalog]] — downstream catalog resolver ownership.
