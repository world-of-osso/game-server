# Mount vendor cost provenance

Verified input: 2026-10-12. [Mount Journal contract](specs/mount-journal.md); [runtime](wiki/systems/mounts.md#vendor-acquisition).

## Sources

- Local CASC Retail12.1.0.69933, build key `dcfc90fffd79ba00406ae46f5f657592`, FileDataID801681, WDC5 layout22331DAB. Diagnostic input: game-engine `data/diagnostics/mount-journal-2026-10-11/vendor-db2/801681.db2`; all selected records readable.
- WoWDBDefs ItemExtendedCost: noninline ID, Money, RequiredArenaRating, ArenaBracket, Flags, MinFactionID, MinReputation, RequiredAchievement, ItemID[5], ItemCount[5], CurrencyID[5], CurrencyCount[5]. All selected constraint fields are zero; producer rejects future unsupported constraints rather than guessing.
- Read-only world.db TDB1210 `tdb_npc_vendor`; teaching links `item_x_item_effect → item_effect.TriggerType6 → mount.SourceSpellID`.199 ExtendedCost mount offers select40 unique cost IDs. No authored mount direct-learn vendor entries.
- Local primary-source `primary-sources/DBCEnums.h:863`, `CurrencyTypesFlags::_100_Scaler=0x8`. Currency395 flags8280 means cost2550 raw217500 becomes2175 exactly. Producer and runtime reject fractional normalized costs.
- Names/icons: `currency_types(Name_lang,InventoryIconFileID)` and `content_item.Name JOIN item.IconFileDataID`. Reputation: mount ItemSparse MinFactionID/MinReputation; rank0–7 follows shared CharacterReputation standing, not race approval.

Pinned numeric CSV and input/output SHA256 hashes: `crates/server/src/merchant/mount_vendor_costs.csv` and adjacent `.provenance.json`. CSV keeps raw counts; runtime normalizes using authenticated metadata.

## Reproduce

Run through `agent-run mount-vendor-costs`:

```text
python3 scripts/export_mount_vendor_costs.py --world data/world.db --db2 /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/mount-journal-2026-10-11/vendor-db2/801681.db2 --reader /home/osso/.worktrees/game-engine-s11/scripts/export_db2_csv.py --output crates/server/src/merchant/mount_vendor_costs.csv
```

## Data boundary

CurrencyTypes392 and3252 are absent in this Retail metadata. Their costs remain recorded in the CSV; runtime omits affected offers and reports each missing identifier once per catalog load. It never invents metadata, substitutes IDs or drops only the missing part of a price. Non-mount ExtendedCost behavior remains unchanged. Vendor PlayerCondition/arena/guild/achievement evaluation and reputation discounts are not added.

Concrete fixtures: cost5976 =15 Mark of Honor137642/icon1322720;1647 =70 Halaa Battle Token26045 +15 Halaa Research Token26044;2671 =5 Champion's Seal241/icon236246 +5,000,000 copper. Reputation:29227 requires Kurenai978 Exalted7;29102 requires Mag'har941 Exalted7.
