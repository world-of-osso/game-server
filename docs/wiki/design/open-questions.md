# Open Questions

Unresolved design decisions that have architectural implications. Each requires a concrete decision before the affected system can be finalized.

## Tick Rate: Single vs Multi-Rate

**Status**: Unresolved.

Current design: single `FixedUpdate` schedule at 20Hz (50ms). WoW uses ~20Hz for combat, but movement validation benefits from higher rates to reduce input lag and prevent cheating.

Options:
- **Single 20Hz** — simple, consistent, matches AzerothCore's combat cadence
- **Multiple schedules** — e.g., 60Hz for movement validation, 20Hz for combat. Bevy supports multiple `FixedUpdate` schedules at different rates. Adds scheduling complexity and potential system ordering issues.

Decision gate: movement feel testing. If 50ms movement response feels acceptable, stay single-rate.

## World Size: big_space vs Origin-Shifting

**Status**: Unresolved.

WoW maps are ~32km across. At float32 precision, coordinates beyond ~8km accumulate visible jitter (~1mm error at 16km). Two mitigation strategies:

- **`big_space` crate** — grid-cell coordinate system that keeps entities near the origin within each cell. Requires adapting all position math and Lightyear replication to cell-relative coordinates.
- **Origin-shifting** — shift the world origin to stay near the player. Simpler to implement, but requires transforming all entity positions on origin change — expensive with many entities.

Decision gate: actual map scale requirements. If max playable area stays under 8km per zone, f32 is fine and neither approach is needed.

## Instance Management: Separate Worlds vs Isolated System Sets

**Status**: Unresolved.

Dungeons and raids need isolation from the open world. Two models:

- **Separate Bevy worlds** — each instance is an independent `World`. True isolation, no accidental cross-instance queries. Higher overhead per instance (full ECS state), and cross-world communication (e.g., player re-entering world after dungeon) requires serialization.
- **Isolated system sets** — instances run in system sets that filter by an `InstanceId` component. Same world, tagged entities. Cheaper, but requires discipline to avoid cross-instance queries. Mirrors the zone system set model already designed.

The system-sets model is a natural extension of the zone architecture. Separate worlds may be premature until instance counts justify the overhead.

## Hot-Reloading: Scripting Layer vs Dynamic Plugins

**Status**: Unresolved.

Encounter scripts (boss abilities, event sequences) need iteration without a full server restart during development.

- **Lua or Rhai scripting layer** — interpreted scripts loaded at runtime. Rhai is pure Rust, embeds easily, and has Bevy integration examples. Lua (via mlua) has more precedent in WoW emulators (AzerothCore's Eluna). Both add an interpreter boundary and some overhead per call.
- **Dynamic Bevy plugins** — compiled Rust loaded as `.so` files at runtime. Fast (native code), but hot-reloading shared libraries in Rust is fragile (type identity, allocator boundaries, symbol versioning).

The scripting layer is lower risk. The question is Lua vs Rhai — Lua has AzerothCore precedent, Rhai has better Rust integration.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — Open Questions section

## See Also

- [[stack-decisions]] — resolved design decisions
- [[azerothcore-comparison]] — AzerothCore's approach to scripting (Eluna Lua)
- [[crate-structure]] — scripting module placeholder in server/src
