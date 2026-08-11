# AzerothCore Comparison

Side-by-side comparison of this Rust/Bevy server against AzerothCore, the C++ WoW 3.3.5a emulator used as the primary architecture reference.

## Comparison Table

| Aspect | AzerothCore (C++) | This Server (Rust/Bevy) |
|--------|-------------------|-------------------------|
| **Language** | C++17 | Rust (edition 2024) |
| **Game state model** | Custom OOP hierarchy | Bevy ECS (data-oriented, archetype columns) |
| **Tick rate** | ~1ms main loop (variable) | 20Hz FixedUpdate (deterministic 50ms ticks) |
| **Map threading** | Thread pool (MapUpdater) | Bevy parallel system scheduling (automatic) |
| **Networking** | Boost ASIO + custom WoW protocol | UDP + netcode via Lightyear |
| **Persistence** | MySQL (async queries, always live) | redb (embedded; ECS is live state, redb is checkpoint) |
| **Static data** | MySQL world DB (~500MB, loaded at startup) | SQLite `data/world.db` (132MB, loaded at startup) |
| **State sync to clients** | Manual packet construction per system | Automatic via Lightyear replication + delta compression |
| **Client prediction** | None — server authoritative, client waits for ACK | Lightyear prediction + rollback |
| **Scripting** | C++ + Eluna (Lua bindings) | Rhai or Lua (optional, unresolved — see [[open-questions]]) |
| **Build system** | CMake + vcpkg | Cargo |

## Key Architectural Differences

**Zone threading**: AzerothCore's MapUpdater manually assigns zones to a thread pool and synchronizes at map boundaries. Bevy's scheduler makes independent zone system sets run in parallel automatically — the same outcome with less coordination code.

**Persistence model**: AzerothCore writes to MySQL continuously via async queries; the database is always the source of truth. Here the ECS is the source of truth while the server runs; redb is a periodic checkpoint. This eliminates the MySQL server process entirely and means persistence code is a single save system using `Changed<T>` filters.

**Client prediction**: AzerothCore has none. Clients receive authoritative updates and wait. Lightyear gives this server rollback-based prediction out of the box, which is the dominant model in modern game networking.

**Protocol**: AzerothCore implements the WoW 3.3.5a binary protocol over TCP. This server uses Lightyear's UDP transport with netcode and separate reliable/unreliable channels — matching movement to unreliable delivery and combat/chat/inventory to reliable ordered delivery.

## What We Borrow From AzerothCore

AzerothCore at `~/Repos/azerothcore` remains a reference for:

- Combat formulas and spell resolution logic
- Creature AI behavior patterns
- Zone/map boundary handling model (the MapUpdater concept, adapted to system sets)
- The SQL world data dumps that populate `data/world.db`

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — comparison table, zone architecture, persistence strategy
- [CLAUDE.md](../../../CLAUDE.md) — AzerothCore repo path and usage notes
- [crates/server/Cargo.toml](../../../crates/server/Cargo.toml) — Lightyear UDP/netcode features

## See Also

- [[stack-decisions]] — why each layer in this stack was chosen
- [[reference-repos]] — AzerothCore and other external reference repositories
- [[open-questions]] — scripting approach is still unresolved
