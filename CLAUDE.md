# game-server

WoW-inspired multiplayer game server using headless Bevy as ECS runtime + lightyear for networking. Server is authoritative; clients predict locally and reconcile with server state.

See ARCHITECTURE.md for full design.

## Structure

```
crates/
  shared/    — Components, types, game formulas. Used by both client and server.
  server/    — Server binary. Headless Bevy with lightyear server plugin.
  client/    — Client stub. Will link to wow-engine later.
```

## Reference Repos

- `~/Repos/azerothcore` — C++ WoW 3.3.5a server (architecture reference for zone/map threading, combat systems)
- `~/Repos/rustbolt-core` — Rust WoW server attempt (early stage, structure reference)
- `~/Projects/wow/wow-engine` — Sibling Bevy 0.18 project (client renderer)

## Dev

- `cargo run -p server --bin game-server` — Start server (binds to 0.0.0.0:5000)
- `cargo test` — Run all tests
- `./run-tests.sh` — cargo test + clippy
- `ssh sakuin /opt/game-server/game-server-admin <command>` — Admin CLI (runs on sakuin via IPC socket)
- Edition 2024, rust-version 1.89
- Bevy 0.18, lightyear 0.26, redb 3, bitcode 0.6, rusqlite 0.35

## World Data (SQLite)

Static read-only data from AzerothCore, loaded at server startup into `Res<WorldData>`.

- **Database**: `data/world.db` (25MB) — regenerate with `scripts/import-azerothcore.sh`
- **Source**: AzerothCore SQL dumps from `~/Repos/azerothcore/data/sql/base/db_world/`
- **Module**: `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource

| Table | Rows | Purpose |
|-------|------|---------|
| `creature_template` | 29,947 | NPC definitions (name, level, faction, speeds, AI, flags) |
| `creature` | 149,678 | Spawn instances (map, zone, x/y/z, respawn time, wander radius) |
| `creature_template_model` | 40,213 | Display models per creature (DisplayID, scale, probability) |
| `waypoint_data` | 144,657 | Patrol paths (ordered x/y/z points with delay) |

**Movement types**: 0=idle, 1=random wander, 2=waypoint patrol

This is **not** the persistence layer — redb handles mutable player data. SQLite is read-only static world data.
