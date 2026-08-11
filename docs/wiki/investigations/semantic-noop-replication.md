# Semantic No-op Replication Updates

The server previously dirtied replicated components when a system assigned a value equal to the existing value. Bevy 0.19 change ticks treat mutable assignment as a change; Lightyear 0.28.0's server sender delegates send traversal to vendored `bevy_replicon` 0.41.1 and still does not value-filter equal mutable assignments before serialization. The client later suppresses equal ECS replacement, but only after serialization, batching, transport, delivery, deserialization, and comparison. The fixes remove the confirmed recurring no-op writers. No CPU, bandwidth, or network improvement is claimed because direct serialized-update proof and live bandwidth measurement remain blocked/unmeasured.

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

The locked dependency is Lightyear `0.28.0` with vendored `bevy_replicon` `0.41.1` (`Cargo.lock`). Server replication uses `ReplicationSender` with `ReplicationMetadata` configured for a 50 ms interval and `SendUpdatesMode::SinceLastAck` (`crates/server/src/main.rs`, `crates/server/src/networking.rs`).

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

## Second Proof Wave

The second proof wave added five RED cases across four writer categories and then committed the corresponding minimal guards:

1. `1c139e9` — equal derived-stat recalculation dirtied replicated `Health`/`Mana`; the RED test `update_derived_stats_does_not_dirty_equal_health_or_mana` now proves equal results preserve both ticks.
2. `e13e09e` — respawning at the existing spawn position with full health dirtied replicated `Position`/`Health`; `respawn_at_spawn_with_full_health_does_not_dirty_replicated_state` covers the unchanged boundary while preserving real respawn changes.
3. `057dd17` — equal equipment-appearance set and clear requests dirtied `EquipmentAppearance`; `equal_equipment_appearance_request_does_not_dirty_live_component` covers repeated set, and `f84ee17` adds the independently observed RED/GREEN clear-path regression.
4. `41feebb` — accepting the same guild membership value dirtied `GuildMembership`; `guild_accept_preserves_tick_for_repeated_membership_value` covers the repeated accept.

`945eb0f` covered unchanged gold synchronization and ruled it out as a remaining writer: the existing synchronization already preserves the replicated `Gold` change tick when the value is equal.

## Fixes

First-wave fixes remain preserved:

- `86d73f5` — guards NPC `Position` writes in wander, waypoint, chase, and return paths.
- `74edc39` — avoids unchanged player `Position` writes when effective movement produces no displacement.
- `f33c95a` — assigns `PresenceStatus` only when the desired Online/Afk state differs.
- `bd31161` — avoids reinserting unchanged active `CombatStatus(true)`.
- `2927382` — guards repeated player `Rotation`/`MovementSpeed` writes.
- `ae81c65` — guards unchanged gravity-derived `Position`, `VerticalVelocity`, `FallTracker`, and `Health` state.

Second-wave fixes are listed above. They preserve real movement, combat, presence, stat, respawn, equipment, guild, and gold transitions without changing protocol semantics.

## Subsequent NPC Death Transition (`c8762ed`)

Commit `c8762ed` removed the 20 Hz all-living-NPC health scan. NPC death is now handled directly at the sole authoritative NPC damage boundary, `try_swing`, called by `auto_attack_system`: after damage is clamped, a lethal NPC result inserts `DeathState`, removes `Replicate`, and emits the existing `Death` `CombatEvent` alongside the melee-damage event.

Coverage distinguishes the transition boundaries: nonlethal damage leaves the NPC alive and replicated; lethal damage preserves death-state, replication-removal, and event behavior; and a zero-health NPC without an authoritative damage transition is intentionally not discovered by a global poll. Respawn semantics remain unchanged: `respawn_system` restores health and spawn position, re-adds replication/visibility, removes `LastDamager`, and emits `Respawn` over the existing combat-event path.

Future NPC damage writers must route through the same authoritative death transition. Adding another health scan would reintroduce the removed polling model and could diverge death-state, replication, or event behavior.

## Coverage Status

| Boundary / writer | Status |
|---|---|
| NPC `Position` wander/waypoint/chase/return | Confirmed no-op; fixed in `86d73f5` |
| Player `Position` zero-displacement input | Confirmed no-op; fixed in `74edc39` |
| Active `CombatStatus(true)` maintenance | Confirmed no-op; fixed in `bd31161` |
| Auto-AFK `PresenceStatus` synchronization | Confirmed no-op; fixed in `f33c95a` |
| Derived `Health`/`Mana` equal recalculation | Confirmed no-op; fixed in `1c139e9` |
| Respawn at existing position/full health | Confirmed no-op; fixed in `e13e09e` |
| Equal equipment appearance set/clear requests | Confirmed no-op; fixed in `057dd17`, both paths covered through `f84ee17` |
| Repeated equal guild acceptance | Confirmed no-op; fixed in `41feebb` |
| Gold synchronization | Ruled out / already guarded; covered by `945eb0f` |
| Player `Rotation`/`MovementSpeed` normal input | Ruled out by `2927382` |
| Gravity-derived replicated state | Ruled out by `ae81c65` |
| Spawn-only `Npc`, `ModelDisplay`, and `Zone` | Ruled out for steady-state writes |
| NPC death transition at the authoritative melee damage boundary | Confirmed in `c8762ed`; all-living-NPC health scan removed; nonlethal, lethal, and zero-without-transition behavior covered |
| Admin teleport `Position`, manual presence requests, mount-triggered speed, and other event-driven writers | Remaining unproven; no speculative changes made |
| Lightyear serialized-update boundary | Blocked: no direct integration assertion distinguishes zero emitted updates from client equality suppression |
| Live component update frequency and bytes | Unmeasured: no bounded per-component serialized-update or bandwidth capture was completed |
| CPU, bandwidth, and wire-count improvement | Not claimed |

## Sources

- [server networking](../../../crates/server/src/networking.rs) — replication sender configuration and 50 ms cadence
- [NPC AI](../../../crates/server/src/creature_ai.rs) — movement writer paths
- [player networking](../../../crates/server/src/networking.rs) — player movement writer
- [presence system](../../../crates/server/src/friends.rs) — Online/Afk synchronization
- [combat system](../../../crates/server/src/combat.rs) — active combat state maintenance
- [dependency versions](../../../Cargo.lock) — Bevy `0.19.0`, Lightyear `0.28.0`, and `bevy_replicon` `0.41.1` lockfile resolution
- `game-server` commits `86d73f5`, `74edc39`, `f33c95a`, `bd31161`, `1c139e9`, `e13e09e`, `057dd17`, `41feebb`, `945eb0f`, `f84ee17`, and `c8762ed` — no-op guards, direct NPC death transition, ruled-out Gold coverage, and regression tests
- [vendored Lightyear provenance](../../../vendor/lightyear_replication/UPSTREAM.md) — Lightyear `0.28.0` integration boundary
- [vendored bevy_replicon provenance](../../../vendor/bevy_replicon/UPSTREAM.md) — `bevy_replicon` `0.41.1` dirty-candidate integration
- Local Lightyear `0.28.0` sender/receiver source — cadence-gated candidate selection and client equality suppression

## See Also

- [[networking]] — replication configuration and transport model
- [[tick-model]] — 20 Hz server schedule and replication stage
- [[creature-ai]] — NPC movement states and writer paths
- [[combat]] — authoritative damage, death, respawn, and CombatEvent semantics
