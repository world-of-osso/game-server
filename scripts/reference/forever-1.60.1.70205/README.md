# Local Forever source snapshots

Product `wow_classic_beta`, build `1.60.1.70205`, build key `842b2e5d11f8d6fe257a5b73bd5cf6c6`. Extracted from local CASC, not CDN, at `game-engine/data/forever-1.60.1.70205/skyborne-probe/`. Exact original DB2 bytes; hashes pinned in [skyborne_data.py](../../skyborne_data.py).

- `1361579.db2`: FactionTemplate, layout `22B6DC22`; rows 3628/3629 only are imported. Field order checked against local WoWDBDefs `FactionTemplate.dbd` layout 22B6DC22 and server `db2_schema.sql`.
- `1349477.db2`: Map, layout `D43AFAC3`; row 2991 only is imported. Field order checked against Forever WoWDBDefs `Map.dbd` build 1.60.1.70205. Three encrypted sections remain unparsed; all requested IDs must resolve from readable records. Directory `2991`, name `Zephras Isle`, WDT 7198644.

Map's inline Corpse[2] and global record/string offsets are decoded, OceanLiquidTypeID has no destination column and is discarded, NavigationMaxDistance is converted from its source int32 to SQLite REAL numerically. Remaining columns retain their source types and order. No coordinates are inferred from Corpse[2].

## Creature display probe

`zephras_creatures.py` pins two additional local-CASC files. Both have two encrypted sections (one row each) excluded and reported. No encrypted-row completeness claim follows.

| FDID | Layout | SHA-256 | Readable rows |
|---|---|---|---|
| 841631 Creature | 6E14C900 | 772706ff900ff31227a0b61f39ec5073f038d48514def9f4f2fad03ae1cc070a | 179 including one copy |
| 1864302 CreatureXDisplayInfo | 2EA19FCF | 9b803b84e2fe4291a8e49f8a372490c4f0e06bbb389ac926a2bb3228a3cf42bb | 185 |

Field order verified against hash-matched local WoWDBDefs (`/home/osso/Repos/wowless/vendor/dbdefs/definitions/Creature.dbd` and `CreatureXDisplayInfo.dbd`): Creature = non-inline ID, Name/NameAlt/Title/TitleAlt strings, Classification int8, CreatureType uint8, CreatureFamily uint16, StartAnimState int8, DisplayID[4], DisplayProbability float[4], AlwaysItem[3]. CreatureXDisplayInfo = non-inline ID, CDI int32, Probability/Scale float, OrderIndex uint8, Field_11_0_0_54210_004 uint16, non-inline CreatureID relation. Matching layout hashes, not the DBD's different listed build numbers, establish the decode order. None of the 295 captured Zephras NPC IDs occurs in either readable table; neither giver's display is resolved. WoW client Creature is not a full server NPC catalog.

These small pinned sources make the normal Retail DB2 rebuild deterministic without requiring another extraction or replacing the Retail table build provenance. Per-row build, FDID, layout and SHA-256 are stored in `db2_row_source`.
