# Tick Loop Audit

Source-grounded audit of every production `game-server` system registered in a recurring Bevy schedule. It answers whether the current loop design is correct, identifies work that is unnecessary every tick/update, and separates confirmed defects from systems that need measurement before redesign. Commits `a4e7aeb` and `c46b322` record completed Who and durability request/change gating; no live CPU savings claim is made.

## Content

## Verdict

**The current loop design is not fully correct.** The 20 Hz cadence and Bevy/Lightyear schedule split are reasonable, and the largest historical full-world scans have been removed. However:

| Area | Verdict |
|---|---|
| Periodic persistence | **Confirmed correctness bug:** the 300-tick early return discards most `Changed<T>` history |
| Fixed-step ordering | **Incomplete:** several same-tick outcomes are scheduler-order-dependent |
| Application polling | **Several confirmed unnecessary loops:** gold, Who, rest, durability, inactive auto-attacks |
| Lightyear/interest | **Major root/entity scans fixed:** remaining recurring work is connection/dirty-state plumbing |
| Receiver fan-out | **Real but unmeasured:** 74 `Update` systems poll typed connection receivers |

**Server audit status: 6/7 complete.** Gold (#1), Who (#2), rest (#3), durability (#4), calendar (#6), and inactive auto-attacks (#5) are complete. Trade lookup (#7) remains open.

The design should not be approved as complete until the persistence bug and fixed-order contract are resolved.

## Coverage

A production-source inventory found:

| Schedule | Registration calls | Expanded systems |
|---|---:|---:|
| `Update` | 86 | 105 |
| `FixedUpdate` | 13 | 21 |
| `PostUpdate` | 3 | 3 |
| `Last` | 1 | 1 |
| **Recurring total** | **103** | **130** |

`Startup`, `PostStartup`, tests, and observers are excluded from the recurring count. “Registration calls” counts each production `.add_systems(...)` invocation, including multiline calls; “expanded systems” expands tuple/chain registrations. Lightyear-owned transport/message/replication systems are audited separately below.

Verdict labels:

- **BUG** — current behavior can miss state or produce scheduler-dependent correctness
- **REMOVE/GATE** — confirmed unnecessary steady-state work
- **OPTIMIZE** — correctness currently depends on polling; replace only with an equivalent state/event model
- **MEASURE** — recurring work exists, but no evidence proves it is a material cost
- **KEEP** — correctly change-, event-, dirty-, or active-state-bounded

## Blocking Correctness Findings

### Periodic Saves Lose Earlier Changes

`save_dirty_characters` runs every `FixedUpdate`, increments a local counter, and returns for 299 of every 300 executions. Its query uses:

```text
Changed<Position> OR Changed<Health> OR Changed<Player>
```

Bevy updates a function system's `last_run` every time the system executes, even when its body returns before iterating the query. The due execution therefore sees changes since approximately the preceding fixed tick, not the whole 300-tick interval.

Confirmed sequence:

```text
tick 1:   Position changes
ticks 2-299: periodic system executes and returns
tick 300: due query does not contain the player
```

A temporary Bevy 0.18 reproduction produced:

```text
change before an early-return execution → due changed=[]
change immediately before the due execution → due changed=[entity]
```

**Impact:** periodic persistence can miss a position/health/player change indefinitely until another change happens immediately before a save boundary, disconnect, or shutdown.

**Required direction:** gate the entire system at the scheduler boundary so skipped ticks do not run it, or maintain an explicit dirty-character set. Add a behavioral regression covering a change early in the 300-tick window.

### Fixed Systems Lack a Global Gameplay Order

Only local edges are explicit:

- player input → gravity
- gravity → player death detection
- equipment stats → derived stats
- chase → leash → return-to-spawn
- dirty forwarding → Lightyear replication buffer

The following outcomes can be one fixed step late or depend on scheduler order:

| Missing edge | Observable consequence |
|---|---|
| movement → combat | Melee range may use pre- or post-movement positions |
| combat → player death | Lethal combat damage may transition the player this tick or next tick |
| movement → terrain streaming | A tile boundary may stream this tick or next tick |
| movement/lifecycle → interest | Visibility may remain stale for one fixed step |
| movement → duel/trade enforcement | Range cancellation may happen this tick or next tick |
| state writers → periodic persistence | A due save may observe pre- or post-writer state |

Conflicting ECS access forces serialization, but serialization order is not a gameplay contract. The phase sets proposed in [[tick-loop-design]] are needed for correctness, not just readability.

## Confirmed Unnecessary Recurring Work

| Priority | System | Current steady-state work | Verdict / root direction |
|---:|---|---|---|
| 1 | `sync_player_gold_from_inventory` | Every `Update`, every player, synchronous redb inventory load | **REMOVE/GATE.** Update replicated `Gold` at inventory mutation boundaries or through a dirty-character queue |
| 2 | `who::process_query_requests` | Builds/clones the full online-player list before checking whether any request exists | **COMPLETED.** `a4e7aeb` drains requests first, returns idle with zero roster materializations, and builds one shared roster only when requests exist |
| 3 | `rest::update_player_rest_state` | Every `Update`, every authenticated player, linear scan of all innkeepers | **COMPLETED.** `fffa86a` re-evaluates only initial state or `Changed<Position>`/`Changed<Level>` players; add spatial lookup only if profiling justifies it |
| 4 | `send_changed_durability_snapshot` | Every `Update`, every player with durability, then `Ref::is_changed()` in the body | **COMPLETED.** `c46b322` gates the query with `Changed<EquippedDurability>` and retains the `is_added()` exclusion |
| 5 | inactive `AutoAttack` entities | Clearing a target leaves `AutoAttack { active: false }`; fixed combat still queries it forever | **COMPLETED.** `152ccbd` removes `AutoAttack` on target loss; component presence now defines participation |
| 6 | calendar query snapshot | Rebuilds the calendar snapshot before checking for `QueryCalendar` messages | **COMPLETED.** `18be416` drains eligible requests first and builds one shared snapshot only when requests exist |
| 7 | trade offer lookup | Resolves each connection's player before checking whether that typed receiver is empty | **REMOVE/GATE.** Drain first, then resolve player state for non-empty requests |

### Durability Snapshot Gating

Before `c46b322`, `send_changed_durability_snapshot` entered every `Update`, iterated every player with `EquippedDurability`, and checked `Ref::is_changed()` inside the body. The query now uses `Changed<EquippedDurability>`, eliminating idle per-player visits while preserving the `is_added()` exclusion for newly added durability.

The RED proof is `/tmp/claude/game-server/durability-changed-filter-red.log`; GREEN is `/tmp/claude/game-server/durability-changed-filter-green.log` (**6 passed**). Formatting and readability artifacts are `/tmp/claude/game-server/durability-changed-filter-fmt.log` and `/tmp/claude/game-server/durability-changed-filter-readability.json`. The proof covers unchanged, newly added, changed, routed, and snapshot-content behavior. No live CPU savings claim is made.

### Calendar Query Snapshot Construction

Before `18be416`, `calendar::process_query_requests` built and cloned the sorted calendar snapshot before checking eligible `QueryCalendar` receivers. The fix drains authenticated + `InWorld` receivers first, returns on idle updates, and builds one shared snapshot only when requests exist. All queued empty-payload queries still drain and produce one refresh response per requesting connection. Schedule/signup mutation order and the initial `Added<InWorld>` snapshot path are unchanged.

RED: `/tmp/claude/game-server/calendar-query-drain-red.log`. GREEN: `/tmp/claude/game-server/calendar-query-drain-green.log` (**6 passed**). Formatting/readability artifacts: `/tmp/claude/game-server/calendar-query-drain-fmt.log` and `/tmp/claude/game-server/calendar-query-drain-readability.json`. No live CPU savings claim is made.

### Who Query Roster Construction

Before `a4e7aeb`, `who::process_query_requests` materialized the full online roster before inspecting `QueryWho` receivers, so repeated idle `Update`s cloned names and allocated snapshot fields without a request. The fix drains every eligible receiver first and keeps only its latest request. If none are pending, the system returns without materializing the roster; when requests exist, it builds one shared roster and clones that snapshot per response.

The protocol contract remains unchanged: only `Authenticated` + `InWorld` connections are eligible; multiple connections are routed independently; multiple requests from one connection produce one response for the latest request; filtering remains case-insensitive substring matching after trimming; results remain sorted case-insensitively by name; responses retain the existing snapshot, `who: N result(s)` message, and error fields. The focused tests also prove ineligible connections produce neither responses nor roster materialization.

RED artifacts: `/tmp/claude/game-server/who-idle-roster-red.log` and `/tmp/claude/game-server/who-request-contract-red.log`. GREEN artifact: `/tmp/claude/game-server/who-drain-first-green.log`. Formatting/readability artifacts: `/tmp/claude/game-server/who-drain-first-fmt.log`, `/tmp/claude/game-server/who-drain-first-readability-audit.json`, and `/tmp/claude/game-server/who-drain-first-readability-metrics.json`. No live CPU savings claim is made.

One-time initialization systems for auctions and rest also wake every `Update` after completion, but only execute a boolean check. Moving them to a startup boundary would be cleaner; they are not material steady-state scans.

## Correctness-Dependent Polls

These loops are real, but removing them without replacing their transition model would break behavior:

| System | Current scope | Safe direction |
|---|---|---|
| `apply_terrain_gravity` | Every player every fixed step; terrain lookup even when stationary and grounded | Track grounded/sleeping versus airborne/dirty players; horizontal moves and teleports must wake grounding |
| `detect_player_deaths` | Every player every fixed step | Central lethal-health transition or audited health-dirty queue; preserve fall, combat, and future damage writers |
| `tick_auto_afk` | Every player every `Update` for a 300-second deadline | Store activity timestamps/deadlines and process due players at a lower cadence |
| `stream_terrain_tiles` | Every connection every fixed step; expensive 3×3 work already tile-gated | Wake from player tile changes instead of probing every connection |
| creature AI | Five fixed systems; several scan broad creature-state queries and branch on enum variants | Partition active states or combine state-machine passes only after profiling entity counts and behavior |
| `enforce_duel_outcomes` | Active-duel list each fixed step | **KEEP** while active; optional run condition when the list is empty |
| `enforce_trade_range` | Active-trade sessions each fixed step | **KEEP** while active; position-dirty participants could narrow work later |
| `tick_pending_gather_casts` | Entities with `PendingGatherCast` only | **KEEP.** Already active-state-bounded |

## FixedUpdate Matrix

| System | Steady-state shape | Verdict |
|---|---|---|
| `tick_log` | Local counter and modulo each tick | **KEEP**; negligible |
| `process_player_inputs` | Per-connection receiver scan; drains all queued inputs | **MEASURE** queue depth/budget; semantically required |
| `apply_terrain_gravity` | All players, terrain lookup and state calculation | **OPTIMIZE** idle/grounded participation |
| `stream_terrain_tiles` | All connections; same-tile early return | **OPTIMIZE** remaining tile probe |
| `start_attack_system` | `Changed<CombatTarget>` only | **KEEP** |
| `auto_attack_system` | Every entity retaining `AutoAttack` | **KEEP** active attackers fixed-step; `152ccbd` removes inactive participants on target loss |
| `maintain_in_combat_state_system` | Entities with `CombatState` only | **KEEP** active-state-bounded |
| `respawn_system` | Dead NPCs with `DeathState` only | **KEEP** active-state-bounded |
| `wander_system` | All `WanderState` entities; idle/walking timer | **MEASURE** NPC count; simulation-required today |
| `waypoint_system` | All `WaypointState` entities; delay/movement timer | **MEASURE** NPC count; simulation-required today |
| `chase_system` | All spawned creatures with `CreatureState`; variant branch | **OPTIMIZE** state partition if profiling justifies it |
| `leash_check_system` | Same broad creature population; pursuing branch | **OPTIMIZE** state partition if profiling justifies it |
| `return_to_spawn_system` | Same broad creature population; returning branch | **OPTIMIZE** state partition if profiling justifies it |
| `update_stats_from_equipment` | `Changed<EquippedItems>` | **KEEP** |
| `update_derived_stats` | `Changed<UnitStats>` or `Changed<Level>` | **KEEP** |
| `detect_player_deaths` | All players; health/death-state branch | **OPTIMIZE**, after complete writer audit |
| `update_interest` | Dirty interest/observer/target queues | **KEEP**; expensive work is dirty-driven |
| `expire_auctions` | External one-second run condition; scans auctions only when due | **KEEP**; measure auction-map size later |
| `enforce_duel_outcomes` | Active duel list | **KEEP** |
| `enforce_trade_range` | Active trade sessions | **KEEP** |
| `save_dirty_characters` | Internal 300-tick early return plus `Changed` query | **BUG**; change window is lost |

## Update Matrix

Most `Update` systems are request-driven. The table lists all module groups covering the 105 expanded systems.

| Module | Registered work | Idle shape / verdict |
|---|---|---|
| Achievement | initial snapshot; level achievement update | `Added<InWorld>` / `Changed<Level>` — **KEEP** |
| Admin IPC | `poll_admin_ipc` | Empty channel poll; unbounded drain on bursts — **MEASURE** |
| Auction house | one-time init; open/query/inventory/owned/bid/mail/create/buyout/cancel handlers | Init boolean after first load; receivers otherwise — **KEEP**, cleanup init placement later |
| Barber shop | initial snapshot; query/apply handlers | Added + receivers — **KEEP** |
| Calendar | initial snapshot; query; schedule/signup processing | Added + receivers; query snapshot is drain-first — **KEEP**; initial snapshot and mutation order unchanged |
| Collections | initial state; summon/dismiss mount/pet | Added + receivers — **KEEP** |
| Currency | initial state; earn/spend handlers | Added + receivers — **KEEP** |
| Death service | initial state; query/release/resurrect/healer/stuck handlers | Added + receivers — **KEEP** |
| Duel service | challenge/accept/decline handlers | Receivers — **KEEP** |
| Durability | initial/query; appearance sync; persistence; changed snapshot | Added/Changed; changed snapshot now query-gated — **KEEP** |
| Friends | initial/query/add/remove/presence; auto-AFK; changed/removed presence; dirty snapshots | Receiver/change/dirty work is good; auto-AFK is **OPTIMIZE** |
| Guild | initial state; query/profile/officer-note handlers | Added + receivers — **KEEP** |
| Ignore list | initial/query/add/remove | Added + receivers — **KEEP** |
| Inspect | query handler | Receiver — **KEEP** |
| LFG | status/queue/dequeue/role response; matchmaking advance | Receivers + queue-state check — **KEEP/MEASURE** |
| Networking/auth | chat/history/guild/target; login/register/create/delete/select; gold sync; auth timeout; forced disconnect | Receivers and state timers are acceptable; gold sync is **REMOVE/GATE** |
| Professions | query/craft/gather; pending gather casts | Receivers + active cast entities — **KEEP** |
| PvP | initial state; query/queue/rated/dequeue | Added + receivers — **KEEP** |
| Reputation | initial snapshot | `Added<InWorld>` — **KEEP** |
| Rest | registry init; player rest-state update | Init guard plus unconditional player×innkeeper scan — **REMOVE/GATE** player scan |
| Talents | query/apply/reset | Receivers — **KEEP** |
| Trade service | initiate/accept/decline/cancel/item/money/confirm handlers | Receivers; some pre-drain player lookups — **REMOVE/GATE** pre-work |
| Who | query handler | Unconditional online roster build before request — **REMOVE/GATE** |
| World map | initial snapshot; zone discovery | Added + `Changed<Zone>` — **KEEP** |

## Receiver Fan-Out

Seventy-four recurring `Update` systems directly or indirectly drain typed `MessageReceiver`s. Even with no messages, each system iterates every connection entity matching that receiver query before discovering an empty buffer.

This is widespread recurring work, but **not yet a proven performance defect**. No production measurement records:

- matched connections per receiver system
- non-empty receivers or messages drained
- empty-poll time per system
- total receiver-poll share of `Update`
- scheduler contention from 74 independent systems

Lightyear 0.26.4 supports trigger/event delivery through `RemoteEvent<M>` and `AppTriggerExt`, but converting request/response messages would be a protocol and routing redesign. Do not replace all receiver systems based only on static inspection.

Immediate pre-drain fixes are still justified for Who, calendar, and trade because they remove work without changing delivery semantics.

## PostUpdate and Last

| System | Gate | Verdict |
|---|---|---|
| `forward_position_changes_to_replication` | Dirty queue drain before Lightyear buffer | **KEEP** |
| `despawn_pending` | `PendingDespawn` entities only | **KEEP** |
| `process_npc_kill_reputation` | `Added<DeathState>` | **KEEP** |
| `flush_all_characters_on_exit` | Returns unless `AppExit` exists | **KEEP**; exit-only full player scan |

## Lightyear and Framework Loops

Required recurring plumbing:

- UDP/link receive and send per connected transport
- transport channel buffering
- message receive, send, and cleanup
- ACK/NACK and sender-timer maintenance
- replication dirty-candidate fanout to connected senders

The previously expensive steady-state all-root/all-component replication traversal is gone. Initial sync still visits all roots once for a new/reset sender. Interest no longer performs a recurring entities×players scan.

The vendored replication buffer still performs connection-scaled sender passes, including dirty fanout and `new_spawns` metadata completion. These are not world-root scans and are not proven unnecessary; measure sender counts and idle cost before changing them.

## Already Eliminated or Gated

- Auction expiration: 20 Hz body → one-second cadence
- Terrain streaming: repeated 3×3 send work → tile-transition gate
- Friends snapshots: all-owner rebuilds → dirty affected owners
- Interest: entities×players scan → dirty queues plus spatial buckets
- Lightyear replication: all roots/components per sender → dirty candidates
- Who requests: roster materialization before receiver inspection → drain-first shared roster
- NPC death: living-NPC health scan → authoritative damage transition
- Equal replicated assignments: guarded across proven writers

These completed slices do not prove that the rest of the server has no recurring scans.

## Recommended Order

1. Fix the periodic persistence `Changed<T>` bug and add a real 300-tick-window regression.
2. Remove per-player redb gold polling; make inventory mutation own `Gold` synchronization.
3. Add explicit fixed phase ordering for movement, combat, death, lifecycle, interest, and persistence.
4. Convert trade pre-work to request boundaries; Who is complete in `a4e7aeb`, calendar in `18be416`, and durability change gating in `c46b322`.
5. Instrument receiver fan-out, gravity, AI, interest target fanout, database latency, and fixed catch-up before broader redesign.

## Proof Limits

This is a static source audit plus a focused Bevy 0.18 change-detection reproduction. It proves registration coverage, query/gate shape, the persistence bug, and possible scheduler-order-dependent outcomes. It does not prove whole-server CPU cost, bandwidth reduction, production tick overruns, or the benefit of any proposed optimization.

## Sources

- [crates/server/src/main.rs](../../../crates/server/src/main.rs) — runner cadence and server plugin assembly
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — input, gravity, terrain, Update handlers, and schedule registration
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — unconditional per-player Gold synchronization
- [crates/server/src/who.rs](../../../crates/server/src/who.rs) — drain-first QueryWho handling, shared roster construction, filtering, sorting, and response routing
- `game-server` commit `a4e7aeb` — drain Who requests before building roster
- [crates/server/src/rest.rs](../../../crates/server/src/rest.rs) — initial and change-filtered player/innkeeper evaluation
- `game-server` commit `fffa86a` — rest-state evaluation gate and transition proof
- [crates/server/src/durability.rs](../../../crates/server/src/durability.rs) — initial/query durability handling, persistence, appearance sync, and changed-player snapshots
- `game-server` commit `c46b322` — query-level durability change gating with added-component exclusion
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — attack participation, combat state, and respawn
- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — five fixed AI passes and state branches
- [crates/server/src/death.rs](../../../crates/server/src/death.rs) — all-player death detection
- [crates/server/src/friends.rs](../../../crates/server/src/friends.rs) — auto-AFK and dirty friend snapshots
- [crates/server/src/calendar.rs](../../../crates/server/src/calendar.rs) — drain-first calendar query handling, initial snapshot, and schedule/signup mutation ordering
- `game-server` commit `18be416` — drain calendar queries before building a shared snapshot
- [calendar tests](../../../crates/server/src/calendar.rs) — idle, shared-snapshot, routing, and eligibility proof
- [crates/server/src/trade.rs](../../../crates/server/src/trade.rs) — receiver polling and range enforcement
- [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs) — periodic save gate and `Changed` filters
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — dirty interest and replication forwarding
- [vendor/lightyear_replication/src/send/buffer.rs](../../../vendor/lightyear_replication/src/send/buffer.rs) — dirty candidates and sender metadata passes
- [Cargo.lock](../../../Cargo.lock) — locked Bevy 0.18.0 and Lightyear 0.26.4 versions
- `bevy_ecs 0.18.0 src/system/function_system.rs:445-451,680-684` — system `last_run` advances on execution
- `bevy_ecs 0.18.0 src/query/filter.rs:1083-1094` — `Changed<T>` compares against system `last_run`

## See Also

- [[tick-loop-design]] — proposed fixed phase and cadence contract
- [[tick-model]] — observed schedule topology
- [[recurring-loop-elimination]] — previously completed loop-specific changes
- [[semantic-noop-replication]] — writer-specific no-op replication proof
- [[networking]] — Lightyear, interest, and terrain behavior
