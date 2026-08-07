# Wiki Log

Chronological record of wiki operations.

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
