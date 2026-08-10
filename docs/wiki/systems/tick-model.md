# Tick Model

The current server uses a 50 ms outer runner and a 50 ms Bevy fixed timestep configured through Lightyear, producing a nominal 20 Hz authoritative simulation. Runtime work spans multiple Bevy schedules; there is no implemented global seven-stage fixed pipeline. The proposed ordering contract is in [[tick-loop-design]], and the exhaustive correctness/recurring-work review is [[tick-loop-audit]].

## Content

## Runtime Topology

| Boundary | Current responsibility |
|---|---|
| `Startup` | Open redb, load SQLite world data and terrain, start Lightyear server |
| `PostStartup` | Spawn Elwynn creatures and build graveyard state |
| `PreUpdate` | Lightyear transport, message, and replication receive work |
| `FixedUpdate` | Authoritative simulation and cadence-gated maintenance |
| `Update` | Auth, chat, social, economy, progression, and admin IPC requests |
| `PostUpdate` | Replication dirty forwarding, Lightyear buffering/sending, deferred despawn |
| `Last` | Lightyear message cleanup and `AppExit` character flush |

The admin Unix-socket accept loop is independent of the Bevy runner. It forwards commands through an `mpsc` channel; `poll_admin_ipc` applies them to the ECS world in `Update`.

## Actual Cadence

`ScheduleRunnerPlugin` targets one outer `App::update()` every 50 ms. `ServerPlugins { tick_duration: 50 ms }` passes the same duration into Lightyear; Lightyear's timeline plugin sets Bevy's `Time<Fixed>` timestep to that duration.

The two loops are not guaranteed to execute one-for-one:

- Healthy operation normally produces one fixed step per outer update.
- Startup may produce no fixed step until 50 ms accumulates.
- A slow outer update may execute multiple catch-up fixed steps.
- Bevy's default 250 ms virtual-time clamp bounds newly accumulated catch-up to five 50 ms steps per outer update.

`Time` read inside `FixedUpdate` therefore advances in exact 50 ms simulation increments even when several steps execute during one outer update.

## Current FixedUpdate Work

| Area | Systems or responsibility |
|---|---|
| Player movement | `process_player_inputs`, `apply_terrain_gravity`, `stream_terrain_tiles` |
| Creature AI | wander, waypoint, chase, leash, return-to-spawn |
| Combat and stats | equipment/derived stats, attack start, auto-attacks, combat status, NPC respawn |
| Player/world rules | lethal death transitions, duel outcomes, trade range enforcement |
| Visibility | dirty interest reconciliation |
| Maintenance | auction expiration gate, periodic dirty-character save, tick logging |

This table groups responsibilities; it does not imply a total execution order.

## Explicit Ordering

Only the following relevant order is encoded in current game-server source:

- `process_player_inputs` → `apply_terrain_gravity`
- combat and `apply_terrain_gravity` → player death-transition consumer
- `update_stats_from_equipment` → `update_derived_stats`
- `chase_system` → `leash_check_system` → `return_to_spawn_system`
- Initial world-map snapshot → zone-discovery update in `Update`
- `forward_position_changes_to_replication` runs in PostUpdate `ReplicationBufferSystems::BeforeBuffer`

Other fixed systems are scheduled according to ECS data conflicts and Bevy's scheduler. Plugin registration order is not a global gameplay pipeline.

## Networking Boundary

Lightyear owns transport and replication schedules:

1. Receive transport/messages in `PreUpdate`.
2. Game-server fixed systems drain `PlayerInput` and mutate authoritative components.
3. Request handlers run in `Update`.
4. Game-server hooks forward dirty state before Lightyear's PostUpdate replication buffer.
5. Lightyear buffers replication/messages and flushes them to transport.

Each connection receives a `ReplicationSender` with a 50 ms `SinceLastAck` cadence. Dirty-candidate processing avoids steady-state traversal of unchanged replication roots while preserving initial sync, visibility, and ACK/NACK behavior.

## Lower-Frequency and State-Driven Work

- **Auction expiration:** registered in `FixedUpdate`; runs immediately once, then after each accumulated one-second duration.
- **Periodic character save attempt:** every 300 fixed steps, nominally 15 seconds. This is currently incorrect: the system executes and returns on intervening ticks, advancing Bevy's change-detection window, so the due `Changed<Position|Health|Player>` query misses changes made earlier in the interval. See [[tick-loop-audit]].
- **Tick log:** every 200 fixed steps, which is 10 seconds of simulation time.
- **Auto-AFK:** activity deadlines remove the per-player `Update` scan; `process_due_auto_afk` still peeks the next deadline every `Update` and processes only due players.
- **Terrain streaming:** remains fixed-step and still probes every connection's position/tile; only the 3×3 neighborhood/send work is skipped when the tile is unchanged.
- **Interest:** dirty queues and spatial buckets remove the full entity×player scan, but `update_interest` still invokes every fixed step.
- **Replication:** dirty entity candidates remove steady-state full-root traversal; dirty entities still traverse replicated components.

Disconnect saves are observer-driven. Shutdown saves run in `Last` only when `AppExit` is present.

## Current Limits

- Periodic save change tracking is a confirmed correctness bug; the 300-tick counter does not preserve a 300-tick `Changed<T>` window.
- No global `SystemSet` phase chain orders input, AI, movement, combat, lifecycle, visibility, and maintenance.
- Per-zone system sets and runtime zone-transfer events are not implemented; current `ZonePlugin` performs `PostStartup` Elwynn creature spawning.
- Many gameplay requests run in `Update`, after the fixed schedule within the same outer update, rather than inside one monolithic game tick.
- Periodic redb writes run synchronously on the Bevy thread.
- The preserved Lightyear, interest, friends, and terrain loop-removal requirements remain partial; see [[tick-loop-audit]] for their literal completion criteria.

## Sources

- [crates/server/src/main.rs](../../../crates/server/src/main.rs) — runner, Lightyear tick duration, plugin registration, tick log
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — fixed movement, Update requests, PostUpdate despawn, connection lifecycle
- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — fixed AI registration and local chain
- [crates/server/src/stats.rs](../../../crates/server/src/stats.rs) — chained stat recalculation
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — fixed combat and NPC respawn
- [crates/server/src/death.rs](../../../crates/server/src/death.rs) — player death-transition consumer and lifecycle ordering
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — persisted zero-health transition queue
- `game-server` commit `18c936b` — replace player death polling with lethal transitions
- [crates/server/src/friends.rs](../../../crates/server/src/friends.rs) — activity-deadline auto-AFK scheduling
- `game-server` commit `16c821c` — replace per-player auto-AFK accumulation with deadline scheduling
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — interest and replication dirty schedules
- [crates/server/src/auction_house.rs](../../../crates/server/src/auction_house.rs) — one-second expiration gate
- [crates/server/src/persistence.rs](../../../crates/server/src/persistence.rs) — persistence schedule registration
- [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs) — 300-tick save cadence and shutdown flush
- [crates/server/src/admin_ipc.rs](../../../crates/server/src/admin_ipc.rs) — Update polling boundary
- [crates/server/src/admin_ipc_server.rs](../../../crates/server/src/admin_ipc_server.rs) — independent socket/Tokio loop
- [vendor/lightyear_replication/src/send/plugin.rs](../../../vendor/lightyear_replication/src/send/plugin.rs) — PostUpdate replication pipeline
- [Cargo.lock](../../../Cargo.lock) — Bevy and Lightyear locked versions

## See Also

- [[tick-loop-audit]] — complete 130-system recurring-loop audit and prioritized findings
- [[tick-loop-design]] — proposed explicit phase and cadence contract
- [[networking]] — transport, interest, and dirty replication details
- [[persistence]] — save triggers and storage ownership
- [[zone-architecture]] — proposed zone isolation versus current spawn-only implementation
- [[recurring-loop-elimination]] — removal of recurring world scans
