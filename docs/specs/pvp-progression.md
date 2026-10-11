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
