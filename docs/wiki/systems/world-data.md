# World Data

Creature spawn orientation is loaded from `creature.orientation` (WoW heading radians) and replicated as `Rotation.y = orientation + π/2`, matching the network's +Z-forward yaw convention. The client adapts M2 +X-forward geometry below the NPC actor with a −π/2 visual rotation. A SQLite-to-spawn regression checks cardinal and non-cardinal headings; authored NPCs no longer lose facing at import/load time.

`fc46567` also loads `creature_template.name` into `CreatureTemplate` and copies it into the replicated shared `Npc.name` at spawn. A spawn without its authoritative template now fails explicitly instead of receiving default creature data. The SQLite-to-spawn fixture preserves the UTF-8 name `Loup — Écorché`; client delivery/rendering is outside this server proof.

Static, read-only world data loaded from SQLite at server startup. Two Bevy resources: `WorldData` (creatures, spawns, waypoints) and `SpellDatabase` (spell definitions from Retail DB2).

## Database

File: `data/world.db` (132 MB). Regenerate with `scripts/import-azerothcore.sh`.

## Tables

### Creature Data (AzerothCore SQL Dumps)

Source: `~/Repos/azerothcore/data/sql/base/db_world/`

| Table | Rows | Purpose |
|-------|------|---------|
| `creature_template` | 29,947 | NPC definitions: name, level range, faction, speeds, AI flags, health modifier |
| `creature` | 149,678 | Source spawn instances, including event-only rows; runtime selection applies event membership |
| `game_event_creature` | Source-dependent | Signed event membership keyed by spawn GUID and event ID |
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

## Event-dependent creature spawns

The importer must preserve `game_event_creature.sql`. No events are currently activated by this server: ordinary spawns remain eligible, positive-only event memberships do not spawn, and negative memberships remain eligible while their event is inactive. A GUID with a negative and a positive membership remains eligible under this inactive-event policy. Source creature rows and coordinates are preserved; no client grounding correction is applied. Event scheduling/activation is not implemented.

The loader requires the membership table and fails explicitly when it is absent. Existing databases need that table populated from the matching AzerothCore dump **before restarting** the corrected server. Do not regenerate an existing populated world database just to add this table: the full import script deletes/recreates `world.db` and can discard separately seeded data.

On September 9, 2026, missing memberships exposed Arena Tournament event31 spawns in normal Northshire: GUID96194 (Arena Organizer),96198 (Fight Promoter),96204 (Paymaster). Their authored elevations were above the loaded terrain. The source membership rows—not visual height guesses—identify why they should be absent. Brother Neals GUID79968 remains at his authored upstairs elevation113.24.

Regression coverage: `startup_tests::world_data_excludes_inactive_tournament_spawns_without_lowering_ordinary_npcs` uses file-backed SQLite and checks selection, unchanged upstairs height, map indexing, and retained source rows. `python scripts/tests/test_import_azerothcore.py` executes the actual importer against isolated fixtures, checking signed memberships and non-destructive missing-input failure.

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
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — authoritative template lookup and spawned `Npc.name`
- [shared `Npc` component](../../../../shared-protocol/src/components.rs) — replicated template ID and UTF-8 name

## See Also

- [[persistence]] — SQLite is NOT the persistence layer; redb handles mutable player data
- [[creature-ai]] — movement types drive wander/waypoint AI component assignment
- [[combat]] — `spell_misc`/`spell_effect` tables feed combat formula inputs
