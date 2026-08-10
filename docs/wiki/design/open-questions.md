# Open Questions

Unresolved design decisions that have architectural implications. Each requires a concrete decision before the affected system can be finalized.

## Content

## Tick Rate: Single vs Multi-Rate

**Status**: Draft decision pending review.

Current runtime uses one 50 ms `FixedUpdate` timestep configured through Lightyear. [[tick-loop-design]] recommends retaining that single 20 Hz authoritative simulation while adding explicit phase ordering.

Options:
- **Single 20Hz** — current runtime and recommended baseline; simple ordering and one authoritative time domain
- **Multiple rates** — custom higher-frequency movement work plus 20 Hz combat; adds another time boundary, cross-rate ordering, and more catch-up behavior

Decision gate: measured movement feel or validation evidence that 50 ms simulation is insufficient. Keep the single-rate design until that evidence exists.

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

- [[tick-loop-design]] — draft decision and migration plan for the server loop
- [[stack-decisions]] — resolved design decisions
- [[azerothcore-comparison]] — AzerothCore's approach to scripting (Eluna Lua)
- [[crate-structure]] — scripting module placeholder in server/src
