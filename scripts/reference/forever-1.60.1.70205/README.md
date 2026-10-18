# Local Forever source snapshots

Product `wow_classic_beta`, build `1.60.1.70205`, build key `842b2e5d11f8d6fe257a5b73bd5cf6c6`. Extracted from local CASC, not CDN, at `game-engine/data/forever-1.60.1.70205/skyborne-probe/`. Exact original DB2 bytes; hashes pinned in [skyborne_data.py](../../skyborne_data.py).

- `1361579.db2`: FactionTemplate, layout `22B6DC22`; rows 3628/3629 only are imported. Field order checked against local WoWDBDefs `FactionTemplate.dbd` layout 22B6DC22 and server `db2_schema.sql`.
- `1349477.db2`: Map, layout `D43AFAC3`; row 2991 only is imported. Field order checked against Forever WoWDBDefs `Map.dbd` build 1.60.1.70205. Three encrypted sections remain unparsed; all requested IDs must resolve from readable records. Directory `2991`, name `Zephras Isle`, WDT 7198644.

Map's inline Corpse[2] and global record/string offsets are decoded, OceanLiquidTypeID has no destination column and is discarded, NavigationMaxDistance is converted from its source int32 to SQLite REAL numerically. Remaining columns retain their source types and order. No coordinates are inferred from Corpse[2].

## Local data paths

Raw `.db2` files are not tracked. The complete reference directory (DB2 bytes plus text) lives in the untracked cache `/home/osso/Projects/world-of-osso/game-engine/data/db2/forever-1.60.1.70205/`. This repository retains the decoded JSON, WoWDBDefs definitions and provenance only. Tests continue reading those text files here.

Set `FOREVER_DB2_SOURCE` to that cache directory for Python tests and `scripts/import-azerothcore.sh`; an unset variable fails explicitly. Pass `--source-dir "$FOREVER_DB2_SOURCE"` to `skyborne_data.py`, `zephras_content.py`, `skyborne_kit_fixture.py`, `add_player_create_info.py`, `db2.py import`, and `content_policy.py` rebuild/check. None defaults to an in-repository DB2 source. CSV-only `db2.py check` and non-rebuild content-policy modes do not require raw DB2s.

The selected sparse fixture lives at `/home/osso/Projects/world-of-osso/game-engine/data/db2/fixtures/item_sparse_6fcc3191_selected.db2`; tests require `ITEM_SPARSE_SELECTED_FIXTURE` pointing to it. See [sparse decoding tests](../../../docs/db2-casc-sparse.md).

## Starting kit sources

The eight additional original DB2 snapshots come from engine `data/forever-1.60.1.70205/starter-kit-probe/` in the same local product/build. Their exact FDIDs, layout hashes and SHA-256 hashes are pinned in `skyborne_data.py::SOURCES`. `definitions/` preserves the matching official WoWDBDefs and `schema-provenance.json` records their supplied source URLs/hashes.

`starter-kit-rows.json` is the concrete decoded fixture: CharacterLoadout 2373–2378 (classes 4/3/1/8/7/11), Purpose 9, ModID 75, source extra field 0, masks (0,3), and 45 CharacterLoadoutItem memberships referencing 30 distinct items. Inline loadout/appearance relationships must agree with the WDC relationship map. Source-only ItemPetFoodID and ItemSparse AmmunitionType have no destination columns; Item's existing unknown Field_12_0_0_63534_010 receives its source AmmunitionType byte. ItemSparse keeps signed values, arrays and floats from the sparse decoder unchanged. Fixed ItemEffect palettes are narrowed per DBD: Charges signed16 (-1), SpellCategoryID unsigned16 (food 11/drink 59), and signed/unsigned widths of the other narrow fields; SQLite's INTEGER type alone does not encode those widths.

All 30 requested ItemSparse IDs are readable, including the 17 absent from Retail. Readable appearance/effect membership rows select required parent IDs explicitly; missing referenced parents fail. Unrelated encrypted sections are not decoded and no completeness claim covers hidden membership rows.

Shared IDs have concrete creation/count conflicts with Retail (2947 throwing weapon versus junk, 2512 ammunition versus junk, 2101 quiver versus bag, and binding changes). Existing Retail rows cannot be silently relabeled or replaced. Approved isolation stores source definitions in six `skyborne_item*` tables, preserving IDs; `skyborne_content_item` is the existing item projection over those tables. Collisions inside that namespace still fail. `retail-common-items.json` contains the 13 existing shared Item/ItemSparse/content rows copied read-only from the main-owned 6878f6a scratch DB (`target/skyborn-e2e/data/world.db`, Retail 12.1.0.69933); tests preserve them byte-for-value while importing conflicting Forever definitions.

## Creation fixture

`retail-kit-runtime-rows.json` contains read-only Retail rows for actual loadouts 1277 (Human Mage) and 1486 (the old race-95 Warrior kit), memberships and their item/appearance/effect dependencies from the same main-owned scratch DB. `scripts/skyborne_kit_fixture.py` imports those rows, applies the pinned Forever importer, regenerates content twice, checks original Retail values/masks, and emits SQL inside `crates/server/src/skyborne_kit_fixture.rs`. Desktop snapshots already admit Rust fixture modules. No database assembly or extraction is performed.

Produce with `python3 scripts/skyborne_kit_fixture.py --source-dir "$FOREVER_DB2_SOURCE" --output crates/server/src/skyborne_kit_fixture.rs`. Native `player_create_info::skyborne_tests` consumes this fixture through OwnedKit and actual redb creation/reopen, asserting all ten authored lists/counts and concrete gear. Runtime source-marker propagation remains a separate main-owned boundary.

## Creature display probe

`zephras_creatures.py` pins two additional local-CASC files. Both have two encrypted sections (one row each) excluded and reported. No encrypted-row completeness claim follows.

| FDID | Layout | SHA-256 | Readable rows |
|---|---|---|---|
| 841631 Creature | 6E14C900 | 772706ff900ff31227a0b61f39ec5073f038d48514def9f4f2fad03ae1cc070a | 179 including one copy |
| 1864302 CreatureXDisplayInfo | 2EA19FCF | 9b803b84e2fe4291a8e49f8a372490c4f0e06bbb389ac926a2bb3228a3cf42bb | 185 |

Field order verified against hash-matched local WoWDBDefs (`/home/osso/Repos/wowless/vendor/dbdefs/definitions/Creature.dbd` and `CreatureXDisplayInfo.dbd`): Creature = non-inline ID, Name/NameAlt/Title/TitleAlt strings, Classification int8, CreatureType uint8, CreatureFamily uint16, StartAnimState int8, DisplayID[4], DisplayProbability float[4], AlwaysItem[3]. CreatureXDisplayInfo = non-inline ID, CDI int32, Probability/Scale float, OrderIndex uint8, Field_11_0_0_54210_004 uint16, non-inline CreatureID relation. Matching layout hashes, not the DBD's different listed build numbers, establish the decode order. None of the refreshed 298 captured/referenced NPC IDs occurs in either readable table; neither giver's display is resolved by these DB2s. Refreshed captured Forever NPC pages instead resolve Ailee to 136968 and Ventaari to 139694; `zephras_content.py` uses those page appearances exclusively. WoW client Creature is not a full server NPC catalog.

These small pinned sources make the normal Retail DB2 rebuild deterministic without requiring another extraction or replacing the Retail table build provenance. Per-row build, FDID, layout and SHA-256 are stored in `db2_row_source`.
