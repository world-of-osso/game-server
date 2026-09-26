# Retail World Content

On Eastern Kingdoms (map 0) and Kalimdor (map 1), world content comes from TrinityCore TDB1210.26091, the Retail 12.1.0 world DB. AzerothCore (Wrath 3.3.5) is used only where TrinityCore has nothing for the same NPC or quest.

TrinityCore content covers:
- creature and game object spawns;
- templates;
- quests;
- vendors;
- gossip;
- loot.

Why: the client renders Retail terrain and WMOs. Wrath positions put interior NPCs in walls, outside buildings or under floors. For example, Stormwind's bankers stand in the street outside the Retail bank, and its auctioneers sit under the Retail auction house floor. Wrath content also has no Cataclysm-and-later NPCs or quests.

Where it's built:
- `scripts/retail_content.py` resolves the tables. It runs as a step of `scripts/content_policy.py`.
- The server reads the resulting AzerothCore-shaped `content_*` tables.

How it works: [content import](../wiki/systems/content-import.md).

## What it must do

Spawn and id rules
- [x] Maps 0/1 take TrinityCore base-phase rows only: `PhaseId` 0, `PhaseGroup` 0, `terrainSwapMap` -1, and difficulty 0 in `spawnDifficulties`.
  - This applies to `content_creature` and to `content_gameobject`.
  - Game objects whose only event membership is positive (holiday decorations) don't spawn.
  - Other maps keep AzerothCore's rows.
- [x] Retail spawn guids are the TrinityCore guid + 20,000,000 (`RETAIL_GUID_OFFSET`), because 10 TrinityCore creature guids collide with AzerothCore spawns kept on other maps.
- [x] TrinityCore loot template and reference ids are also + 20,000,000 (`TDB_LOOT_OFFSET`), in creature `lootid` and in chest and fishing-hole `Data1`.
- [x] Raw AzerothCore tables, `tdb_*` tables and DB2 tables stay verbatim. Every dropped or replaced row is in `data/reports/content-dropped.csv` with its reason.
- [x] Newton Burnside stands at the Retail bank counter (-8908.35, 639.24). Auctioneer Jaxon stands above the Retail auction house floor (z 99.62 > 98.06).

Creature templates and levels
- [x] `content_creature_template` is AzerothCore's templates plus TrinityCore's template, difficulty-0 values and models for every Retail-spawned entry AzerothCore lacks.
- [x] Every template TrinityCore tunes carries `ContentTuningID` (difficulty 0).
  - `minlevel`/`maxlevel` are that ContentTuning's range, following TrinityCore `GetContentTuningData` under Expansion 11: calc types add +0, +1, +90 or +80, clamped to 1..123.
  - AzerothCore's fixed levels stay only where there is no ContentTuningID.
  - Examples: Hogger 1-30; Homeless Stormwind Citizen 5-30.
- [x] ContentTuning comes from local CASC: FDID 1962930, layout A3E13004, via `db2.py extract`. It is imported as `content_tuning`.
- [x] Retail-spawned NPCs use TrinityCore loot when TrinityCore has rows for their LootID, with gold from difficulty 0. Otherwise they use AzerothCore's loot id.
- [x] Retail-spawned NPCs use TrinityCore's lowest `creature_template_gossip` menu when they have one.

Services
- [x] A Retail-spawned vendor with a TrinityCore list sells that list, items only. Currency rows are dropped.
- [x] Gossip mapping:
  - The menus Retail-spawned NPCs reach (plus their submenus) come from TrinityCore: `gossip_menu`, `gossip_menu_option` and `npc_text`.
  - `GossipOptionNpc` maps to AzerothCore's option type, npcflag and icon. Vendor, taxi, trainer, spirit healer, binder, banker, petition, tabard, battlemaster, auctioneer and stablemaster are mapped; the others are hidden.
  - Every other menu stays AzerothCore's.
- [x] `content_item` adds every Retail item TrinityCore's content references, with no `SourceEntry`.

Quests
- [x] Quests that Retail-spawned NPCs and objects start or end, and that are in Retail QuestV2, come from TrinityCore.
  - `QuestLevel` is -1 (scales to the player).
  - `MinLevel` is ContentTuning's minimum, and `ContentTuningID` is carried.
  - `quest_objectives` map to shape as follows:
    - monster → `RequiredNpcOrGo`; game object → a negative `RequiredNpcOrGo`;
    - item → `RequiredItemId`;
    - player kills → `RequiredPlayerKills`.
  - Other objective types are reported.
- [x] Retail-spawned NPCs and objects give and end TrinityCore relations only. AzerothCore relations of other entries stay.
- [x] Reward-item and choice-reward groups:
  - Where the TrinityCore group is all zero, AzerothCore's mapped group fills it.
  - TrinityCore sniffed choice rewards for only 1 of 3,212 Retail quests.
- [x] `RewardMoney` is AzerothCore's where the quest exists there, else 0.
- [x] Addon, offer/request text and POIs come from TrinityCore where it has rows, AzerothCore's otherwise.
  - A POI keeps its lowest `BlobIndex`.
  - `WorldMapAreaId` holds TrinityCore's `UiMapID`.

Server behaviour on Retail content
- [x] The Retail Northshire warrior chain works end to end: Beating Them Back! (6 Blackrock Worgs) → Lions for Lambs → Join the Battle! (turned in at Deputy Willem, 50 XP at level 1) → They Sent Assassins.
- [x] Riverpaw Gnoll Bounty counts 8 bag items and grants the chosen reward.
- [x] Retail vendor lists, banker, auction and innkeeper gossip, guild vaults, mailboxes, patrols, escorts, mounts, auras, training dummies and aggro work on the Retail data.

## How it works

- [Content import](../wiki/systems/content-import.md): Retail world content section, counts, mappings and drop reasons
- [World data](../wiki/systems/world-data.md): the world.db tables the server loads
- [Creature AI](../wiki/systems/creature-ai.md): waypoint and formation movement

## Implementation inventory

- `scripts/retail_content.py`: spawns, templates and ContentTuning levels, addons, waypoints, formations, events, game objects, vendors, loot, gossip, quests
- `scripts/content_policy.py`: AzerothCore mapping, and runs the `retail_content` steps
- `scripts/content_common.py`: `clone_table`, `Report`
- `scripts/db2.py`, `scripts/db2_casc.py`: local-CASC `extract` of ContentTuning (WDC5 decoder)
- `scripts/tdb_schema.sql`, `scripts/import-azerothcore.sh` (`TDB_TABLES`): the `tdb_*` tables
- `data/reference/tdb1210/*.sql`: TDB1210 excerpts (provenance in `SOURCE.txt`; not in git)
- `crates/server/src/world_data.rs`: loads the `content_creature*`, `content_waypoint_data` and `content_game_event_creature` tables
- `crates/server/src/npc_interaction/gossip.rs`: reads the `content_gossip_*` and `content_npc_text` tables
- `crates/server/src/quest_data.rs`, `loot/tables.rs`: read `content_creature_template`

## Tests asserting this spec

- `scripts/tests/test_import_azerothcore.py`
  - Fixtures: Retail spawns; vendors and loot; game objects; gossip; quests.
  - `ContentTuningLevelTests`.
  - `LocalContentTuningExtractTests`: real CASC.
  - `AzerothCoreContentTests`: real dumps (vaults, quests, items).
- `crates/server/src/zones_world_db_tests.rs` (`--ignored`):
  - banker and auctioneer positions;
  - TrinityCore-only template;
  - patrol, escort, mount, aura, dummy, health and aggro on Retail guids.
- `crates/server/src/startup_tests.rs` `world_db_*`: content tables present, Hogger at 1-30, no orphan spawns.
- `crates/server/src/quest_rules_tests.rs`, `quest_tests.rs`: Retail Northshire chain and Riverpaw Gnoll Bounty.
- `crates/server/src/merchant_tests.rs`, `npc_interaction_tests.rs`, `game_objects_tests.rs`, `bank_test_harness.rs`: Retail vendor lists, gossip menus, vaults (205107) and mailboxes (20209585).

## Known gaps (current cycle)

- [ ] **Levels and health.** The server spawns creatures at `minlevel`; per-player scaling within the ContentTuning range is a separate task.
  - `creature_classlevelstats` has real base HP only up to level 83, so a level-90 Stormwind City Guard has 2 HP.
  - Retail health comes from ExpectedStat, which is not imported.
- [ ] **Quest levels.** `QuestLevel` -1 scales quests to the player without clamping to ContentTuning's range.
- [ ] **Quest money.** TrinityCore quest money needs QuestMoneyReward (FDID 1139047, array-pallet compression not decoded). 652 of 6,078 TrinityCore quests have AzerothCore money.
- [ ] **Unmapped quest data.**
  - 260 objectives of other types (talk-to, area trigger, criteria tree, …) are reported, not tracked.
  - `RewardMoneyMultiplier`, `RewardDisplaySpell2-3`, reputation requirements (addon only) and TrinityCore's 64-bit `AllowableRaces` (truncated to 32 bits) are not mapped.
- [ ] **Gossip limits.**
  - 176 npc_text BroadcastText ids are missing from AzerothCore's `broadcast_text`; Retail BroadcastText is not imported.
  - Gossip conditions are not modelled: the lowest menu is used, and 4 NPCs have more than one.
- [ ] **Waypoints and spawn overrides.**
  - TrinityCore `waypoint_path.Flags`/`Velocity` are ignored.
  - Spawn `modelid`/`equipment_id` and spawn-level `npcflag`/`unit_flags` are not applied.
  - Game object `Data24-34` (608 templates) are not in the AzerothCore schema.
- [ ] **Gathering nodes.** Loot ids of Retail gathering nodes (type 50) are not remapped. The server spawns only mailboxes and guild vaults so far.

## Out of scope

- `smart_scripts`: the server has no consumer. There are 8,296 TrinityCore rows for Retail-spawned entries, not imported.
- Spawn groups and pools:
  - None of the base-phase spawns on maps 0/1 is manual-spawn: all are in legacy group 2, except one in a group with flags 25.
  - Pools are ignored, as for AzerothCore.
- Skinning and pickpocket loot, `conditions`, `quest_details`/`quest_greeting`, `points_of_interest` (the server does not read them).
- Holiday quest givers (`game_event_creature_quest`): the server activates no events.
