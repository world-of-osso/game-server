# Wiki Log

Chronological record of wiki operations.

## [2026-08-09] investigation | Gate terrain streaming on tile transitions

Updated [[recurring-loop-elimination]], [[tick-model]], and [[networking]] for commit `3c5ceca`. Recorded the behavioral RED/GREEN boundary: same-tile movement previously dirtied `SentTiles` through repeated 3×3 duplicate insert attempts; it now preserves the component tick, while initial entry, reconnect, boundary crossing, and teleport semantics remain active.

## [2026-08-09] investigation | Gate auction expiration to one second

Created [[recurring-loop-elimination]] and updated [[tick-model]] for commit `e02ce0a`. Recorded the scheduler-boundary RED/GREEN proof: the first due auction expires immediately, repeated sub-second fixed ticks skip expiration, and the next scan occurs at the one-second boundary while existing persistence, sale, and mail behavior remains unchanged. A deadline heap was intentionally excluded as unnecessary complexity.

## [2026-08-09] investigation | Record direct NPC death transition

Updated [[combat]], [[tick-model]], [[networking]], and [[semantic-noop-replication]] for commit `c8762ed`: NPC death now occurs at the sole authoritative `try_swing` damage boundary; the 20 Hz all-living-NPC health scan was removed; nonlethal, lethal, and zero-health-without-transition behavior are covered; respawn and reliable `CombatEvent` semantics remain; future NPC damage writers must use the same death transition. Updated `index.md` and cross-links; no source or tests changed.

## [2026-08-09] investigation | Record second semantic no-op proof wave

Updated [[semantic-noop-replication]], [[networking]], and `index.md` for second-wave RED/GREEN commits `1c139e9` (equal derived `Health`/`Mana`), `e13e09e` (equal respawn `Health`/`Position`), `057dd17` (equal equipment appearance), and `41feebb` (equal guild membership). `f84ee17` adds the independently observed equipment-clear RED/GREEN regression. Recorded `945eb0f` as Gold synchronization already guarded and ruled out. Preserved first-wave commits and marked admin teleport, manual presence, mount-triggered speed, and other event-driven writers as remaining unproven. The Lightyear serialized-update boundary and live component frequency/byte measurement remain blocked or unmeasured; no CPU, bandwidth, or wire-count improvement claim is made.

## [2026-08-09] investigation | Document semantic no-op replication

Created [[semantic-noop-replication]] and updated [[networking]] for the completed investigation and fixes in commits `86d73f5`, `74edc39`, `f33c95a`, and `bd31161`. Recorded Lightyear `0.26.4` sender tick-only selection, 50 ms `SinceLastAck` cadence, client equality suppression after wire cost, seven RED change-tick failures, preserved real behavior, and the blocked direct serialized-update integration proof/live bandwidth measurement. No CPU or network improvement claim.

## [2026-08-07] investigation | Legacy inventory rows required eager encoding migration

Updated `systems/persistence.md` for commit `01eaf52`: production rows 6 and 7 used the pre-April 6, 2026 `InventoryItemData` encoding without `unique_auction`; startup now rewrites recognized legacy bytes with `unique_auction = false` and fails explicitly for unknown encodings instead of retaining a permanent load fallback.

## [2026-08-07] investigation | Missing `pvp_states` table caused character-selection restart loop

Updated `systems/persistence.md` with the root cause and fix from commit `ec5ff52`: `create_tables()` omitted `PVP_STATES`, so existing `game.redb` databases caused character selection to panic. `run-dev.sh` restarted the server, the client re-entered `InWorld`, and repeated rendering initialization eventually reached GPU OOM. Added the legacy-database regression-test reference.

## [2026-04-09] ingest | Systems wiki build from ARCHITECTURE.md, CLAUDE.md, AGENTS.md + crate source exploration

Created 8 systems pages:
- `systems/ecs-storage.md` — archetype tables, columnar layout, zero-overhead queries, Changed<T>
- `systems/networking.md` — Lightyear/QUIC, replication, PlayerInput, channels, CombatLog
- `systems/persistence.md` — redb tables, save triggers, what is/isn't persisted, SQLite distinction
- `systems/world-data.md` — all SQLite tables with row counts, movement type enum, chr_races
- `systems/tick-model.md` — 20Hz FixedUpdate pipeline steps, zone-parallel scheduling
- `systems/zone-architecture.md` — system sets, Zone component, zone transfers, AzerothCore comparison
- `systems/combat.md` — CombatPlugin systems, melee hit table, spell/stat formulas, DR curves
- `systems/creature-ai.md` — movement types, WanderState, WaypointState, chase, leash, evade

Updated index.md with systems/ table.

## [2026-04-09] ingest | Initial ingest from ARCHITECTURE.md, CLAUDE.md, AGENTS.md

Bootstrapped wiki from the three root documentation files. Created pages across systems/, design/, and reference/ categories.

## [2026-04-09] ingest | Full wiki build from ARCHITECTURE.md, CLAUDE.md, AGENTS.md, CODE_INDEX_TRIAGE.md + crate exploration

Created 7 pages and index.md:
- `design/stack-decisions.md` — Bevy/Lightyear/redb rationale, SpacetimeDB rejection
- `design/azerothcore-comparison.md` — full side-by-side comparison table
- `design/open-questions.md` — tick rate, world size, instances, hot-reloading
- `reference/reference-repos.md` — AzerothCore, Rustbolt Core, SpacetimeDB, wow-engine
- `reference/dev-commands.md` — cargo commands, admin CLI, world data import, dependency versions
- `reference/crate-structure.md` — shared/server/client/integration-tests with module listings from actual source tree
- `reference/code-health.md` — 4,838 symbols, 88 untested candidates, dead-code guidance
- `index.md` — content catalog
