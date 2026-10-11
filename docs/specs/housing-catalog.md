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

No verified first-house exterior/interior grant source is available. Do not infer default grants from component membership, UnlockItemID=0 or the placeholder House row. Rank3 may use one bounded authentic vendor-item redemption path backed by world.db/TDB npc_vendor or an authored DB2 item effect (lead approval, 2026-10-11); never turn StartingQuantity into ownership without grant semantics.

## Behavioral proof

Focused `scripts/tests/test_import_housing.py`: exact authored fireplace/plot data; unknown-field retention; no ownership/currency invention; idempotence; unrelated table preservation; wrong build/key and altered CSV/DB2 refusal; transactional rollback across tables.
