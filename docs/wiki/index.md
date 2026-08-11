# Wiki Index

Content catalog for the game-server wiki.

## design/

Architecture decisions and design specifications.

| Page | Summary |
|------|---------|
| [[stack-decisions]] | Why Bevy + Lightyear + redb; SpacetimeDB rejection rationale; bitcode and SQLite choices |
| [[azerothcore-comparison]] | Side-by-side: language, ECS vs OOP, tick rate, threading, networking, persistence, prediction, scripting |
| [[open-questions]] | Unresolved: tick rate (single vs multi-rate), world size (big_space vs origin-shifting), instance management, hot-reloading scripting |
| [[tick-loop-design]] | Draft normative server-loop design: 20Hz fixed phases, Update/PostUpdate boundaries, completed player-death and broad auto-AFK scan reductions, catch-up, replication, persistence, and review decisions |

## reference/

External resources, tooling, data sources, development workflow.

| Page | Summary |
|------|---------|
| [[reference-repos]] | AzerothCore (architecture + data), Rustbolt Core (Rust patterns), SpacetimeDB (rejected), wow-engine (sibling renderer) |
| [[dev-commands]] | cargo run, cargo test, run-tests.sh, admin CLI via SSH, dependency versions |
| [[crate-structure]] | Three crates (shared, server, client) with purpose and module breakdown |
| [[code-health]] | 4,838 symbols, 88 untested production candidates, dead-code situation, top hotspots |

## systems/

Server gameplay systems.

| Page | Summary |
|------|---------|
| [[ecs-storage]] | Bevy ECS as columnar in-memory database: archetype tables, zero-overhead queries, built-in change detection |
| [[networking]] | Lightyear 0.28.0 over UDP with vendored bevy_replicon 0.41.1: dirty-candidate replication, 50 ms ReplicationMetadata cadence, local NetworkVisibility plus VisibilityExt, GoldUpdateQueue timing, change-filtered durability/rest snapshots, drain-first calendar/trade requests, PlayerInput model, completed player-death and broad auto-AFK scan reductions, channels, and interest management |
| [[persistence]] | Dual-layer storage: redb for mutable player data, login-loaded gold, mutation-boundary GoldUpdateQueue replication, and SQLite for static world data |
| [[world-data]] | SQLite world.db: all tables with row counts, AzerothCore/Retail DB2 sources, WorldDataPlugin, SpellDatabasePlugin |
| [[tick-model]] | Observed server loop: 50ms runner/fixed cadence, schedule ownership, explicit ordering, completed player-death and broad auto-AFK scan reductions, maintenance gates, and IPC boundary |
| [[zone-architecture]] | Proposed zone/instance partitioning; current implementation is startup-only Elwynn spawning with no per-zone sets or transfers |
| [[combat]] | Auto-attack system, NPC/player lethal transitions, split death proofs, melee hit table (miss/dodge/parry/glancing/block/crit/hit), spell formulas, stat calculations, DR curves |
| [[creature-ai]] | Three movement types (idle/wander/waypoint), SpawnPoint, chase system, 40-yard leash/evade, return-to-spawn |

## investigations/

Debug logs and root cause analyses.

| Page | Summary |
|------|---------|
| [[semantic-noop-replication]] | First- and second-wave server equal-assignment proofs/fixes, Bevy 0.19 change ticks, Lightyear 0.28.0/bevy_replicon sending, direct NPC death transition, Gold ruled out, and blocked wire-level proof |
| [[recurring-loop-elimination]] | Tests-first reductions and eliminations across auction, terrain, friends, auto-AFK, interest, Lightyear, Who roster, gold, durability, calendar, trade-offer, inactive auto-attack, rest-state, and player-death scans; broad death/auto-AFK scans are eliminated while wider scheduler/dirty-boundary requirements remain partial |
| [[tick-loop-audit]] | Exhaustive recurring-schedule audit: 103 registrations / 130 expanded systems; seven narrow removal slices plus player-death and broad auto-AFK scan reductions complete, while preserved Lightyear, interest, friends, and terrain requirements remain partial alongside persistence/order risks and receiver-fan-out measurement |
