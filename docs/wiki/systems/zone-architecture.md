# Zone Architecture

Zones are implemented as Bevy system sets that can execute in parallel. Each zone is an independent unit of game logic; cross-zone interactions go through events at tick boundaries.

## Zone System Sets

Each zone is a named system set in the `FixedUpdate` schedule:

```rust
app.configure_sets(
    FixedUpdate,
    (
        ZoneSet::EasternKingdoms,
        ZoneSet::Kalimdor,
        ZoneSet::Outland,
    ).chain() // or .ambiguous_with() for parallel execution
);
```

Parallel execution (`.ambiguous_with()`) is used for zones with no shared entities. Chaining (`.chain()`) is used when zones need to observe each other's changes in the same tick.

## Entity Zone Tagging

Entities are tagged with a `Zone` component. Zone-specific systems filter using `With<Zone<X>>`:

```rust
fn update_combat(
    query: Query<(&mut Health, &CombatState), With<Zone<Stormwind>>>
) { ... }
```

This means a combat system for Stormwind never touches Elwynn data — no lock contention, no false sharing.

## Zone Transfers

When an entity crosses a zone boundary:
1. Entity **despawns** from the source zone
2. A **transfer event** is sent
3. Entity **respawns** in the target zone next tick

Lightyear handles the client side transparently: entity despawn + spawn with the same data looks like a seamless zone transition to the client.

## Current Implementation

`crates/server/src/zones.rs` implements `ZonePlugin`, which spawns creature entities for the Elwynn Forest region on `PostStartup`. Creatures are placed within Elwynn spatial bounds and assigned AI components based on their `movement_type` from `WorldData`.

## Comparison with AzerothCore

AzerothCore uses a `MapUpdater` thread pool where each map runs on a dedicated thread. The Bevy approach uses the ECS scheduler directly — zones become system set labels rather than OS threads, letting Bevy decide optimal parallelism based on data dependencies.

| Aspect | AzerothCore | This Design |
|--------|-------------|-------------|
| Parallelism | Thread per map | Bevy parallel scheduler |
| Cross-zone sync | Mutex + message queues | Events at tick boundary |
| Zone definition | C++ Map class | System set label + Zone component |

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Zone Architecture" section and AzerothCore comparison
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — `ZonePlugin`, `spawn_elwynn_creatures`

## See Also

- [[tick-model]] — parallel zone sets run within the tick pipeline
- [[creature-ai]] — creatures are assigned AI components during zone spawn
- [[networking]] — zone transitions are transparent to clients
