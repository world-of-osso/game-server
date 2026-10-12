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

CurrencyTypes392 and3252 are absent in this Retail metadata (77 offers). Item124669 has ItemSparse/name and Item/icon rows but no content_item/template row (cost6130, Darkwater Skate142398 at vendor55103: one offer). All remain recorded in the CSV; runtime omits these78 offers and reports each missing identifier once per catalog load. The other121 ExtendedCost mount offers have complete cost metadata. It never invents metadata, substitutes IDs or drops only the missing part of a price. Non-mount ExtendedCost behavior remains unchanged. Vendor PlayerCondition/arena/guild/achievement evaluation and reputation discounts are not added.

## Development proof

`81a38b41`: six HostClient tests RED for omitted costs/absent reputation. `bf0ad0c5`:43 merchant regressions passed, including original gold-only purchase→learn/no-transmog; two new currency fixtures needed their authored Stormwind Exalted requirement. `e94f73a3`: eight `merchant::tests::mount_costs::mount_costs_` tests GREEN: multiplied item/currency/gold payment, insufficient payment/full-bag rollback, database reopen, minimum reputation boundary, faction rejection, two-token rollback and no legacy currency alias/free missing-cost offer.

Runner: `agent-run mount-vendor-costs python3 /home/osso/.worktrees/game-engine-s11/scripts/desktop-server-build.py --root /home/osso/.worktrees/game-server-s11 --test -p server --bin game-server mount_costs_`, with `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol-s9`. No check/lint/broad/final acceptance gate.

Existing `owned_commerce_sources_buyback_keeps_both_catalogs_across_vendor_and_storage_reopen` fails before merchant setup: `owned_commerce_source_tests.rs:31` loads a snapshot without `mount`, already required by item_templates at base89847221 (query introduced4fc0598d). Outside this slice's owned files; main owns fixture reconciliation and integration.

Concrete fixtures: cost5976 =15 Mark of Honor137642/icon1322720;1647 =70 Halaa Battle Token26045 +15 Halaa Research Token26044;2671 =5 Champion's Seal241/icon236246 +5,000,000 copper. Reputation:29227 requires Kurenai978 Exalted7;29102 requires Mag'har941 Exalted7.
