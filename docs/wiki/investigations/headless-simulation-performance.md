# Headless simulation performance

Profile-driven speedups for seeded headless keystones and follower DPS parity, made after keystone runs became deterministic. Every change kept the records byte-identical: the same seed and frozen world DB produce the same keystone JSON and parity report, except wall time.

## Measured result (2026-11-02)

Base: origin/master `4ca19e9e7d`. Perf: branch `sim-perf-v2` (`a32632f63c`). Frozen world DB, debug test profile, base and perf runs interleaved on a shared host.

| Run | Base | Perf | Speedup | Output |
|---|---|---|---|---|
| Keystone Azure 2515 +2, party 581,257,63,262,70, seed 70448, process CPU | 285 s, 291 s | 162 s, 173 s | 1.72× | byte-identical (completed, 424/410, 555000 ms) |
| Follower 63 tick loop | 22.6 s, 25.6 s | 6.1 s, 4.7 s | about 4.4× | identical except `wall_seconds` |
| Follower 262 tick loop | 58.5 s, 42.9 s | 7.7 s, 5.6 s | about 7.6× | identical except `wall_seconds` |

## Hotspots and fixes

- **Spell override resolution.** `caster_knows_spell` and `resolve_actor_action` resolved every known spell, each time scanning all aura effects and allocating a `HashSet`. A caster's action-bar override effects are now collected once, in aura order, and reused (`ActionOverrides`, `Aliases` in `spell_cast/knowledge.rs`).
- **Navigation startup.** `Worker::new` read and blake3-hashed every baked map's tiles (13.7 GB over 32 maps) to verify portal graphs, about 11% of a keystone run. `Worker::headless` reads a map's hierarchy on its first hierarchy route; production `Worker::new` still preloads.
- **Script hook dispatch.** Each of about 170 dispatch functions scanned all ~1276 registered hooks for one variant. `SpellScriptRegistry` now holds per-kind lists in module order, built once per static module list.
- **Per-call `SystemState`.** `move_follower` built a `SystemState` (several queries over every archetype) per unit per tick. The simulator movement system keeps one in a `Local<FollowerMoves>`.
- Earlier: `SpellStore` id index, `KnownSpells` membership set, hashed walking-route tables.

## Gotchas

- **Fresh and cached Bevy queries iterate in different orders.** Bevy 0.19 `QueryState::new` finds archetypes through `component_index` (a hash map), so a fresh `world.query()` iterates in that order. A cached `QueryState` adds archetypes as they are created. Caching a query that is iterated changed movement order and the keystone result (425 vs 424 forces). Cache only queries used for lookups by entity.
- **New resources shift entity IDs.** Bevy 0.19 resources are entities. Keystone records contain about 20,000 entity IDs, so a new resource created at runtime breaks byte comparison. Keep caches in system state (`Local`, exclusive-system params).
- **`replicon FilterBit` in profiles is not replicon work.** Identical-code folding names Bevy's `TypeId → ComponentId` `IndexMap` lookup (`world.get`, `world.resource`) after a replicon symbol. Frame-pointer stacks show the real callers.
- Profile with `RUSTFLAGS="-C force-frame-pointers=yes"` in a separate target directory; DWARF unwinding of the test binary gave no Rust frames.

## Sources

- [overrides.rs](../../../crates/server-combat/src/auras/overrides.rs), [knowledge.rs](../../../crates/server-combat/src/spell_cast/knowledge.rs), [script_registry.rs](../../../crates/server-combat/src/spell_cast/script_registry.rs), [worker.rs](../../../crates/navigation/src/worker.rs), [movement.rs](../../../crates/server-simulation/src/dungeon_simulator/movement.rs)
- Measurements: game-engine `data/diagnostics/sim-perf-2026-11-01/` (`v2-*`, `fv2-*`, `prof3/`)

## See Also

- [[dungeon-simulator]] — the keystone runner measured here
- [[spell-script-crate]] — the registry whose dispatch is indexed
- [[tick-loop-audit]] — server-side recurring-schedule costs
