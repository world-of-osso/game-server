# Wiki Index

Content catalog for the game-server wiki.

## design/

Architecture decisions and design specifications.

| Page | Summary |
|------|---------|
| [[stack-decisions]] | Why Bevy + Lightyear + redb; SpacetimeDB rejection rationale; bitcode and SQLite choices |
| [[azerothcore-comparison]] | Side-by-side: language, ECS vs OOP, tick rate, threading, networking, persistence, prediction, scripting |
| [[open-questions]] | Unresolved: tick rate (single vs multi-rate), world size (big_space vs origin-shifting), instance management, hot-reloading scripting |
| [[tick-loop-design]] | Draft normative server-loop design: 20Hz fixed phases, Update/PostUpdate boundaries, catch-up, replication, persistence, and review decisions |

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
| [[networking]] | Lightyear over QUIC: dirty-candidate replication, GoldUpdateQueue timing, change-filtered durability/rest snapshots, PlayerInput model, channels, and interest management |
| [[persistence]] | Dual-layer storage: redb for mutable player data, login-loaded gold, mutation-boundary GoldUpdateQueue replication, and SQLite for static world data |
| [[world-data]] | SQLite world.db: all tables with row counts, AzerothCore/Retail DB2 sources, WorldDataPlugin, SpellDatabasePlugin |
| [[tick-model]] | Observed server loop: 50ms runner/fixed cadence, schedule ownership, explicit ordering, maintenance gates, and IPC boundary |
| [[zone-architecture]] | Proposed zone/instance partitioning; current implementation is startup-only Elwynn spawning with no per-zone sets or transfers |
| [[combat]] | Auto-attack system, melee hit table (miss/dodge/parry/glancing/block/crit/hit), spell formulas, stat calculations, DR curves |
| [[creature-ai]] | Three movement types (idle/wander/waypoint), SpawnPoint, chase system, 40-yard leash/evade, return-to-spawn |

## investigations/

Debug logs and root cause analyses.

| Page | Summary |
|------|---------|
| [[semantic-noop-replication]] | First- and second-wave server equal-assignment proofs/fixes, Lightyear 0.26.4 tick-only sending, direct NPC death transition, Gold ruled out, and blocked wire-level proof |
| [[recurring-loop-elimination]] | Tests-first removal of auction, terrain, friends, interest, Lightyear, Who roster, gold, durability, inactive auto-attack, and rest-state recurring scans; calendar and trade remain open |
| [[tick-loop-audit]] | Exhaustive 130-system recurring-schedule audit: 5/7 named loops complete, persistence correctness bug, missing order, and remaining calendar/trade measurement targets |
