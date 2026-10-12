# World quests

Contract and World of Osso rotation policy: [engine world-quest spec](../../game-engine/docs/specs/world-quests.md). This is incomplete feature work, not deployed behavior.

## Private import

`python3 scripts/import_world_quests.py --source-db /home/osso/Projects/world-of-osso/game-server/data/world.db --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --output-db /home/osso/.worktrees/world-quests-lead/world.db`

The command opens the source read-only, SQLite-backups it into an exclusively created output, and imports only QuestInfo world-quest definitions whose ManagedWorldState has an authenticated positive UpTimeSecs in the local CASC receipt. Existing output files and source/output identity are rejected. No canonical database write is needed.

It maps TDB objectives and static POIs, preserving authentic race masks and raw reward/template data in `content_world_quest.source_template_json`. It never fills missing rewards from other products. Unknown-duration quests remain excluded. Unsupported objectives abort the import instead of being dropped into an auto-completable quest.

## Source-area audit

`python3 scripts/audit_world_quest_areas.py --db2-dir /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/db2 --duration-receipt /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/source-census.json --output /home/osso/.worktrees/world-quests-lead/area-poi-joins.json`

The matching WoWDBDefs layouts decode3,909 AreaPOI,1,070 AreaPOIState and154 ManagedWorldStateBuff records. AreaPOI stores its continent relationship inline; `db2_casc.decode(..., relationship_column=15)` validates that stored value against the relationship map instead of duplicating it. Source joins find zero matching quest/stage/boss-name POIs or managed buffs for the four admitted quests. No acceptance radius is inferred.

## Runtime

`world_quests.rs` computes half-open per-zone windows from the policy; `quest_time.rs` refreshes them with the realm ServerClock and removes expired task logs/watches. Definitions load from `content_world_quest`; invalid/zero durations and missing runtime quest templates fail loading. No table means the import has not been installed, not a substitute activation source.

Area-entry acceptance, automatic source-backed reward settlement, availability transport, native pins/tooltips/task tracking and UDP5861 both-skin proof remain open. Rotation tests do not establish those behaviors.
