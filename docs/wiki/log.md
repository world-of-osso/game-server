# Wiki Log

## [2026-08-10] investigation | Replace recurring gold persistence scan

Updated [[persistence]], [[networking]], and [[recurring-loop-elimination]] for commits `4e4c73d`, `c3b3c04`, `9d2067c`, `91bbedd`, and `8920feb`. Documented login-loaded persisted gold; successful barber, completed-trade, auction, and mail-claim saves recording exact values in `GoldUpdateQueue`; same-frame coalescing; `PostUpdate` application before Lightyear buffering; equal-value suppression; offline discard with reload-on-login; profession saves not changing gold; sale proceeds remaining mail until claim; and removal of `sync_player_gold_from_inventory`. Recorded RED/GREEN and readability artifacts under `/tmp/claude/game-server/`. No live CPU or Character-stage claim.

## [2026-08-10] investigation | Gate rest-state evaluation on relevant changes

Updated [[recurring-loop-elimination]], [[networking]], [[tick-loop-audit]], and `index.md` for `fffa86a`: `update_player_rest_state` now evaluates only players without `PlayerRestState` or with changed `Position`/`Level`, instead of scanning every authenticated player and all innkeepers every `Update`. Initial snapshots, enter/leave transitions, and level-based rested-XP recalculation remain intact. Recorded RED `/tmp/claude/game-server/rest-dirty-evaluation-red.log`, GREEN `/tmp/claude/game-server/rest-dirty-evaluation-green.log`, and Rust readability evidence at `/tmp/claude/game-server/rest-dirty-evaluation-readability`. No live CPU or Character-stage claim is made before measurement.

## [2026-08-10] investigation | Remove inactive auto-attack participation

Updated [[combat]], [[recurring-loop-elimination]], and [[tick-loop-audit]] for `152ccbd`: clearing `CombatTarget` now removes `AutoAttack` instead of retaining an inactive component in the fixed combat query. Active swing cadence, combat-grace behavior, damage/death, respawn, and reliable combat events remain unchanged. Recorded RED `/tmp/claude/game-server/autoattack-inactive-removal-red.log`, GREEN `/tmp/claude/game-server/autoattack-inactive-removal-green.log`, and cargo fmt/readability evidence. No live CPU or Character-stage claim is made before measurement.

Chronological record of wiki operations.

## [2026-08-10] audit | Reconcile tick-loop audit schema and persistence summary

Added required `## Content` sections to edited legacy pages [[recurring-loop-elimination]] and [[networking]]. Corrected [[persistence]] from the stale five-minute/whole-period `Changed<T>` description to the source-backed 300-fixed-tick nominal 15-second cadence and confirmed early-return bug. Added cross-links to [[tick-loop-audit]]. No runtime behavior changed.

## [2026-08-10] investigation | Audit every recurring server schedule

Created [[tick-loop-audit]] from a mechanical inventory of 103 recurring registration calls expanding to 130 systems: 105 `Update`, 21 `FixedUpdate`, 3 `PostUpdate`, and 1 `Last`. Confirmed that `save_dirty_characters` loses earlier `Changed<T>` state because its internal 300-tick early returns still advance Bevy's system `last_run`; recorded the Bevy 0.18 source semantics and focused reproduction. Classified definite recurring waste (`sync_player_gold_from_inventory`, Who roster construction, rest scans, durability probing, inactive auto-attacks), correctness-dependent polls, 74 receiver systems, already-eliminated loops, and missing fixed-order consequences. Updated [[tick-loop-design]], [[tick-model]], [[recurring-loop-elimination]], [[networking]], `index.md`, and `ARCHITECTURE.md`. No runtime behavior changed and no production CPU claim is made.

## [2026-08-10] audit | Align tick-loop documentation with source

Audited [[tick-loop-design]], [[tick-model]], [[zone-architecture]], [[open-questions]], `index.md`, and `ARCHITECTURE.md`. Added required `## Content` sections, corrected the root architecture page's stale implemented-zone wording, and confirmed wiki/source links resolve. No runtime behavior changed.

## [2026-08-10] design | Draft explicit game tick-loop contract

Created [[tick-loop-design]] as the reviewable normative design for the server runner, Bevy schedules, fixed simulation phases, Lightyear receive/replication boundaries, bounded catch-up, maintenance cadence, persistence, and admin IPC. Rewrote [[tick-model]] as observed current state: Lightyear sets `Time<Fixed>` to 50 ms, most request handlers run in `Update`, replication buffers in `PostUpdate`, and only local ordering constraints exist. Corrected [[zone-architecture]] to proposed status, updated [[open-questions]] and `index.md`, and linked `ARCHITECTURE.md` to the current/design split. No runtime behavior changed.

## [2026-08-10] investigation | Wake replication for zero-translation movement state

Updated [[recurring-loop-elimination]] and [[networking]] for `aa5dddb`. Recorded the discovered gap where turn-in-place or zero-translation inputs changed replicated `Rotation` or `MovementSpeed` without producing a Lightyear dirty candidate. Documented the fix and `networking_movement.rs` extraction: replication dirtying now covers rotation/speed changes, while `InterestDirtyQueue` remains position-only. Recorded the RED no-op regression and 43-test GREEN focused server proof. No broader CPU or Character claim.

## [2026-08-10] investigation | Replace Lightyear root traversal with dirty candidates

Updated [[recurring-loop-elimination]], [[networking]], [[tick-model]], and `index.md` for `269655a` and `30b5078`. Recorded vendored `lightyear_replication` `0.26.4` provenance, removal of the per-sender 50 ms all-root/all-component traversal, global and sender-targeted dirty candidates, initial full sync, ACK/NACK retention, visibility and component lifecycle triggers, and authoritative writer marks. Recorded test-only candidate/visit diagnostics: the RED idle case measured exactly 160 root visits; GREEN reduced idle candidates and `replicate_entity` visits to zero after initial sync. Recorded in-process integration coverage for changed roots, ACK retention, targeted visibility, component insert/remove, initial sync, and unchanged idle roots. The strict-Empty client CPU gate remains pending; no Character advancement or `<=10%` CPU claim is made.

## [2026-08-09] investigation | Replace recurring interest scan with dirty spatial updates

Updated [[recurring-loop-elimination]] and [[networking]] for commits `4523373` and `a85b035`. The former removed the every-500ms `NetworkVisibility`-entities × players scan: authoritative position and lifecycle triggers now dirty observers/targets, a 100-yard X/Z spatial index narrows candidates, and exact 3D `<= 100²` visibility transitions remain per connection. The latter split the real fixed-tick integration tests into `interest_tests.rs` without runtime changes. Recorded proof for unchanged ticks, new targets, same-bucket radius crossings, independent players, removal/re-add, reconnect, spawn/despawn, teleports, boundaries, corpses, and other preserved visibility behavior. Lightyear's 50ms per-sender traversal remains the only pending recurring-loop slice; this update makes no broader Empty-completion claim.

## [2026-08-09] investigation | Cache and dirty friend snapshots

Updated [[recurring-loop-elimination]] for commit `914d570`: cached online presence, reverse `owners_by_friend` links, and dirty-owner fanout replace every-frame all-owner snapshot rebuilding and offline database lookups. Recorded behavioral proof for unchanged offline data, affected-only presence, login/logout, zone/level changes, and friend-list reverse-index replacement. Privacy and ignore state are not friend-snapshot inputs and remain unchanged.

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
