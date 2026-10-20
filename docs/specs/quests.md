# Quests

Quest givers, the quest log and objective tracking for the Retail-style quest dialog, quest log and objective tracker (game-engine in-game UI plan, phase 6). Contract: `shared::protocol` `quest_messages.rs` + `QuestLogSnapshot` (shared-protocol). Server: `crates/server/src/quest.rs`, `quest_requests.rs`, `quest_rules.rs`, `quest_view.rs`, `quest_data.rs` (world.db `content_quest_*`), `persistence_quests.rs`. See [quest system](../wiki/systems/quests.md) for the mechanism.

## What it must do

- [x] Quest giver markers are per player: `QuestGiverStatusQuery{npcs}` answers `QuestGiverStatusMultiple` (None, Unavailable grey `!`, LowLevelAvailable trivial `!`, Incomplete grey `?`, Available `!`, Reward `?`; highest wins). After any change to the player's quests the server re-sends the markers of every NPC the connection queried.
- [x] `QuestGiverHello{npc}` lists quests in the log the NPC ends (Complete/Incomplete) and quests it starts that the player can take now (Available/LowLevelAvailable).
- [x] `QuestGiverQueryQuest` answers `QuestGiverQuestDetails` (texts, level, objectives, money, fixed and choice reward items) for quests the NPC starts and the player can take.
- [x] `QuestGiverAcceptQuest` checks level (`MinLevel`, addon `MaxLevel`), class and race masks, already in log / already done (non-repeatable), prerequisites (`PrevQuestID` and reverse `NextQuestID`, AzerothCore sign rules), positive exclusive groups, `RewardNextQuest` chains, 35-quest log cap. Accepted quests are auto-watched; a `StartItem` is given.
- [x] Kill objectives count creature kills by the killer and group members within 74 yards of the creature (melee and spell kills).
- [x] Item objectives count the items in the bags, whatever path added them; removing items lowers progress.
- [x] Exploration/event quests (AzerothCore `SpecialFlags` 2; content import maps TrinityCore `QUEST_OBJECTIVE_AREATRIGGER` and `QUEST_FLAGS_COMPLETION_EVENT`/`_AREA_TRIGGER` to it) are not complete on accept. A living player not in flight walking into one of the quest's `areatrigger_involvedrelation` triggers (`AreaTrigger.db2` sphere or box) explores it (`HandleAreaTriggerOpcode` → `AreaExploredOrEventHappens`); a first position on a map only records the triggers it stands in. The requirement has no protocol objective kind, so it shows only through the entry's `completed`.
- [x] `QuestGiverCompleteQuest` at the ender answers `QuestGiverOfferReward` when objectives are done, else `QuestGiverRequestItems{can_complete: false}`.
- [x] `QuestGiverChooseReward` requires a valid choice when choice items exist, free bag cells for every granted item, and gold for negative `RewardMoney`; destroys required and provided items; grants money and items; marks the quest turned in; grants QuestXP experience ([experience](experience.md)); answers `QuestGiverQuestComplete` (with its `xp`); offers the chain's next quest (`QuestGiverQuestDetails`) when the same NPC starts it.
- [x] Ordinary quests pay authored selected-TC `RewardBonusMoney` in addition to base money only at the player's maximum level; quest Flags0x100 suppresses it. The quest's tuning ceiling is not the player's cap. Gold, completion packet and quest-money criterion use one settlement total; XP stays0 at cap. Base dialog/log/offer previews, world quests, captured rewards and legacy source provenance remain unchanged. Required `content/quest-reward-bonus.json` is shipped beside world.db; missing runtime catalog fails startup explicitly. Pushed native62 cap/missing-file RED→fresh ignored4/0 GREEN33.87s at607a8da1a, then enabled. Merged protocol566 family89/0 plus disjoint rules/data/view19/0 (one existing private-population ignore), producer/deploy3/0, helper check/fmt0. Independent lead landing remains separate.
- [x] `AbandonQuest` removes the quest and its provided start item; `SetQuestWatched` toggles tracking.
- [x] Enter world sends `QuestLogSnapshot` (entries with objectives and counts, completion, level, sort id, texts, watched flag, POI polygons); later changes send `QuestLogUpdate{changed, removed, watched_quest_ids}`. Owned entries include full story, current player-scaled money/fixed/choice rewards and pushability (`Flags` shareable bit and not failed), including after database reopen without an NPC dialog. Client Share additionally requires current group membership.
- [x] Quest log, progress, turned-in quests and watch list persist per character (redb `quest_logs`).
- [x] Rejections answer `QuestFailed{quest_id, reason}`; NPCs must be alive and within 10 yards. Item-ownership-cap rejection instead sends the owner `InventoryError::ItemMaxCount`, not a misleading full-inventory quest failure.
- [x] Starter and reward creation use the shared inventory storage boundary, including same-source personal-bank ownership limits. Cap refusal leaves original GUID/quantity, gold, bank and quest-log/reward state unchanged; no direct inventory-vector push bypass remains. Genuine full-bag failures retain `QuestFailedReason::InventoryFull`. Existing Retail quest item authority remains explicit, without fabricated definitions. Retain authentic source item metadata rather than pruning it to the original reference list; valid later starter/reward references must not become missing-definition failures. An empty item-grant batch is a no-op: XP/money-only turn-ins must not validate unrelated inventory templates or require item creation metadata.

## Tests asserting this spec

- [GO/spell objective census and proof](../quest-objectives-coverage.md) — exact per-Type before/after routes, data limits and real-quest evidence.
- `crates/server/src/quest_objectives_tests.rs` — GO use and held-key validation, next-frame chest loot retention, actual item acquisition/cast and personal/group spell credit through rewards.

- `crates/server/src/quest_tests.rs` — real rows through the message path: The Fargodeep Mine (62) and The Jasperlode Mine (76) stay incomplete until the player enters trigger 88 / 87 (not a ghost, not the other mine's trigger); A Threat Within (783) Willem → McBride persisted with Kobold Camp Cleanup (7) offered next; 7 blocked until 783 turned in; 7 progresses on Kobold Vermin kills and pays 25 copper; Brotherhood of Thieves (18) counts bag bandanas, rejects a missing choice, grants Militia Hammer; abandon; untrack persists across re-entry; out-of-range giver.
- `crates/server/src/quest_rules_tests.rs` — markers, prerequisites, kill credit, trivial level, race mask, log cap, `quest-complete` objective fill.
- `crates/server/src/admin_ipc.rs` tests — `quest-complete` persists the filled counts and marks the log dirty.
- shared-protocol `src/protocol_quest_tests.rs` — wire round trips, marker order, registration.

## Known gaps (current cycle)

- [x] Type 2 game-object objectives receive matching goober-use credit after life, map-copy, model-box reach, quest and reuse-time checks. Nearby party members share credit. Consumable objects cannot be repeatedly used before their authored respawn timer.
- [x] Chest loot uses authored content game-object loot tables and the existing inventory path; opening alone does not complete an item objective.
- [x] Local authored Lock.db2 alternatives gate GO use: Retail-source held keys and zero-skill ordinary interaction locks. Missing lock definitions fail explicitly; unsupported skill/spell-key alternatives do not award credit. Required runtime input and exporter: [game objects](../wiki/systems/game-objects.md#quest-objects-2026-10-11).
- [ ] Profession/spell-key opening, condition, event, GO spell and animation/despawn semantics remain unimplemented; these paths are not full Retail GO parity.
- [x] SmartAI action152 sets event completion through persisted scripted quest results. Authentic Feero976 all-wave credit/death failure has server proof; [escort contract](escort-quests.md) records the incomplete native/data acceptance.
- [ ] Wrath race masks: `AllowableRaces` 1101/690 lists Wrath races only, so post-Wrath races (Worgen, Void Elf, Pandaren, ...) fail race-restricted quests.
- [x] Authored spell effects 90 and 134 advance Type 0 monster-credit objectives for the hit player, without requiring a creature death. Effect90 is personal; effect134 shares with nearby same-map-copy party members. Unrelated objectives and completed counts do not advance. Retail defines no cast-spell objective Type: Type5 means learn-spell.
- [x] Extinguishing Hope26391 retains its existing item-use → Spray Water → SmartAI spell-hit → monster credit path (`quest_spell_tests.rs`).
- [ ] Effect16 quest-completion events, effect316/317 label credit, arbitrary dummy/scripted spell credit and Type5 learn-spell objectives are not covered by this slice.
- [ ] Daily/weekly quests have no reset; they are one-time. Timed quests, `RequiredFactionId`/reputation, skill requirements, reward spells, reputation rewards and `ItemDrop` quest-item drops are not applied.
- [ ] Creature `KillCredit1/2` entries do not give credit; kill credit ignores tap rules and dead group members.
- [ ] `zone` in `QuestEntrySnapshot` is empty (no area names server-side); clients resolve `sort_id`. POI `world_map_area_id` is a Wrath `WorldMapArea` id, not a Retail `UiMap` id.
- [ ] Quest texts keep `$N`/`$B`/`$g` tokens for the client to substitute.
- [ ] Reward items always take a new bag cell (no stacking onto existing stacks when checking space).

## Out of scope

Gossip menus, quest greetings, shared quests and world quests. Escort interpreter behavior is governed by the separate [escort contract](escort-quests.md).
