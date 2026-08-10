# Networking

Lightyear handles all game networking over QUIC (via `quinn`). The server is authoritative; clients predict locally and reconcile on correction.

## Content

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
- **Future zone transitions** — Lightyear can carry visibility-driven removal/insertion, but `game-server` does not yet implement runtime zone handoff

The server's Lightyear `0.26.4` sender still wakes on its 50 ms `SinceLastAck` cadence, but commits `269655a` and `30b5078` remove the steady-state per-sender traversal of every replicated root and component. The sender now processes dirty global/per-sender candidates and retains candidates until ACK/NACK and pending-action semantics permit removal. A new or reset sender queues every current root once for initial full sync; subsequent unchanged roots do not enter the candidate traversal.

The client compares decoded replicated components and suppresses equal ECS replacement only after serialization, batching, transport, delivery, and deserialization. First-wave fixes cover NPC movement, player displacement, presence, combat status, movement state, and gravity; second-wave commits `1c139e9`, `e13e09e`, `057dd17`, and `41feebb` cover equal derived stats, respawn state, equipment appearance, and guild membership. `f84ee17` adds independent clear-path coverage for the equipment guard, while `945eb0f` rules out unchanged Gold synchronization as already guarded. See [[semantic-noop-replication]] for the separate equal-assignment coverage; no Character advancement or `<=10%` CPU claim is made.

### Dirty Candidate Replication

`ReplicationDirtyQueue::mark` targets all connected senders; `mark_for_sender` targets one connection for visibility-specific work. Component inserts/configuration and `NetworkVisibility` removal enqueue replication roots. Visibility gain/loss, room changes, hierarchy children, spawn/despawn actions, ACK retention, NACK retry, replication groups, priorities, delta compression, entity mapping, and existing action/update encoding remain inside Lightyear's semantic pipeline.

Authoritative marks are emitted at real state boundaries: player input/gravity, NPC movement/leash restoration, combat transitions, player lethal-death transitions, derived-stat recalculation, GoldUpdateQueue application, barber appearance updates, friend/presence and chat/emote activity, plus interest/visibility transitions. The queue is not a replacement for component change-tick filtering; it narrows candidates before the existing component and ACK checks.

`aa5dddb` separates movement's two dirty domains. `networking_movement.rs` reports `position_changed` for translation and `replicated_state_changed` for any replicated movement state. A turn-in-place or zero-translation input can therefore enqueue `Rotation` or `MovementSpeed` replication without dirtying `InterestDirtyQueue`; interest remains position-only. The zero-translation regression was RED before the fix, and the focused server test slice passed with 43 tests GREEN after it. `networking.rs` retains orchestration while movement calculations live in the extracted module.

The sender change is a vendored patch rather than a protocol fork. `vendor/lightyear_replication/UPSTREAM.md` records Lightyear `0.26.4`, crates.io checksum `a451536b73e618035472548e8d7f1af51e193649f556318f7836ddc70e6f73d2`, upstream commit `7082c5eb09463fc110461f1cc928994460611e98`, and MIT OR Apache-2.0 licensing. Test-only diagnostics count root candidates and `replicate_entity` visits; they are enabled only for integration proof.

The candidate traversal reduction is **PARTIAL** against the original entity/component-queue requirement: queue entries identify entities only, and dirty entities still traverse replicated components. Component-aware dirty tracking, exhaustive writer coverage, and strict-Empty live client CPU verification remain open; no `<=10%` CPU result is claimed.

## Interest Management

The pre-`4523373` implementation scanned every `NetworkVisibility` entity against every player every 500 ms. Current interest management is trigger-driven: authoritative position writers mark changed players/NPCs through `InterestDirtyQueue`, while `NetworkVisibility` and `PlayerLink` add/remove observers register or clear lifecycle state. `InterestIndex` stores target positions in 100-yard X/Z buckets, evaluates only dirty observers/targets, and considers the observer's 3×3 bucket neighborhood plus already visible entities.

Buckets only reduce candidate work. Final visibility still uses the existing exact 3D squared-distance comparison (`<= 100²` yards), with independent per-connection `gain_visibility`/`lose_visibility` transitions. Initial nearby visibility, reconnects, spawns, despawns, teleports, multiple observers, radius boundaries, corpse visibility, and stale visibility cleanup remain covered. `update_interest` still invokes every `FixedUpdate` to drain dirty queues, and no movement-revision counter is modeled, so the literal position-change-only requirement remains **PARTIAL**. `a85b035` split the real fixed-tick integration tests from `interest.rs`; it did not change runtime semantics. See [[recurring-loop-elimination]].

## Terrain Streaming

Each connection keeps both the set of terrain tiles already sent and the last player tile evaluated by the streaming system. Initial world entry and reconnect begin with no last tile, forcing one evaluation. Every later fixed tick still resolves the linked player position and computes tile coordinates; movement within the same tile then skips the 3×3 neighborhood loop, while boundary crossings and teleports evaluate the destination neighborhood. Existing `SentTiles` membership continues to suppress duplicate `LoadTerrain` messages. This is **PARTIAL** against tile-change-only execution. See [[recurring-loop-elimination]].

## Player Death Transitions

Commit `18c936b` removes the recurring `detect_player_deaths` player scan. Combat and fall damage emit `PlayerDeathTransition` only on a positive-to-zero health crossing; character selection queues the transition for persisted zero health. The FixedUpdate consumer is explicitly ordered after combat and gravity, then performs the existing death lifecycle: clamp, deduplicate, preserve `LastDamager`, apply durability loss, create corpse/graveyard state, mark replication dirty, and route `"you died"`. Resurrection, second death, stale-entity handling, and login behavior remain covered by RED/GREEN artifacts under `/tmp/claude/game-server/` (`death-idle-scan`, `death-transition-consumer`, `player-combat-death`, `player-fall-death`, `player-login-death`, and `death-second-life-tests`). No live CPU savings claim is made.

## Rest-State Synchronization

`RestPlugin` builds the innkeeper registry once, then evaluates authenticated players only when their `Position` or `Level` changes, or when they have no `PlayerRestState` yet. Before `fffa86a`, every `Update` recomputed the linear innkeeper-radius scan and mutated existing rest state even when neither input changed.

Initial world entry still sends a `RestStateUpdate` snapshot. Position changes preserve enter/leave transitions; level changes refresh rested-XP maximums. Unchanged players do not scan innkeepers, dirty `PlayerRestState`, or emit duplicate updates. Client receiver and IPC snapshot behavior remain unchanged.

The focused regression test proves both boundaries: unchanged updates preserve the component change tick, while movement into an innkeeper area updates the rest state. RED/GREEN artifacts are recorded in [[recurring-loop-elimination]]. No live CPU or Character-stage claim is made from this code proof.

## Durability Snapshot Updates

`send_changed_durability_snapshot` broadcasts `DurabilityStateUpdate` only for players whose `EquippedDurability` changed. Before `c46b322`, the system ran every `Update`, visited every durability-bearing player, and called `Ref::is_changed()` in the body; unchanged players therefore still incurred query iteration and change-tick probes.

The query-level `Changed<EquippedDurability>` filter now excludes unchanged players before the system body. The existing `is_added()` check remains, so newly inserted durability does not produce a changed snapshot; later mutations retain the existing authenticated `PlayerLink` routing, `DurabilityStateUpdate` message, repair-cost snapshot, and error behavior.

RED/GREEN and readability evidence is recorded in [[recurring-loop-elimination]]. No live CPU savings claim is made.

## Gold Replication Updates

Login still loads persisted inventory gold into the player's replicated `Gold`. Runtime mutations no longer rely on an unconditional database scan. Barber payments, completed trades, auction creation/bids/refunds/buyouts, and mail claims record exact persisted values in `GoldUpdateQueue` only after successful inventory saves. Same-frame values coalesce by character ID.

`apply_gold_updates` drains the queue in `PostUpdate` before Lightyear's replication buffer. It applies only values for matching online players, skips equal values without dirtying `Gold`, and discards offline updates because the next login reloads redb. Profession inventory saves do not change gold; auction sale proceeds remain mail until a claim mutates the inventory. The old `sync_player_gold_from_inventory` online-player/redb scan was removed across commits `4e4c73d`, `c3b3c04`, `9d2067c`, `91bbedd`, and `8920feb`.

Behavioral RED/GREEN and readability artifacts are recorded in [[recurring-loop-elimination]]. This proves mutation boundaries and replication behavior; it does not claim a live CPU threshold result.

## Calendar Query Requests

`calendar::process_query_requests` drains `QueryCalendar` receivers before materializing the calendar snapshot. Repeated idle `Update`s therefore build no calendar snapshot; an Update with one or more eligible `Authenticated` + `InWorld` requests builds one shared sorted snapshot and sends one `calendar refreshed` response per requesting connection. All queued empty-payload requests are drained, preserving the prior response count and routing. Schedule/signup mutation ordering and the initial `Added<InWorld>` calendar snapshot path remain unchanged. Commit `18be416` covers this behavior. See [[recurring-loop-elimination]] and [[tick-loop-audit]]; no live CPU savings claim is made.

## Trade Offer Requests

The generic `process_trade_offer_updates` helper handles `ClearTradeItem` and `SetTradeMoney`. Before `969f2e0`, it resolved each authenticated connection's `PlayerLink` to a `Player` before checking the typed receiver, so idle `Update`s performed unnecessary player-query work.

The helper now checks `MessageReceiver::has_messages()` first. Empty receivers skip player resolution; queued requests still drain and apply in order, with one player resolution per batch. Existing persistence, trade-session mutation, broadcast/routing, and missing-player errors remain unchanged. The other six trade receiver systems are outside this named audit slice. RED/GREEN and readability evidence is recorded in [[recurring-loop-elimination]]; no live CPU savings claim is made.

## Who Query Requests

`who::process_query_requests` drains `QueryWho` receivers before materializing the online roster. Repeated idle `Update`s therefore build zero rosters; an Update with one or more eligible requests builds one shared roster and derives each response from it. Only `Authenticated` + `InWorld` connections participate. Each connection receives the response for its latest queued request, multiple connections route independently, and existing trimmed case-insensitive substring filtering, case-insensitive name sorting, `WhoSnapshot` content, `who: N result(s)` message, and error fields remain unchanged. Commit `a4e7aeb` covers this behavior. See [[recurring-loop-elimination]] and [[tick-loop-audit]]; no live CPU savings claim is made.

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
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — authoritative NPC/player damage transitions and combat events
- [crates/server/src/death.rs](../../../crates/server/src/death.rs) — player death-transition consumer and routed lifecycle updates
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — persisted zero-health transition queue
- `game-server` commit `18c936b` — replace player death polling with lethal transitions
- [crates/server/src/rest.rs](../../../crates/server/src/rest.rs) — innkeeper registry and change-filtered rest-state snapshots
- [crates/server/src/rest.rs](../../../crates/server/src/rest.rs) — rest-state transition test coverage
- `game-server` commit `fffa86a` — update rest state only after relevant changes
- [crates/server/src/durability.rs](../../../crates/server/src/durability.rs) — change-filtered durability snapshots and routing
- `game-server` commit `c46b322` — gate durability snapshots on changed players
- [crates/server/src/calendar.rs](../../../crates/server/src/calendar.rs) — initial snapshot, drain-first query handling, and schedule/signup mutation order
- `game-server` commit `18be416` — drain calendar queries before building a shared snapshot
- [crates/server/src/who.rs](../../../crates/server/src/who.rs) — QueryWho drain-first handling and response contract tests
- `game-server` commit `a4e7aeb` — drain Who requests before building roster
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

- [[tick-loop-audit]] — recurring networking/application polls and remaining proof limits
- [[tick-model]] — replication spans fixed state mutation and PostUpdate buffering; no global numbered pipeline exists
- [[zone-architecture]] — zone handoff is proposed, not current runtime behavior
- [[combat]] — CombatEvent broadcast over CombatChannel
- [[recurring-loop-elimination]] — removal of recurring replication, interest, and streaming scans
