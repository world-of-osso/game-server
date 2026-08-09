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

The server's Lightyear `0.26.4` sender runs every 50 ms with `SinceLastAck`. The client compares decoded replicated components and suppresses equal ECS replacement only after serialization, batching, transport, delivery, and deserialization. See [[semantic-noop-replication]] for the confirmed no-op writers and unmeasured wire-level proof boundary.

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

The `CombatChannel` carries `CombatEvent` structs (attacker, target, amount, spell_id, event_type). These are also written to the server-side `CombatLog` ring buffer (capacity 10,000 entries).

## Serialization

`bitcode` 0.6 is used for binary serialization — approximately 10x faster than serde-json and more compact than bincode.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Networking with Lightyear" section
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — server plugin, channel types, interest radius
- [CLAUDE.md](../../../CLAUDE.md) — dependency versions

## See Also

- [[tick-model]] — replication runs at step 6 of the 20Hz tick pipeline
- [[zone-architecture]] — zone transfers are transparent to clients via despawn/respawn
- [[combat]] — CombatEvent broadcast over CombatChannel
