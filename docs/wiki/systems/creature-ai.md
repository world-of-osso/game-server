# Creature AI

Creature behavior is driven by a state machine in `server/src/creature_ai.rs`. Each creature has a `CreatureState` (Idle / Pursuing / Returning) that gates which movement subsystem runs.

## Movement Types

Determined by `creature.movement_type` in SQLite (see [[world-data]]):

| Type | Component | Behavior |
|------|-----------|----------|
| 0 | none | Idle — stands at spawn point |
| 1 | `WanderState` | Random wander within `wander_distance` of spawn |
| 2 | `WaypointState` | Follows the spawn's creature-addon `path_id` in `waypoint_data` |

## Wander System

`WanderState` is a two-phase state machine (`Idle` / `Walking`):

- **Idle phase**: counts down `idle_remaining` (2–5s random); on expiry, picks a random target point using a simple LCG seeded per-creature
- **Walking phase**: moves toward target at `speed_walk * BASE_WALK_SPEED (2.5 m/s)`; on arrival, transitions back to Idle with a new random pause

Wander respects `SpawnPoint.wander_distance` — targets are always within that radius of the spawn.

## Waypoint System

`waypoint_data.id` is a **path ID**, not a creature guid. A MovementType 2 spawn walks `path_id` from its `creature_addon` row, else its `creature_template_addon` row (AzerothCore `Creature::GetCreatureAddon` → `LoadCreaturesAddon` sets `m_path_id`; `WaypointMovementGenerator::LoadPath` reads `GetWaypointPath()`). With no addon path the spawn idles: AzerothCore never derives `guid*10` (`ObjectMgr::LoadCreatureAddons` resets such spawns to idle). Of 6,554 MovementType 2 spawns, 6,543 resolve a path this way.

Per point (`WaypointMovementGenerator::StartMove`/`OnArrived`): walk to the point, pause `delay` **milliseconds**, then start the next leg; the path wraps after the last point. `move_type` 0 walks (`speed_walk * 2.5`), 1 runs (`speed_run * 7.0`); 2/3 (land/takeoff) also use run speed since flight is not modelled. A point's `orientation` is faced (`Rotation`) on arrival only when it has a delay. `waypoint_data.action` (waypoint_scripts) is not run. Waypoint coordinates are WoW space, converted via `wow_to_bevy()`.

## Formations

`creature_formations` members with `GROUP_AI_FLAG_FOLLOW_LEADER` (0x200) get `FormationMember { leader, follow_dist, follow_angle }` when their leader is spawned; groups whose leader has no own row never move (AzerothCore `CreatureGroup::AddMember`). `formation_system` runs after `waypoint_system` (`CreatureGroup::LeaderMoveTo`): each tick the leader moves, a member heads for the leader's current point offset by `follow_dist` at `follow_angle` (degrees in the table) from the leg direction, closing its distance in the leader's proportion so both arrive together. Members hold while the leader pauses or fights. Dead members, and members of a dead leader, stay where they are (`LeaderMoveTo` skips dead members; a dead leader never reaches `StartMove`; `setDeathState` dismisses the group with `FormationReset(true, false)`). AzerothCore mirrors the angle when the leader turns more than 90° from its facing; the server always uses the travel-direction offset. Assist/evade group AI flags are not implemented.

## Death and Respawn

Wander, waypoint and formation systems skip entities with `DeathState`. Every spawn carries a `SpawnPoint` and a `RespawnDelay` of `creature.spawntimesecs` + corpse decay by rank (60 s normal, 300 s elite/rare/rare elite, 600 s world boss; AzerothCore `Creature::setDeathState` `m_respawnDelay + m_corpseDelay`, default `Corpse.Decay.*`). `combat::respawn_system` restores the creature at its spawn when the timer ends; a patroller restarts its path from the first point (`WaypointState::restart`; AzerothCore `Creature::Respawn` → `Motion_Initialize`). A respawned escort, or the escorts of a respawned leader, rejoin their slots on the leader's next leg.

## Chase and Aggro

`CreatureState::Pursuing { target }` overrides wander/waypoint AI. When active, `chase_system` moves the creature toward the target entity each tick at walk speed.

Threat is tracked in the `ThreatTable` component (from `shared/src/threat.rs`). When threat results in a new top-threat target, `CreatureState` transitions to `Pursuing`.

## Leash and Evade

`leash_check_system` runs after chase each tick. If the creature exceeds **40 yards** (2D distance) from its `SpawnPoint`:
1. `CreatureState` → `Returning`
2. `ThreatTable` is reset
3. Health is restored to max (immediate full heal on evade)

`return_to_spawn_system` then walks the creature back to its spawn point at walk speed. On arrival, `CreatureState` → `Idle` and normal AI resumes.

## Creature addon mount and auras

Spawn-time `creature_addon`/`creature_template_addon` fields (`zones.rs`): `mount` → replicated `Mounted { mount_display_id }`; `auras` → self-cast through the aura runtime on the next FixedUpdate (`apply_addon_auras`; spells missing from `SpellStore` are logged). `bytes1` (stand state), `bytes2` (sheath), `emote` (`UNIT_NPC_EMOTESTATE`) and `visibilityDistanceType` are imported but not applied: the protocol has no persistent stand/emote state.

## SpawnPoint Component

Every spawned creature carries a `SpawnPoint`:
- `x, y, z` — original spawn coordinates
- `wander_distance` — radius for random wander
- `speed_walk` — creature's walk speed multiplier (from `creature_template.speed_walk`)

Also used by `combat.rs` to reset position on respawn after death.

## Grid Activation

`grid_activity.rs` follows AzerothCore grid activation (`Map::Update` → `MarkNearbyCellsOf`, `Creature::IsUpdateNeeded`): map 0 is bucketed into 533.3333 yd grids (`SIZE_OF_GRIDS`) and a creature runs its per-tick systems only while one of its home grids is within one grid (3x3) of a player's grid. Home grids are the spawn grid plus every grid the creature's patrol path, or its formation leader's path, crosses. Other creatures carry `GridDormant`; wander, waypoint, formation, chase, leash, return-to-spawn, `update_auras` and `sync_unit_auras` skip them. The active set is recomputed in `FixedPreUpdate` only when a player changes grid or enters/leaves the world, and only creatures in grids that changed are toggled. Unlike AzerothCore, patrollers and formation members are not updated everywhere; covering their path grids keeps them active wherever a player can see them.

Timers: dormant state resumes where it stopped. Aura expiry and periodic tick times are absolute, so on reactivation expired auras end and missed ticks run at once. `respawn_system` is not gated (it only visits dead creatures): a dormant creature respawns at its spawn point on its normal timer.

With all 24,521 map 0 creatures and no players, an idle dev server uses about 8% of one core (81% without grid activation).

## Coordinate System

Creature data is stored in WoW coordinate space. The AI operates in Bevy space (X-right, Y-up, Z-back). The `wow_to_bevy()` function in `terrain_height.rs` handles the conversion at waypoint load time.

## Sources

- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — CreatureAiPlugin, all AI systems
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — AI component assignment on creature spawn
- [CLAUDE.md](../../../CLAUDE.md) — movement type values (0/1/2)

## See Also

- [[world-data]] — `creature`, `creature_template`, `waypoint_data`, `creature_addon`, `creature_formations` tables
- [[combat]] — `CombatTarget` drives the Pursuing state; leash evade restores HP
- [[tick-model]] — AI systems run at step 2 of the 20Hz pipeline
