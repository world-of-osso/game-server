# World quests

Contract and World of Osso rotation policy: [engine world-quest spec](../../game-engine/docs/specs/world-quests.md). Implemented for the currently representable subset; whole coverage remains open at5 polygon quests:3 area-trigger tasks and2 labels absent from local CreatureLabel data. Not deployed to the shared realm.

## Private import

`python3 scripts/import_world_quests.py --source-db /home/osso/Projects/world-of-osso/game-server/data/world.db --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --poi-db2-dir /home/osso/Projects/world-of-osso/game-engine/data/db2/active --policy-duration 86400 --output-db /home/osso/.worktrees/world-quests-lead/world.db`

The command opens the source read-only and SQLite-backups into an exclusively created output. QuestPOIBlob, QuestPOIPoint, QuestXP, CreatureLabel and QuestFactionReward CSVs all come from the same `--poi-db2-dir` installed-build directory; no separately pinned binary inputs remain. The former CreatureLabel binary had75,552 rows versus75,820 in installed70391, so matching binary layout alone did not establish current source identity. It admits world quests with a nondegenerate polygon from pinned QuestPOIBlob/QuestPOIPoint and objectives supported by the existing quest system. Single points are pins, not areas. Authentic ManagedWorldState UpTimeSecs takes precedence; other admitted quests use `--policy-duration` (default86400 seconds), explicit World of Osso policy. Existing output files and source/output identity are rejected.

It maps TDB objectives and static POIs, preserving authentic race masks and raw reward/template data in `content_world_quest.source_template_json`. It never fills missing rewards from other products. Unsupported or empty objective sets are excluded, never silently made auto-completable. `content_world_quest_area` retains ordered source X/Y/Z vertices and authentic MapID/UiMapID. The derived area table is rebuilt on import; source Z is required, and old XY-only exports must be reimported before the new runtime can load them. No zero-height compatibility path is provided. An installed XY-only area table fails with a diagnostic naming `content_world_quest_area.z` and the maintained importer command; publish the rebuilt world-quest tables before starting the server. `duration_source` distinguishes authentic and policy durations.

## Objective coverage (verified2026-10-11)

Private `polygon-world-v7.db` admits239/244 polygon quests, all policy durations;12 lack supported reward data. Dynamic objectives preserve authored ordering, optional/hidden/contributor/sequenced flags, weights and Flags2. Types0/1/2/3/9/14/15/21 are admitted when their required source joins resolve. Sequence gates reject premature credit and recognize derived progress-bar completion. Successful authoritative NPC openings credit TalkTo; combat deaths credit personal player-kill objectives. CreatureLabel FDID3055733/layout25B2B438 joins TDB CreatureDifficultyID to entry and difficulty mode; only real tapped creature deaths credit labels, not KillCredit aliases or scripted kill credits. Labels4346/4348 have no local rows, leaving86938/86945 excluded.

Quest-bound item objectives persist counts instead of reading bag counts; validated loot acquisition suppresses item creation for a single updated non-source bound objective. Completed bound objectives stop quest-only drops; reward settlement does not consume unrelated bag copies. Real request/temp-redb tests cover capped loot, replay and difficulty-specific label credit. Other item-acquisition routes remain unverified. Current admitted bound items have zero mapped item effects; broader item-effect compatibility is not certified.

The3 authored criteria trees are unconditional All→Single amount-one leaves. Persist their completion in the active task objective counter, never lifetime achievement state: creature/script credit225891/227455 and authoritative GainAura420847. Other criteria-tree shapes are not certified. Type19 source is `AreaTrigger::HandleUnitEnter`→template entry, not static trigger-position IDs or task polygons (`reference/AreaTrigger.cpp:167–170,1017`); all six required template bindings are absent from inspected TDB data and all156 decoded local AreaTriggerCreateProperties rows (FDID1937227). Amount0 is loaded as an unset boolean requiring1; production type19 credit remains unwired pending authentic bindings, and these quests are not admitted. Trigger quests82468/86610/91804 remain excluded, without fabricated geometry.

Imported reputation caps apply before positive settlement (`Player.cpp:6645`); cap0 is unlimited, losses remain allowed. Real redb regression verifies quest45068's cap7 does not award beyond already-Exalted standing. Numeric and profession-knowledge rewards share the staged wallet in the merged transaction; ordinary master knowledge-only batch behavior is unchanged and outside this task's proof.

## Five admission gaps — verified 2026-10-16

A fresh installed70391 QuestPOIBlob/QuestPOIPoint polygon census joined to the private TDB-backed quest source leaves exactly these five of244 polygon tasks outside239 admitted definitions:

| Quest | Required source / producer still missing | Disposition |
|---|---|---|
|82468 Let Them Win|Type19 objective450336, template32539; authored trigger/create-properties/spawn binding and enter-credit producer|Named gap; do not substitute the task polygon for the ring.|
|86610 Nor Void of Night|Type19 objectives457126/457127/457128/457132, templates35987/35988/35989/35990; bindings and enter-credit producer|Named gap; no invented delivery volumes.|
|91804 Fish for the Fire|Type19 objective466299, template40181; binding and enter-credit producer|Named gap; no invented airflow volume.|
|86938 Void Perforations|Type21 objective457598, label4346; installed CreatureLabel has zero members|Named source gap; existing label kill producer needs authentic membership.|
|86945 Shadowed Supplies|Type21 objective457610, label4348; installed CreatureLabel has zero members|Named source gap; no guessed creature entries.|

Pinned TC `AreaTrigger::HandleUnitEnter` credits its template `GetEntry()`, not a static `AreaTrigger.db2` position ID or a task polygon. The six template IDs have no TDB template rows, no create-properties rows referencing `AreaTriggerId`, and no spawns joined through those properties. Installed70391 AreaTriggerCreateProperties has159 rows; checking its independent ID namespace is not a template-binding join. Installed CreatureLabel has75,820 rows and no members for4346/4348. The existing runtime decodes type19 amount0 as an unset boolean requiring one entry, but no general authoritative enter producer is wired. Do not admit these tasks by omitting optional objectives or guessing geometry. No admission code or source content changed. Private reproducible queries and source hashes: `world-quests2-proof/five-admission-gaps-70391.json` (outside tracked checkout).

## Source-area audit

`python3 scripts/audit_world_quest_areas.py --db2-dir /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/db2 --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --output /home/osso/.worktrees/world-quests-lead/area-poi-joins.json`

The matching WoWDBDefs layouts decode3,909 AreaPOI,1,070 AreaPOIState and154 ManagedWorldStateBuff records. AreaPOI stores its continent relationship inline; `db2_casc.decode(..., relationship_column=15)` validates that stored value against the relationship map instead of duplicating it. Source joins find zero matching quest/stage/boss-name POIs or managed buffs for the four admitted quests. No acceptance radius is inferred.

## Current source choice census

Verified2026-10-16 against239 imported70391 source-template JSON rows: zero tasks have a nonzero `RewardChoiceItemID1` through `RewardChoiceItemID6`. Reward choice is not applicable to these tasks; no unsourced automatic choice, reward substitution or fabricated choice quest is added. The engine contract owns that boundary.

## Runtime

`world_quests.rs` computes half-open per-zone windows from the policy; `quest_time.rs` refreshes them with the realm ServerClock and removes expired task logs/watches. Definitions load from `content_world_quest`; invalid/zero durations and missing runtime quest templates fail loading. No table means the import has not been installed, not a substitute activation source.

Area entry accepts and tracks; departure untracks without discarding progress. Existing objective credit automatically settles complete tasks. The reward transaction persists quest history, items/money and imported currency/reputation together; XP uses pinned QuestXP rather than synthetic quest XP. Numeric reward amounts follow TrinityCore Player.cpp6627–6638 (override/100 or QuestFactionReward difficulty). WorldQuestAvailability publishes per-character eligible windows, every source area's world/UI map and componentwise mean-vertex X/Y/Z representative, shared expiry and rewards previews. It no longer drops all but the first area: task82521 retains its lower2256 and main2255 areas under one logical quest. Acceptance still checks any actual source polygon; no acceptance geometry or window policy is changed. [Wire contract](../../shared-protocol/docs/world-quests.md). Native pins/tooltips, task progress/timers and toasts are integrated. Private5861 both-skin86941 completion uses administrator-supplied authentic items. Subsequent matching current binaries complete real86948 using administrator-driven deaths of authored NPC46327:7/8→automatic completion,11765XP, living actors, Modern relog remains empty. Actor level80 fixture health is restored with the existing revive lifecycle; no terrain code changed. These are controlled acquisition/combat proofs, not natural gameplay certification. Unsupported objective shapes are an implementation gap, not an approved exclusion policy. Rotation tests do not establish those behaviors.
