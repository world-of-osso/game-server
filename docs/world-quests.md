# World quests

Contract and World of Osso rotation policy: [engine world-quest spec](../../game-engine/docs/specs/world-quests.md). Implemented for the currently representable subset; whole coverage remains open at8 polygon quests:3 criteria trees,3 area-trigger tasks and2 labels absent from local CreatureLabel data. Not deployed to the shared realm.

## Private import

`python3 scripts/import_world_quests.py --source-db /home/osso/Projects/world-of-osso/game-server/data/world.db --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --poi-db2-dir /home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933 --policy-duration 86400 --faction-reward-db2 /home/osso/.worktrees/world-quests-data/db2/1282622.db2 --creature-label-db2 /home/osso/.worktrees/world-quests-data/db2/3055733.db2 --output-db /home/osso/.worktrees/world-quests-lead/world.db`

The command opens the source read-only and SQLite-backups into an exclusively created output. It admits world quests with a nondegenerate polygon from pinned QuestPOIBlob/QuestPOIPoint and objectives supported by the existing quest system. Single points are pins, not areas. Authentic ManagedWorldState UpTimeSecs takes precedence; other admitted quests use `--policy-duration` (default86400 seconds), explicit World of Osso policy. Existing output files and source/output identity are rejected.

It maps TDB objectives and static POIs, preserving authentic race masks and raw reward/template data in `content_world_quest.source_template_json`. It never fills missing rewards from other products. Unsupported or empty objective sets are excluded, never silently made auto-completable. `content_world_quest_area` retains ordered world-coordinate vertices and authentic MapID/UiMapID. `duration_source` distinguishes authentic and policy durations.

## Objective coverage (verified2026-10-11)

Private `polygon-world-v5.db` admits236/244 polygon quests, all policy durations;12 lack supported reward data. Dynamic objectives preserve authored ordering, optional/hidden/contributor/sequenced flags, weights and Flags2. Types0/1/2/3/9/15/21 are admitted when their required source joins resolve. Sequence gates reject premature credit and recognize derived progress-bar completion. Successful authoritative NPC openings credit TalkTo; combat deaths credit personal player-kill objectives. CreatureLabel FDID3055733/layout25B2B438 joins TDB CreatureDifficultyID to entry and difficulty mode; only real tapped creature deaths credit labels, not KillCredit aliases or scripted kill credits. Labels4346/4348 have no local rows, leaving86938/86945 excluded.

Quest-bound item objectives persist counts instead of reading bag counts; validated loot acquisition suppresses item creation for a single updated non-source bound objective. Completed bound objectives stop quest-only drops; reward settlement does not consume unrelated bag copies. Real request/temp-redb tests cover capped loot, replay and difficulty-specific label credit. Other item-acquisition routes and fresh live proof remain unverified.

## Source-area audit

`python3 scripts/audit_world_quest_areas.py --db2-dir /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/db2 --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --output /home/osso/.worktrees/world-quests-lead/area-poi-joins.json`

The matching WoWDBDefs layouts decode3,909 AreaPOI,1,070 AreaPOIState and154 ManagedWorldStateBuff records. AreaPOI stores its continent relationship inline; `db2_casc.decode(..., relationship_column=15)` validates that stored value against the relationship map instead of duplicating it. Source joins find zero matching quest/stage/boss-name POIs or managed buffs for the four admitted quests. No acceptance radius is inferred.

## Runtime

`world_quests.rs` computes half-open per-zone windows from the policy; `quest_time.rs` refreshes them with the realm ServerClock and removes expired task logs/watches. Definitions load from `content_world_quest`; invalid/zero durations and missing runtime quest templates fail loading. No table means the import has not been installed, not a substitute activation source.

Area entry accepts and tracks; departure untracks without discarding progress. Existing objective credit automatically settles complete tasks. The reward transaction persists quest history, items/money and imported currency/reputation together; XP uses pinned QuestXP rather than synthetic quest XP. Numeric reward amounts follow TrinityCore Player.cpp6627–6638 (override/100 or QuestFactionReward difficulty). WorldQuestAvailability publishes per-character eligible windows, authentic geometry centers, expiry and rewards previews. Native pins/tooltips, task progress/timers and toasts are integrated. Private5861 both-skin86941 completion uses administrator-supplied authentic items, not natural gathering; terrain/death-state limitations remain explicit in the engine investigation. Unsupported objective shapes are an implementation gap, not an approved exclusion policy. Rotation tests do not establish those behaviors.
