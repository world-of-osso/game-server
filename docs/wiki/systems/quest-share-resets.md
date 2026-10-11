# Quest sharing, resets and timers

Authoritative quest lifecycle extensions, verified 2026-10-11 on unmerged quest-share. [Contract](../../specs/quest-share-resets.md).

## Persistence and clock

Startup migrates legacy quest_logs into quest_logs_v2 in the same table-creation transaction, then deletes the old table. Active entries retain counts and add deadline/failure; completed_at retains last per-character reward time while rewarded retains prerequisite history. Unknown legacy completion times are stamped once at migration; legacy timed quests without recoverable start time fail on the next runtime refresh. Migration never grants a new timer.

QuestTime refresh uses existing maps::difficulty::ServerClock (fixed clock in tests, clock-advance on the private admin socket). It refreshes QuestDb's evaluation time before initial snapshots/requests, fails elapsed timers and dirties logs. On a daily boundary it dirties online markers; offline eligibility is recomputed at login/accept. Periodic reward checks use next_reset_time from maps/locks.rs. Its existing policy is daily15:00 UTC / Tuesday15:00 UTC; this is local configuration/code, not a universal Retail schedule. Exclusive alternatives use current periodic locks, not lifetime rewarded history.

TimeAllowed is in content_quest_template (TDB/legacy normalized schema), not local quest_template_addon. QuestInfo and daily/weekly/repeatable flags already come from authored tables. Sharing requires the authored0x8 sharable flag and active source log; timed shares inherit the original deadline, not fresh duration. Failure precedes request processing and completion checks independently reject elapsed/failed entries.

## Proof boundary

Protocol wire tests5, server quest tests48, engine quest-flow tests15 after recipient-result coverage; compile/format proof in canonical game-engine `data/diagnostics/quest-share-2026-10-11/`. Reset boundary and exclusive-alternative RED/GREEN logs retained. Live proof uses actual content relations (not legacy creature_queststarter): quest62, daily26153/Robby Flay with item57137×6, timed938/Mist with540 seconds. Both skins/private UDP5517 observed. Maps0/1 are hosted; the attempted map530 transfer was rejected by existing MapNotAllowed policy, not changed here.

## Sources

- [Contract and test inventory](../../specs/quest-share-resets.md).
- `crates/server/src/{persistence_quests,quest_time,quest_rules,quest_requests,quest_data,quest_view}.rs`.
- [Native UI proof](../../../../game-engine/docs/wiki/systems/quest-share-resets.md).
- TrinityCore a352b1fa QuestHandler.cpp party push; local Retail GlobalStrings and QUEST_ACCEPT popup.

## See Also

- [Local reset calendar](../../../crates/server/src/maps/locks.rs).
