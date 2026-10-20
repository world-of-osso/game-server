# Retail World Content

On Eastern Kingdoms (map 0), Kalimdor (map 1), The Stockade (map 34), Grim Batol (map 670) and Mainline Warsong Gulch (map 2106), world content comes from TrinityCore TDB1210.26091, the Retail 12.1.0 world DB. AzerothCore (Wrath 3.3.5) is used only where TrinityCore has nothing for the same NPC or quest.

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

## Open-world population (2026-10-11)

- [x] `content_policy.py --mode world-promote --maps <IDs>` promotes only explicit open-world maps on an existing private database copy; refuses canonical-name, linked and instance destinations/maps.
- [x] Source coordinates, IDs, phase 0/169 without phase-use flags/groups/terrain swaps and authored difficulty 0 remain authentic. No phase progression is flattened.
- [x] Unrelated map spawns, shared templates, authored creature/gameobject placements on the promoted map and their addons, and existing pool dependencies survive; repeated promotion is idempotent. This mode does not run full policy regeneration or clear overlays.
- [ ] Every locally loadable open-world map has a source/content/phase/difficulty/event/map-filter census; missing source rows remain explicit, not synthesized.
- [ ] The31 promoted local-terrain expansion maps are hosted as shared continent maps, not dungeon copies: normal admission and startup NPC/object spawning use the same existing host list; other unsupported maps remain refused.

Implementation: `scripts/world_population.py` reuses the [instance import functions](../wiki/systems/content-import.md#instance-population-2026-10-11). Process/copy proof: `scripts/tests/test_world_population.py`; admission/private-copy ECS tests in `maps/tests.rs` and `zones.rs`. [Census and observed both-skin Boralus proof](../world-population-counts.md) distinguish imported rows, living actors, source-empty maps and client appearance gaps.

## Zone population continuation (2026-10-11)

- [x] Read-only census ranks each hosted continent's zones using AreaTable/UiMap, distinguishes declared/inferred/unassigned placements, and counts TDB versus content creatures, gameobjects, quests and giver/ender identities.
- [x] The existing world-promote/instance importer promotes authentic source rows on an unlinked copy, retaining effective source unit flags, overlays and unrelated content; reports per-table before/after/added/removed row counts.
- [x] Missing zone quests use existing Retail converters. Unsupported objective kinds or slot overflow exclude the whole new quest; no synthetic placements, relations or quest objectives. Item-only vendors, source trainer tables and DB2 taxi data supply services, including authored taxi mount models without world spawns.
- [x] Ground/deploy-file and navmesh inventories cover every hosted map; missing assets/bakes and unsupported surfaces remain explicit data boundaries, not claimed as working navigation.
- [x] Private UDP5916 native proof visits at least three newly populated zones, accepts/rewards one authentic quest each and uses a flight master in both skins. Canonical writes, UDP5000, fabricated quest credit and full suites are excluded.

[Ranked census, copy command and evidence](../zone-population-counts.md) own measured coverage. Template/player-immunity flags must remain effective on the client interaction path, so neutral immune quest enders are spoken to rather than auto-attacked.

## Instance population (2026-10-11)

Maps from local `Map.db2` with `InstanceType` dungeon/raid (1/2) and `MapDifficulty.db2` rows, union all canonical TDB `instance_template` maps, take authentic TDB creature/gameobject spawns, templates, models, addons, formations, equipment and paths through the copy-only [instance importer](../wiki/systems/content-import.md#instance-population-2026-10-11). The historical four-map importer below remains the world/quest/vendor policy; instance population runs after it during regeneration.

Garrison directories are excluded from the DB2 expansion, not from the existing TDB union. Missing or sparse authored placements remain data gaps, never generated population. Expanding the roster preserves existing maps' content rows; shared entries retain their existing-map base difficulty before considering newly imported maps. Per-map difficulty rows still use each authored spawn difficulty. [Coverage and reachability](../instance-coverage-counts.md) records the private-copy proof.

- [ ] Every instance map imports source-backed base-phase rows at its MapDifficulty difficulties; always-visible flag1 rows remain eligible, while conditional phases, terrain swaps and unsupported/inverse phase-use flags are counted separately, never flattened into visible base-phase spawns.
- [ ] Same-ID legacy instance templates/models must not override TDB identity. Missing source templates remain explicit deficits; missing display/asset metadata never causes fabricated models.
- [ ] A map copy spawns its eligible creatures independent of client appearance-cache coverage, at the copy's difficulty and excluding dead bosses/event/pool restrictions.
- [ ] Copy importer refuses `world.db`, symlinks and hardlinks; repeated application preserves world-map spawns and unrelated source-backed loot references. Imported instance base/difficulty templates bind their resolved TDB LootID (+20,000,000 when nonzero), GoldMin and GoldMax, including gold-only rows; referenced TDB loot is imported through the existing Retail item policy. When the resolved TDB LootID is zero, existing source-backed Wowhead bindings are retained without inventing drops; nonzero TDB loot remains authoritative.
- [ ] Private-port client proof covers visible, attackable creatures in representative dungeons and both skins.

Raid admission/lockout UX (rank 5), encounter scripting (rank 2), phase progression and unsupported gameobject interaction types remain separate gaps; importing their authentic rows is not proof those gameplay systems work.

## Installed item definitions

- [x] `content_item` contains every matching installed `Item` + `ItemSparse` ID, independent of older TDB vendor, loot, quest, toy or starting-kit references. All attributes and default appearance come from the installed DB2 tables; reviewed AzerothCore `SourceEntry` mappings survive refresh. Definitions do not grant availability.
- [x] `content_policy.py --mode items-refresh --db <private-copy>` refreshes item projections transactionally without rebuilding world population or fetching external sources. Isolated Skyborne definitions remain separate.
- [x] Source-exact regression covers all nineteen reported battleground gear IDs, repeated projection and refresh, and retained `SourceEntry`. Generated Skyborne SQLite fixture must match its producer exactly.

Implementation and pinned TrinityCore rule: [content import](../wiki/systems/content-import.md#installed-item-projection).

## What it must do

Spawn and id rules
- [x] Retail maps (0, 1, 34, 670, 2106) take TrinityCore base-phase rows only: `PhaseId` 0 or 169, `PhaseGroup` 0, `terrainSwapMap` -1, and one of the map's spawn difficulties in `spawnDifficulties`. `DEFAULT_PHASE` 169 is unphased (`PhaseShift.h:31`, `PhasingHandler::InitDbPhaseShift`). The spawn difficulties follow the map's `Map.db2` InstanceType: open world 0 (`DIFFICULTY_NONE`), a party dungeon every `MapDifficulty.db2` DifficultyID of the map (the Stockade 1; Grim Batol 1, 2, 8, 23); battlegrounds use authored difficulty 0 (WSG2106 TDB1210 rows); other instance types fail the import. `content_creature.spawnDifficulties` and `content_gameobject.spawnDifficulties` keep the spawn's difficulties the map offers ("0" on a continent, "1,2" for Grim Batol's 529 creatures).
  - This applies to `content_creature` and to `content_gameobject`.
  - Game objects whose only event membership is positive (holiday decorations) don't spawn.
  - Other non-instance maps keep AzerothCore's rows; canonical TDB instances take the instance-population step above.
  - WSG2106 inclusion is import-policy support, not match admission or live objective proof. TDB1210 has28 creature and14 gameobject source rows; world.db already stores them in `tdb_*` but the previous map filter excluded them from `content_*`. Lead must rebuild content; this task never writes world.db. Capture triggers and flag metadata also require the separate [battleground source import](battleground-lifecycle.md).
- [x] Spawns (verified 2026-09-26, `world-stockade-20260926.db`; before → after the phase-169 and Stockade import): creatures map 0 45,317 → 50,930, map 1 49,853 → 52,309, map 34 93 → 143; game objects map 0 11,582 → 12,654, map 1 10,295 → 11,250, map 34 18 → 34. The Stockade's 143 creatures (Hogger, Lord Overheat, Randolph Moloch, Riverpaw and fire-elemental trash, prisoners, guards) replace AzerothCore's 93 Classic ones (Targorr, Bazil Thredd, ...); its game objects are 33 fires and a bonfire (Retail has no Stockade doors or chests).
- [x] An entry's template values come from the `creature_template_difficulty` row of its spawns' difficulty (`CreatureTemplate::GetDifficulty`: the difficulty's row, else its `Difficulty.db2` `FallbackDifficultyID` chain Mythic → Heroic → Normal → None). `content_creature_template` takes the row of the entry's lowest spawn difficulty: a Stockade entry's (ContentTuning, health, loot, gold) its `DifficultyID` 1 row, else 0. Hogger: ContentTuning 215 (10-30), LevelScalingDelta +2. Bruegal Ironknuckle keeps loot 1720.
- [x] `content_creature_template_difficulty` has one row per dungeon difficulty an entry spawns at (entry, DifficultyID, levels, exp, ContentTuningID, HealthScalingExpansion, LevelScalingDelta, Health/Mana/Armor/Damage modifiers, lootid, mingold, maxgold) from its resolved row: General Umbriss Normal ContentTuning 253 (30-35), Heroic 1118 (35); Trogg Dweller Heroic has no row and takes Normal's. 138 rows (Grim Batol).
- [x] Loot per difficulty: the difficulty row's TrinityCore `LootID` (+20,000,000) when it has loot, else Wowhead drops of that difficulty's mode (entry + 30,000,000 at the lowest spawn difficulty, entry + 30,000,000 + difficulty × 1,000,000 at the others), else none and no money.
- [x] Retail spawn guids are the TrinityCore guid + 20,000,000 (`RETAIL_GUID_OFFSET`), because 10 TrinityCore creature guids collide with AzerothCore spawns kept on other maps.
- [x] TrinityCore loot template and reference ids are also + 20,000,000 (`TDB_LOOT_OFFSET`), in creature `lootid` and in chest and fishing-hole `Data1`.
- [x] Explicitly approved derived merchant placements may supplement missing native spawns using genuine merchant identity, native template/model rows, source-backed vendor data, and an approximate UI-map point grounded in client terrain. Derived coordinates and local facing/default policies must remain labelled, never represented as native Retail capture. Rebuilds retain the configured placement and supplier rows; invalid required manifests fail preflight before database deletion. Source tables and unrelated content remain unchanged. Zaralda 243531 is the approved current case; currency/limited-stock suppliers remain unsupported.
- [x] Zaralda's manifest explicitly records Wowhead reaction `[1,1]` (React A/H both q2 green), approved `derived_faction_template` 35, and a `faction_policy` declaring runtime faction derived, not a Retail faction capture. Missing evidence, other reactions/mappings, or a missing/mismatched template 35 fail before writes; there is no unknown-faction fallback. Existing template 35 must retain Faction 31, Flags 0, FactionGroup 0, FriendGroup 1 (player), EnemyGroup 0, no enemies, Friend_0 31 and remaining friends 0. Only the derived content template's faction changes; raw TDB faction 0 and other native fields remain intact alongside existing vendor-role/level policies.
- [x] An already-published derived Zaralda spawn may upgrade its provenance comment only when its core identity/placement and prior derived-source manifest exactly match the approved record apart from the new reaction/faction-policy fields. Unrelated GUIDs, changed placements or changed provenance are rejected. Reapplication is idempotent; SQL failures roll back faction/comment changes and unrelated transaction work. Behavioral proof: `scripts/tests/test_derived_merchants.py`, including the historical published manifest → native transform → approved-policy upgrade ordering.
- [x] Native Zaralda right-click opens merchant interaction without auto-attack in the saved native-friendly exit0 flow; [observed proof](../wiki/investigations/midnight-economy-content.md#observed-native-success-task-evidence-2026-10-01) covers actual pick/title, authored244586 stock1 tooltip, backpack and close. Independent artifact gate116 gives scoped PASS; importer proof is separate, not whole-world acceptance.
- [x] Spawn pools follow TrinityCore `PoolMgr` (PoolMgr.cpp): `content_pool_template` / `content_pool_members` / `content_game_event_pool` are TDB1210's `pool_template`, `game_event_pool` and the `pool_members` of the Retail spawns (creature and gameobject `spawnId` at their content guid) plus every child pool. A map copy spawns only `max_limit` members of each pool not in a game event and not a child (`InitPoolsForMap`; child pools spawn through their mother); a member's slot is filled by explicit chance (`max_limit` 1) or uniformly, gameobjects before creatures on one shared count. When a pooled creature's respawn time comes, the dead entity leaves and its pool (or its mother pool) re-picks among every member not spawned, itself included (`Map::ProcessRespawns` → `UpdatePool`). Gameobject members take slots but do not spawn (the server does not spawn ore, herb or chest types). Lost Soul (pool 351): 1 of 3 spawns alive; map 0 starts 119 of 391 pooled creature spawns, map 1 0 of 102 (Felwood ore spawn points pick ore over their Deadwood creatures). AzerothCore's `pool_*` are not imported: no AzerothCore-sourced map spawns creatures.
- [x] Raw AzerothCore tables, `tdb_*` tables and DB2 tables stay verbatim. Every dropped or replaced row is in `data/reports/content-dropped.csv` with its reason.
- [x] Newton Burnside stands at the Retail bank counter (-8908.35, 639.24). Auctioneer Jaxon stands above the Retail auction house floor (z 99.62 > 98.06).

Wowhead loot
- [x] A Retail-spawned creature without TrinityCore loot (`LootID` 0) whose Wowhead drops are scraped (`data/reference/wowhead/<scrape>/<entry>.json`, the npc page's `drops` Listview; `scripts/wowhead_loot.py`) drops them: loot id entry + 30,000,000, one ungrouped row per item with `Chance` = count / outof of the spawns' difficulty mode, `MinCount`/`MaxCount` from `stack`, `QuestRequired` for Quest-class (12) items, `Comment` `wowhead <scrape>`. Raw rows stay in `wowhead_creature_drop` with the scrape name. TrinityCore loot wins where it exists (Bruegal Ironknuckle, Lord Overheat). Scrape `stockade-20260926`: 15 Stockade creatures, 3,335 rows (e.g. Hogger: Corpse Rompers 31.1 %).

Creature templates and levels
- [x] `content_creature_template` is AzerothCore's templates plus TrinityCore's template, difficulty-0 values and models for every Retail-spawned entry AzerothCore lacks.
- [x] It also keeps TrinityCore's template, models and template addon of every entry a TRANSFORM aura (56) names in its MiscValue, spawned or not (`retail_content.TRANSFORM_ENTRIES`), so a transform into Tentacle of Iso'rath 45409 shows its model instead of the unknown-template pig. `scripts/add_transform_templates.py` applies the rule to an already-imported world.db additively: on 2026-10-01 it added 2,411 templates, 3,918 model rows and 64 template addons and changed no other row of the 218 tables. Other non-spawn references (MOUNTED 78: 174 entries; SUMMON effect 28: 20,273) have no server consumer and are not kept.
- [x] Every creature spawned on maps 0/1 wears a Retail CreatureDisplayInfo display. An AzerothCore template whose display is absent from Retail takes TrinityCore's models when both use the same name, when a reviewed override covers it, or when a Retail spawn references the entry. The last case covers TrinityCore renames: Pygmy Surf Crawler 3106 goes from display 1307 to 1938.
- [x] Each spawn picks its display by `ObjectMgr::ChooseDisplayId` (ObjectMgr.cpp:1601-1613), once, when it spawns (`zones::choose_display`):
  - the spawn's `creature.modelid` (`content_creature.modelid`, from TDB `creature.modelid`) wins. A modelid that Retail CreatureDisplayInfo lacks is dropped to 0 and reported. 183 spawns on maps 0/1/34 carry one, including every Stockade spawn.
  - otherwise a template model by `Probability` (`GetRandomValidModel`): a single model is used even at Probability 0, and when every weight is 0 the pick is uniform (Stockade Guard 46405: 2989/2990/2992, never 2991).
  - a template with `CREATURE_FLAG_EXTRA_TRIGGER` (0x80) takes its first invisible model (`GetFirstInvisibleModel`), else display 11686. A display is invisible when its CreatureModelData `FileDataID` is one of TrinityCore's trigger models (124640, 124641, 124642, 343863, 439302), read from `creature_model_data` (DB2 CreatureModelData). Summon Enabler Stalker 53488 gets 16925, not 169.
- [x] Preserve the selected template model's `DisplayScale` alongside its display ID. Replicate it through `UnitScale` as an additional multiplier over client CDI × CMD scale. Explicit spawn modelid uses native multiplier1, as TrinityCore's spawn `CreatureModel`. Aura61 changes and removal recompute from the preserved native scale instead of resetting scaled NPCs to1. Concrete boundary: Unstable Corruption40923/display116569 retains0.45 through spawn and aura reconciliation. Weighted and trigger selection retain the scale of the same selected row; no second roll.
- [x] Creatures replicate `UnitFlags`: the spawn's `creature.unit_flags` when nonzero, else the template's (`ChooseCreatureFlags`). Summon Enabler Stalker carries `UNIT_FLAG_NOT_SELECTABLE`. A `UNIT_FLAG_UNINTERACTIBLE` (NOT_SELECTABLE) unit is neither attacked nor assisted (`IsValidAttackTarget`/`IsValidAssistTarget`): no auto-attack, and explicit and area spells skip it. `SPELL_ATTR11_CAN_ASSIST_UNINTERACTIBLE` is not honoured.
- [x] Every template TrinityCore tunes carries `ContentTuningID` (difficulty 0; Stockade entries difficulty 1).
- [x] An AzerothCore template TrinityCore also has takes TrinityCore's `type`, `family` and its difficulty row's `TypeFlags` (taming, [hunter taming](hunter-taming.md)): Old Blanchy 582 is a Beast of family 299 with TAMEABLE, not a Critter.
  - `minlevel`/`maxlevel` are that ContentTuning's range, following TrinityCore `GetContentTuningData` under Expansion 11: calc types add +0, +1, +90 or +80, clamped to 1..123.
  - AzerothCore's fixed levels stay only where there is no ContentTuningID.
  - Examples: Hogger 1-30; Homeless Stormwind Citizen 5-30.
- [x] ContentTuning comes from local CASC: FDID 1962930, layout A3E13004, via `db2.py extract`. It is imported as `content_tuning`. The locally decoded ContentTuning, QuestMoneyReward, ExpectedStat, ExpectedStatMod and ContentTuningXExpected match wago.tools' 12.1.0.69933 exports on every row (`LocalDb2ExtractTests`). The ExpectedStat tables are imported (`expected_stat`, `expected_stat_mod`, `content_tuning_x_expected`) for Retail creature health.
- [x] Retail-spawned NPCs use TrinityCore loot when TrinityCore has rows for their LootID, with gold from difficulty 0. Otherwise they use AzerothCore's loot id.
- [x] Retail-spawned NPCs with no loot table in either source (7,745 entries) drop nothing and have no money, even where TrinityCore's difficulty 0 has gold (54 entries). Each one is in content-dropped.csv. This is a user decision.
- [x] Retail-spawned NPCs use TrinityCore's lowest `creature_template_gossip` menu when they have one.

Services
- [x] A Retail-spawned vendor with a TrinityCore list sells that list, items only. Currency rows are dropped.
- [x] Gossip mapping:
  - The menus Retail-spawned NPCs reach (plus their submenus) come from TrinityCore: `gossip_menu`, `gossip_menu_option` and `npc_text`.
  - `GossipOptionNpc` maps to AzerothCore's option type, npcflag and icon. Vendor, taxi, trainer, spirit healer, binder, banker, petition, tabard, battlemaster, auctioneer and stablemaster are mapped; the others are hidden.
  - Every other menu stays AzerothCore's.
- [x] Gossip texts resolve through `content_broadcast_text`: TrinityCore's Retail BroadcastText (TDB hotfixes `broadcast_text`, latest `VerifiedBuild` per ID, `Text`/`Text1` → `MaleText`/`FemaleText`), plus AzerothCore's rows for the ids TrinityCore lacks. Every npc_text BroadcastText id of the Retail menus resolves; before this, 176 were missing. Farley's bind option reads "Bind your hearthstone to this inn."
- [x] `content_item` adds every Retail item TrinityCore's content references, with no `SourceEntry`.

Quests
- [x] Quests that Retail-spawned NPCs and objects start or end, and that are in Retail QuestV2, come from TrinityCore.
  - `QuestLevel` is -1 (scales to the player).
  - `MinLevel` is ContentTuning's minimum, and `ContentTuningID` is carried.
  - `quest_objectives` map to shape as follows:
    - monster → `RequiredNpcOrGo`; game object → a negative `RequiredNpcOrGo`;
    - item → `RequiredItemId`;
    - player kills → `RequiredPlayerKills`;
    - area trigger, or quest flag `COMPLETION_EVENT`/`COMPLETION_AREA_TRIGGER` (0x2/0x4) → AzerothCore `SpecialFlags` 2 (`EXPLORATION_OR_EVENT`); TrinityCore's own `SpecialFlags` 2 (`AUTO_PUSH_TO_PARTY`, 2 quests) is cleared and reported. `content_areatrigger_involvedrelation` holds TrinityCore's `areatrigger_involvedrelation` rows plus each area trigger objective's own trigger, for quests with that flag and triggers in `AreaTrigger.db2` (`ObjectMgr::LoadQuestAreaTriggers`); other quests keep AzerothCore's rows.
  - Other objective types are reported.
- [x] Retail-spawned NPCs and objects give and end TrinityCore relations only. AzerothCore relations of other entries stay.
- [x] Reward-item and choice-reward groups:
  - Where the TrinityCore group is all zero and the AzerothCore quest with the same ID has one, AzerothCore's mapped group fills it: 31 reward-item groups and 97 choice groups, each in content-dropped.csv.
  - TrinityCore sniffed choice rewards for only 1 of 3,212 Retail quests.
- [x] `RewardMoney` is TrinityCore's `Quest::MaxMoneyValue` (QuestDef.cpp:486-494 at a352b1fa): `QuestMoneyReward[ContentTuning MaxLevel].Difficulty[RewardMoneyDifficulty] × RewardMoneyMultiplier`, truncated. It is 0 without a ContentTuning.
  - 2,696 of 6,078 TrinityCore quests have MaxMoneyValue money. For example, Beating Them Back! has 2000.
  - `RewardMoneyDifficulty`, `RewardMoneyMultiplier` and `ContentTuningID` are carried.
- [x] A TrinityCore quest (`ContentTuningID` != 0) pays and offers each player `Quest::MoneyValue` (QuestDef.cpp:478-484): `QuestMoneyReward[Player::GetQuestLevel].Difficulty[RewardMoneyDifficulty] × RewardMoneyMultiplier`, truncated, or 0 without a row.
  - `GetQuestLevel` (Player.cpp:14339-14353) is `min(player level, MaxLevel)` once the player is at least `GetQuestMinLevel`, and that minimum otherwise.
  - `GetQuestMinLevel` (Player.cpp:14315-14329) is `MinLevelWithDelta`. For a player outside the ContentTuning's scaling faction group (Flags Horde 0x08 → 5, Alliance 0x10 → 3) it is `MaxLevel`.
  - Examples: Beating Them Back! pays 55 at level 1 and 2000 from level 10.
  - AzerothCore quests pay their fixed `RewardMoney`.
  - `content_tuning_level` holds the GetContentTuningData levels (`minlevel`, `maxlevel`, `MinLevelWithDelta`, `ScalingFactionGroup`) for the import and the server.
- [x] QuestMoneyReward comes from local CASC: FDID 1139047, layout 01D6219A, with a pallet-array field and a non-inline ID list. It is imported as `quest_money_reward` (130 rows). Level 10 gives 0/200/500/1000/1500/2000/4000/6000/8000/0.
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
- `scripts/db2.py`, `scripts/db2_casc.py`: local-CASC `extract` of ContentTuning and QuestMoneyReward (WDC5 decoder)
- `scripts/tdb_schema.sql`, `scripts/import-azerothcore.sh` (`TDB_TABLES`): the `tdb_*` tables
- `data/reference/tdb1210/*.sql`: TDB1210 excerpts (provenance in `SOURCE.txt`; not in git)
- `crates/server/src/spawn_pools.rs`: PoolMgr port (load, `InitPoolsForMap`, `UpdatePool` on a pooled creature's respawn)
- `crates/server/src/world_data.rs`: loads the `content_creature*`, `content_waypoint_data` and `content_game_event_creature` tables
- `crates/server/src/npc_interaction/gossip.rs`: reads the `content_gossip_*` and `content_npc_text` tables
- `crates/server/src/quest_data.rs`, `loot/tables.rs`: read `content_creature_template`

- `scripts/derived_merchants.py`, `scripts/derived-merchants.json` — validated, explicitly derived merchant publication inputs.
- `scripts/terrain_height.py` — client ADT triangle-height sampling for derived placement. Bytes are decoded by the `wow-formats-dump adt-height` CLI at the revision pinned in `scripts/wow_formats_dump.py` (a missing binary fails with its `cargo install` command); the script keeps chunk selection, validation, error messages and interpolation.

## Tests asserting this spec

- `scripts/tests/test_derived_merchants.py` — source identity, price/bundle, atomicity, preservation, idempotency and collision behavior.
- `scripts/tests/test_terrain_height.py` — concrete terrain heights, axis/tile checks and invalid height data.
- `scripts/tests/test_import_azerothcore.py` `test_derived_merchant_survives_two_full_rebuilds`, `test_invalid_derived_manifest_preserves_existing_database_bytes` — full importer persistence and preflight preservation.
- `scripts/tests/test_import_azerothcore.py`
  - Fixtures: Retail spawns; Stockade spawns (`test_retail_stockade_spawns_replace_azerothcore_classic_stockade`); vendors and loot; game objects; gossip; quests.
  - `ContentTuningLevelTests`.
  - `LocalContentTuningExtractTests`: real CASC.
  - `AzerothCoreContentTests`: real dumps (vaults, quests, items).
- `crates/server/src/zones_world_db_tests.rs` (`--ignored`):
  - banker and auctioneer positions;
  - TrinityCore-only template;
  - patrol, escort, mount, aura, dummy, health and aggro on Retail guids.
  - display choice: `summon_enabler_stalker_spawns_invisible_and_not_selectable`, `stockade_models_are_chosen_by_probability`, `models_without_weights_are_chosen_uniformly`.
- `scripts/tests/test_import_azerothcore.py` `test_retail_spawns_keep_their_modelid_and_unit_flags`; `crates/server/src/zones.rs` unit tests (single model, spawn modelid, default invisible display).
- `crates/server/src/startup_tests.rs` `world_db_*`: content tables present, Hogger at 1-30, no orphan spawns.
- `crates/server/src/quest_rules_tests.rs`, `quest_tests.rs`: Retail Northshire chain and Riverpaw Gnoll Bounty; quest money per player level (`beating_them_back_pays_quest_money_for_the_players_level`, faction group, minimum level, AzerothCore fixed money); Beating Them Back! turned in at level 1 pays 55.
- `scripts/tests/test_import_azerothcore.py` `ContentTuningLevelTests`: `MinLevelWithDelta`, scaling faction group.
- `crates/server/src/spawn_pools_world_db_tests.rs` (`--ignored`): pooled spawns alive at startup on maps 0/1, Lost Soul respawn re-pick.
- `crates/server/src/merchant_tests.rs`, `npc_interaction_tests.rs`, `game_objects_tests.rs`, `bank_test_harness.rs`: Retail vendor lists, gossip menus, vaults (205107) and mailboxes (20209585).

## Known gaps (current cycle)

- [ ] **Stockade money.** TDB1210 gives the Retail Stockade creatures no gold and Wowhead has no money data, so they drop no coins (grey items still become coins).

- [x] **Levels and health** are per-player ContentTuning scaling with ExpectedStat health ([level-scaling](level-scaling.md)).
- [ ] **Quest levels.** `QuestLevel` -1 scales quests to the player without clamping to ContentTuning's range.
- [ ] **Quest money at the level cap.** Retail pays `RewardMoneyMaxLevel` instead of XP at the cap; it is not implemented, so a capped player gets MoneyValue and no XP.
- [ ] **Unmapped quest data.**
  - 260 objectives of other types (talk-to, area trigger, criteria tree, …) are reported, not tracked.
  - `RewardMoneyMultiplier`, `RewardDisplaySpell2-3`, reputation requirements (addon only) and TrinityCore's 64-bit `AllowableRaces` (truncated to 32 bits) are not mapped.
- [ ] **Gossip limits.**
  - BroadcastText comes from TDB's hotfixes SQL, not local CASC: the local CASC resolution cache maps `broadcasttext.db2` (FDID 841604) to encoding key 599c2108…, which no local archive holds. The likely cause is asset-resolver's locale-blind root resolution (last record wins); I have not confirmed it.
  - Gossip conditions are not modelled: the lowest menu is used, and 4 NPCs have more than one.
- [ ] **Waypoints and spawn overrides.**
  - TrinityCore `waypoint_path.Flags`/`Velocity` are ignored.
  - Spawn `equipment_id` and spawn-level `npcflag` are not applied; spawn `unit_flags` reach `UnitFlags` only, not aggro (`AggroMode` reads the template's).
  - `creature_model_info` (`DisplayID_Other_Gender`, the 50 % gender swap of `GetCreatureModelRandomGender`) is not imported.
  - Game object `Data24-34` (608 templates) are not in the AzerothCore schema.
- [ ] **Gathering nodes.** Loot ids of Retail gathering nodes (type 50) are not remapped. The server spawns only mailboxes and guild vaults so far.

## Out of scope

- `smart_scripts`: the server has no consumer. There are 8,296 TrinityCore rows for Retail-spawned entries, not imported.
- Spawn groups and pools:
  - None of the base-phase spawns on maps 0/1 is manual-spawn: all are in legacy group 2, except one in a group with flags 25.
  - Pools are ignored, as for AzerothCore.
- Skinning and pickpocket loot, `conditions`, `quest_details`/`quest_greeting`, `points_of_interest` (the server does not read them).
- Holiday quest givers (`game_event_creature_quest`): the server activates no events.
