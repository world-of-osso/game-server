# Quests

Quest givers, the quest log and objective tracking for the Retail-style quest dialog, quest log and objective tracker (game-engine in-game UI plan, phase 6). Contract: `shared::protocol` `quest_messages.rs` + `QuestLogSnapshot` (shared-protocol). Server: `crates/server/src/quest.rs`, `quest_requests.rs`, `quest_rules.rs`, `quest_view.rs`, `quest_data.rs` (world.db `content_quest_*`), `persistence_quests.rs`. See [quest system](../wiki/systems/quests.md) for the mechanism.

## What it must do

- [x] Quest giver markers are per player: `QuestGiverStatusQuery{npcs}` answers `QuestGiverStatusMultiple` (None, Unavailable grey `!`, LowLevelAvailable trivial `!`, Incomplete grey `?`, Available `!`, Reward `?`; highest wins). After any change to the player's quests the server re-sends the markers of every NPC the connection queried.
- [x] `QuestGiverHello{npc}` lists quests in the log the NPC ends (Complete/Incomplete) and quests it starts that the player can take now (Available/LowLevelAvailable).
- [x] `QuestGiverQueryQuest` answers `QuestGiverQuestDetails` (texts, level, objectives, money, fixed and choice reward items) for quests the NPC starts and the player can take.
- [x] `QuestGiverAcceptQuest` checks level (`MinLevel`, addon `MaxLevel`), class and race masks, already in log / already done (non-repeatable), prerequisites (`PrevQuestID` and reverse `NextQuestID`, AzerothCore sign rules), positive exclusive groups, `RewardNextQuest` chains, 35-quest log cap. Accepted quests are auto-watched; a `StartItem` is given.
- [x] Kill objectives count creature kills by the killer and group members within 74 yards of the creature (melee and spell kills).
- [x] Item objectives count the items in the bags, whatever path added them; removing items lowers progress.
- [x] `QuestGiverCompleteQuest` at the ender answers `QuestGiverOfferReward` when objectives are done, else `QuestGiverRequestItems{can_complete: false}`.
- [x] `QuestGiverChooseReward` requires a valid choice when choice items exist, free bag cells for every granted item, and gold for negative `RewardMoney`; destroys required and provided items; grants money and items; marks the quest turned in; grants QuestXP experience ([experience](experience.md)); answers `QuestGiverQuestComplete` (with its `xp`); offers the chain's next quest (`QuestGiverQuestDetails`) when the same NPC starts it.
- [x] `AbandonQuest` removes the quest and its provided start item; `SetQuestWatched` toggles tracking.
- [x] Enter world sends `QuestLogSnapshot` (entries with objectives and counts, completion, level, sort id, texts, watched flag, POI polygons); later changes send `QuestLogUpdate{changed, removed, watched_quest_ids}`.
- [x] Quest log, progress, turned-in quests and watch list persist per character (redb `quest_logs`).
- [x] Rejections answer `QuestFailed{quest_id, reason}`; NPCs must be alive and within 10 yards.

## Tests asserting this spec

- `crates/server/src/quest_tests.rs` — real rows through the message path: A Threat Within (783) Willem → McBride persisted with Kobold Camp Cleanup (7) offered next; 7 blocked until 783 turned in; 7 progresses on Kobold Vermin kills and pays 25 copper; Brotherhood of Thieves (18) counts bag bandanas, rejects a missing choice, grants Militia Hammer; abandon; untrack persists across re-entry; out-of-range giver.
- `crates/server/src/quest_rules_tests.rs` — markers, prerequisites, kill credit, trivial level, race mask, log cap.
- shared-protocol `src/protocol_quest_tests.rs` — wire round trips, marker order, registration.

## Known gaps (current cycle)

- [ ] Game object objectives never progress (game object templates/spawns are not imported); explore/area-trigger objectives have no data.
- [ ] Wrath race masks: `AllowableRaces` 1101/690 lists Wrath races only, so post-Wrath races (Worgen, Void Elf, Pandaren, ...) fail race-restricted quests.
- [ ] Daily/weekly quests have no reset; they are one-time. Timed quests, `RequiredFactionId`/reputation, skill requirements, reward spells, reputation rewards and `ItemDrop` quest-item drops are not applied.
- [ ] Creature `KillCredit1/2` entries do not give credit; kill credit ignores tap rules and dead group members.
- [ ] `zone` in `QuestEntrySnapshot` is empty (no area names server-side); clients resolve `sort_id`. POI `world_map_area_id` is a Wrath `WorldMapArea` id, not a Retail `UiMap` id.
- [ ] Quest texts keep `$N`/`$B`/`$g` tokens for the client to substitute.
- [ ] Reward items always take a new bag cell (no stacking onto existing stacks when checking space).

## Out of scope

Gossip menus, quest greetings, shared quests, escort scripts, world quests.
