# PvP progression

Retail Mainline behavior applies to both client art skins. Extend existing PvP/rated records and currency wallet; no duplicate economy.

- Use locally sourced Honor1792/Conquest1602. Preserve durable balances/weekly-earned counters when converting obsolete internal1/2 IDs; do not expose those IDs as alternate currencies.
- Pairing queue entries must not create a match outcome, rating change, honor or conquest. A real lifecycle/result and an authored numeric policy are required.
- Lifetime honor XP/levels/rewards are separate from spendable Honor. Persist progression and selected talents/War Mode state in existing PvP storage.
- War Mode changes require authoritative rest-area, combat and instance checks; its source-backed buff and talent activation must follow actual world state.
- PvP talent selection requires source specialization, slot-mask/unlock, level and PlayerCondition eligibility. Unknown conditions cannot be guessed true.

## Source catalogs and import

`scripts/reference/pvp-12.1.0.69933/` contains locally extracted CASC records. CurrencyTypes contains only1792/1602; PvpTalent407 rows, category3, unlock4, PrestigeLevelInfo24. Values come from Retail build keydcfc90fffd79ba00406ae46f5f657592, exported by game-engine `scripts/export_db2_csv.py` at exact layoutsFA3226AB/BE69BB5A/ECB2C5C2/9685CBCC/6582638F. The local TrinityCore `DB2Structure.h` preserved in game-engine `data/diagnostics/scrap-2026-10-10/` defines field semantics. Numeric source receipts remain in `data/diagnostics/pvp-progression-2026-10-11/db2/`.

Lead/operator import (agents do not write canonical world.db):

```sh
python3 scripts/import-pvp-progression.py --db /path/to/explicit/world.db
```

Imports only `pvp_currency`, `pvp_talent`, `pvp_talent_category`, `pvp_talent_slot_unlock`, `pvp_honor_reward` in one transaction, preserving unrelated tables. Requires an existing DB; validates every source first, rejects duplicate IDs and incompatible destination columns. Re-running replaces those owned catalogs without duplicates. Runtime integration is separately required; import alone is not progression completion.

## Gaps

Honor cap15000 is authored. Conquest uses dynamic WorldState31270/AwardCondition146857, not guessed1800/weekly550. Honor/conquest per result and rating policy are not locally derived. PrestigeLevelInfo maps e.g. honor-level5 to achievement12893; no certified Mainline lifetime-XP curve was recovered. The available HonorLevel game table has old Prestige0..32 columns and is not assumed equivalent. Selected talent effects, authored buff application and live/both-skin UI proof remain required until implemented.

## Runtime checkpoints

War Mode follows local TC a352b1fa Player.cpp30965–31054: changes require rest/no combat, enabling requires a friendly zone with AreaFlags2::AllowWarModeToggle0x1000 in the area/ancestor chain. Both desired and active states are explicit; desired is durable in existing PvP storage. The actual aura path applies Enlisted282559 inside enable areas and269083 outside, and removes the opposite/disabled aura. TDB world states17042/17043 author baseline10%, not a faction bonus guessed from UI text. Private live/bonus-consumer proof remains required.

PvP selection uses source spec, class-specific SlotUnlock and category masks. Mainline indices1–3 map to masks2/4/8 (20/30/40); legacy mask1/level10 is not a fourth native talent button. Selection refuses wrong spec/level/category, duplicate choices, death/combat, replacement outside rest and replacement while the old spell is cooling down. Source PlayerCondition IDs that cannot yet be evaluated stay explicitly unavailable, never guessed true. Per-spec selection persists in existing PvP records. Existing KnownSpells separates baseline ownership from transient PvP grants so deactivation cannot erase a class-owned spell; existing action-bar override handling is reused. Selection permissions and ongoing activation are distinct: entering combat must not forget a selected active talent.

Operator prerequisite: `pvp_*` catalogs are required at normal world startup. Apply the explicit importer before starting a realm with this revision; test/runtime agents do not mutate canonical world.db. Catalog import is not a live progression claim.
