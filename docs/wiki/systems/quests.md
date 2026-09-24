# Quests

Requirements and gaps: [quests spec](../../specs/quests.md).

## Data

`quest_data.rs` loads `QuestDb` at startup from world.db ([[content-import]] Retail-first tables): `content_quest_template` + `_addon` + `_offer_reward` + `_request_items`, creature starters/enders, `content_quest_poi` + `_points`, creature names (objective text "`<name>` slain" when `ObjectiveText` is empty) and `content_item` rows the quests reference (names and attributes for granted items). Objectives keep Wrath order: `RequiredNpcOrGo1-4` (negative = game object) then `RequiredItemId1-6`.

Link tables follow AzerothCore `ObjectMgr::LoadQuests`: `prev_quests[q]` = `PrevQuestID` of q plus every quest whose `NextQuestID` is q (sign kept); `prev_chain_quests` from `RewardNextQuest`; `exclusive_groups`.

## Rules (`quest_rules.rs`)

Pure functions over `QuestDb` + `QuestLogData` + `QuestPlayer{level, race, class}`, after AzerothCore `Player`:

- `can_take` = `CanTakeQuest` + log cap: already on / done (non-repeatable), class and race masks (`1 << (id-1)`), exclusive group (positive: no other member taken), previous quests (any positive one turned in, or any negative one in the log; a previous quest in a negative group needs the whole group turned in), next/prev chain, `MinLevel`/`MaxLevel`, 35 quests.
- Markers (`GetQuestDialogStatus`): enders give Reward/Incomplete; starters give Available, LowLevelAvailable when the player is more than 4 levels above the quest (`LOW_LEVEL_HIDE_DIFF`), Unavailable when below `MinLevel` but within 7 (`HIGH_LEVEL_HIDE_DIFF`). The highest wins.
- Progress: kill counts are stored per objective; item progress is the bag count, capped at the requirement.

## Runtime (`quest.rs`, `quest_requests.rs`)

- Storage: redb `quest_logs` (character id → `QuestLogData{active: [ActiveQuestData{quest_id, counts}], rewarded (sorted), watched}`).
- `send_initial_quest_log` (`Added<InWorld>`) sends `QuestLogSnapshot` and stores the view in `SentQuestLog`.
- `process_quest_requests` (Update) drains all eight request types; each loads the log and inventory, applies, saves, and inserts the character into `QuestDirty`. Bag/gold edits go through `persistence::save_inventory` + `GoldUpdateQueue`, so [[inventory]] deltas and gold replication follow.
- PostUpdate, before `send_inventory_deltas`: `credit_quest_kills` (`Added<DeathState>` + `LastDamager`, as reputation), `mark_quest_inputs_changed` (characters in `GoldUpdateQueue::changed_inventories` and `Changed<Level>`), `send_quest_updates` (diff against `SentQuestLog` → `QuestLogUpdate`; markers for `QueriedQuestGivers` → `QuestGiverStatusMultiple`).

Marker delivery is query-driven rather than replicated because the marker depends on the viewer; the connection remembers queried NPCs so changes push without a new query.

## See Also

- [[inventory]] — bag model, `free_bag_cells`, delta path
- [[experience]] — quest and kill XP (turn-in queues `quest_xp`)
- [[combat]] — NPC death (`DeathState`, `LastDamager`)
- [[content-import]] — quest tables and mapping policy
