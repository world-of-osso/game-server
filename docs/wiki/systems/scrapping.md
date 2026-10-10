# Scrapping Machine

Verified: 2026-10-10. Server-only preflight exists; **no item can currently be scrapped**. [Contract](../../specs/scrapping.md). Execution intentionally refuses eligible requests until authentic loot is established; no fabricated loot table, material yields or disposal collection rule.

## Retail evidence

1. Local Retail `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_ScrappingMachineUI/Blizzard_ScrappingMachineUI.lua:5–20` creates a 3×3 grid, indices0–8. `:31–36` uses `C_ScrappingMachineUI.ScrapItems` / `HasScrappableItems`; `:81–101` tracks the scrap spell and removes the current item on successful cast. Generated `ScrappingMachineUIDocumentation.lua` documents the API/events; `ItemDocumentation.lua:50–61` documents `C_Item.CanScrapItem(ItemLocation)`. This is not evidence for server loot yields or batch atomicity.
2. [Pinned TrinityCore ItemTemplate.h:303](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Item/ItemTemplate.h#L303): `ITEM_FLAG4_SCRAPABLE = 0x00000020`, i.e. **ItemSparse.Flags_3**, not Flags_0. Local Retail `data/db2/12.1.0.69933/GlobalStrings.csv:16940–16942` provides `ITEM_SCRAPABLE`, `ITEM_SCRAPABLE_NOT` and `ERR_CANT_BE_SCRAPPED` ("You can't scrap that item"). [SharedDefines.h](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Miscellaneous/SharedDefines.h) has spell failure278 and UI error1049. The flag is the supported preflight, not a claim that no machine-specific restrictions exist.
3. [GameObjectData.h:725–736](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/GameObject/GameObjectData.h#L725-L736) defines type48 UI_LINK, Data0 UILinkType2 = Scrapping Machine. Actual local TDB1210 Scrapper293132 has that metadata; newer Data7 interaction metadata is not implemented here.
4. [SpellEffects.cpp:353](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp#L353) maps effect261 SCRAP_ITEM to **EffectNULL**. `DB2Structure.h` and TDB1210 world SQL have no scrap table/schema. Local community-listfile contains no DBFilesClient name containing `scrap`; no ItemScrappingLoot FDID/exported table was found. Consequently no CASC table could be identified for `game-engine/scripts/export_db2_csv.py`; **no export was performed**, and this bounded name/schema search does not prove every CASC payload's absence. No Wago/Wowhead/CDN data used.
5. [Blizzard Warbands article](https://news.blizzard.com/en-gb/article/24115313/get-the-band-together-for-warbands) names disenchanting/deleting/selling, not scrapping. Bounded official-site search and the local UI do not explicitly establish scrap collection. Keep [established bind-time behavior](../investigations/appearance-acquisition.md); don't promote scrap-as-destruction inference to Retail evidence.

## Server boundary

`ScrappingPlugin` accepts **internal Bevy messages**, not network requests. `ScrapRequest` carries server-resolved player/machine entities and GUIDs. It checks alive/map-copy/model-box reach, cardinality/duplicates, bag ownership, Retail source and scrappable flag. `ScrapResponse` carries explicit failures; an otherwise eligible request returns `LootUnavailable`, not success or a false Retail eligibility error. No inventory save, item removal, material grant, collection grant or owner delta occurs on this path.

Type48/Data0=2 active-content spawns load with `ScrappingMachine` markers. This is not a `UseGameObject` frame implementation. A future wire adapter must derive the player from an authenticated InWorld connection, not trust client entity/character IDs, and recheck the target at execution. Client/shared-protocol work is lead-owned.

## Content and loot census

`world.db` active `content_gameobject_template` has **zero** type48 scrapping templates; thus Skyborne has no active machine. Raw TDB contains Scrap-O-Matic1000 entry293132 spawns501905 and501994 on map1643, plus Unraveling Sands532597 and Infinite Obliterum Forge536878 on map1220. Raw reference rows are not active Skyborne content and must not be promoted silently. No matching Shred-Master spawn found. No NPC scrapping role evidence found in the inspected UI/link metadata; NPC routing was not invented.

ItemSparse contains10,499 flagged rows. Real fixtures: Corlain Breastplate154840 is unbound BoE, appearance36157; Trashmaster's Mantle168970 is BoP; Worn Shortsword25 is unscrappable. Bind-time grants use existing inventory receipts and account persistence. Reopen tests distinguish the bound grant from an unbound item's absence; **they do not prove appearance collection after successful scrap**.

No database imports or canonical writes occurred. Lead needs an authentic loot source, its selector/rates/counts, and a separately approved active machine placement. Existing disenchant/material or item-container loot is not a substitute.

## Sources

- [Server](../../../crates/server/src/scrapping.rs), [spawn loader](../../../crates/server/src/game_objects.rs), [fixtures](../../../crates/server/src/scrapping_tests.rs).
- Local evidence: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/scrap-2026-10-10/`: pinned TC files, `retail-census.json`, `tdb-scrap-census.txt`, targeted test logs.
- Raw TDB archive: `/home/osso/Projects/world-of-osso/game-server/data/reference/tdb1210/TDB_full_1210.26091_2026_09_09.7z`.

## See Also

- [Appearance acquisition](../investigations/appearance-acquisition.md) — atomic receipts and explicitly bounded disposal rules.
- [Game objects](game-objects.md) — active content, map copies and model-box reach.
