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

The server's Lightyear `0.26.4` sender runs every 50 ms with `SinceLastAck`. The client compares decoded replicated components and suppresses equal ECS replacement only after serialization, batching, transport, delivery, and deserialization. First-wave fixes cover NPC movement, player displacement, presence, combat status, movement state, and gravity; second-wave commits `1c139e9`, `e13e09e`, `057dd17`, and `41feebb` cover equal derived stats, respawn state, equipment appearance, and guild membership. `f84ee17` adds independent clear-path coverage for the equipment guard, while `945eb0f` rules out unchanged Gold synchronization as already guarded. See [[semantic-noop-replication]] for the coverage matrix; no CPU, bandwidth, or wire-count improvement is claimed.

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
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — player movement dirty marking and terrain tile-transition gating
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — dirty-position interest index and visibility transitions
- [crates/server/src/interest_tests.rs](../../../crates/server/src/interest_tests.rs) — fixed-tick interest behavior proof
- [CLAUDE.md](../../../CLAUDE.md) — dependency versions

## See Also

- [[tick-model]] — replication runs at step 6 of the 20Hz tick pipeline
- [[zone-architecture]] — zone transfers are transparent to clients via despawn/respawn
- [[combat]] — CombatEvent broadcast over CombatChannel
- [[recurring-loop-elimination]] — removal of recurring replication, interest, and streaming scans
