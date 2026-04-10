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
- **Delta compression** — only sends changed components
- **Entity lifecycle** — spawn/despawn packets as entities enter/leave client view
- **Zone transitions** — entity despawn + respawn on the client side is transparent

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
