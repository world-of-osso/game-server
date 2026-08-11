# Stack Decisions

Why the game-server uses Bevy + Lightyear + redb: the reasoning behind each layer, what alternatives were evaluated, and why they were rejected.

## ECS: Bevy (headless)

Bevy's archetype ECS is a columnar in-memory storage engine with zero query overhead — systems iterate `&mut [Component]` slices directly, maximizing CPU cache utilization. This makes Bevy the storage layer for live game state, not just a scheduler. Parallel system scheduling replaces the thread-pool-per-zone model used by AzerothCore's MapUpdater automatically, without manual coordination.

The shared crate means client and server use the same component types and game logic. This is the enabling condition for client-side prediction: the client runs the same systems locally.

## Networking: Lightyear

Lightyear provides everything game networking needs as a Bevy-native plugin:

- **Interest management** — only replicates entities near each client
- **Delta compression** — sends component diffs, not full state
- **Client prediction + rollback** — client predicts locally, reconciles on server correction
- **UDP + netcode transport** — reliable ordered channels for combat/chat/inventory; unreliable for movement

The alternative (roll a custom networking layer) was not evaluated seriously. Lightyear solves known-hard problems (rollback, relevance, delta compression) with Bevy integration that would take months to replicate correctly.

## Persistence: redb

redb is embedded, pure Rust, ACID, and append-only. It requires no server process and no async query layer. The persistence model is simple: ECS `Changed<T>` filters identify dirty entities, a system writes them to redb every N ticks and on disconnect. The ECS is the live database; redb is the durable checkpoint.

Alternatives considered:

- **MySQL (AzerothCore model)** — requires a server process, async query layer, and schema migrations. Adds operational complexity for no benefit when the server already holds all state in memory.
- **SQLite** — viable, but redb's pure-Rust guarantee and append-only B-tree are a better fit for the write pattern (periodic batch writes, no ad-hoc queries against character state).
- **SpacetimeDB** — rejected outright. See [[spacetimedb-rejection]].

## Why Not SpacetimeDB

SpacetimeDB was evaluated against `~/Repos/SpacetimeDB/ARCHITECTURE.md` and rejected on four grounds:

1. **No tick loop control** — reducers fire as they arrive. Games need deterministic 20Hz batching.
2. **WASM overhead** — 10–30% cost over native (bounds checks, no SIMD, indirect calls). Relevant at 2000 entities per combat tick.
3. **Global write lock** — a reducer holds the lock for its full execution. A 50ms tick = 50ms blocked reads.
4. **Protocol mismatch** — forces WebSocket + BSATN. UDP with Lightyear's reliable and unreliable channels is required for movement and ordered game messages.

SpacetimeDB solves persistence and state sync; those are the easy problems. It cannot solve tick control, and the write lock is architectural, not fixable.

## Static Data: SQLite

Read-only world data (creature templates, spells, spawn points) is loaded once at startup into `Res<WorldData>` and `Res<SpellDatabase>`. SQLite at `data/world.db` (132MB, sourced from AzerothCore SQL dumps and retail DB2 CSVs). Not the persistence layer — redb handles all mutable player data.

## Serialization: bitcode

~10x faster than serde-json, smaller wire format than bincode. Used for network messages. No alternatives evaluated — it was the clear choice for the latency budget.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — stack table, SpacetimeDB rejection rationale, Bevy ECS storage analysis, persistence strategy
- [CLAUDE.md](../../../CLAUDE.md) — dependency versions, world.db description
- [crates/server/Cargo.toml](../../../crates/server/Cargo.toml) — Lightyear UDP/netcode features

## See Also

- [[azerothcore-comparison]] — full side-by-side with the C++ reference implementation
- [[open-questions]] — tick rate, world size, and instance management remain unresolved
- [[crate-structure]] — how the three crates map to this stack
