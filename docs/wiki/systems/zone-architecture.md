# Zone Architecture

Proposed architecture for partitioning future zone and instance simulation inside the server tick. Current code does not implement per-zone system sets or runtime zone transfers; it spawns every map 0 (Eastern Kingdoms) creature during startup. Tick ordering is defined separately in [[tick-loop-design]].

**Status:** Proposed; not implemented.

## Content

## Current Implementation

`ZonePlugin` registers one `PostStartup` system, `spawn_eastern_kingdoms_creatures`. It reads `WorldData`, spawns every map 0 creature (map 1 stays unspawned: `Position` has no map, so interest would overlap the continents), assigns numeric `Zone` data and AI components, and adds Lightyear replication markers.

Current source has no:

- `ZoneSet` system-set type
- Per-zone fixed schedules
- Cross-zone transfer event pipeline
- Zone despawn/respawn handoff
- Dedicated thread or Bevy `World` per zone

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

Dungeons and raids remain unresolved. Two candidate models are tracked in [[open-questions]]:

- One Bevy `World` with `InstanceId`/`Zone` filtering
- Separate Bevy `World`s per instance

The base tick-loop design does not choose between them. Both must preserve a clear handoff to the global transport, persistence, and replication boundaries.

## Comparison with AzerothCore

| Aspect | AzerothCore | Proposed Bevy design |
|---|---|---|
| Parallelism | MapUpdater thread pool | ECS scheduler across zone-filtered systems |
| Cross-zone sync | Map queues and locks | Explicit transfer data applied at a fixed phase boundary |
| Zone definition | Map object | Numeric zone/instance data plus indexes |
| Tick ownership | Per-map update under server loop | One global fixed timestep unless a later design changes it |

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — original proposed zone model and AzerothCore comparison
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — current `PostStartup` map 0 creature spawning
- [crates/server/src/world_data.rs](../../../crates/server/src/world_data.rs) — numeric zone and spawn data loaded from SQLite

## See Also

- [[tick-loop-design]] — proposed global fixed phases and ordering invariants
- [[tick-model]] — observed current schedule behavior
- [[creature-ai]] — fixed-step creature movement systems
- [[networking]] — interest and replication boundaries
- [[open-questions]] — unresolved instance isolation model
