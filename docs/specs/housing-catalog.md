# Build-qualified housing catalog

## Contract

- Import only local DB2 exports authenticated against the audit CSV/DB2 SHA-256 receipts and exact active Retail product/build/build key.
- Preserve every authored column and literal unknown field without reinterpretation. JSON field values retain the CSV representation.
- Key records by build key, DB2 table name and record ID. Keep older builds separate; replace one build atomically and idempotently without touching unrelated world tables.
- Preserve source FDID/layout/hash/row count, encrypted dropped-record counts and absent-table receipts. Missing source data is not a gameplay default.
- Catalog entries, StartingQuantity and raw plot Cost do not grant ownership, establish grant semantics, or establish currency units.

## Lead import (canonical database mutation; agents must not run)

From game-server after landing the housing-core importer:

```text
python3 scripts/import_housing.py --evidence /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/housing-audit-2026-10-11 --db /home/osso/Projects/world-of-osso/game-server/data/world.db --build 12.1.0.69933 --build-key dcfc90fffd79ba00406ae46f5f657592
```

The same command with an isolated output database verifies the actual 35-table export. It also records three absent initiative tables and 54 unreadable records. Static catalogs are not proof of playable neighborhoods, Housing Dashboard, decor acquisition, editor or visitor replication.

## Housing map metadata

Load authored Map.db2 types 7 (House) and 8 (Neighborhood), retaining map2735/2736/2783 names and WDT FDIDs. They are not dungeons and generic continent/dungeon admission remains refused until the housing service authorizes a particular neighborhood/owner/visitor copy. Loading map metadata is not hosting playable housing.

Property copies are keyed by authored map ID and authoritative neighborhood/house identity. Housing authority must reserve entry for a character before ordinary teleport/arrival can resolve that copy; unreserved characters and reservations for another property are refused. Copies use the common instance-ID allocator but have no synthetic MapDifficulty, dungeon locks or reset schedule. This admission primitive alone does not prove playable housing.

No verified first-house exterior/interior grant source is available. Do not infer default grants from component membership, UnlockItemID=0 or the placeholder House row. Rank3 may use one bounded authentic vendor-item redemption path backed by world.db/TDB npc_vendor or an authored DB2 item effect (lead approval, 2026-10-11); never turn StartingQuantity into ownership without grant semantics.

## Authoritative catalog coordinates

The server reads build-qualified NeighborhoodMap entries and NeighborhoodPlot cornerstone/teleport coordinates from the imported catalog. Supported shipping neighborhoods are map2735/2736; record4/map2640 has no matching Map.db2 row. Plot prices retain authored copper amounts under the Retail cornerstone MoneyFrame/GetMoney contract. Names may be authentically empty; do not generate neighborhood/plot names or grants. Missing catalog imports must produce an explicit unavailable error.

## Atomic ownership and basic storage

The housing persistence table records account ownership, paid plot claims, storage counts and stable placed-instance IDs independently of character serialization. Purchase and inventory copper debit commit in one transaction; competing buyers cannot both claim one plot. No first-house exterior/room/decor grants are inferred.

A bounded redemption candidate is authenticated by vendor252312/item245259/decor1693 (Small Val'sharah Bookcase), ExtendedCost0, item_sparse.BuyPrice500000 and collect spell1238340/effect324/MiscValue1693. Storage redemption consumes one owned Retail bag item and grants one stored copy atomically; it does not create vendor items or waive acquisition costs.

Only an owner inside that house edits. Place consumes stored quantity, remove restores it, move/rotate preserve quantity, nonfinite transforms are refused. Account ownership/storage/layouts survive database reopen. Visitors require explicit owner HouseAccessAnyone configuration, receive the same saved layout, cannot edit and never receive the owner's private account storage. Unset access remains unknown and grants no visitor permission.

These are persistence/domain contracts, not proof of live neighborhood admission, cornerstone interaction, models, native editor or networking integration.

## Behavioral proof

Focused `scripts/tests/test_import_housing.py`: exact authored fireplace/plot data; unknown-field retention; no ownership/currency invention; idempotence; unrelated table preservation; wrong build/key and altered CSV/DB2 refusal; transactional rollback across tables.
