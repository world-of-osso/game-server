# ECS Storage

Bevy's ECS functions as the server's in-memory database. All live game state lives in archetype tables during runtime; disk (redb) is only a periodic checkpoint.

## How Archetypes Store Data

When an entity is spawned with a component set, Bevy assigns it to an **archetype** — a table where every entity sharing that exact component composition is stored. Each component type occupies a contiguous array:

```
Archetype: (Position, Health, Mana, Player)
  Position[]: [pos_0, pos_1, ...]   ← contiguous in memory
  Health[]:   [hp_0,  hp_1,  ...]
  Mana[]:     [mp_0,  mp_1,  ...]
```

This is a columnar layout identical to analytical databases (DuckDB, ClickHouse). The difference: Bevy has zero query overhead — no SQL parsing, no query planner, no index lookup.

## Queries Are Direct Memory Access

A Bevy query compiles to a loop over a `&mut [T]` slice — a contiguous memory region. No hash lookups, no pointer chasing, no serialization boundary. Iterating 10,000 entities with `(Position, Health)` touches only those two arrays, maximizing CPU cache line utilization.

Archetype filtering is O(1): "give me all entities with `Position` and `Health` but not `Dead`" resolves to a set intersection of archetype indices, then a direct array scan with no false positives.

## Change Detection

Bevy tracks component mutations per-entity automatically using bitflags. The `Changed<T>` filter queries only entities whose component changed since last tick:

```rust
fn log_damage(query: Query<(&Player, &Health), Changed<Health>>) { ... }
```

This powers persistence: `Changed<Health>.or(Changed<Position>).or(Changed<Inventory>)` on the save query means only actually-mutated entities touch disk. No dirty-flag boilerplate; the ECS maintains it at zero cost when nothing changes.

## Comparison with SpacetimeDB

| Aspect | Bevy ECS | SpacetimeDB |
|--------|----------|-------------|
| Layout | Columnar (archetype tables) | Row-based (64 KiB pages) |
| Query | Direct array iteration | Scan pages + BSATN decode |
| Change detection | Built-in bitflag per component | WAL diff after commit |
| Threading | Parallel system scheduling | Single writer per database |

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Bevy ECS as In-Memory Storage" section

## See Also

- [[persistence]] — redb as the durable checkpoint behind the ECS
- [[tick-model]] — when queries run within the 20Hz FixedUpdate loop
