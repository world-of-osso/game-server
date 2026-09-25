# World Data

Creature spawn orientation is loaded from `creature.orientation` (WoW heading radians) and replicated as `Rotation.y = orientation + π/2`, matching the network's +Z-forward yaw convention. The client adapts M2 +X-forward geometry below the NPC actor with a −π/2 visual rotation. A SQLite-to-spawn regression checks cardinal and non-cardinal headings; authored NPCs no longer lose facing at import/load time.

`fc46567` also loads `creature_template.name` into `CreatureTemplate` and copies it into the replicated shared `Npc.name` at spawn. A spawn without its authoritative template now fails explicitly instead of receiving default creature data. The SQLite-to-spawn fixture preserves the UTF-8 name `Loup — Écorché`; client delivery/rendering is outside this server proof.

Static, read-only world data loaded from SQLite at server startup. Two Bevy resources: `WorldData` (creatures, spawns, waypoints) and `SpellStore` / `ClassSpells` (Retail DB2 spells and class lists, see [[spell-info]]).

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
| `content_creature_template_model` | 40,186 | `creature_template_model` with Wrath-only displays replaced by TrinityCore Retail models ([[content-import]]) |
| `waypoint_data` | 144,657 | Patrol points keyed by path ID: delay (ms), move_type, orientation |
| `creature_addon` | 35,153 | Per-spawn path_id, mount, bytes1/2, emote, visibility, auras |
| `creature_template_addon` | 11,983 | Same fields per template entry; fallback when a spawn has no row |
| `creature_formations` | 5,829 | leaderGUID/memberGUID, dist, angle (degrees), groupAI, point_1/2 |

Quest, vendor, trainer, loot, gossip and item tables are imported from the same dumps; tables, row counts, the Wrath→Retail ID mapping and the derived Retail-first `content_*` tables (the ones the server should read) are in [[content-import]]. No server code reads them yet.

`scripts/mysql_dump.py` loads every dump table (MySQL string escapes decoded; `--schema world_schema.sql` reloads the dump tables into a populated `world.db` in one transaction without touching DB2 tables).

### Spell/Class Data (Retail WoW DB2 CSVs)

Source: wago.tools exports pinned to Retail `12.1.0.69933` (the local install's `.build.info` version), stored in `../game-engine/data/db2/<build>/<Table>.csv`.

- `scripts/db2.py fetch` downloads every table in `db2.TABLES` (skips files already present; retries 429/5xx with backoff + jitter, honours `Retry-After`).
- `scripts/db2.py import --db data/world.db` replaces only the DB2 tables, in one transaction, so it is safe on a populated `world.db` and idempotent. `import-azerothcore.sh` calls it after rebuilding creature tables.
- DDL lives in `scripts/db2_schema.sql`. Table names are the snake_case DB2 names (`SpellXSpellVisual` → `spell_x_spell_visual`). The import fails, leaving the previous rows, when a CSV header differs from the schema; a new build needs a schema update.
- `db2_build(table_name, build)` records the source build of each table.
- wago.tools has no Retail `SpellIcon` table (icons are FileDataIDs in `spell_misc`).

| Table | Rows (12.1.0.69933) | Purpose |
|-------|------|---------|
| `spell_name` | 414,027 | Spell ID → name |
| `spell` | 414,027 | Spell descriptions and aura text |
| `spell_misc` | 417,635 | Attributes, cast time index, range index, school, projectile speed |
| `spell_effect` | 629,424 | Per-effect coefficients, targets (multiple effects per spell) |
| `spell_power` | 5,715 | Resource costs (mana, rage, energy, etc.) |
| `spell_cast_times` | 272 | CastingTimeIndex → base cast time (ms) |
| `spell_range` | 222 | RangeIndex → min/max range (yards, hostile/friendly) |
| `spell_cooldowns` | 36,334 | Per-spell cooldowns and GCD recovery times (ms) |
| `spell_duration` | 435 | DurationIndex → duration / max duration (ms) |
| `skill_line_ability` | 17,957 | Skill line (class/spec) → spell grants |
| `spell_aura_options` | 32,164 | Stacks, proc chance/charges/PPM per spell |
| `chr_specialization` | 60 | Specs (ID, class, role, mastery) |
| `trait_node` | 11,875 | Talent tree nodes |

Item/quest/taxi DB2 tables (`item`, `item_sparse`, `item_appearance`, `item_modified_appearance`, `quest_v2`, `taxi_nodes`, `taxi_path`, `taxi_path_node`) feed [[content-import]]; taxi tables are read directly.

Other imported tables: spell categories, levels, class options, procs-per-minute (+mods), shapeshift (+forms), interrupts, target restrictions, equipped items, radius, scaling, empower (+stages), `spell_x_spell_visual`, `skill_line`, `chr_classes`, `specialization_spells`, `power_type`, `chr_classes_x_power_types`, the `trait_*` / `skill_line_x_trait_tree` talent tables with their cost/condition link tables, `trait_currency_source` and `spec_set_member`. `spell_info` reads the spell, class-option, skill-line-ability, specialization, power-type and trait tables ([[spell-info]], [[talents]]).

## Event-dependent creature spawns

The importer must preserve `game_event_creature.sql`. No events are currently activated by this server: ordinary spawns remain eligible, positive-only event memberships do not spawn, and negative memberships remain eligible while their event is inactive. A GUID with a negative and a positive membership remains eligible under this inactive-event policy. Source creature rows and coordinates are preserved; no client grounding correction is applied. Event scheduling/activation is not implemented.

The loader requires the membership table and fails explicitly when it is absent. Existing databases need that table populated from the matching AzerothCore dump **before restarting** the corrected server. Do not regenerate an existing populated world database just to add this table: the full import script deletes/recreates `world.db` and can discard separately seeded data.

On September 9, 2026, missing memberships exposed Arena Tournament event31 spawns in normal Northshire: GUID96194 (Arena Organizer),96198 (Fight Promoter),96204 (Paymaster). Their authored elevations were above the loaded terrain. The source membership rows—not visual height guesses—identify why they should be absent. Brother Neals GUID79968 remains at his authored upstairs elevation113.24.

Regression coverage: `startup_tests::world_data_excludes_inactive_tournament_spawns_without_lowering_ordinary_npcs` uses file-backed SQLite and checks selection, unchanged upstairs height, map indexing, and retained source rows. `python scripts/tests/test_import_azerothcore.py` executes the actual importer against isolated fixtures, checking signed memberships and non-destructive missing-input failure. The same file covers DB2 re-import/rollback on fixtures and, when the pinned CSVs are present (`DB2_CSV_ROOT` overrides the location), known Retail rows such as Kick 1766 Effect 68.

## Movement Types

Encoded in `creature.movement_type`:
- `0` — idle (stands still)
- `1` — random wander (within `wander_distance` of spawn)
- `2` — waypoint patrol (follows the addon `path_id` in `waypoint_data`, see [[creature-ai]])

## Modules

- `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource. Loads templates, spawns, waypoints (by path ID), creature addons (by guid / template entry) and follow formations (by member guid) into `HashMap`s.
- `crates/server/src/spell_info/` — `SpellInfoPlugin`, `SpellStore`, `ClassSpells` ([[spell-info]]).

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
- [[content-import]] — quest/vendor/trainer/loot/gossip/item tables and ID mapping
