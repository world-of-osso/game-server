# Networking

Lightyear handles all game networking over QUIC (via `quinn`). The server is authoritative; clients predict locally and reconcile on correction.

## Transport

QUIC provides both reliable and unreliable streams natively, with built-in encryption and connection migration. Lightyear uses `quinn` under the hood. The server binds on port 5000 (`SERVER_PORT`).

## Server → Client Replication

Components are registered for automatic replication:

```rust
app.replicate::<Position>();
app.replicate::<Health>();
app.replicate::<Mana>();
app.replicate::<CharacterAppearance>();
app.replicate::<CastingSpell>();
```

Lightyear handles:
- **Interest management** — only replicates entities within the client's spatial relevance radius (`INITIAL_VISIBILITY_RADIUS_SQ = 100² yards`)
- **Change-tick replication** — the server sender selects components from Bevy change ticks; equal mutable assignments are not value-filtered before serialization
- **Entity lifecycle** — spawn/despawn packets as entities enter/leave client view
- **Zone transitions** — entity despawn + respawn on the client side is transparent

The server's Lightyear `0.26.4` sender still wakes on its 50 ms `SinceLastAck` cadence, but commits `269655a` and `30b5078` remove the steady-state per-sender traversal of every replicated root and component. The sender now processes dirty global/per-sender candidates and retains candidates until ACK/NACK and pending-action semantics permit removal. A new or reset sender queues every current root once for initial full sync; subsequent unchanged roots do not enter the candidate traversal.

The client compares decoded replicated components and suppresses equal ECS replacement only after serialization, batching, transport, delivery, and deserialization. First-wave fixes cover NPC movement, player displacement, presence, combat status, movement state, and gravity; second-wave commits `1c139e9`, `e13e09e`, `057dd17`, and `41feebb` cover equal derived stats, respawn state, equipment appearance, and guild membership. `f84ee17` adds independent clear-path coverage for the equipment guard, while `945eb0f` rules out unchanged Gold synchronization as already guarded. See [[semantic-noop-replication]] for the separate equal-assignment coverage; no Character advancement or `<=10%` CPU claim is made.

### Dirty Candidate Replication

`ReplicationDirtyQueue::mark` targets all connected senders; `mark_for_sender` targets one connection for visibility-specific work. Component inserts/configuration and `NetworkVisibility` removal enqueue replication roots. Visibility gain/loss, room changes, hierarchy children, spawn/despawn actions, ACK retention, NACK retry, replication groups, priorities, delta compression, entity mapping, and existing action/update encoding remain inside Lightyear's semantic pipeline.

Authoritative marks are emitted at real state boundaries: player input/gravity, NPC movement/leash restoration, combat transitions, derived-stat recalculation, gold synchronization, barber appearance updates, friend/presence and chat/emote activity, plus interest/visibility transitions. The queue is not a replacement for component change-tick filtering; it narrows candidates before the existing component and ACK checks.

`aa5dddb` separates movement's two dirty domains. `networking_movement.rs` reports `position_changed` for translation and `replicated_state_changed` for any replicated movement state. A turn-in-place or zero-translation input can therefore enqueue `Rotation` or `MovementSpeed` replication without dirtying `InterestDirtyQueue`; interest remains position-only. The zero-translation regression was RED before the fix, and the focused server test slice passed with 43 tests GREEN after it. `networking.rs` retains orchestration while movement calculations live in the extracted module.

The sender change is a vendored patch rather than a protocol fork. `vendor/lightyear_replication/UPSTREAM.md` records Lightyear `0.26.4`, crates.io checksum `a451536b73e618035472548e8d7f1af51e193649f556318f7836ddc70e6f73d2`, upstream commit `7082c5eb09463fc110461f1cc928994460611e98`, and MIT OR Apache-2.0 licensing. Test-only diagnostics count root candidates and `replicate_entity` visits; they are enabled only for integration proof.

The remaining strict-Empty gate is live client CPU verification with zero terrain/world rendering. The candidate traversal proof passes, but no `<=10%` CPU result is claimed.

## Interest Management

The pre-`4523373` implementation scanned every `NetworkVisibility` entity against every player every 500 ms. Current interest management is trigger-driven: authoritative position writers mark changed players/NPCs through `InterestDirtyQueue`, while `NetworkVisibility` and `PlayerLink` add/remove observers register or clear lifecycle state. `InterestIndex` stores target positions in 100-yard X/Z buckets, evaluates only dirty observers/targets, and considers the observer's 3×3 bucket neighborhood plus already visible entities.

Buckets only reduce candidate work. Final visibility still uses the existing exact 3D squared-distance comparison (`<= 100²` yards), with independent per-connection `gain_visibility`/`lose_visibility` transitions. Initial nearby visibility, reconnects, spawns, despawns, teleports, multiple observers, radius boundaries, corpse visibility, and stale visibility cleanup remain covered. `a85b035` split the real fixed-tick integration tests from `interest.rs`; it did not change runtime semantics. See [[recurring-loop-elimination]].

## Terrain Streaming

Each connection keeps both the set of terrain tiles already sent and the last player tile evaluated by the streaming system. Initial world entry and reconnect begin with no last tile, forcing one evaluation. Later fixed ticks compare the current tile first; movement within the same tile skips the 3×3 neighborhood loop, while boundary crossings and teleports evaluate the destination neighborhood. Existing `SentTiles` membership continues to suppress duplicate `LoadTerrain` messages. See [[recurring-loop-elimination]].

## Client → Server Inputs

Clients send input events, never raw state. The server validates and applies; the client predicts locally:

```rust
enum PlayerInput {
    Move { direction: Vec3 },
    CastSpell { spell_id: SpellId, target: EntityId },
    Interact { target: EntityId },
    Jump,
    StopMove,
}
```

## Channels

| Channel | Reliability | Use |
|---------|-------------|-----|
| Movement | Unreliable | Position updates, interpolation |
| Combat | Reliable ordered | Spell casts, damage events, health changes |
| Chat | Reliable ordered | Chat messages, system notifications |
| Inventory | Reliable ordered | Item moves, loot, trade |
| Terrain | Reliable ordered | Map tile load commands |

The `CombatChannel` carries `CombatEvent` structs (attacker, target, amount, spell_id, event_type), including melee damage, NPC death, and NPC respawn events. These are also written to the server-side `CombatLog` ring buffer (capacity 10,000 entries). The NPC death transition remains authoritative at the server damage boundary; networking carries its resulting event and replication lifecycle.

## Serialization

`bitcode` 0.6 is used for binary serialization — approximately 10x faster than serde-json and more compact than bincode.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Networking with Lightyear" section
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — server plugin, channel types, interest radius
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — authoritative NPC damage/death transition and combat events
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — player movement dirty routing and terrain tile-transition gating
- [crates/server/src/networking_movement.rs](../../../crates/server/src/networking_movement.rs) — translation, rotation, speed, and movement dirty-domain results
- [crates/server/src/networking_tests/physics.rs](../../../crates/server/src/networking_tests/physics.rs) — zero-translation rotation/speed regression proof
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — dirty-position interest index and visibility transitions
- [crates/server/src/interest_tests.rs](../../../crates/server/src/interest_tests.rs) — fixed-tick interest behavior proof
- [vendor/lightyear_replication/UPSTREAM.md](../../../vendor/lightyear_replication/UPSTREAM.md) — Lightyear `0.26.4` provenance
- [vendor/lightyear_replication/src/send/dirty.rs](../../../vendor/lightyear_replication/src/send/dirty.rs) — dirty global/per-sender queue and lifecycle marks
- [vendor/lightyear_replication/src/send/candidate.rs](../../../vendor/lightyear_replication/src/send/candidate.rs) — initial sync and candidate processing
- [vendor/lightyear_replication/src/send/sender.rs](../../../vendor/lightyear_replication/src/send/sender.rs) — ACK/NACK retention
- [crates/integration-tests/src/tests.rs](../../../crates/integration-tests/src/tests.rs) — in-process replication integration proof
- [CLAUDE.md](../../../CLAUDE.md) — dependency versions

## See Also

- [[tick-model]] — replication runs at step 6 of the 20Hz tick pipeline
- [[zone-architecture]] — zone transfers are transparent to clients via despawn/respawn
- [[combat]] — CombatEvent broadcast over CombatChannel
- [[recurring-loop-elimination]] — removal of recurring replication, interest, and streaming scans
