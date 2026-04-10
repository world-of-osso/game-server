# Creature AI

Creature behavior is driven by a state machine in `server/src/creature_ai.rs`. Each creature has a `CreatureState` (Idle / Pursuing / Returning) that gates which movement subsystem runs.

## Movement Types

Determined by `creature.movement_type` in SQLite (see [[world-data]]):

| Type | Component | Behavior |
|------|-----------|----------|
| 0 | none | Idle — stands at spawn point |
| 1 | `WanderState` | Random wander within `wander_distance` of spawn |
| 2 | `WaypointState` | Follows ordered `waypoint_data` patrol path |

## Wander System

`WanderState` is a two-phase state machine (`Idle` / `Walking`):

- **Idle phase**: counts down `idle_remaining` (2–5s random); on expiry, picks a random target point using a simple LCG seeded per-creature
- **Walking phase**: moves toward target at `speed_walk * BASE_WALK_SPEED (2.5 m/s)`; on arrival, transitions back to Idle with a new random pause

Wander respects `SpawnPoint.wander_distance` — targets are always within that radius of the spawn.

## Waypoint System

`WaypointState` follows `WaypointPath` loaded from `WorldData.waypoints`. Points loop (`current_index % path_len`). Each point has an optional `delay` (seconds); the creature pauses before advancing to the next. Waypoint coordinates are stored in WoW space and converted to Bevy space via `wow_to_bevy()`.

## Chase and Aggro

`CreatureState::Pursuing { target }` overrides wander/waypoint AI. When active, `chase_system` moves the creature toward the target entity each tick at walk speed.

Threat is tracked in the `ThreatTable` component (from `shared/src/threat.rs`). When threat results in a new top-threat target, `CreatureState` transitions to `Pursuing`.

## Leash and Evade

`leash_check_system` runs after chase each tick. If the creature exceeds **40 yards** (2D distance) from its `SpawnPoint`:
1. `CreatureState` → `Returning`
2. `ThreatTable` is reset
3. Health is restored to max (immediate full heal on evade)

`return_to_spawn_system` then walks the creature back to its spawn point at walk speed. On arrival, `CreatureState` → `Idle` and normal AI resumes.

## SpawnPoint Component

All creatures carry a `SpawnPoint`:
- `x, y, z` — original spawn coordinates
- `wander_distance` — radius for random wander
- `speed_walk` — creature's walk speed multiplier (from `creature_template.speed_walk`)

Also used by `combat.rs` to reset position on respawn after death.

## Coordinate System

Creature data is stored in WoW coordinate space. The AI operates in Bevy space (X-right, Y-up, Z-back). The `wow_to_bevy()` function in `terrain_height.rs` handles the conversion at waypoint load time.

## Sources

- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — CreatureAiPlugin, all AI systems
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — AI component assignment on creature spawn
- [CLAUDE.md](../../../CLAUDE.md) — movement type values (0/1/2)

## See Also

- [[world-data]] — `creature`, `creature_template`, `waypoint_data` tables
- [[combat]] — `CombatTarget` drives the Pursuing state; leash evade restores HP
- [[tick-model]] — AI systems run at step 2 of the 20Hz pipeline
