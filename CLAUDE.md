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
- `../game-engine` — Sibling Bevy 0.19 project (client renderer)

## Dev

- `cargo run -p server --bin game-server` — Start server (binds to 0.0.0.0:5000)
- `cargo test` — Run all tests
- `./run-tests.sh` — cargo test + clippy
- `ssh sakuin /opt/game-server/game-server-admin <command>` — Admin CLI (runs on sakuin via IPC socket)
- Edition 2024, rust-version 1.95
- Bevy 0.19, lightyear 0.28, redb 3, bitcode 0.6, rusqlite 0.35

## World Data (SQLite)

Static read-only data loaded at server startup into `Res<WorldData>` and `Res<SpellDatabase>`.

- **Database**: `data/world.db` (132MB) — regenerate with `scripts/import-azerothcore.sh`
- **Sources**:
  - Creature, quest, vendor, trainer, loot, gossip, item data: AzerothCore SQL dumps from `~/Repos/azerothcore/data/sql/base/db_world/` (content tables and Wrath→Retail ID mapping: `docs/wiki/systems/content-import.md`)
  - Spell/class/talent data: build-pinned Retail DB2 CSVs from `../game-engine/data/db2/<build>/`, imported by `scripts/db2.py` (tables, build, row counts: `docs/wiki/systems/world-data.md`)
- **Modules**:
  - `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource
  - `crates/server/src/spell_db.rs` — `SpellDatabasePlugin`, `SpellDatabase` resource

| Table | Rows | Source | Purpose |
|-------|------|--------|---------|
| `creature_template` | 29,947 | AzerothCore | NPC definitions (name, level, faction, speeds, AI, flags) |
| `creature` | 149,678 | AzerothCore | Spawn instances (map, zone, x/y/z, respawn time, wander radius) |
| `creature_template_model` | 40,213 | AzerothCore | Display models per creature (DisplayID, scale, probability) |
| `waypoint_data` | 144,657 | AzerothCore | Patrol paths (ordered x/y/z points with delay) |

**Movement types**: 0=idle, 1=random wander, 2=waypoint patrol

This is **not** the persistence layer — redb handles mutable player data. SQLite is read-only static world data.
