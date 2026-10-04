# Local Forever source snapshots

Product `wow_classic_beta`, build `1.60.1.70205`, build key `842b2e5d11f8d6fe257a5b73bd5cf6c6`. Extracted from local CASC, not CDN, at `game-engine/data/forever-1.60.1.70205/skyborne-probe/`. Exact original DB2 bytes; hashes pinned in [skyborne_data.py](../../skyborne_data.py).

- `1361579.db2`: FactionTemplate, layout `22B6DC22`; rows 3628/3629 only are imported. Field order checked against local WoWDBDefs `FactionTemplate.dbd` layout 22B6DC22 and server `db2_schema.sql`.
- `1349477.db2`: Map, layout `D43AFAC3`; row 2991 only is imported. Field order checked against Forever WoWDBDefs `Map.dbd` build 1.60.1.70205. Three encrypted sections remain unparsed; all requested IDs must resolve from readable records. Directory `2991`, name `Zephras Isle`, WDT 7198644.

Map's inline Corpse[2] and global record/string offsets are decoded, OceanLiquidTypeID has no destination column and is discarded, NavigationMaxDistance is converted from its source int32 to SQLite REAL numerically. Remaining columns retain their source types and order. No coordinates are inferred from Corpse[2].

These small pinned sources make the normal Retail DB2 rebuild deterministic without requiring another extraction or replacing the Retail table build provenance. Per-row build, FDID, layout and SHA-256 are stored in `db2_row_source`.
