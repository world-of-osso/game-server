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
  - Creature data: AzerothCore SQL dumps from `~/Repos/azerothcore/data/sql/base/db_world/`
  - Spell data: Retail WoW DB2 CSVs from `../game-engine/data/` (extracted from CASC)
- **Modules**:
  - `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource
  - `crates/server/src/spell_db.rs` — `SpellDatabasePlugin`, `SpellDatabase` resource

| Table | Rows | Source | Purpose |
|-------|------|--------|---------|
| `creature_template` | 29,947 | AzerothCore | NPC definitions (name, level, faction, speeds, AI, flags) |
| `creature` | 149,678 | AzerothCore | Spawn instances (map, zone, x/y/z, respawn time, wander radius) |
| `creature_template_model` | 40,213 | AzerothCore | Display models per creature (DisplayID, scale, probability) |
| `waypoint_data` | 144,657 | AzerothCore | Patrol paths (ordered x/y/z points with delay) |
| `spell_name` | 399,649 | Retail DB2 | Spell ID → name |
| `spell` | 399,649 | Retail DB2 | Spell descriptions and aura text |
| `spell_misc` | 403,223 | Retail DB2 | Attributes, cast time, range, school, speed (per difficulty) |
| `spell_effect` | 607,032 | Retail DB2 | Effects with coefficients, targets (multiple per spell) |
| `spell_power` | 5,677 | Retail DB2 | Resource costs (mana, rage, energy, etc.) |
| `spell_cast_times` | 268 | Retail DB2 | CastingTimeIndex → base cast time (ms) |
| `spell_range` | 222 | Retail DB2 | RangeIndex → min/max range (yards, hostile/friendly) |
| `spell_cooldowns` | 35,547 | Retail DB2 | Per-spell cooldowns and GCD recovery times (ms) |
| `spell_duration` | 428 | Retail DB2 | DurationIndex → duration/max duration (ms) |

**Movement types**: 0=idle, 1=random wander, 2=waypoint patrol

This is **not** the persistence layer — redb handles mutable player data. SQLite is read-only static world data.
