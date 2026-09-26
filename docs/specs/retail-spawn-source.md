# Retail Spawn Source

The client renders Retail 12.1.0 terrain and WMOs, so the creature spawns on Eastern Kingdoms (map 0) and Kalimdor (map 1) come from TrinityCore TDB1210.26091 (Retail world DB), not AzerothCore (Wrath 3.3.5). Wrath positions put interior NPCs in walls, outside buildings or under floors. For example, Stormwind's bankers stand in what is now the street outside the Retail bank, and its auctioneers sit under the Retail auction house floor. `scripts/content_policy.py` builds the server-read `content_creature*` tables, and `crates/server/src/world_data.rs` loads them.

## What it must do

Spawn source
- [x] On maps 0 and 1, `content_creature` holds TrinityCore `creature` rows in the base phase only: `PhaseId` 0, `PhaseGroup` 0, `terrainSwapMap` -1 and difficulty 0 in `spawnDifficulties`. No AzerothCore spawns remain on those maps, and other maps keep AzerothCore's spawns.
- [x] A Retail spawn's guid is its TrinityCore guid + 20,000,000 (`RETAIL_GUID_OFFSET`). The offset is needed because 10 TrinityCore guids collide with AzerothCore spawns kept on other maps.
- [x] The raw AzerothCore tables (`creature`, `creature_template`, `creature_addon`, …) and the TrinityCore `tdb_*` tables stay verbatim.
- [x] Every AzerothCore spawn dropped on maps 0/1 and every TrinityCore row outside the base phase is written to `data/reports/content-dropped.csv` with its reason.
- [x] Newton Burnside stands at the Retail bank counter (-8908.35, 639.24), not AzerothCore's (-8935.27, 613.14). Auctioneer Jaxon stands above the Retail auction house floor (z 99.62 > 98.06).

Templates
- [x] `content_creature_template` is AzerothCore's `creature_template` plus TrinityCore's template for every spawned entry AzerothCore lacks. AzerothCore rows are kept unchanged when the entry exists in both.
- [x] A TrinityCore-only template:
  - takes `minlevel`/`maxlevel`/`exp` from the median, by level, of the 5 nearest same-map Retail spawns with an AzerothCore template. Critters, non-combat pets, triggers, NPCs with any npcflag and game-event spawns don't count.
  - takes `HealthModifier`, gold, `TypeFlags` and damage/mana/armor modifiers from `creature_template_difficulty` (DifficultyID 0).
  - uses TrinityCore's `Classification` as `rank` and the low 32 bits of `npcflag`.
  - has `lootid`, `pickpocketloot`, `skinloot` and `gossip_menu_id` set to 0.
- [x] TrinityCore-only templates take TrinityCore's `creature_template_model` rows. Homeless Stormwind Citizen (42384) spawns with one of its Retail displays.

Spawn-keyed data
- [x] `content_creature_addon`, `content_creature_formations` and `content_game_event_creature` hold AzerothCore rows for kept AzerothCore spawns plus TrinityCore rows for Retail spawns, with guids offset.
- [x] TrinityCore addons pack `StandState`/`VisFlags`/`AnimTier` into `bytes1` and `SheathState`/`PvPFlags` into `bytes2`.
- [x] `content_creature_template_addon` holds AzerothCore's rows plus TrinityCore's rows for TrinityCore-only templates.
- [x] `content_waypoint_data` holds AzerothCore's paths plus the TrinityCore `waypoint_path_node` rows of every path a Retail addon uses.
  - The path's `MoveType` is copied onto every point, and a NULL `Orientation` becomes 0.
  - The import fails if a TrinityCore path id collides with an AzerothCore one.
- [x] Stormwind City Patroller walks its 14-point TrinityCore path in order and pauses 12 s and 20 s.
- [x] Retail spawn behaviour:
  - Royal Guard escorts hold their 5 yd slots.
  - Harbor guards and General Marcus Jonathan are mounted.
  - Cenarion Hold Infantry carries aura 18950.
  - Stormwind's Retail training dummies get the dummy script and stand still.
  - Hostile Elwynn creatures aggro, and civilians and dummies do not.

Server
- [x] The server reads `content_creature`, `content_creature_template`, `content_creature_addon`, `content_creature_template_addon`, `content_waypoint_data`, `content_creature_formations` and `content_game_event_creature`:
  - spawns, templates, addons, paths, formations and events come from `world_data.rs`.
  - names come from `quest_data.rs`.
  - gossip menus come from `npc_interaction/gossip.rs`.
  - loot ids come from `loot/tables.rs`.

## How it works

- [Content import](../wiki/systems/content-import.md) — Retail spawns section, counts and drop reasons
- [World data](../wiki/systems/world-data.md) — world.db tables the server loads
- [Creature AI](../wiki/systems/creature-ai.md) — waypoint and formation movement

## Implementation inventory

- `scripts/content_policy.py` — `build_creatures` (spawns, templates, models, addons, waypoints, formations, events), `SpawnGrid` level lookup
- `scripts/tdb_schema.sql` — `tdb_creature*`, `tdb_waypoint_path*`, `tdb_game_event_creature` DDL
- `scripts/import-azerothcore.sh` — `TDB_TABLES` excerpts loaded with `mysql_dump.py --prefix tdb_`
- `data/reference/tdb1210/*.sql` — TDB1210 excerpts (provenance in `SOURCE.txt`; not in git)
- `crates/server/src/world_data.rs` — loads the `content_creature*` tables
- `crates/server/src/quest_data.rs`, `npc_interaction/gossip.rs`, `loot/tables.rs` — template reads

## Tests asserting this spec

- `scripts/tests/test_import_azerothcore.py` `test_retail_spawns_replace_azerothcore_spawns_on_maps_0_and_1` — fixture covering phase filters, guid offset, TrinityCore-only template levels, models, addons, waypoints, formations, events and drop report
- `crates/server/src/zones_world_db_tests.rs` (real world.db, `--ignored`):
  - `stormwind_banker_and_auctioneer_stand_at_retail_positions`
  - `retail_only_creature_spawns_from_its_trinitycore_template`
  - patrol, escort, mount, aura, dummy, health and aggro tests on Retail guids
- `crates/server/src/startup_tests.rs` `world_db_*` — content tables present, no orphan spawns, ordered waypoints
- `crates/server/src/npc_interaction_tests.rs` `orgrimmar_auctioneer_with_quest_flag_offers_gossip_option_to_horde_auction_house` — Retail-only Auctioneer Drezmit

## Known gaps (current cycle)

- [ ] TrinityCore-only templates have no fixed levels in TDB, only ContentTuning scaling ranges, so their levels are estimated from neighbours. On AzerothCore entries with a known level, the same rule lands within 4 levels for 79% and within 9 for 86%.
- [ ] 49,130 of 95,170 Retail spawns (4,720 entries) use TrinityCore-only templates. None of them has loot, a gossip menu, vendor items or quests:
  - 565 vendor spawns sell nothing.
  - 955 quest-flagged spawns offer nothing.
  - 1,438 gossip-flagged spawns show the default menu.
- [ ] TrinityCore `waypoint_path.Flags` (FollowPathBackwardsFromEndToStart, ExactSplinePath) and `Velocity` are ignored. Every path wraps to point 1.
- [ ] TrinityCore spawn `modelid` and `equipment_id` overrides and spawn-level `npcflag`/`unit_flags` are not applied (the same holds for AzerothCore spawns).
- [ ] TrinityCore event ids in `content_game_event_creature` index TrinityCore's `game_event`. The server activates no events, so only the sign matters.

## Out of scope

- Game object spawns (`content_gameobject`) stay AzerothCore on every map, because this cycle covers creatures only.
- Spawn groups and pools are not modelled. No base-phase Retail spawn on maps 0/1 is in a manual-spawn group: they are all in the legacy group 2 plus one in group flags 25. AzerothCore pools are not imported either.
- Importing TrinityCore loot, gossip, vendor or quest tables: those use Retail item and quest ids, which need their own mapping policy.
