# Game Tick Loop Design

Draft normative design for the `game-server` runtime loops. It keeps one 20 Hz authoritative simulation, makes dependency ordering explicit, and separates fixed simulation from request handling, replication, maintenance, and background I/O. Current behavior is documented in [[tick-model]]; the source-grounded correctness and recurring-work review is [[tick-loop-audit]]. This page proposes the target and does not change runtime behavior.

## Content

## Status and Scope

**Status:** Draft for review; player lethal-transition ownership is implemented in `18c936b`, but periodic persistence, global fixed phases, ten definite idle-work systems, and the four preserved loop requirements remain open.

**Review gate:** fix periodic change tracking, define global fixed-phase ordering, and complete the literal Lightyear, interest, friends, and terrain boundaries before treating this design as approved.

**Proposed baseline:** Keep a single 50 ms `FixedUpdate` cadence shared by Bevy and Lightyear.

In scope:

- Headless Bevy runner and schedule boundaries
- Authoritative fixed-step simulation
- Lightyear receive and replication boundaries
- Lower-frequency maintenance work
- Admin IPC handoff to the ECS world
- Catch-up behavior and ordering invariants

Out of scope:

- Client rendering, prediction, and interpolation loops
- Per-zone parallelism and dungeon/raid instance isolation
- Network protocol changes
- Moving persistence to a worker before profiling proves it necessary

## Loop Topology

```text
Process startup
  ├─ Startup: database, world data, terrain, network listener
  └─ PostStartup: world/NPC and graveyard initialization

ScheduleRunner target: one App::update() every 50 ms
  ├─ PreUpdate
  │    Lightyear transport receive → message receive → replication receive
  ├─ RunFixedMainLoop: zero or more 50 ms fixed steps
  │    ├─ FixedFirst: Lightyear local tick increment
  │    └─ FixedUpdate: authoritative game simulation
  ├─ Update
  │    Request, social, auth, economy, and admin-command handlers
  ├─ PostUpdate
  │    Replication dirty forwarding → Lightyear buffer → message/transport send
  └─ Last
       Message cleanup; shutdown persistence when AppExit is present

Independent admin IPC thread
  Unix socket accept loop → mpsc command → Update poll → mpsc response
```

The runner interval and fixed timestep are related but not identical concepts. `ScheduleRunnerPlugin` paces outer `App::update()` calls. Lightyear's `TimelinePlugin` applies the configured 50 ms tick duration to Bevy's `Time<Fixed>`. A healthy update normally executes one fixed step, but Bevy may execute zero steps on startup or multiple catch-up steps after a slow update.

## Authoritative Fixed-Step Phases

Introduce named `SystemSet`s for dependency boundaries. Systems inside one phase may still run in parallel when their ECS access permits it; only required edges are serialized.

| Phase | Responsibility | Current examples |
|---|---|---|
| **Input** | Drain and validate simulation inputs | `process_player_inputs` |
| **Movement** | Apply player and creature movement before range-sensitive rules | `apply_terrain_gravity`, wander, waypoint, chase, leash, return-to-spawn |
| **Resolution** | Derive stats and resolve combat or proximity rules | stat recalculation, auto-attacks, trade range, duel outcomes |
| **Lifecycle** | Apply death, respawn, spawn/despawn, and other state transitions | player death-transition consumer, NPC respawn |
| **Visibility** | Reconcile interest and terrain state after authoritative positions settle | `update_interest`, `stream_terrain_tiles` |
| **Maintenance** | Run cadence-gated work that observes the completed tick | auction expiration, dirty saves, tick logging |

Required high-level order:

```text
Input → Movement → Resolution → Lifecycle → Visibility → Maintenance
```

Local order remains explicit inside a phase. Examples: equipment stats before derived stats; player input before gravity; combat and fall damage before player death handling; respawn position changes before interest reconciliation.

Plugin registration order is not an ordering contract. Every required edge must use phase sets, `.before()`, `.after()`, or `.chain()`.

## Schedule Assignment Policy

Use `FixedUpdate` when work advances simulation time or must observe a deterministic authoritative state boundary:

- Position, movement, combat, death, respawn, and range enforcement
- Fixed-duration gameplay timers
- Visibility reconciliation based on fixed-step state

Use `Update` for request-driven service work that does not need one execution per simulation tick:

- Authentication and character management
- Chat, guild, friends, auction commands, collections, and other transactions
- Admin IPC command dispatch

An `Update` handler that mutates replicated simulation state must declare its boundary:

1. Mark the replication root when a replicated component changes.
2. Mark interest state when position, visibility, or lifecycle changes.
3. Treat the mutation as available to `PostUpdate` replication in the current outer update and to fixed simulation rules on the next fixed step.

Do not add recurring full-world polling to either schedule. Prefer change filters, events, dirty queues, state-bounded queries, and duration gates.

### Player Death Transition (`18c936b`)

Player death is now owned by an internal `PlayerDeathTransition` message emitted at authoritative lethal boundaries. Combat and fall damage emit only when health crosses from positive to zero; character selection queues the same transition when persisted health is already zero. The FixedUpdate consumer runs after both combat and gravity, clamps health, deduplicates already-dead players, preserves `LastDamager`, applies durability loss, creates the corpse/graveyard state, marks replication dirty, and sends the existing `"you died"` update. The old broad `detect_player_deaths` player scan is removed.

The transition boundary preserves relogging dead characters, resurrection and second-life behavior, stale-entity safety, and existing routing. This is a concrete lifecycle edge inside the proposed phase model; it does not claim that all six phase sets are already migrated.

## Cadence Policy

| Work | Cadence | Semantics |
|---|---:|---|
| Outer Bevy update | Target 50 ms | Service-frame pacing; not guaranteed one-to-one with fixed steps |
| Authoritative fixed simulation | 50 ms / 20 Hz | Simulation time |
| Lightyear tick | 50 ms / 20 Hz | Sets `Time<Fixed>` and network timeline |
| Per-sender replication | 50 ms | PostUpdate sender gate with ACK/NACK state |
| Auction expiration | 1 second | Duration-gated maintenance |
| Character dirty save | Currently 300 fixed ticks = 15 seconds | Tick-counted simulation time |
| Tick log | 200 fixed ticks = 10 seconds | Tick-counted simulation time |

Gameplay cadence may be expressed in ticks. Operational cadence that means elapsed time should use `Duration`; changing the server tick rate must not silently change save, cleanup, or reporting intervals. Under that rule, periodic persistence should become duration-based if the 15-second interval is retained.

## Catch-Up and Overrun Policy

Keep Bevy's bounded fixed-step catch-up behavior for the first implementation:

- A slow outer update may execute multiple 50 ms fixed steps.
- Bevy's default 250 ms virtual-time clamp limits one outer update to at most five new fixed steps.
- Time beyond the clamp is discarded rather than replayed as an unbounded backlog.
- No custom sleep, spin loop, or second simulation clock should be added.

Implementation should expose fixed steps per outer update and schedule duration before changing this policy. The design favors bounded recovery over an indefinitely growing simulation backlog.

## Replication Boundary

Authoritative component mutation finishes before Lightyear's PostUpdate replication buffer.

Required invariant:

```text
fixed state writes
  → position/replication dirty forwarding
  → ReplicationBufferSystems::BeforeBuffer
  → Buffer
  → AfterBuffer
  → Flush
  → transport send
```

Interest reconciliation must observe every authoritative position or lifecycle writer that should affect same-tick visibility. Replication dirtying and interest dirtying are separate domains: a rotation-only change may require replication without spatial-interest work, while a teleport requires both.

## Persistence and Blocking Work

Current redb reads and writes execute synchronously on the Bevy thread. This design does not add an asynchronous persistence layer without measured tick overruns or write latency proving it necessary.

Persistence rules:

- Disconnect save remains event-driven.
- Shutdown save remains tied to `AppExit` in `Last`.
- Periodic dirty save runs only at its configured maintenance cadence.
- Database latency must not be hidden; failures stay explicit and save duration should be observable.

Admin socket I/O remains on its dedicated thread. All ECS mutation remains serialized through the `Update` command poll; the socket task never accesses the Bevy `World` directly.

## Zone Boundary

Per-zone `SystemSet`s and cross-zone transfer events are not implemented today. They are not prerequisites for the base tick loop and must not be implied by the fixed phase model.

A future zone or instance design may place zone-filtered systems inside the Movement, Resolution, and Lifecycle phases while preserving the global replication boundary. That work remains a separate decision in [[zone-architecture]] and [[open-questions]].

## Required Invariants

1. Runner interval, Lightyear tick duration, and intended fixed timestep derive from one server tick constant.
2. Required gameplay ordering is encoded in scheduler constraints, never inferred from plugin registration order.
3. All position and lifecycle writers update the required replication and interest dirty domains.
4. Replication buffering runs after authoritative state mutation for the outer update.
5. Every lower-frequency task states whether its cadence is simulation ticks or elapsed duration.

## Verification Plan

1. Assert the built server app has a 50 ms `Time<Fixed>` after Lightyear plugin setup.
2. Add an integration trace proving phase order while allowing parallel systems inside a phase.
3. Advance manual time by multiple periods and prove catch-up executes the expected fixed-step count.
4. Prove state written in the final simulation phase is visible to PostUpdate replication buffering.
5. Prove the selected persistence cadence independently of unrelated fixed systems.

## Review Decisions

1. **Single-rate simulation:** retain one 20 Hz fixed schedule now; reconsider multi-rate movement only after measured movement-feel evidence.
2. **Explicit phases:** add the six phase sets and migrate current fixed systems without changing their gameplay logic.
3. **Catch-up:** retain Bevy's default 250 ms clamp and bounded catch-up.
4. **Request boundary:** keep service/request handlers in `Update`; simulation effects enter fixed rules on the next fixed step.
5. **Persistence cadence:** preserve 15 seconds, but gate the whole save system or use an explicit dirty set; the current internal 300-tick early return loses `Changed<T>` history.

## Sources

- [crates/server/src/main.rs](../../../crates/server/src/main.rs) — runner cadence, Lightyear tick configuration, plugin registration, and tick logging
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — input, movement, request, startup, disconnect, and replication-sender boundaries
- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — fixed-step AI systems and local ordering
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — combat damage, lethal transition emission, and respawn systems
- [crates/server/src/death.rs](../../../crates/server/src/death.rs) — player death-transition consumer and lifecycle ordering
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — queued transition for persisted zero health
- `game-server` commit `18c936b` — replace player death polling with lethal transitions
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — fixed interest reconciliation and PostUpdate replication dirty forwarding
- [crates/server/src/persistence.rs](../../../crates/server/src/persistence.rs) — startup, periodic, and shutdown persistence schedules
- [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs) — 300-tick dirty-save cadence
- [crates/server/src/admin_ipc.rs](../../../crates/server/src/admin_ipc.rs) — ECS-side IPC polling
- [crates/server/src/admin_ipc_server.rs](../../../crates/server/src/admin_ipc_server.rs) — background socket/Tokio loop
- [vendor/lightyear_replication/src/send/plugin.rs](../../../vendor/lightyear_replication/src/send/plugin.rs) — PostUpdate replication buffer ordering
- [Cargo.lock](../../../Cargo.lock) — locked Bevy 0.18.0 and Lightyear 0.26.4 versions

## See Also

- [[tick-loop-audit]] — exhaustive recurring-system inventory and correctness verdict
- [[tick-model]] — observed current schedules and ordering
- [[networking]] — Lightyear transport, interest, and dirty replication
- [[persistence]] — character save boundaries and storage ownership
- [[recurring-loop-elimination]] — state- and dirty-driven replacements for recurring scans
- [[open-questions]] — unresolved tick-rate and instance decisions
