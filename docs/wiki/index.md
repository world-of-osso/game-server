# Wiki Index

Content catalog for the game-server wiki.

## design/

Architecture decisions and design specifications.

| Page | Summary |
|------|---------|
| [[stack-decisions]] | Why Bevy + Lightyear + redb; SpacetimeDB rejection rationale; bitcode and SQLite choices |
| [[azerothcore-comparison]] | Side-by-side: language, ECS vs OOP, tick rate, threading, networking, persistence, prediction, scripting |
| [[open-questions]] | Unresolved: tick rate (single vs multi-rate), world size (big_space vs origin-shifting), instance management, hot-reloading scripting |

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
| [[networking]] | Lightyear over QUIC: replication, PlayerInput model, channels (unreliable movement, reliable combat/chat/inventory), interest management |
| [[persistence]] | Dual-layer storage: redb for mutable player data (characters, inventory, guilds, mail, auctions), SQLite for static world data |
| [[world-data]] | SQLite world.db: all tables with row counts, AzerothCore/Retail DB2 sources, WorldDataPlugin, SpellDatabasePlugin |
| [[tick-model]] | 20Hz FixedUpdate pipeline: inputs → AI → movement → combat → zone transitions → replication → persistence flush |
| [[zone-architecture]] | Zone system sets, parallel execution, entity Zone tagging, zone transfers, AzerothCore MapUpdater comparison |
| [[combat]] | Auto-attack system, melee hit table (miss/dodge/parry/glancing/block/crit/hit), spell formulas, stat calculations, DR curves |
| [[creature-ai]] | Three movement types (idle/wander/waypoint), SpawnPoint, chase system, 40-yard leash/evade, return-to-spawn |

## investigations/

Debug logs, root cause analyses. (No pages yet.)
