# World Data

Static, read-only world data loaded from SQLite at server startup. Two Bevy resources: `WorldData` (creatures, spawns, waypoints) and `SpellDatabase` (spell definitions from Retail DB2).

## Database

File: `data/world.db` (132 MB). Regenerate with `scripts/import-azerothcore.sh`.

## Tables

### Creature Data (AzerothCore SQL Dumps)

Source: `~/Repos/azerothcore/data/sql/base/db_world/`

| Table | Rows | Purpose |
|-------|------|---------|
| `creature_template` | 29,947 | NPC definitions: name, level range, faction, speeds, AI flags, health modifier |
| `creature` | 149,678 | Spawn instances: map, zone, x/y/z position, respawn time, wander radius, movement type |
| `creature_template_model` | 40,213 | Display models per creature (DisplayID, scale, probability) |
| `waypoint_data` | 144,657 | Ordered patrol path points with per-point delay (ms) |

### Spell Data (Retail WoW DB2 CSVs)

Source: `../game-engine/data/` (extracted from CASC archives)

| Table | Rows | Purpose |
|-------|------|---------|
| `spell_name` | 399,649 | Spell ID → name |
| `spell` | 399,649 | Spell descriptions and aura text |
| `spell_misc` | 403,223 | Attributes, cast time index, range index, school, projectile speed |
| `spell_effect` | 607,032 | Per-effect coefficients, targets (multiple effects per spell) |
| `spell_power` | 5,677 | Resource costs (mana, rage, energy, etc.) |
| `spell_cast_times` | 268 | CastingTimeIndex → base cast time (ms) |
| `spell_range` | 222 | RangeIndex → min/max range (yards, hostile/friendly) |
| `spell_cooldowns` | 35,547 | Per-spell cooldowns and GCD recovery times (ms) |
| `spell_duration` | 428 | DurationIndex → duration / max duration (ms) |

## Movement Types

Encoded in `creature.movement_type`:
- `0` — idle (stands still)
- `1` — random wander (within `wander_distance` of spawn)
- `2` — waypoint patrol (follows `waypoint_data` path)

## Modules

- `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource. Loads templates, spawns, waypoints into `HashMap`s keyed by entry/guid.
- `crates/server/src/spell_db.rs` — `SpellDatabasePlugin`, `SpellDatabase` resource.

## Race Metadata

A `chr_races` table holds playable race metadata. Source: Wago DB2 `ChrRaces` export. Seeded by `scripts/seed_chr_races.sql`.

## Sources

- [CLAUDE.md](../../../CLAUDE.md) — World Data section with table row counts
- [AGENTS.md](../../../AGENTS.md) — `chr_races` table and seed script
- [crates/server/src/world_data.rs](../../../crates/server/src/world_data.rs) — loader implementation

## See Also

- [[persistence]] — SQLite is NOT the persistence layer; redb handles mutable player data
- [[creature-ai]] — movement types drive wander/waypoint AI component assignment
- [[combat]] — `spell_misc`/`spell_effect` tables feed combat formula inputs
