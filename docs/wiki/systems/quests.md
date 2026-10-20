# Quests

Requirements and gaps: [quests spec](../../specs/quests.md). Exact [GO/spell census and proof](../../quest-objectives-coverage.md).

## Data

`quest_data.rs` loads `QuestDb` at startup from world.db ([[content-import]] Retail-first tables): `content_quest_template` + `_addon` + `_offer_reward` + `_request_items`, creature starters/enders, `content_quest_poi` + `_points`, creature names (objective text "`<name>` slain" when `ObjectiveText` is empty) and `content_item` rows the quests reference (names and attributes for granted items). Objectives keep Wrath order: `RequiredNpcOrGo1-4` (negative = game object) then `RequiredItemId1-6`, then, for `SpecialFlags` 2 (`QUEST_SPECIAL_FLAGS_EXPLORATION_OR_EVENT`), a server-only `ExplorationOrEvent` objective (required 1, text `AreaDescription`) that the protocol view leaves out. `content_areatrigger_involvedrelation` joined with `AreaTrigger.db2` (`area_trigger`) gives `QuestDb::area_triggers` (map → trigger shape + quests).

Link tables follow AzerothCore `ObjectMgr::LoadQuests`: `prev_quests[q]` = `PrevQuestID` of q plus every quest whose `NextQuestID` is q (sign kept); `prev_chain_quests` from `RewardNextQuest`; `exclusive_groups`.

## Authored max-level money — 2026-10-19

`retail_content.py` copies selected TC `RewardBonusMoney` into nullable producer metadata: NULL means legacy/unselected, while selected authored0 remains0. After its database commit `content_policy.py` exports those selected values to tracked `content/quest-reward-bonus.json`; deploy.sh ships it to the runtime world.db sibling. Existing databases need a content rebuild before producers export the new metadata. Runtime never borrows a raw same-ID TC bonus for a legacy row. [Catalog provenance](../../../content/quest-reward-bonus-source.md).

`load_quest_db` requires that sibling JSON and propagates missing/invalid-file errors into the existing startup panic. `quest_bonus_money::settlement` adds the authored bonus only for ordinary quests at the existing player cap, unless Quest.Flags0x100 suppresses it. World-quest and captured branches retain their settlement; unrepresentable integer totals reject before mutation. One amount feeds inventory, completion packet and achievement money event. `quest_money` and dialog/log/offer previews remain base, as TC `BuildQuestRewards` specifies.

Authority: TC a352b1fa QuestDef.cpp97–101/525–531/578–586; Player.cpp15201–15218/17303–17313; QuestDef.h222. Money rates default1 (World.cpp970/1019–1020); no new rate infrastructure. Native quest62 has base105000+bonus120, tuning ceiling30 versus player cap90. Pushed65805b8eb: actual cap settlement105000 instead of105120 and missing-file startup boundary RED; below-cap89 control passes. Producer/deploy delivery tests cover selected zero versus unselected legacy and actual shipped bytes with fake remote commands. Fresh ignored607a8da1a/protocol505 GREEN4/0/33.87s/no warnings: cap packet+durable delta105120/XP0, below-cap105000/XP13000, base previews, criterion4092/duplicate/reopen, native flag/zero/legacy and missing-file startup. Then all four are enabled. Producer/deployment byte-transfer2/0; broader merged integration remains pending, no deployment claim.

## Rules (`quest_rules.rs`)

Pure functions over `QuestDb` + `QuestLogData` + `QuestPlayer{level, race, class}`, after AzerothCore `Player`:

- `can_take` = `CanTakeQuest` + log cap: already on / done (non-repeatable), class and race masks (`1 << (id-1)`), exclusive group (positive: no other member taken), previous quests (any positive one turned in, or any negative one in the log; a previous quest in a negative group needs the whole group turned in), next/prev chain, `MinLevel`/`MaxLevel`, 35 quests.
- Markers (`GetQuestDialogStatus`): enders give Reward/Incomplete; starters give Available, LowLevelAvailable when the player is more than 4 levels above the quest (`LOW_LEVEL_HIDE_DIFF`), Unavailable when below `MinLevel` but within 7 (`HIGH_LEVEL_HIDE_DIFF`). The highest wins.
- Progress: creature and game-object event counts persist per matching objective kind/ID, capped at the requirement; item progress is the Retail-source bag count. Chest opening is not item credit.
- Spell effects90/134 call the same counted Type0 objective path for the hit player (not the NPC/caster):90 personal,134 nearby same-map-copy group. These are monster-credit objectives, not a separate cast-spell Type; Retail Type5 means learn-spell. Existing Spray Water/SmartAI credit remains separate. Effect16 events,316/317 label credit, arbitrary scripted/Dummy credit and learn-spell objectives remain outside this proof.

## Runtime (`quest.rs`, `quest_requests.rs`)

- Storage: redb `quest_logs` (character id → `QuestLogData{active: [ActiveQuestData{quest_id, counts}], rewarded (sorted), watched}`).
- `send_initial_quest_log` (`Added<InWorld>`) sends `QuestLogSnapshot` and stores the view in `SentQuestLog`.
- `process_quest_requests` (Update) drains all eight request types; each loads the log and inventory, applies, saves, and inserts the character into `QuestDirty`. Bag/gold edits go through `persistence::save_inventory` + `GoldUpdateQueue`, so [[inventory]] deltas and gold replication follow.
- PostUpdate, before `send_inventory_deltas`: `credit_quest_kills` (`Added<DeathState>` + `LastDamager`, as reputation), `credit_quest_area_triggers` (`Changed<Position>`: `QuestTriggerPresence` remembers the triggers the player stands in per map; a newly entered one explores its quests in the log, as `maps::area_triggers` does for teleports), `mark_quest_inputs_changed` (characters in `GoldUpdateQueue::changed_inventories` and `Changed<Level>`), `send_quest_updates` (diff against `SentQuestLog` → `QuestLogUpdate`; markers for `QueriedQuestGivers` → `QuestGiverStatusMultiple`).

Marker delivery is query-driven rather than replicated because the marker depends on the viewer; the connection remembers queried NPCs so changes push without a new query.

## Sources

- [Quest contract](../../specs/quests.md).
- `quest_rules::credit_objective`, `quest::event_credit`, `spell_cast/effects.rs`, `quest_objectives_tests.rs` and `quest_spell_tests.rs`.
- TrinityCore a352b1fa `QuestDef.h`, `Player::KillCreditGO`, `EffectKillCreditPersonal` and `EffectKillCredit`.

## See Also

- [[game-objects]] — GO use validation, authored locks and chest-loot lifetime
- [[inventory]] — bag model, `free_bag_cells`, delta path
- [[experience]] — quest and kill XP (turn-in queues `quest_xp`)
- [[combat]] — NPC death (`DeathState`, `LastDamager`)
- [[content-import]] — quest tables and mapping policy
