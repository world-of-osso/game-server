# Content Import

Quest, vendor, loot, gossip and item content comes from AzerothCore (Wrath 3.3.5) dumps imported into `data/world.db`; the client and spell/class data are Retail `12.1.0.69933`. Raw dump tables keep their Wrath IDs verbatim. `scripts/content_policy.py` derives the Retail-first `content_*` tables that the server should read (policy chosen 2026-09-23, option 4 below). This page records the imported tables, the policy and its drop counts, and the ID measurements that motivated it.

## Zone quest/service promotion (2026-10-11)

The same `world-promote` transaction now adds missing TDB quests attributed by AreaTable continent/QuestSortID or by selected-map giver/ender relations. It reuses `retail_content`'s template/objective/text/POI/reward converters and vendor importer; existing quests and authored relations are not rebuilt. QuestV2 membership and the existing four creature/GO and six item slots are required. Unsupported objective types or overflowing slot counts exclude the entire new quest, rather than silently making it completable. Completion-event/area-trigger flags retain their existing exploration requirement. Missing source giver/ender placements remain missing: no synthetic NPCs, links or quest objectives.

Vendors use authored TDB item rows with present Retail items; currency rows and absent items are reported and excluded. Taxi lookup/open/takeoff now use the NPC's `InMap.map_id`; the reply's continent follows its authored current TaxiNodes row, removing the obsolete hardcoded map0 boundary. TaxiNodes mount creature models absent from content are copied from TDB model rows, including mounts with no world spawn. No source model means no fabricated takeoff model and an explicit report entry. Trainer services already read `tdb_creature_trainer`/`tdb_trainer`/`tdb_trainer_spell` directly; flight masters use the source NPC flag and existing DB2 taxi nodes/paths. Promotion supplies their authentic placements/templates, not invented service relations. Strict world phase/difficulty and positive-only GO event filters remain unchanged. Existing shared templates, unrelated rows and same-map overlays remain protected. The legacy content schema requires non-null spawn flags, so source NULL is encoded as zero; the existing runtime treats zero/null as template inheritance: Lord Autumnvale127080 loads flags320 in ECS. The separate client attack check must respect player immunity (0x100) before selecting its attack path; a generic attackability check is insufficient for neutral quest enders. A same-map placement matching a raw AzerothCore row's GUID, map, entry and coordinates is legacy import content, not an overlay: it is replaced by TDB rather than duplicated. Twelve previously unhosted source-backed open maps (369,530,571,609,646,648,654,730,732,860,861,974) join the host list. Ground selection for deploy.sh now unions creature and gameobject maps, including GO-only zones; missing ground/navmesh is reported, not fabricated.

`--mode world-census --maps <hosted IDs> --csv-root <Retail CSV build directory>` is read-only and ranks zones by distinct TDB QuestSortID/AreaTable parent rollup. Spawns use declared area/zone first; unassigned rows may be attributed to the smallest covering world-system zone UiMapAssignment rectangle with an AreaTable identity. Inferred counts are explicit and do not change placement data; unmatched rows stay in zone0. Givers/enders count distinct creature/GO entries with authored relations, not spawn duplicates. Census numbers do not imply phase/script/runtime availability. `world-promote` reports multiset before/after/added/removed row counts for every content table.

## Open-world population (2026-10-11)

`content_policy.py --mode world-promote --maps <explicit IDs>` bypasses full policy regeneration and overlay clearing. `world_population.py` reuses the instance import functions with open-world difficulty0 and strict unflagged phase0/169/no groups/no terrain swap. TDB-owned GUIDs use +20m; authored placements outside that namespace survive even on a promoted map, with their addons/formations/events. Shared templates, existing waypoint rows and unrelated pools are restored by indexed snapshots inside the same transaction. Copy-only destination protections are shared with the instance importer.

The old runtime host list (0/1/2991) independently prevented admission and startup spawning. `HOSTED_CONTINENTS` now includes the same31 locally present, source-backed expansion maps. [Full589-map/762-zone census,31-map counts and both-skin Boralus evidence](../../world-population-counts.md) own coverage and limitations. Source-empty maps, conditional phases and client appearance coverage remain explicit gaps; Zandalar's only eligible NPC is an inactive event Ghost, not an invented populated hub. Scoped promotion must be reapplied after a full content regeneration.

Sources: [contract](../../specs/retail-world-content.md#open-world-population-2026-10-11), `scripts/world_population.py`, `scripts/tests/test_world_population.py`, `crates/server/src/maps/instances.rs`, `maps/tests.rs`, `zones.rs`.

## Instance population (2026-10-11)

`import_instance_population.py` applies authentic TDB instance spawns to an existing **copy**, refusing `world.db`, symlinks and hardlinks. `content_policy.apply_policy` also runs this step after the world/quest/vendor policy, so regeneration retains the fix. Source tables remain unchanged; GUIDs and instance waypoint paths use +20,000,000 namespaces. Same-ID legacy creature templates/models are replaced by TDB identity; addons, equipment, formations and events follow their source. Unrelated source-backed loot references remain intact. Instance templates and every resolved spawn difficulty rebind authored TDB LootID (+20,000,000 when nonzero) and GoldMin/GoldMax, including gold-only rows. `retail_content.insert_tdb_loot` imports creature/reference rows using the existing Retail item policy. Nonzero TDB loot remains authoritative. Zero TDB LootID retains existing source-backed Wowhead bindings (the master-red-fix Hogger regression); no new loot rows or loot-roll implementation are fabricated.

The initial importer rebuilt difficulty rows with zero loot/gold and only restored preexisting offset bindings. Cobrahn3669 Normal1 therefore lost authored LootID3669/gold87–168. The binding step now follows the same resolved difficulty rows as template construction; copy/idempotency regression checks all imported base/difficulty bindings plus Cobrahn's eight authored item rows.

Root causes were the four-map Retail importer and five-map runtime NPC appearance allowlist. `zones::spawn_map_creatures` now trusts the copy's map/difficulty content, not client cache coverage. Gameobject loading includes all model-bearing instance rows (`spawnDifficulties` other than continent-only `0`), preserving the existing six-type world-map policy. New object types gain rendering, not invented interaction/scripts.

The original pinned SQL audit matched all16 instance-scoped reference tables (spawns, templates/models/difficulties, addons, equipment, formations, events and waypoint paths/nodes) on all124 stored TDB instance maps. That proved source equivalence for those124 maps, not completeness of the dungeon/raid roster. Copy imports 34,333 creatures and 7,446 gameobjects from 34,630/8,082 raw source rows. Phase/difficulty exclusions, inactive events, six missing source gameobject templates and model-less objects are explicit in the [per-map table](../../dungeon-population-counts.md). Always-visible flag1 preserves both authored Font of Power rows (maps2515/2521); inverse/conditional phase progression, source-empty maps remain gaps; [raid admission/locks](raid-entry.md) extend the imported maps. No rows/models are fabricated.

Proof: importer copy/idempotency/preservation tests; loader census across all maps; live ECS creation of Normal Deadmines/Wailing Caverns/Ragefire 420/148/138 NPCs. Private UDP5536 client entered all three; Wailing Caverns Ravager HP fell 18,772→18,634. Full both-skin visible/attackable proof remains **blocked**: 14 missing verified asset receipts and 24 display coverage misses in the Forever run; Modern run retained a dead/transfer state after setup revive. Every retained PNG was inspected. Detailed ledger, screenshots and no-install FDIDs: `/home/osso/.worktrees/handoff-dungeon-population.md`, engine `data/diagnostics/dungeon-population-2026-10-11/`, `/syncthing/AgentShared/2026-10-11/dungeon-population/`.

### DB2 roster expansion

`instance_template` defines parent/script metadata, not every authored dungeon/raid. Selection now unions its124 maps with imported local `Map.db2` InstanceType1/2 maps having `MapDifficulty` rows, excluding garrison directories from that expansion. Selected fields exactly match local `data/db2/12.1.0.69933/Map.csv`; all1908 difficulty rows match `MapDifficulty.csv`. The roster has231 maps. Shared entries choose their lowest existing-template-map spawn difficulty for base template/Wowhead binding, then the lowest new-map difficulty only for entries absent from the old roster. This prevents four shared base templates changing when newly covered maps offer lower difficulties.

[Per-map census and reachability](../../instance-coverage-counts.md) owns counts, sparse-source gaps, existing124-map row-diff proof and live evidence. Party maps need no `instance_template` row to be hosted; [raid admission/locks](raid-entry.md) now govern raids. Placement import does not create missing entrance teleports or terrain.

Sources: [contract](../../specs/retail-world-content.md#instance-population-2026-10-11), `scripts/import_instance_population.py`, `scripts/tests/test_import_instance_population.py`, `crates/server/src/zones.rs`, `crates/server/src/game_objects_tests.rs`.

## Disenchant source restoration (2026-10-10)

The missing excerpt was restored from [TDB1210.26091](https://github.com/TrinityCore/TrinityCore/releases/tag/TDB1210.26091), the same release already identified by [Retail world content](../../specs/retail-world-content.md). Archive `TDB_full_1210.26091_2026_09_09.7z` (156449334 bytes) SHA256: `fc5513334d7534a19f533124a95910193c8150379e5ed8d0db3e547ac2c5a3f5`. The disenchant excerpt (6902 bytes) SHA256: `6fdbcbbf84199ff114039872d57facb01a5da3e01abbc91450a2e1348de718ba`. All 14669 reference-loot rows matched `tdb_reference_loot_template` exactly; no reference replacement. Canonical world.db was backed up beside itself as `world.db.pre-disenchant-20261010`, then only the new table was imported through `mysql_dump.py --prefix tdb_` using its DDL from `tdb_schema.sql`.

Local Retail 12.1.0.69933 CASC `ItemDisenchantLoot` (FDID1134087, layout5845A4AA) has 166 records, all with `SkillRequired=0`. Legacy `item_template.RequiredDisenchantSkill` is not equivalent. Item727 matches Retail entry3 (level5), not legacy DisenchantID21. Companion unmerged `disenchant` branches now supply the item-GUID contract, typed refusals and [gameplay pipeline](disenchant.md); this is not a master/deployment claim.

## Imported tables

`scripts/import-azerothcore.sh` loads every table in its `DUMP_TABLES` list from `~/Repos/azerothcore/data/sql/base/db_world/<table>.sql` (AzerothCore `90d7228`, 2026-02-24) via `scripts/mysql_dump.py`. DDL is in `scripts/world_schema.sql`; column order matches the dump `CREATE TABLE` because the dumps use positional `INSERT`s.

| Table | Rows | Content |
|-------|------|---------|
| `quest_template` | 9,464 | Quest text, levels, objectives, rewards (Wrath item/spell IDs) |
| `quest_template_addon` | 9,464 | Prev/next quest chains, class/skill requirements |
| `quest_offer_reward` / `quest_request_items` | 8,660 / 7,757 | Turn-in and progress text |
| `quest_poi` / `quest_poi_points` | 18,771 / 57,166 | Quest map blobs |
| `creature_queststarter` / `creature_questender` | 7,445 / 7,901 | NPC → quest |
| `areatrigger_involvedrelation` | 76 | AreaTrigger.db2 id → exploration quest |
| `gameobject_queststarter` / `gameobject_questender` | 457 / 457 | Object → quest |
| `gameobject_template` / `gameobject_template_addon` / `gameobject` | 21,581 / 21,493 / 96,459 | Game object templates and spawns ([[game-objects]]) |
| `npc_vendor` | 37,585 (2,018 vendors) | Vendor item lists; negative `item` = reference to another vendor |
| `tdb_trainer` / `tdb_trainer_spell` / `tdb_creature_trainer` / `tdb_skill_tiers` | 192 / 5,402 / 975 / 59 | Trainers come from TrinityCore TDB1210.26091 (Retail 12.1.0) excerpts, not AzerothCore (whose trainer tables are no longer imported: its profession trainers teach the Apprentice/Journeyman/Expert/Artisan ranks Retail removed, its class trainers Wrath ranks) in `data/reference/tdb1210/` (source: `SOURCE.txt` there), loaded with `mysql_dump.py --prefix tdb_` and `scripts/tdb_schema.sql`; Retail profession trainers and skill tier caps ([professions](professions.md)) |
| `creature_loot_template` / `gameobject_loot_template` / `reference_loot_template` / `item_loot_template` | 249,466 / 17,957 / 44,536 / 4,997 | Loot tables |
| `tdb_disenchant_loot_template` | 180 | TDB1210.26091 material loot, keyed by Retail `ItemDisenchantLoot.ID`; restored 2026-10-10; [unmerged gameplay handler](disenchant.md). |
| `gossip_menu` / `gossip_menu_option` | 6,055 / 4,604 | Gossip menus |
| `npc_text` / `broadcast_text` | 8,361 / 73,039 | Gossip and NPC text |
| `points_of_interest` | 463 | Gossip map POIs |
| `item_template` | 46,096 | Wrath item data: stats, prices, quality, class/subclass, displayid |

`creature_template.npcflag` (already imported) marks roles: 2,137 templates with vendor bit 128, 827 with trainer bit 16.

**Taxi:** AzerothCore has no taxi data in SQL; `taxinodes_dbc`/`taxipath_dbc`/`taxipathnode_dbc` are empty override tables (the core reads the client's DBCs). Taxi statistics below use Wrath Classic DB2 as the stand-in.

### Skyborne overlay

[[skyborne]] documents the pinned local Forever faction/map overlay applied inside `db2.py import`, including regeneration behavior and row provenance.

### Forever Zephras reference capture

Verified 2026-10-04; [contract](../../specs/wowhead-forever-reference.md). `scripts/wowhead_forever.py` captures individual `/forever/` pages under `data/reference/wowhead/forever-zephras-<YYYYMMDD>/{zone,quest,npc,object}/<id>.json`. URL, UTC timestamp, scrape name, raw HTML, Listviews, Gatherer/entity objects, mapper data and quest text remain inspectable. HTTPS is sequential with at least 1.5 seconds between requests; transient GET failures retry boundedly, but 403/429 stop. `--source` accepts inspected browser-rendered HTML; it does not bypass access controls.

The refreshed `forever-zephras-20261004` snapshot contains 1 zone, 147 quest, 298 NPC and 3 object pages, with empty missing-page lists. Earlier CloudFront/browser 403s are historical; the retry captured quest 97967 and intro giver 251362. `zephras-content.json` retains 146 zone quests: seed 93739 remains excluded because its mapper places it outside Zephras. All 298 captured NPC pages expose displays (235 unique CDI IDs). The capture now includes quest-referenced NPCs outside the zone; only calibrated terrain-supported Zephras observations become spawns. Null fields remain unknown. Interaction labels are not converted into kill credit.

Calibration uses local Forever UiMapAssignment FDID 1957219, hash-matched layout `C9CC8DFB`: assignment 69852 maps UiMap 2665 to map 2991. `bounds.json` and `assignments.json` retain decoded bounds and SHA-256. Wowhead's UiMap 2521 assignment differs only by 0.003052 yards at minimum world X. Percent inversion swaps screen axes and reverses them: world X derives from map V; world Y from U. `calibration-proof.json` records a retail Goldshire check against the engine's existing map fixture: absolute errors 0.091 X and 0.154 Y yards, below percent-rounding uncertainty.

`Terrain.position` uses WDT 7198644 MAID and ports server `terrain_height.rs` MCVT fan/hole sampling. After synchronization, existing normalization re-samples captured coordinates against recovered roots, without HTTP. `target/forever-resync/terrain-vs-native.json` compares all 72 centers with the refreshed real server payload: 22 supported heights agree within 0.01 yards; 50 centers are authored holes, none have missing roots. Earlier `terrain-proof.json` is historical incomplete-cache evidence. Current calibrated NPC/object observations include 1,690 terrain-supported and 25 hole/outside positions. These remain mapper observations, not authored spawns; no WMO floors, liquid safety, facing or phases are inferred.

`start-proposals.json` retains capture-time proposals; [[skyborne]] owns the subsequently approved starts and the race-96 inference. Coming of Age 92460 keeps source race bit 32; Skysight 92598 keeps its source mask 0 and level-2 gate. Source masks are not widened or narrowed.

### Forever Zephras overlay

[Contract](../../specs/zephras-content.md). `scripts/zephras_content.py --db <scratch.db> --report <report.json>` reads the normalized snapshot above and pinned `zephras_creatures.py` sources. `content_policy.py` preflights the capture, then applies the overlay after Retail content in the same transaction. Normal `import-azerothcore.sh` regeneration therefore reapplies it. `--zephras-content` selects an inspected snapshot; missing input stops preflight, not a source fallback. The capture remains ignored data and must be retained/provisioned separately.

| Reconstructed rows | Count |
|---|---:|
| NPC templates / grounded spawn observations / model rows | 278 / 1,662 / 278 |
| Quest templates / addons / offer text / native metadata | 146 each |
| NPC starter / ender links | 130 / 131 |
| Explicit A/H reaction templates | 6 |

`zephras_row_source` tags every row with scrape, URL, `reconstructed` and source JSON. Stable spawn GUIDs use NPC ID plus observation index; repeated exact points are deduplicated. Reapplication removes only previously tagged rows. Reports retain normalized SHA-256, unresolved links, missing displays, unsupported objectives, dropped rewards and counts. A failed standalone import rolls back schema and rows; two full scratch regeneration cycles are behavior-tested.

NPC A/H react −1/0/1 becomes explicit enemy/neutral/friend towards Forever Faction IDs 2795/2794, using derived template IDs 90000000–90000008. Missing sides use the approved neutral-identity/friendly policy. Faction identity/groups remain zero rather than guessing a nation. NPC→player reactions match captured pairs; friendliness is symmetric, but reverse hostility is not invented. Levels come from Wowhead; 20 unknown/9999-level templates and their 31 observations are omitted. Post-sync re-normalization/import recovers all 463 formerly missing-root eligible positions; 14 hole/outside and 6 uncalibrated/out-of-zone coordinates remain skipped. SQL comparison preserves all 1,199 previous spawn rows unchanged and adds exactly 463. Publication is new scratch `target/forever-resync/world-scratch.db`, not live deployment.

Missing NPC stats policy (2026-10-05): current normalized NPC pages and decoded Creature catalog contain no authored class/health fields. Reconstructed templates explicitly receive the user-approved coarse `unit_class=1`, `HealthModifier=1.0` estimate, not an authored Warrior classification. Template `source_json.stat_estimate` records fields, basis, analogue IDs 713 (Balir Frosthammer), 714 (Talin Keeneye), 23904 (Sir Thomas), and the current `creature_classlevelstats` class-1 `basehp0` curve at captured `minlevel` (42 HP at level 1, 102 at level 5, modifier 1). These normal-rank exp-0 quest givers support a coarse baseline, not inferred spellcaster classes or full Classic combat. Captured source JSON/URL, levels, rank/type/family and all non-overlay stats remain intact; repeat regeneration and collision rollback are behavior-tested. No source field aliases or runtime fallback are added. Class-1 curve rows cover levels1–60. Estimates concern NPC health/unit-class only; displays, scripts and rewards are not estimated. Ailee136968 texture/profile remains unresolved.

Verified2026-10-05 at server `c5cf88ab7d85bcdc86fc0f2e95c6f882b5f1f7f8`: map2991 startup gate includes the overlay and the unignored imported-row test asserts1662 spawns, Ailee current/max42 and Ventaari current/max102. Independent [bounded startup ledger](../../../target/estimated-npc-startup-proof/owned-ledger.md) records6 pass (startup + five quest boundaries), one ground case ignored. Independent [fresh whole-workspace ledger](../../../target/estimated-npc-integration-proof/ledger.md) records one execution:1448 passed,0 failed,57 ignored, one user-requested latency skip, exit0, zero compiler warnings. This replaces the need for a composed pass count at this revision, not historical failure records or ignored-case coverage.

Only owned scratch and desktop fixtures were published, with backups retained. Consistent uploaded fixture SHA256 `acf9f6ca73187743158f2bf1105eb5ef0a8fba6110f17b5784eeed7a0d6fb662` differs physically from working `target/skyborn-e2e/data/world.db` SHA256 `2ec30aed19ec77ef2ddf7ef9ca4608eefae1909670677a7e539978b21a66ba5b`; bounded startup ledger records logical query agreement, while the whole-workspace verifier does not independently claim logical equivalence. Canonical DB was not mutated. Current export receipts: `target/skyborn-e2e/estimated-server-export*`. [Engine native SSOT](../../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#current-scoped-capability-matrix) owns saved race95 QuestFrame92460 acceptance, race96 visible Ventaari and exact visual/exit/gameplay limits; independent native artifact gate remains pending.

All 146 normalized zone quests now have captured pages and are imported; 27 giver links still reference omitted templates. Kill/item objectives use native columns; interactions, unknown action labels and counted prose-only objectives stay in provenance and block completion (`zephras_quest_meta`). 49 quests have that block. Coming of Age has no counted objective and turns in at its captured ender; Skysight's prose event cannot auto-complete. Fixed captured XP avoids a guessed Retail difficulty. Item rewards require Retail/runtime IDs, spells require Retail IDs: 109 absent rewards are dropped and reported. Reputation, faction-side restrictions, unsupported branches and reward-spell execution remain native gaps, not fabricated substitutes.

Hash-matched local Forever Creature 841631 (`6E14C900`, 179 readable rows) and CreatureXDisplayInfo 1864302 (`2EA19FCF`, 185 readable rows) resolve **0/298** captured NPCs; two encrypted sections in each remain excluded. Appearance IDs come exclusively from captured Forever NPC pages, not client-table substitutions. All 298 pages have a display; 278 valid-level templates receive model rows. Ailee Farheart 251362→136968 and Ventaari Brightwish 251487→139694 are now resolved. `engine-needed-display-ids.txt` beside each report lists all 235 captured CDI IDs, including omitted/ungrounded/referenced NPCs; `spawn_display_ids` in refreshed `target/forever-resync/import-report.json` identifies 218 used by grounded spawns (previously 174). Client CDI rows/model assets remain separate provisioning, not rendered-client proof. No engine checkout or live world/player storage was changed. Three referenced objects, loot, vendor inventories, gossip, scripted events and phasing are not reconstructed.

Evidence: checkout `target/zephras-content-proof/` (scratch DB, reports, native/Python logs and ledger). Full base-policy scratch regeneration passed; local/desktop scratch overlay counts agree. This is server data/message proof, not complete zone gameplay or rendered-client acceptance.

### DB2 refresh preserves derived factions

Verified2026-10-14: `db2.py import` regenerates the six approved Zephras reaction templates from existing `zephras_row_source.source_json` via `zephras_content.faction_row`, after replacing Retail tables and reapplying Skyborne DB2 rows, before commit. `db2_row_source` marks these as derived (receipt scrape, FDID0, layout`derived`, exact source-JSON SHA256); malformed keys/collisions roll back. Build receipts owned by other importers remain untouched. No capture fetch or copying stale runtime values. [Regression receipts and118-table audit](../../master-reds-70391.md#zephras-derived-factions-and-replacement-audit) distinguish six lost derived rows from retained Skyborne rows and two unavailable old-source comparisons. Canonical application remains lead-owned.

### Loader

`mysql_dump.py` tokenizes dump `INSERT`s and decodes MySQL string escapes before binding parameters. The previous awk/sed path rewrote only `\'`, so 168 creature names were stored with literal backslashes (`Remy \"Two Times\"`); they now load as `Remy "Two Times"`. `--schema` runs `world_schema.sql` inside the load transaction, so the dump tables can be reloaded into a populated `world.db` without touching DB2 tables, and a failed parse rolls back. The full script still deletes and rebuilds `world.db`.

## Derived merchant rebuild provenance

Verified 2026-10-01: tracked [`scripts/derived-merchants.json`](../../../scripts/derived-merchants.json) records user-approved approximate Zaralda placement and two source-backed unlimited suppliers. `content_policy.py --derived-merchants` loads the required manifest on `--check` before destructive rebuild; invalid/missing provenance stops preflight. Integration commit `e43f4d3` follows terrain `830157a` and publisher `94e6d88`. [[midnight-economy-content]] owns exact source/terrain coordinates, local publication deltas and evidence paths; this is not a captured Retail spawn or independent integration acceptance. Classic catalog and running service are unchanged.

## ID mapping measurements

Reproduce with `scripts/content_mapping.py --db data/world.db` (CSV defaults: the db2.py root). The Retail CSVs (Item, ItemSparse, ItemAppearance, ItemModifiedAppearance, ItemDisplayInfo, ItemXBonusTree, SpellName, CreatureDisplayInfo, CreatureModelData, QuestV2, QuestLineXQuest, TaxiNodes, TaxiPath, TaxiPathNode) and the Wrath Classic `3.4.0.47659` CSVs (SpellName, CreatureDisplayInfo, CreatureModelData, TaxiNodes, TaxiPath) are wago.tools exports under the db2.py CSV root `game-engine/data/db2/<build>/` (one root for every build; `3.4.0.47659/` holds the Wrath Classic files). Wrath Classic stands in for 3.3.5 names because AzerothCore ships no spell names or display data. "Name" comparisons are exact; case-only differences are counted separately.

### Installed item projection

`retail_content.retail_item_ids` selects every installed `ItemSparse` row with a matching `Item` row. TDB1210 references describe available content, not the definition catalog; filtering by those older references omitted new 70391 gear such as 270635. This matches TrinityCore a352b1fa `ObjectMgr::LoadItemTemplates` (`ObjectMgr.cpp:2991-3000`: iterate `sItemSparseStore`, require `sItemStore.LookupEntry(sparse->ID)`).

After importing the installed DB2 tables, run `python3 scripts/content_policy.py --mode items-refresh --db <private-copy>`. It rebuilds only `content_item` and the existing isolated Skyborne projection, retaining reviewed `SourceEntry` mappings. Lead owns canonical publication. Full policy regeneration uses the same installed catalog. The additive kit updater no longer depends on unrelated Toy/TDB/drop tables for definition membership.

Tests: `scripts/tests/test_installed_content_items.py` exercises the nineteen reported gear definitions, source-exact attributes, idempotence and retained mappings. `skyborne_kit_fixture.py` regenerates the native fixture; stale currency/vendor table DDL was removed by regeneration, not by narrowing equality assertions. The historical counts below predate installed-build projection.

### Items: `item_template.entry` vs Retail `ItemSparse.ID`

| Set | Same id + name | Case-only | Same id, other name | Id absent, name unique | Unmatched |
|-----|----|----|----|----|----|
| All 46,096 | 35,250 (76.5%) | 221 | 1,962 (4.3%) | 149 | 8,498 (18.4%, mostly `OLD…`/test items) |
| Vendor items, 7,647 | 7,316 (95.7%) | 17 | 291 (3.8%) | 0 | 22 |
| Quest reward items, 5,051 | 4,865 (96.3%) | 60 | 84 (1.7%) | 2 | 38 |
| Creature loot items, 8,204 | 7,971 (97.2%) | 59 | 102 (1.2%) | 15 | 56 |

Same id with a different name is sometimes a rename (`1181` Scroll of Spirit → Scroll of Versatility, `2593` Flask of Port → Flask of Stormwind Tawny) and sometimes a different item (`1360` Stormwind Chain Gloves → Stormwind Plate Gloves, `2817` Soft Leather Tunic → Hard Leather Tunic, `1019` Red Linen Bandana → Linen Scrap). Name matches for absent ids point at unrelated Retail rows (`5397` Defias Gunpowder → `221485`, `17` Martin Fury → `244855`).

For the 35,250 same-id-same-name items, AzerothCore attributes agree with Retail only partly: class/subclass 94.0%, InventoryType 97.5%, quality 96.7%, BuyPrice 39.8%, SellPrice 40.5%, RequiredLevel 40.1%, ItemLevel 16.2% (level squish), displayid 20.1% (Retail consolidated appearances: `38` Recruit's Shirt AzerothCore display `9891`, Retail `ItemModifiedAppearance`→`ItemAppearance` display `5729`). Of 8,492 quests whose id exists in Retail, 388 reference a reward/required/start item that does not map by id + name (case-insensitive), e.g. `21` Skirmish at Echo Ridge (`2186`), `38` Westfall Stew (`732` Okra → Ripe Okra).

### Spells: AzerothCore spell ids vs Retail `SpellName`

| Set | Same id + name | Same id, other name | Id absent, name ambiguous | Id absent, name unique | Unmatched / no name |
|-----|----|----|----|----|----|
| All `trainer_spell`, 3,592 | 1,454 (40.5%) | 190 | 1,520 (42.3%) | 81 | 347 |
| Class trainers (`trainer.Type=0`), 2,014 | 260 (12.9%) | 39 | 1,519 (75.4%) | 67 | 129 |
| `quest_template.RewardSpell`, 235 | 161 (68.5%) | 10 | 17 | 2 | 6 / 39 |
| `quest_template.RewardDisplaySpell`, 76 | 53 (69.7%) | 7 | 3 | 0 | 2 / 11 |

Class trainer spells are mostly Wrath ranks (`143` Fireball rank 2, `591`/`598` Smite) whose ids no longer exist; Retail has one spell per ability. Resolving a name within the trainer's class via Retail `skill_line_ability` gives one Retail spell for only 244 of 1,519 (`205` Frostbolt → `116`, `598` Smite → `585`). Even same-id-same-name class spells are in Retail `skill_line_ability` for only 142 of 260 (`53` Backstab, `139` Renew, `348` Immolate are not). Same-id renames include behaviour changes (`527` Dispel Magic → Purify, `528` Cure Disease → Dispel Magic, `99` Demoralizing Roar → Incapacitating Roar). Quest reward spells matching by id are mostly teach/summon/cosmetic spells; only 2 of 161 are class abilities.

### Creature displays: `creature_template_model.CreatureDisplayID` vs Retail `CreatureDisplayInfo`

21,381 distinct display ids: 21,128 (98.8%) exist in Retail; 253 are absent (e.g. `30`, `79`, `239`). Those are Wrath-only ids; `content_creature_template_model` replaces them with TrinityCore's Retail models (below). Of those present, 6,684 resolve to the same model file as Wrath Classic and 14,444 to a different file, which are the HD model upgrades (display `26`: `creature/kobold/kobold.m2` → `creature/kobold2/kobold2.m2`; `38`: `tallstrider.m2` → `tallstrider2.m2`). The display id still names the same creature appearance.

### Quests: `quest_template.ID` vs Retail `QuestV2.ID`

Retail DB2 has quest ids but no titles, so only id presence is measurable. 8,492 of 9,464 (89.7%) exist; for creature-started quests 6,719 of 7,227 (93.0%). 2,078 appear in a Retail `QuestLineXQuest`. Absent ids are quests removed in Cataclysm (`16` Give Gerard a Drink, `117` Thunderbrew Lager, `579` Stormwind Library). Presence does not prove the Retail quest has the same objectives; Retail reworked zones (Northshire's `783` A Threat Within still exists by id).

### Taxi: Wrath Classic vs Retail

TaxiNodes: 335 of 363 (92.3%) same id + name; 22 same id renamed by Cataclysm (`21` Kargath → New Kargath, `26` Auberdine → Lor'danel); 6 absent. TaxiPath: 879 of 915 (96.1%) same id with the same endpoints.

## Content policy (content_policy.py)

`import-azerothcore.sh` runs `content_policy.py` after the DB2 import; it rebuilds every `content_*` table in one transaction (about 1 s) and writes every dropped row or zeroed reward to `data/reports/content-dropped.csv` (`table,key,source_id,reason`). The server should read `content_*`, not the raw dump tables, for items, vendors, quests and loot. Gossip, `npc_text`, `broadcast_text` and `points_of_interest` carry no mapped ids and are read raw.

**Mapping rule.** An item or spell id is kept when the Retail row with the same id has the same name after trim + case-fold, or when [`scripts/content_overrides.csv`](../../../scripts/content_overrides.csv) lists it. Item names come from `item_template`; Wrath spell names from the Wrath Classic `SpellName.csv`. The override CSV (`kind,source_id,retail_id,source_name,retail_name,reason`) holds 62 reviewed item renames and 4 creature renames (below) (Okra → Ripe Okra, Scroll of Spirit → Scroll of Versatility, possessive/hyphen/spelling fixes). The import fails if a reviewed name no longer matches either side. Excluded on review: different items behind the same id (Red Linen Bandana → Linen Scrap, Wrath poisons → Empty Poison Vial, mail → plate conversions, gem stat changes).

| Table | Rule | Kept | Dropped |
|-------|------|------|---------|
| `content_item` | Mapped `item_template` ids; every column from Retail `item_sparse`/`item` (quality, item level, required level, prices, inventory type, class/subclass, bonding, delay, flags, `StatModifier_bonusStat_N`/`StatPercentEditor_N`) and `DisplayInfoID` from `item_modified_appearance` (modifier 0) → `item_appearance`; `SourceEntry` = AzerothCore entry | 35,731 | 10,365 (8,663 no Retail row, 1,702 name differs) |
| `content_npc_vendor` | Row kept if item maps (negative reference rows kept) | 34,497 | 3,088 |
| `content_quest_template` | Id in Retail `quest_v2`; unmapped `RewardItemN`/`RewardChoiceItemIDN` (+ amounts) and `RewardSpell`/`RewardDisplaySpell` set to 0 | 8,492 | 972 (+225 reward items, 247 reward spells zeroed) |
| `content_quest_template_addon`, `_offer_reward`, `_request_items`, `_poi`, `_poi_points`, `content_{creature,gameobject}_quest{starter,ender}` | Rows of kept quests | 8,492 / 7,904 / 6,936 / 16,999 / 52,201 / 6,922 / 7,270 / 393 / 391 | 972 / 756 / 821 / 1,772 / 4,965 / 523 / 631 / 64 / 66 |
| `content_gameobject_template`, `_template_addon`, `content_gameobject` | Template kept if `displayId` is 0 or a Retail `game_object_display_info` row; addons/spawns need a kept template | 21,269 / 21,181 / 95,979 | 312 / 312 / 480 |
| `content_{creature,gameobject,reference,item}_loot_template` | Row kept if item maps (reference rows kept); `item_loot_template` also needs a mapped container | 244,583 / 17,794 / 44,132 / 4,933 | 4,883 / 163 / 404 / 64 |
| `content_creature_template_model` | Copy of `creature_template_model`; a template with any display absent from Retail `creature_display_info` takes all model rows of the same entry in TrinityCore's Retail world DB when that entry has the same name (or a `creature` override, or a Retail spawn of maps 0/1 references the entry: TrinityCore renamed Pygmy Surf Crawler 3106 to Surf Crawler) and all its displays exist in Retail | 40,186 rows; 361 templates' 469 rows replaced | 43 templates kept with a Wrath-only display (28 TrinityCore display also absent, 15 renamed without override), none of them spawned on maps 0/1 |

Taxi is not derived: `db2.py` imports Retail `taxi_nodes` (1,464), `taxi_path` (8,690) and `taxi_path_node` (127,438) directly. The same import adds `item`, `item_sparse`, `item_appearance`, `item_modified_appearance`, `quest_v2`, and `durability_costs` (1,300 item levels) / `durability_quality` (18) for repair costs ([merchant](merchant.md)). **Creature displays.** Retail models come from TrinityCore `TDB1210.26091` (12.1.0 world DB; `data/reference/tdb1210/SOURCE.txt`): `creature_template.sql` and `creature_template_model.sql`, cut from `TDB_full_world_1210.26091_2026_09_09.sql` and imported as `tdb_creature_template` / `tdb_creature_template_model` (`TDB_TABLES` in `import-azerothcore.sh`, `TDB_DIR` default `data/reference/tdb1210`). AzerothCore rows whose displays all exist in Retail are kept even when TrinityCore differs (Hogger `448`: `384`, TrinityCore `108956`). On map 0, 53 spawned templates (681 spawns, 40 displays) had a Wrath-only first display; all 53 now have Retail displays, mostly the HD model of the same creature (Forest Lurker `1195` `827` → `909` tarantula2, Daggerspine Raider `2595` `11254` → `4763` naga male, Silt Crawler `922` `9571` → `342` crab2). The four creature overrides are Retail renames of Wetlands/Hillsbrad spiders: Moss Stalker `1780` → Skitterweb Striker, Mist Creeper `1781` → Skitterweb Lurker, Giant Moss Creeper `2349` → Domesticated Creeper, Forest Moss Creeper `2350` → Forest Creeper. `db2.py` imports Retail `creature_display_info` (120,984) for the existence check. The server's `load_models` reads `content_creature_template_model`; the raw table stays verbatim.

Concrete rows (tests): Corina Steele (`54`) sells Gladius at Retail item level 4 / display 1542 (AzerothCore 9 / 22078); Barkeep Dobbins' (`465`) Refreshing Spring Water costs the Retail 5 copper (AzerothCore 25); `41` OLDRecruit's Belt and `2892` Deadly Poison are dropped with Jasper Fel's (`1325`) vendor row; quest `16` Give Gerard a Drink is dropped, `783` A Threat Within kept with Deputy Willem.

**Open follow-ups (not applied):**
- 147 kept quests need an item that does not map (`RequiredItemIdN`, `StartItem`, `ItemDrop`), e.g. `83` Red Linen Goods (`1019`); they cannot be completed as authored.
- 12,887 kept vendor rows have a Wrath `ExtendedCost` (honor/badge costs); Retail `ItemExtendedCost` is not imported or mapped.
- 2 kept quests have a `PrevQuestID` that was dropped.

## Retail world content (maps 0 and 1)

Spec: [retail-world-content](../../specs/retail-world-content.md). The client renders Retail WMOs, and AzerothCore's Wrath content doesn't match them. For example, Stormwind bankers stand at (-8935, 613), which is now the street outside the Retail bank, while TrinityCore has them at (-8908, 639). Auctioneers sit at z 97.46, under the Retail auction house floor at 98.06. Wrath content also lacks Cataclysm's rebuilt zones.

`content_policy.py` runs the AzerothCore mapping first. Then `retail_content.py` resolves maps 0/1 from TrinityCore TDB1210.26091:
- the excerpts are imported verbatim as `tdb_*` tables through `mysql_dump.py --prefix tdb_`, with DDL in `tdb_schema.sql`;
- unsigned 64-bit values are stored as their signed bit pattern.

Each TrinityCore row is mapped into the AzerothCore-shaped `content_*` table that the server already reads.

### Coverage (measured before switching, maps 0/1)

| Measure | AzerothCore | TrinityCore | Source used |
|---|---|---|---|
| Creature spawns (base phase for TDB) | 60,412 | 95,170 | TrinityCore |
| Spawned creature entries (both / AC-only / TDB-only) | 7,118 | 9,674 (4,783 / 2,335 / 4,891) | TrinityCore |
| Game object spawns | 44,960 | 39,268 (21,877 after holiday-only and phased rows) | TrinityCore |
| Quests started/ended by spawned NPCs/objects | 4,327 | 6,545 | TrinityCore |
| Vendor entries with rows / rows | 985 / 15,382 | 1,287 / 18,565 | TrinityCore; AzerothCore for vendors TDB lacks |
| Gossip menus of spawned NPCs | 1,263 | 1,470 | TrinityCore |
| Non-service entries with loot | 2,669 of 3,543 | 1,749 of 5,001 | TrinityCore per NPC; AzerothCore's for 147 NPCs TDB has none for |
| Quests with choice rewards | – | 1 of 3,212 Retail quests (sniffed offers) | same-ID AzerothCore group fills an empty TrinityCore one (97 choice, 31 reward-item groups) |
| smart_scripts rows for spawned entries | – | 8,296 | not imported (no server consumer) |

**Quest starters are not thinner in TrinityCore.** At first, TDB seemed to lack starters for 2,068 of its quests that AzerothCore NPCs give. But 1,993 of them are quests TrinityCore flags `0x4000` with no ContentTuning; Cataclysm removed these:
- 783 "[DEPRECATED] A Threat Within"
- 7 Kobold Camp Cleanup
- 18 Brotherhood of Thieves
- the old Westfall chain

The other 75 are holiday quests, which TrinityCore gives through `game_event_creature_quest`, and removed mount exchanges. Filling them in from AzerothCore would put Wrath's Northshire back on Retail NPCs, so Retail-spawned NPCs give only TrinityCore's relations.

### Rules and counts

| Domain | Rule | Result |
|---|---|---|
| Spawns | Base phase: `PhaseId` 0, `PhaseGroup` 0, `terrainSwapMap` -1, difficulty 0. Guid + 20,000,000 (10 TrinityCore guids collide with kept AzerothCore spawns on maps 229/230/530/609). Addons (`bytes1` = StandState \| VisFlags<<16 \| AnimTier<<24, `bytes2` = SheathState \| PvPFlags<<8), formations, waypoint paths (path `MoveType` per point) and event memberships follow | 95,170 creatures, 60,412 AzerothCore spawns replaced; 21,877 objects, 45,186 replaced; 17,391 holiday-only and 3,377 phased TDB objects skipped |
| Creature templates | AzerothCore's, plus TrinityCore template + difficulty 0 + models for 4,720 entries it lacks. `ContentTuningID` on 16,003 templates; `minlevel`/`maxlevel` = its range (below) | 34,667 templates |
| Game object templates | TrinityCore's template replaces AzerothCore's for Retail-spawned entries: `Data0-23`, chest/fishing-hole `Data1` + 20,000,000; `Data24-34` reported for 608 templates. Retail display required | 6,142 TrinityCore templates |
| Items (historical reference-filtered import) | Superseded by [installed item projection](#installed-item-projection); unmapped installed definitions have `SourceEntry` NULL | Historical: 19,136 Retail-only items |
| Vendors | TrinityCore list (type 1 items) for Retail-spawned vendors; currency rows dropped | 2,434 vendors |
| Loot | Retail-spawned NPC with TrinityCore rows for its LootID → `lootid` + 20,000,000, gold from difficulty 0; references + 20,000,000; ItemType 2 (currency) dropped; exact repeated rows (the TDB loot tables have no key) kept once; item loot for TDB containers replaces AzerothCore's | 1,783 NPCs on TrinityCore loot, 147 keep AzerothCore's; 7,745 Retail-spawned entries have no loot table anywhere and get no money (54 had TrinityCore gold) |
| Gossip | Lowest `creature_template_gossip` menu per Retail-spawned NPC (4 have more; no conditions). Menus reached through submenus come from TrinityCore. `GossipOptionNpc` → AzerothCore type / npcflag / icon: 0→1/GOSSIP/0, 1→3/VENDOR/1, 2→4/FLIGHTMASTER/2, 3→5/TRAINER/3, 4→6/SPIRITHEALER/4, 5→8/INNKEEPER/5, 6→9/BANKER/6, 7→10/PETITIONER/7, 8→11/TABARDDESIGNER/8, 9→12/BATTLEMASTER/9, 10→13/AUCTIONEER/6, 12→14/STABLEMASTER/0, others hidden (12 options). `npc_text` = BroadcastText ids + probabilities; the text resolves through `content_broadcast_text` = TrinityCore's Retail BroadcastText from TDB's hotfixes SQL (237,420 ids, latest `VerifiedBuild`), plus AzerothCore rows for ids TDB lacks; none missing (local CASC can't serve `broadcasttext.db2`, FDID 841604: its encoding key isn't in any local archive). Checked against wago.tools' 12.1.0.69933 BroadcastText export, which holds only 10,670 rows (ids 1-318265): every one is in TDB; 10,567 match on text, language, emotes and flags, and the other 103 differ only in line endings (wago `\r\n`, TDB `\n`, e.g. 92062 "War Mill / Level 1") | 6,765 menus |
| Quests | Quests Retail-spawned NPCs/objects start or end, in QuestV2 (367 not). Mapping: `QuestLevel` -1, `MinLevel` = ContentTuning minimum, `ContentTuningID` column. `quest_objectives` Monster/GameObject → `RequiredNpcOrGo1-4` (objects negative), Item → `RequiredItemId1-6`, PlayerKills → `RequiredPlayerKills`; 260 objectives of other types reported. `RewardDisplaySpell` = `RewardDisplaySpell1`. Items missing from ItemSparse (29) and spells missing from SpellName (114) zeroed. `RewardMoney` = TrinityCore `Quest::MaxMoneyValue` (QuestMoneyReward at ContentTuning MaxLevel × multiplier; QuestMoneyReward FDID 1139047 from local CASC); the server pays per-player `Quest::MoneyValue` from `content_tuning_level` + `quest_money_reward` (spec). Addon, offer/request text and POIs from TrinityCore where it has rows; POI `id` = `Idx1` (lowest `BlobIndex`), `WorldMapAreaId` = `UiMapID` | 6,078 TrinityCore quests (4,018 with objectives, 2,696 with money; reward groups filled from AzerothCore: 31 item, 97 choice), 5,802 AzerothCore quests kept |

**Levels.** TrinityCore creature levels are ContentTuning ranges. ContentTuning (FDID 1962930, layout A3E13004, 2,929 rows) is read from the local CASC install with `casc-local` and decoded by `db2_casc.py` (`db2.py extract`); its columns come positionally from `db2_schema.sql`. The CSV matches an independently written decoder.

Levels follow `DB2Manager::GetContentTuningData`:
- `min = MinLevelSquish + adjust(MinLevelScalingOffset)` and `max = MaxLevelSquish + adjust(MaxLevelScalingOffset)`, clamped to 1..123.
- `adjust` is 0, +1, +90 (`GetMaxLevelForExpansion(11)`, worldserver.conf default `Expansion = 11`) or +80.
- The wago column names correspond positionally to TrinityCore's MinLevel/MaxLevel/MinLevelType/MaxLevelType.

Examples:
- Hogger 1-30 (CT 73)
- Newton Burnside 1-90 (CT 864)
- Stormwind City Guard 90-90 (CT 883)

TrinityCore-only templates without a ContentTuning get level 1 (reported), because the area fallback needs AreaTable, which isn't imported. `exp` comes from `HealthScalingExpansion` clamped to 0..2 (-1 → 2). Until the server scales levels per player, spawns take `minlevel`, and `creature_classlevelstats` only has real base HP up to level 83.

**Paths.** TrinityCore paths start on the spawn point and repeat a node, with `Orientation` + `Delay`, to pause facing a direction. For example, Stormwind City Patroller path `25104480` has points 6/7 and 13/14. Legs of zero length arrive at once. `waypoint_path.Flags` (12 FollowPathBackwards, 567 ExactSplinePath) and `Velocity` (443) are ignored.

**Spawn groups and pools.** No base-phase spawn on maps 0/1 is manual-spawn: all are in legacy group 2 (flags 9), apart from one in a group with flags 25. Pools are ignored, as they are for AzerothCore.

## Mapping policy options

Chosen: option 4, plus option 3's override table.

1. **Keep ids whose Retail row has the same name (case-insensitive); drop the rest.** Covers 96–98% of referenced items and ~99% of displays; drops 388 quests' item references and most class trainer spells.
2. **Option 1 + name remap for absent ids with a unique Retail name.** Adds ≤0.3% of items; the examples show unrelated Retail items, so this adds wrong mappings.
3. **Option 1 + a reviewed override table** (`content_id_map(kind, wrath_id, retail_id)`) for same-id renames and hand-picked remaps (Okra → Ripe Okra, Scroll of Spirit → Scroll of Versatility). Most control, manual cost proportional to the ~2–4% residue.
4. **Per-domain split, Retail authoritative for attributes.** Items: keep AzerothCore content references to ids that pass option 1, but read item stats, prices, levels, quality and display from Retail DB2 by id, not `item_template` (40% price and 16% item-level agreement). Class spells: do not map AzerothCore class trainers; class abilities come from Retail `skill_line_ability` / `ClassSpells`. Displays: use AzerothCore display ids as-is, drop the 253 absent. Quests: keep ids present in `QuestV2`. Taxi: import Retail TaxiNodes/TaxiPath/TaxiPathNode directly.

Option 4 matches the Retail client (item tooltips and models come from Retail DB2) and avoids hand-mapping ~1,500 ranked class spells that Retail no longer has.

## Sources

- [scripts/import-azerothcore.sh](../../../scripts/import-azerothcore.sh), [scripts/mysql_dump.py](../../../scripts/mysql_dump.py), [scripts/world_schema.sql](../../../scripts/world_schema.sql) — import path
- [scripts/content_mapping.py](../../../scripts/content_mapping.py) — mapping measurements above
- [scripts/content_policy.py](../../../scripts/content_policy.py), [scripts/content_overrides.csv](../../../scripts/content_overrides.csv) — Retail-first content tables
- [scripts/tests/test_import_azerothcore.py](../../../scripts/tests/test_import_azerothcore.py) — fixture and real-dump tests (A Threat Within `783` Deputy Willem → Marshal McBride; Corina Steele `54` weapon list)
- TrinityCore TDB1210.26091 `creature_template` / `creature_template_model` (`data/reference/tdb1210/`) — Retail creature models
- TrinityCore TDB1210.26091 excerpts (`TDB_TABLES` in `import-azerothcore.sh`): Retail world content of maps 0/1 ([scripts/retail_content.py](../../../scripts/retail_content.py))

## See Also

- [[world-data]] — world.db layout, creature and DB2 tables
- [[spell-info]] — Retail class spell lists (`ClassSpells`)
- [[class-progression]] — known spells come from Retail `skill_line_ability`
