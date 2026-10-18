# World Data

Creature spawn orientation is loaded from `creature.orientation` (WoW heading radians) and replicated as `Rotation.y = orientation + π/2`, matching the network's +Z-forward yaw convention. The client adapts M2 +X-forward geometry below the NPC actor with a −π/2 visual rotation. A SQLite-to-spawn regression checks cardinal and non-cardinal headings; authored NPCs no longer lose facing at import/load time.

`fc46567` also loads `creature_template.name` into `CreatureTemplate` and copies it into the replicated shared `Npc.name` at spawn. A spawn without its authoritative template now fails explicitly instead of receiving default creature data. The SQLite-to-spawn fixture preserves the UTF-8 name `Loup — Écorché`; client delivery/rendering is outside this server proof.

Static, read-only world data loaded from SQLite at server startup. Two Bevy resources: `WorldData` (creatures, spawns, waypoints) and `SpellStore` / `ClassSpells` (Retail DB2 spells and class lists, see [[spell-info]]).

## Template source selection

Verified2026-10-17: `zephras_content.py::OverlayWriter.insert` records authored overlay provenance in `zephras_row_source`. `world_data::load_authored_template_ids` reads only `table_name='content_creature_template'`, `kind='reconstructed'` and the JSON `row_key.entry`, retaining those IDs in `WorldData.authored_templates`. Ordinary spawning selects their authored fixed stats before considering regional Retail tuning. This is source selection, not a fallback: absence of tuning, level shape and absence of a TDB row never classify a template. Yala Windwatcher249363 has the actual reconstructed marker and retains level5/102HP; Retail source17536CT0 still requires real map/area context. [Scaling contract](../../specs/level-scaling.md).

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
| `content_creature_template_model` | 40,186 | `creature_template_model` with Wrath-only displays replaced by TrinityCore Retail models ([[content-import]]); `world_data.rs` `load_models` reads this table with each display's weight and trigger status; spawns choose one per TrinityCore `ChooseDisplayId` ([retail world content](../../specs/retail-world-content.md)) |
| `content_creature` | 184,436 | Server-read spawns: TrinityCore Retail base-phase spawns on maps 0/1 (guid + 20,000,000), AzerothCore spawns elsewhere ([[content-import]], [retail-world-content](../../specs/retail-world-content.md)) |
| `content_creature_template` | 34,667 | Server-read templates: AzerothCore's plus 4,720 TrinityCore templates for Retail-only entries; `ContentTuningID` + its level range for the 16,003 templates TrinityCore tunes |
| `content_tuning` | 2,929 | Retail ContentTuning DB2, extracted from local CASC (`db2.py extract`) |
| `content_gossip_menu` / `content_gossip_menu_option` / `content_npc_text` | 6,975 / 5,619 / 8,578 | Server-read gossip: TrinityCore rows for the menus Retail-spawned NPCs reach, AzerothCore's for the rest |
| `content_creature_addon` / `content_creature_template_addon` / `content_creature_formations` / `content_game_event_creature` / `content_waypoint_data` | 20,379 / 16,143 / 5,730 / 4,941 / 174,818 | Server-read spawn-keyed rows, following each spawn's source |
| `creature_classlevelstats` | 400 | Base health per (level, unit_class), `basehp0..2` by `creature_template.exp`. An untuned template's spawn health is `ceil(basehp × HealthModifier)` at its `minlevel` (`zones.rs` `resolve_health`, AzerothCore `Creature::SelectLevel`). Tuned templates use ExpectedStat instead ([[level-scaling]]) |
| `expected_stat` / `expected_stat_mod` / `content_tuning_x_expected` | 232 / 335 / 1,072 | Retail creature health and DPS per (level, expansion), with their per-ContentTuning and per-class mods. Local CASC extract (`db2.py extract`). Tuned `content_creature_template` rows add `ContentTuningID`, `HealthScalingExpansion` and `LevelScalingDeltaMin/Max` ([[level-scaling]]) |
| `waypoint_data` | 144,657 | Patrol points keyed by path ID: delay (ms), move_type, orientation |
| `creature_addon` | 35,153 | Per-spawn path_id, mount, bytes1/2, emote, visibility, auras |
| `creature_template_addon` | 11,983 | Same fields per template entry; fallback when a spawn has no row |
| `creature_formations` | 5,829 | leaderGUID/memberGUID, dist, angle (degrees), groupAI, point_1/2 |
| `content_pool_template` / `content_pool_members` / `content_game_event_pool` | 3,993 / 4,832 / 30 | TrinityCore spawn pools of the Retail spawns (493 creature, 1,499 gameobject members, 2,840 child pools); `spawn_pools.rs` spawns `max_limit` members per pool and re-picks on respawn ([retail world content](../../specs/retail-world-content.md)). Added to an existing world.db by `scripts/import-spawn-pools.sh` |
| `content_creature_equip_template` | 13,529 | Virtual items (main hand, off hand, ranged: ItemID + AppearanceModID) per (entry, `creature.equipment_id`): TrinityCore's rows (`tdb_creature_equip_template`, 42,035) for Retail-spawned entries, AzerothCore's (`creature_equip_template`, 10,851) for the rest. Added to an existing world.db by `scripts/import-creature-equipment.sh` |

Quest, vendor, trainer, loot, gossip and item tables are imported from the same dumps; tables, row counts, the Wrath→Retail ID mapping and the derived Retail-first `content_*` tables (the ones the server should read) are in [[content-import]]. No server code reads them yet.

`scripts/mysql_dump.py` loads every dump table (MySQL string escapes decoded; `--schema world_schema.sql` reloads the dump tables into a populated `world.db` in one transaction without touching DB2 tables).

### Native Curve type inputs

The maintained publisher includes `curve(ID, Type, Flags)` rather than guessing interpolation from `curve_point`. [Curve publication](../../db2-active-inputs.md#native-curve-type-publication) owns installed-build provenance, readable-row coverage and explicit encrypted exclusions. This data-input change does not implement area-trigger scale or certify canonical publication.

### Spell/Class Data (Retail WoW DB2 CSVs)

Source: wago.tools exports pinned to Retail `12.1.0.69933` (the local install's `.build.info` version), stored in `../game-engine/data/db2/<build>/<Table>.csv`. `db2.py extract` decodes the `LOCAL_TABLES` (ContentTuning 1962930, QuestMoneyReward 1139047, ExpectedStat 1937326, ExpectedStatMod 1969773, ContentTuningXExpected 2976765) from the local CASC install with `scripts/db2_casc.py`. The decoder handles pallet arrays, non-inline ID lists, copy tables and relationship maps. Floats are written as exact float32 values; the earlier 7-digit rendering lost precision, e.g. ExpectedStat CreatureHealth 29552.18 instead of 29552.177734375. All five tables were checked against wago.tools' 12.1.0.69933 exports (`data/reference/wago-verify/`, fetched 2026-09-26 for verification only) and match on every row at float32 precision (`LocalDb2ExtractTests`). ExpectedStat, ExpectedStatMod and ContentTuningXExpected have layouts newer than the vendored WoWDBDefs, so their column order follows the wago export.

- `scripts/db2.py fetch` downloads every table in `db2.TABLES` (skips files already present; retries 429/5xx with backoff + jitter, honours `Retry-After`).
- `scripts/db2.py import --source-dir "$FOREVER_DB2_SOURCE" --db data/world.db` replaces only the DB2 tables, in one transaction, so it is safe on a populated `world.db` and idempotent. `import-azerothcore.sh` calls it after rebuilding creature tables.
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

- `crates/server/src/world_data.rs` — `WorldDataPlugin`, `WorldData` resource. Loads (from the `content_creature*` / `content_waypoint_data` / `content_game_event_creature` tables) templates, spawns, waypoints (by path ID), creature addons (by guid / template entry) and follow formations (by member guid) into `HashMap`s.
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
