# Networking

Lightyear 0.28.0 handles game networking over UDP with netcode. The server is authoritative; clients predict locally and reconcile on correction. The server and test harnesses explicitly add Bevy 0.19's `StatesPlugin` where state scheduling is used.

## Content

## Transport

Lightyear uses its UDP transport with netcode. The server binds on port 5000 (`SERVER_PORT`).

## Server → Client Replication

Components are registered for automatic replication:

```rust
app.replicate::<Position>();
app.replicate::<Health>();
app.replicate::<Mana>();
app.replicate::<CharacterAppearance>();
app.component::<shared::casting::CastState>().replicate();
```

Lightyear handles:
- **Interest management** — only replicates entities within the client's spatial relevance radius (`INITIAL_VISIBILITY_RADIUS_SQ = 100² yards`)
- **Change-tick replication** — the server sender selects components from Bevy change ticks; equal mutable assignments are not value-filtered before serialization
- **Entity lifecycle** — spawn/despawn packets as entities enter/leave client view
- **Future zone transitions** — Lightyear can carry visibility-driven removal/insertion, but `game-server` does not yet implement runtime zone handoff

Shared-protocol `c8a06c0` makes replicated `Npc` carry UTF-8 `name` with `template_id`; the string makes it `Clone + Eq`, not `Copy`. Server `fc46567` supplies that name from `creature_template` when spawning NPCs. The engine's `79a252be` clone-based snapshot mirror is the client-side boundary; visible nameplate rendering is not established by this protocol/server change.

Lightyear 0.28.0 delegates sender traversal to vendored `bevy_replicon` 0.41.1. `ReplicationMetadata` keeps the sender cadence at 50 ms. The migration preserves dirty global and per-sender candidates, mutation ACK/NACK retention, and one full sync for each new or reset sender; subsequent unchanged roots do not enter candidate traversal.

The client compares decoded replicated components and suppresses equal ECS replacement only after serialization, batching, transport, delivery, and deserialization. First-wave fixes cover NPC movement, player displacement, presence, combat status, movement state, and gravity; second-wave commits `1c139e9`, `e13e09e`, `057dd17`, and `41feebb` cover equal derived stats, respawn state, equipment appearance, and guild membership. `f84ee17` adds independent clear-path coverage for the equipment guard, while `945eb0f` rules out unchanged Gold synchronization as already guarded. See [[semantic-noop-replication]] for the separate equal-assignment coverage; no Character advancement or `<=10%` CPU claim is made.

### Dirty Candidate Replication

`ReplicationDirtyQueue::mark` targets all connected senders; `mark_for_sender` targets one connection for visibility-specific work. Component inserts/configuration and `NetworkVisibility` removal enqueue replication roots. Lightyear's `VisibilityExt` remains the immediate visibility mutation boundary. Visibility gain/loss, room changes, hierarchy children, spawn/despawn actions, ACK retention, NACK retry, replication groups, priorities, delta compression, entity mapping, and existing action/update encoding remain inside the Lightyear/`bevy_replicon` semantic pipeline.

Authoritative marks are emitted at real state boundaries: player input/gravity, NPC movement/leash restoration, combat transitions, player lethal-death transitions, derived-stat recalculation, GoldUpdateQueue application, barber appearance updates, friend/presence and chat/emote activity, plus interest/visibility transitions. The queue is not a replacement for component change-tick filtering; it narrows candidates before the existing component and ACK checks.

`aa5dddb` separates movement's two dirty domains. `networking_movement.rs` reports `position_changed` for translation and `replicated_state_changed` for any replicated movement state. A turn-in-place or zero-translation input can therefore enqueue `Rotation` or `MovementSpeed` replication without dirtying `InterestDirtyQueue`; interest remains position-only. The zero-translation regression was RED before the fix, and the focused server test slice passed with 43 tests GREEN after it. `networking.rs` retains orchestration while movement calculations live in the extracted module.

The sender change is a vendored Lightyear 0.28.0 integration rather than a protocol fork. `vendor/lightyear_replication/UPSTREAM.md` records the crates.io checksum, upstream commit, and MIT OR Apache-2.0 licensing. The vendored Lightyear layer delegates dirty candidate traversal to `bevy_replicon` 0.41.1 and exposes test-only diagnostics for root candidates and replication visits; diagnostics are enabled only for integration proof.

The candidate traversal reduction is **PARTIAL** against the original entity/component-queue requirement: queue entries identify entities only, and dirty entities still traverse replicated components. Component-aware dirty tracking, exhaustive writer coverage, and strict-Empty live client CPU verification remain open; no `<=10%` CPU result is claimed.

## Cast Presentation

`cast_presentation.rs` consumes `SpellCastIntent`/`StopSpellCast` through Lightyear receivers. Only authenticated in-world `PlayerLink` ownership selects the caster. Explicit IDs or unique exact names resolve `SpellStore` records, converted to shared `SpellData` ([[spell-info]]); existing `validate_cast` checks alive/movement, target/range, available resources and active cooldown/GCD. It does not spend resources or trigger cooldowns.

The compiled shared dependency is `../shared-protocol`, not the stale `crates/shared` tree. Its `CastState` carries `spell_name` and serialized timing, and is registered for replication. Start, fixed-step progress, stop, death/movement cancellation and expiry mark `ReplicationDirtyQueue`. Progress runs after player input. Expiry removes presentation only: no spell effects or successful-damage event are fabricated.

Engine `src/network_runtime/replication.rs::EntitySnapshot` enumerates supported components explicitly, so it must copy/apply/remove `CastState` separately; registering the shared component alone does not reach renderer entities. Receiver tests use real HostClient delivery, not wire component replication. See [contract and boundaries](../../specs/cast-presentation.md).

## Interest Management

New replication clients start with every `NetworkVisibility` entity explicitly hidden. Replicon's `ClientVisibility` defaults to visible, so waiting for `PlayerLink` previously sent the initial world population before character selection. Newly spawned targets are also hidden for all replication clients, including clients without a selected character. World entry then grants only nearby entities through the existing interest path. The direct `bevy_replicon` dependency observes this actual visibility lifecycle; it is the same vendored version already used by Lightyear. A local UDP regression exchanges auth messages, verifies no existing/new NPC delivery before entry, then verifies nearby-only delivery and movement-driven replacement.

The pre-`4523373` implementation scanned every `NetworkVisibility` entity against every player every 500 ms. Current interest management is trigger-driven: authoritative position writers mark changed players/NPCs through `InterestDirtyQueue`, while local `NetworkVisibility` and `PlayerLink` state register or clear lifecycle state. Lightyear's `VisibilityExt` applies connection-specific gain/loss transitions; `InterestIndex` stores target positions in 100-yard X/Z buckets, evaluates only dirty observers/targets, and considers the observer's 3×3 bucket neighborhood plus already visible entities.

Buckets only reduce candidate work. Final visibility still uses the existing exact 3D squared-distance comparison (`<= 100²` yards), with independent per-connection `gain_visibility`/`lose_visibility` transitions. Initial nearby visibility, reconnects, spawns, despawns, teleports, multiple observers, radius boundaries, corpse visibility, and stale visibility cleanup remain covered. `update_interest` still invokes every `FixedUpdate` to drain dirty queues, and no movement-revision counter is modeled, so the literal position-change-only requirement remains **PARTIAL**. `a85b035` split the real fixed-tick integration tests from `interest.rs`; it did not change runtime semantics. See [[recurring-loop-elimination]].

## Terrain Height Sampling

The server reads MCNK world X at `0x68`, world Y at `0x6c`, and base elevation at `0x70`. MCVT rows advance toward decreasing world X; columns toward decreasing world Y. Heights use the same four-triangle fan (including the inner center vertex) as the corrected client mesh, not bilinear outer-corner interpolation. Tests cover asymmetric axes, all four triangles, parser coordinates, and the authored Northshire caravan ground height. Earlier swapped-coordinate fixture assertions were incorrect.

## Terrain Streaming

Each connection keeps both the set of terrain tiles already sent and the last player tile evaluated by the streaming system. Initial world entry and reconnect begin with no last tile, forcing one evaluation. Every later fixed tick still resolves the linked player position and computes tile coordinates; movement within the same tile then skips the 3×3 neighborhood loop, while boundary crossings and teleports evaluate the destination neighborhood. Existing `SentTiles` membership continues to suppress duplicate `LoadTerrain` messages. This is **PARTIAL** against tile-change-only execution. See [[recurring-loop-elimination]].

## Player Death Transitions

Commit `18c936b` removes the recurring `detect_player_deaths` player scan. Combat and fall damage emit `PlayerDeathTransition` only on a positive-to-zero health crossing; character selection queues the transition for persisted zero health. The FixedUpdate consumer in `death/transitions.rs` is explicitly ordered after combat and gravity, then performs the existing death lifecycle: clamp, deduplicate, preserve `LastDamager`, apply durability loss, create corpse/graveyard state, mark replication dirty, and route `"you died"`. `b4e5cbd` split the consumer module, `2106cd8` split the focused proof into `combat_tests/death.rs`, and `1b7c5a4` preserves unchanged gravity health ticks. Resurrection, second death, stale-entity handling, and login behavior remain covered by RED/GREEN artifacts under `/tmp/claude/game-server/` (`death-idle-scan`, `death-transition-consumer`, `player-combat-death`, `player-fall-death`, `player-login-death`, and `death-second-life-tests`). No live CPU savings claim is made.

## Auto-AFK Activity Deadlines

Commit `16c821c` replaces the every-`Update` all-player idle-time scan with `AutoAfkScheduler`. Login/presence setup schedules generation-tagged deadlines; player input and chat/emote activity reschedule them; manual AFK/DND invalidates automatic deadlines. `9412352` split the scheduler module for readability, and `16768b3` orders chat activity before deadline processing. `process_due_auto_afk` resolves only entities referenced by due deadlines and ignores stale generations or despawned entities.

The broad auto-AFK player scan is completed/eliminated. The scheduler still peeks its deadline heap every `Update`, so only the wider proposed lower-cadence scheduler invocation remains **PARTIAL**. Tests cover deadline timing, activity invalidation, manual presence, lifecycle, zero idle-player visits, and duplicate suppression; no live CPU savings claim is made.

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

Clients send input events, never raw state. The server validates and applies; the client predicts locally.

`process_player_inputs` drains each connection's queued `PlayerInput` messages once per 50 ms `FixedUpdate`, then applies exactly one movement step from the newest message. Direction, facing, running, and swimming are continuous state, so the newest packet wins. A jump in any drained packet is retained for that tick, and swimming in any drained packet removes `Mounted` even if the newest packet is no longer swimming. Activity marking and position-interest dirtying occur once for the accepted batch.

Before `e74b980`, every queued packet applied a full fixed-step delta. A client sending around 60 input packets per second therefore applied about three 50 ms movement steps during one 20 Hz server tick. Tests in `networking_tests/receiver_rate.rs` exercise the real Lightyear host delivery, `MessageReceiver`, and production `FixedUpdate` system: one and three identical packets move one step; newest continuous state wins; and earlier jump/swimming events survive a later stopped or non-swimming packet. RED 5 at `7b7dc1a`; GREEN 5 at `e74b980`, recorded in `data/diagnostics/receiver-rate/green-e74b980.log`. The local server was not rebuilt or restarted by that proof.


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
- [crates/server/src/death/transitions.rs](../../../crates/server/src/death/transitions.rs) — player death-transition consumer and routed lifecycle updates
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — persisted zero-health transition queue
- [crates/server/src/combat_tests/death.rs](../../../crates/server/src/combat_tests/death.rs) — focused player combat-death proof
- `game-server` commits `18c936b`, `b4e5cbd`, `2106cd8` — player death transition, source split, and focused test split
- [crates/server/src/friends.rs](../../../crates/server/src/friends.rs) — friends lifecycle and deadline-system registration
- [crates/server/src/friends/auto_afk.rs](../../../crates/server/src/friends/auto_afk.rs) — activity deadline scheduling and due-entity processing
- [crates/server/src/friends_tests/auto_afk.rs](../../../crates/server/src/friends_tests/auto_afk.rs) — auto-AFK timing, activity, lifecycle, and idle-visit coverage
- `game-server` commits `16c821c`, `9412352`, `16768b3` — deadline scheduling, module split, and chat ordering
- `game-server` commit `1b7c5a4` — preserve unchanged gravity health ticks
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
- [crates/server/src/networking_tests/receiver_rate.rs](../../../crates/server/src/networking_tests/receiver_rate.rs) — real receiver and fixed-step queued-input regressions
- `game-server` commits `7b7dc1a`, `e74b980`, `4c40480` — RED harness delivery, once-per-tick application, and test-module formatting
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — dirty-position interest index and visibility transitions
- [crates/server/src/interest_tests.rs](../../../crates/server/src/interest_tests.rs) — fixed-tick interest behavior proof
- [vendor/lightyear_replication/UPSTREAM.md](../../../vendor/lightyear_replication/UPSTREAM.md) — Lightyear 0.28.0 and vendored `bevy_replicon` provenance
- [vendor/bevy_replicon/UPSTREAM.md](../../../vendor/bevy_replicon/UPSTREAM.md) — `bevy_replicon` 0.41.1 provenance
- [vendor/bevy_replicon/src/server/dirty.rs](../../../vendor/bevy_replicon/src/server/dirty.rs) — dirty global/per-sender queue and lifecycle marks
- [vendor/bevy_replicon/src/server.rs](../../../vendor/bevy_replicon/src/server.rs) — candidate drain and new-sender full sync
- [vendor/lightyear_replication/src/send.rs](../../../vendor/lightyear_replication/src/send.rs) — Lightyear sender integration, cadence, and ACK/NACK handoff
- [crates/integration-tests/src/tests.rs](../../../crates/integration-tests/src/tests.rs) — in-process replication integration proof
- [Cargo.toml](../../../Cargo.toml) and [Cargo.lock](../../../Cargo.lock) — Bevy, Lightyear, and Rust versions
- `game-server` commit `0b1e2e6` — Bevy 0.19/Lightyear 0.28.0 migration and preserved replication proof
- `game-server` commit `8b400f5` — local `bevy_replicon` dependency for vendored replication tests
- `shared-protocol` commit `a78c958` — Bevy 0.19/Lightyear 0.28.0 protocol boundary
- `shared-protocol` commit `c8a06c0` — replicated `Npc.name` contract
- `game-server` commit `fc46567` — SQLite template name copied into spawned NPCs
- [CLAUDE.md](../../../CLAUDE.md) — dependency versions

## See Also

- [[tick-loop-audit]] — recurring networking/application polls and remaining proof limits
- [[tick-model]] — replication spans fixed state mutation and PostUpdate buffering; no global numbered pipeline exists
- [[zone-architecture]] — zone handoff is proposed, not current runtime behavior
- [[combat]] — CombatEvent broadcast over CombatChannel
- [[recurring-loop-elimination]] — removal of recurring replication, interest, and streaming scans
