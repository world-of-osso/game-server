# Retail World Content

On Eastern Kingdoms (map 0), Kalimdor (map 1), The Stockade (map 34) and Grim Batol (map 670), world content comes from TrinityCore TDB1210.26091, the Retail 12.1.0 world DB. AzerothCore (Wrath 3.3.5) is used only where TrinityCore has nothing for the same NPC or quest.

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
- [x] Retail maps (0, 1, 34, 670) take TrinityCore base-phase rows only: `PhaseId` 0 or 169, `PhaseGroup` 0, `terrainSwapMap` -1, and one of the map's spawn difficulties in `spawnDifficulties`. `DEFAULT_PHASE` 169 is unphased (`PhaseShift.h:31`, `PhasingHandler::InitDbPhaseShift`). The spawn difficulties follow the map's `Map.db2` InstanceType: open world 0 (`DIFFICULTY_NONE`), a party dungeon every `MapDifficulty.db2` DifficultyID of the map (the Stockade 1; Grim Batol 1, 2, 8, 23); other instance types fail the import. `content_creature.spawnDifficulties` and `content_gameobject.spawnDifficulties` keep the spawn's difficulties the map offers ("0" on a continent, "1,2" for Grim Batol's 529 creatures).
  - This applies to `content_creature` and to `content_gameobject`.
  - Game objects whose only event membership is positive (holiday decorations) don't spawn.
  - Other maps keep AzerothCore's rows.
- [x] Spawns (verified 2026-09-26, `world-stockade-20260926.db`; before → after the phase-169 and Stockade import): creatures map 0 45,317 → 50,930, map 1 49,853 → 52,309, map 34 93 → 143; game objects map 0 11,582 → 12,654, map 1 10,295 → 11,250, map 34 18 → 34. The Stockade's 143 creatures (Hogger, Lord Overheat, Randolph Moloch, Riverpaw and fire-elemental trash, prisoners, guards) replace AzerothCore's 93 Classic ones (Targorr, Bazil Thredd, ...); its game objects are 33 fires and a bonfire (Retail has no Stockade doors or chests).
- [x] An entry's template values come from the `creature_template_difficulty` row of its spawns' difficulty (`CreatureTemplate::GetDifficulty`: the difficulty's row, else its `Difficulty.db2` `FallbackDifficultyID` chain Mythic → Heroic → Normal → None). `content_creature_template` takes the row of the entry's lowest spawn difficulty: a Stockade entry's (ContentTuning, health, loot, gold) its `DifficultyID` 1 row, else 0. Hogger: ContentTuning 215 (10-30), LevelScalingDelta +2. Bruegal Ironknuckle keeps loot 1720.
- [x] `content_creature_template_difficulty` has one row per dungeon difficulty an entry spawns at (entry, DifficultyID, levels, exp, ContentTuningID, HealthScalingExpansion, LevelScalingDelta, Health/Mana/Armor/Damage modifiers, lootid, mingold, maxgold) from its resolved row: General Umbriss Normal ContentTuning 253 (30-35), Heroic 1118 (35); Trogg Dweller Heroic has no row and takes Normal's. 138 rows (Grim Batol).
- [x] Loot per difficulty: the difficulty row's TrinityCore `LootID` (+20,000,000) when it has loot, else Wowhead drops of that difficulty's mode (entry + 30,000,000 at the lowest spawn difficulty, entry + 30,000,000 + difficulty × 1,000,000 at the others), else none and no money.
- [x] Retail spawn guids are the TrinityCore guid + 20,000,000 (`RETAIL_GUID_OFFSET`), because 10 TrinityCore creature guids collide with AzerothCore spawns kept on other maps.
- [x] TrinityCore loot template and reference ids are also + 20,000,000 (`TDB_LOOT_OFFSET`), in creature `lootid` and in chest and fishing-hole `Data1`.
- [x] Raw AzerothCore tables, `tdb_*` tables and DB2 tables stay verbatim. Every dropped or replaced row is in `data/reports/content-dropped.csv` with its reason.
- [x] Newton Burnside stands at the Retail bank counter (-8908.35, 639.24). Auctioneer Jaxon stands above the Retail auction house floor (z 99.62 > 98.06).

Wowhead loot
- [x] A Retail-spawned creature without TrinityCore loot (`LootID` 0) whose Wowhead drops are scraped (`data/reference/wowhead/<scrape>/<entry>.json`, the npc page's `drops` Listview; `scripts/wowhead_loot.py`) drops them: loot id entry + 30,000,000, one ungrouped row per item with `Chance` = count / outof of the spawns' difficulty mode, `MinCount`/`MaxCount` from `stack`, `QuestRequired` for Quest-class (12) items, `Comment` `wowhead <scrape>`. Raw rows stay in `wowhead_creature_drop` with the scrape name. TrinityCore loot wins where it exists (Bruegal Ironknuckle, Lord Overheat). Scrape `stockade-20260926`: 15 Stockade creatures, 3,335 rows (e.g. Hogger: Corpse Rompers 31.1 %).

Creature templates and levels
- [x] `content_creature_template` is AzerothCore's templates plus TrinityCore's template, difficulty-0 values and models for every Retail-spawned entry AzerothCore lacks.
- [x] It also keeps TrinityCore's template, models and template addon of every entry a TRANSFORM aura (56) names in its MiscValue, spawned or not (`retail_content.TRANSFORM_ENTRIES`), so a transform into Tentacle of Iso'rath 45409 shows its model instead of the unknown-template pig. `scripts/add_transform_templates.py` applies the rule to an already-imported world.db additively: on 2026-10-01 it added 2,411 templates, 3,918 model rows and 64 template addons and changed no other row of the 218 tables. Other non-spawn references (MOUNTED 78: 174 entries; SUMMON effect 28: 20,273) have no server consumer and are not kept.
- [x] Every creature spawned on maps 0/1 wears a Retail CreatureDisplayInfo display. An AzerothCore template whose display is absent from Retail takes TrinityCore's models when both use the same name, when a reviewed override covers it, or when a Retail spawn references the entry. The last case covers TrinityCore renames: Pygmy Surf Crawler 3106 goes from display 1307 to 1938.
- [x] Each spawn picks its display by `ObjectMgr::ChooseDisplayId` (ObjectMgr.cpp:1601-1613), once, when it spawns (`zones::choose_display_id`):
  - the spawn's `creature.modelid` (`content_creature.modelid`, from TDB `creature.modelid`) wins. A modelid that Retail CreatureDisplayInfo lacks is dropped to 0 and reported. 183 spawns on maps 0/1/34 carry one, including every Stockade spawn.
  - otherwise a template model by `Probability` (`GetRandomValidModel`): a single model is used even at Probability 0, and when every weight is 0 the pick is uniform (Stockade Guard 46405: 2989/2990/2992, never 2991).
  - a template with `CREATURE_FLAG_EXTRA_TRIGGER` (0x80) takes its first invisible model (`GetFirstInvisibleModel`), else display 11686. A display is invisible when its CreatureModelData `FileDataID` is one of TrinityCore's trigger models (124640, 124641, 124642, 343863, 439302), read from `creature_model_data` (DB2 CreatureModelData). Summon Enabler Stalker 53488 gets 16925, not 169.
- [x] Creatures replicate `UnitFlags`: the spawn's `creature.unit_flags` when nonzero, else the template's (`ChooseCreatureFlags`). Summon Enabler Stalker carries `UNIT_FLAG_NOT_SELECTABLE`. A `UNIT_FLAG_UNINTERACTIBLE` (NOT_SELECTABLE) unit is neither attacked nor assisted (`IsValidAttackTarget`/`IsValidAssistTarget`): no auto-attack, and explicit and area spells skip it. `SPELL_ATTR11_CAN_ASSIST_UNINTERACTIBLE` is not honoured.
- [x] Every template TrinityCore tunes carries `ContentTuningID` (difficulty 0; Stockade entries difficulty 1).
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
    - player kills → `RequiredPlayerKills`.
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
- `crates/server/src/world_data.rs`: loads the `content_creature*`, `content_waypoint_data` and `content_game_event_creature` tables
- `crates/server/src/npc_interaction/gossip.rs`: reads the `content_gossip_*` and `content_npc_text` tables
- `crates/server/src/quest_data.rs`, `loot/tables.rs`: read `content_creature_template`

## Tests asserting this spec

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
