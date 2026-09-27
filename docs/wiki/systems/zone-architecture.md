# Zone Architecture

Maps and dungeon copies share one Bevy `World`, partitioned by the `InMap { map_id, instance_id }` component ([instances spec](../../specs/instances.md)); per-zone system sets remain a proposal. Tick ordering is defined separately in [[tick-loop-design]].

**Status:** Proposed; not implemented.

## Content

## Current Implementation

`maps` (crates/server/src/maps/) owns the map model:

- `InMap` on every positioned entity. `ZonePlugin::spawn_continent_creatures` and `game_objects::spawn_continent_game_objects` spawn maps 0 and 1 at `PostStartup` (`NPC_APPEARANCE_MAPS`); `zones::spawn_map_creatures` / `game_objects::spawn_map_game_objects` spawn a dungeon copy when it is created (creatures only for maps with client NPC appearance data).
- `MapStore` (Map, MapDifficulty, Difficulty, DungeonEncounter, AreaTrigger, AreaTriggerActionSet DB2; TDB areatrigger_teleport, world_safe_locs, graveyard_zone, instance_template) loads in `startup_load_world_data`.
- `InstanceManager` keeps dungeon copies (difficulty, recent instance per group/character with its instance owner, owning group, lock owner and expiry, player set, unload time) and owns `InstanceLocks` (the `InstanceLockMgr` port in `locks.rs`: permanent and temporary locks per character and (MapID, LockID), shared state by instance id). Every lock change sets `locks_dirty`; `save_instance_locks` (Update) writes them to game.redb `instance_locks`, `load_instance_locks` (PostStartup) reads them back. Lock times come from `ServerClock` (Unix seconds, shiftable by `clock-advance`), unload timers from `Time`. Observers on `Insert`/`Discard` of a player's `InMap` add and remove players.
- `difficulty.rs`: the player's `DungeonDifficulty` component (saved in `character_dungeon_difficulty`), `SetDungeonDifficulty` / `RequestRaidInfo` / `ResetInstances` / `SetSavedInstanceExtend` handlers, `WorldServerInfo` on every map entry, lock expiry. Group difficulty lives in `GroupRegistry` (`GroupEffect::DungeonDifficulty` → `DifficultyChanges`). Boss `DONE` → `InstanceManager::update_instance_lock` from `scripted_ai::instance_script`.
- `transfer::teleport_to` is the one teleport entry point: same map moves the player; another map removes `InMap`, inserts `MapTransfer` and sends `NewWorld`; `receive_world_port_acks` (Update) resolves the copy and re-inserts `InMap`.
- `area_triggers::detect_area_trigger_teleports` (FixedUpdate after gravity) fires teleport triggers on entering.
- Interest buckets are keyed by `(InMap, x, z)`; a player without a map (in transfer) sees only itself. `GroundMap` holds one `MapGround` per map, opened lazily from the map's WDT.

Current source has no per-zone schedules and no dedicated `World` per instance.

## Proposed Zone Partitioning

Zone work should remain inside the global fixed-step phase contract rather than creating a second game clock. Zone-filtered systems may run within the Movement, Resolution, and Lifecycle phases:

```text
Fixed phase
  ├─ zone A systems
  ├─ zone B systems
  └─ zone C systems
```

Bevy may execute independent systems in parallel when their ECS access permits it. A zone label alone does not guarantee parallelism; queries and resources must avoid unnecessary shared mutable access.

Entities use the numeric `Zone` component as data. Zone-specific logic should query that component or maintain a zone index; it should not model each zone as a different component type.

## Proposed Transfers

A future transfer boundary should be explicit and tick-safe:

1. Detect the destination zone after authoritative movement.
2. Record a transfer request during the current fixed step.
3. Apply the `Zone` change during Lifecycle before interest reconciliation.
4. Recompute visibility and replication targets from the new zone state.
5. Persist the new zone with the next applicable save boundary.

A same-world transfer should not despawn and recreate the authoritative ECS entity unless a later instance-isolation decision requires that behavior. Lightyear visibility may still cause client-side removal and insertion as relevance changes.

## Instance Isolation

Chosen (dungeons phase 1): one Bevy `World` with `InMap` filtering, matching TrinityCore's map/instance-id pair without a second world or clock. Every cross-entity rule that must not reach another copy reads `InMap` (interest, aggro, spell targets, grid activation, chat range, group rewards). Separate `World`s per instance were not needed: the global transport, persistence and replication boundaries stay single.

## Comparison with AzerothCore

| Aspect | AzerothCore | Proposed Bevy design |
|---|---|---|
| Parallelism | MapUpdater thread pool | ECS scheduler across zone-filtered systems |
| Cross-zone sync | Map queues and locks | Explicit transfer data applied at a fixed phase boundary |
| Zone definition | Map object | Numeric zone/instance data plus indexes |
| Tick ownership | Per-map update under server loop | One global fixed timestep unless a later design changes it |

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — original proposed zone model and AzerothCore comparison
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — continent and dungeon-copy creature spawning
- [crates/server/src/maps/](../../../crates/server/src/maps/) — `InMap`, map store, instances, transfers, area triggers
- [crates/server/src/world_data.rs](../../../crates/server/src/world_data.rs) — numeric zone and spawn data loaded from SQLite

## See Also

- [[tick-loop-design]] — proposed global fixed phases and ordering invariants
- [[tick-model]] — observed current schedule behavior
- [[creature-ai]] — fixed-step creature movement systems
- [[networking]] — interest and replication boundaries
- [[open-questions]] — unresolved instance isolation model
