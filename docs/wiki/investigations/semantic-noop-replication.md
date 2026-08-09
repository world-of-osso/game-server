# Semantic No-op Replication Updates

The server previously dirtied replicated components when a system assigned a value equal to the existing value. Bevy change ticks treat mutable assignment as a change; Lightyear 0.26.4's server sender checks those ticks, not component equality. The client later suppresses equal ECS replacement, but only after serialization, batching, transport, delivery, deserialization, and comparison. The fixes remove the confirmed recurring no-op writers. No CPU, bandwidth, or network improvement is claimed because direct serialized-update proof and live bandwidth measurement remain blocked/unmeasured.

## Root Cause

The server's replication sender runs on a 50 ms cadence and uses `SinceLastAck`. Its selection path is effectively:

```text
mutable equal assignment
→ Bevy changed tick advances
→ Lightyear sender selects the component
→ component serializes and enters an update batch
→ transport sends it
→ client deserializes and compares
→ equal client value suppresses ECS replacement
```

The sender does not perform a value-equality check before serialization. `SinceLastAck` can keep a dirty component eligible until the corresponding update is acknowledged. The client equality check therefore prevents downstream client `Changed<T>` work, but does not undo server-side change detection or wire cost.

The locked dependency is Lightyear `0.26.4` (`Cargo.lock`). Server replication uses `ReplicationSender` with a 50 ms interval and `SendUpdatesMode::SinceLastAck` (`crates/server/src/networking.rs`).

## RED Evidence

The pre-fix behavioral batch recorded seven change-tick failures:

1. Wander movement assigned an unchanged `Position` at a zero-radius/current target.
2. Waypoint movement assigned an unchanged `Position` at its target.
3. Chasing assigned target coordinates after the NPC had already arrived.
4. Returning assigned spawn coordinates after the NPC had already arrived.
5. Player movement with nonzero direction and zero effective speed assigned an unchanged `Position`.
6. Auto-AFK synchronization assigned an unchanged `PresenceStatus`.
7. Active-combat maintenance reinserted unchanged `CombatStatus(true)`.

The RED assertions checked both observable value preservation and Bevy `last_changed()` ticks. Existing tests also covered real movement, presence transitions, combat transitions, and the previously fixed player movement/gravity boundaries.

## Fixes

- `86d73f5` — guards NPC `Position` writes in wander, waypoint, chase, and return paths. Tests cover unchanged target/arrival cases and preserve actual movement.
- `74edc39` — avoids unchanged player `Position` writes when effective movement produces no displacement. Real movement remains applied.
- `f33c95a` — assigns `PresenceStatus` only when the desired Online/Afk state differs. Real status transitions remain observable.
- `bd31161` — avoids reinserting unchanged active `CombatStatus(true)`. Entering and expiring combat still change the replicated state.

Together with the earlier `2927382` movement-state guards and `ae81c65` gravity-state guards, these changes target confirmed semantic no-op writers without changing protocol semantics or legitimate gameplay transitions.

## Coverage Status

| Boundary | Status |
|---|---|
| Bevy writer change ticks | Covered by the seven RED/GREEN behavioral cases and existing movement/gravity tests |
| Lightyear server serialized-update boundary | Blocked: no direct integration assertion yet distinguishes zero emitted updates from client equality suppression |
| Live component update frequency and bytes | Unmeasured: no bounded per-component serialized-update or bandwidth capture was completed |
| CPU/network improvement | Not claimed |

The remaining Health, Mana, Gold, equipment, guild, admin, respawn, and other event-driven writer candidates were not changed by this fix series; they require separate RED proof before remediation.

## Sources

- [server networking](../../../crates/server/src/networking.rs) — replication sender configuration and 50 ms cadence
- [NPC AI](../../../crates/server/src/creature_ai.rs) — movement writer paths
- [player networking](../../../crates/server/src/networking.rs) — player movement writer
- [presence system](../../../crates/server/src/friends.rs) — Online/Afk synchronization
- [combat system](../../../crates/server/src/combat.rs) — active combat state maintenance
- [dependency versions](../../../Cargo.lock) — Lightyear `0.26.4` lockfile resolution
- `game-server` commits `86d73f5`, `74edc39`, `f33c95a`, `bd31161` — no-op guards and regression tests
- Local Lightyear `0.26.4` sender/receiver source — tick-only sender selection and client equality suppression

## See Also

- [[networking]] — replication configuration and transport model
- [[tick-model]] — 20 Hz server schedule and replication stage
- [[creature-ai]] — NPC movement states and writer paths
