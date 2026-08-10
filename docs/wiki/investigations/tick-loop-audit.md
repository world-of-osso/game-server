# Tick Loop Audit

Source-grounded audit of every production `game-server` system registered in a recurring Bevy schedule. It inventories 104 recurring registrations expanding to 130 systems, identifies unnecessary every-tick work, and separates completed reductions from literal requirements that remain partial. Commit `18c936b` replaces broad player-death polling with lethal transitions; no live CPU savings claim is made.

## Content

## Verdict

**The current loop design and loop-removal work are not complete.** The 20 Hz cadence and Bevy/Lightyear schedule split are reasonable, and several historical scans have been narrowed or removed. Current status:

| Area | Verdict |
|---|---|
| Periodic persistence | **BUG:** the 300-tick early return discards most `Changed<T>` history |
| Fixed-step ordering | **INCOMPLETE:** several same-tick outcomes remain scheduler-order-dependent |
| Preserved primary requirements | **0/4 complete:** Lightyear, interest, friends, and terrain are all **PARTIAL** against their literal targets |
| Definite idle work | **10 systems remain:** six trade handlers, chat, emote, auction initialization, and rest initialization |
| Player death polling | **COMPLETED:** `18c936b` replaced the all-player scan with lethal transitions |
| Receiver fan-out | **KEEP/MEASURE:** 74 `Update` systems poll typed receivers; source inspection does not prove removal is safe |

The narrower seven-item application slice is complete: gold, Who, rest-state evaluation, durability snapshots, calendar query snapshots, the `ClearTradeItem`/`SetTradeMoney` trade-offer path, and inactive auto-attacks. That subset is not the completion boundary for this audit.

Do not approve the loop design as complete until the persistence bug, fixed-order contract, ten definite idle-work systems, and four preserved partial requirements are resolved.

## Coverage

A production-source inventory found:

| Schedule | Registration calls | Expanded systems |
|---|---:|---:|
| `Update` | 86 | 104 |
| `FixedUpdate` | 13 | 21 |
| `PostUpdate` | 4 | 4 |
| `Last` | 1 | 1 |
| **Recurring total** | **104** | **130** |

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
- combat/fall damage → player death-transition consumer
- equipment stats → derived stats
- chase → leash → return-to-spawn
- dirty forwarding → Lightyear replication buffer

The following outcomes can be one fixed step late or depend on scheduler order:

| Missing edge | Observable consequence |
|---|---|
| movement → combat | Melee range may use pre- or post-movement positions |
| player death transition | **Resolved by `18c936b`:** combat and fall emit lethal transitions, and the FixedUpdate consumer runs after both |
| movement → terrain streaming | A tile boundary may stream this tick or next tick |
| movement/lifecycle → interest | Visibility may remain stale for one fixed step |
| movement → duel/trade enforcement | Range cancellation may happen this tick or next tick |
| state writers → periodic persistence | A due save may observe pre- or post-writer state |

Conflicting ECS access forces serialization, but serialization order is not a gameplay contract. The phase sets proposed in [[tick-loop-design]] are needed for correctness, not just readability.

### Player Death Transition (`18c936b`)

The former `detect_player_deaths` system scanned every player with `Health` each `FixedUpdate`. Commit `18c936b` removes that broad poll and introduces an internal `PlayerDeathTransition` message. Combat and fall emit it only on a positive-to-zero crossing; persisted zero health queues it during character selection. The consumer is explicitly ordered after combat and gravity.

The consumer preserves the existing lifecycle: clamps zero health, deduplicates already-dead entities, preserves `LastDamager`, applies durability loss, creates `PlayerDeathState` with corpse/graveyard data, marks replication dirty, and sends the routed `"you died"` update. Tests cover stale entities, login-loaded dead characters, combat/fall death, resurrection and second death, and the unchanged death message/lifecycle contract.

RED/GREEN artifacts under `/tmp/claude/game-server/`: `death-idle-scan`, `death-transition-consumer`, `player-combat-death`, `player-fall-death`, `player-login-death`, and `death-second-life-tests`. No live CPU savings claim is made.

## Preserved Primary Requirements

These requirements are evaluated against the literal targets preserved from the original list. Reducing expensive downstream work does not count as removing the recurring invocation.

| Requirement | Implemented reduction | Strict status / remaining work |
|---|---|---|
| Lightyear replication | Dirty entity candidates replace steady-state traversal of every replicated root | **PARTIAL.** The queue stores entities, not entity/component entries; each dirty entity still traverses replicated components, and ordinary writers rely on explicit marks |
| Interest management | Dirty observer/target queues plus 100-yard spatial buckets replace the entities×players scan | **PARTIAL.** No movement revision is modeled, and `update_interest` still invokes every `FixedUpdate` to drain empty queues |
| Friends online snapshots | Idle all-owner rebuilds and database reads are removed | **PARTIAL.** Login/logout/presence/zone/level dirty affected owners, but friend-list mutations rebuild/send immediately instead of marking the owner dirty |
| Terrain streaming | Same-tile ticks skip the 3×3 neighborhood and duplicate sends | **PARTIAL.** Every connection still resolves player position and recomputes tile coordinates each `FixedUpdate` |

## Narrow Remove/Gate Slices

| Priority | System | Previous steady-state work | Verdict |
|---:|---|---|---|
| 1 | `sync_player_gold_from_inventory` | Every `Update`, every player, synchronous redb inventory load | **COMPLETED.** `8920feb` removed the scan; persisted mutation values now flow through `GoldUpdateQueue` and `PostUpdate` |
| 2 | `who::process_query_requests` | Builds/clones the full online-player list before checking whether any request exists | **COMPLETED.** `a4e7aeb` drains requests first, returns idle with zero roster materializations, and builds one shared roster only when requests exist |
| 3 | `rest::update_player_rest_state` | Every `Update`, every authenticated player, linear scan of all innkeepers | **COMPLETED.** `fffa86a` re-evaluates only initial state or `Changed<Position>`/`Changed<Level>` players; add spatial lookup only if profiling justifies it |
| 4 | `send_changed_durability_snapshot` | Every `Update`, every player with durability, then `Ref::is_changed()` in the body | **COMPLETED.** `c46b322` gates the query with `Changed<EquippedDurability>` and retains the `is_added()` exclusion |
| 5 | inactive `AutoAttack` entities | Clearing a target leaves `AutoAttack { active: false }`; fixed combat still queries it forever | **COMPLETED.** `152ccbd` removes `AutoAttack` on target loss; component presence now defines participation |
| 6 | calendar query snapshot | Rebuilds the calendar snapshot before checking for `QueryCalendar` messages | **COMPLETED.** `18be416` drains eligible requests first and builds one shared snapshot only when requests exist |
| 7 | trade offer lookup | Resolves each connection's player before checking whether that typed receiver is empty | **COMPLETED.** `969f2e0` gates `process_trade_offer_updates` with `has_messages`; only `ClearTradeItem` and `SetTradeMoney` are in scope |

### Durability Snapshot Gating

Before `c46b322`, `send_changed_durability_snapshot` entered every `Update`, iterated every player with `EquippedDurability`, and checked `Ref::is_changed()` inside the body. The query now uses `Changed<EquippedDurability>`, eliminating idle per-player visits while preserving the `is_added()` exclusion for newly added durability.

The RED proof is `/tmp/claude/game-server/durability-changed-filter-red.log`; GREEN is `/tmp/claude/game-server/durability-changed-filter-green.log` (**6 passed**). Formatting and readability artifacts are `/tmp/claude/game-server/durability-changed-filter-fmt.log` and `/tmp/claude/game-server/durability-changed-filter-readability.json`. The proof covers unchanged, newly added, changed, routed, and snapshot-content behavior. No live CPU savings claim is made.

### Calendar Query Snapshot Construction

Before `18be416`, `calendar::process_query_requests` built and cloned the sorted calendar snapshot before checking eligible `QueryCalendar` receivers. The fix drains authenticated + `InWorld` receivers first, returns on idle updates, and builds one shared snapshot only when requests exist. All queued empty-payload queries still drain and produce one refresh response per requesting connection. Schedule/signup mutation order and the initial `Added<InWorld>` snapshot path are unchanged.

RED: `/tmp/claude/game-server/calendar-query-drain-red.log`. GREEN: `/tmp/claude/game-server/calendar-query-drain-green.log` (**6 passed**). Formatting/readability artifacts: `/tmp/claude/game-server/calendar-query-drain-fmt.log` and `/tmp/claude/game-server/calendar-query-drain-readability.json`. No live CPU savings claim is made.

### Trade Offer Lookup

Before `969f2e0`, the generic `process_trade_offer_updates` helper resolved each authenticated connection's `PlayerLink` to a `Player` before checking the typed receiver. Idle `Update`s therefore paid the player-query lookup for `ClearTradeItem` and `SetTradeMoney` even when no offer request was queued.

The helper now checks `has_messages()` first. Empty receivers skip player resolution; non-empty receivers drain and apply every queued request in order, resolve the player once per batch, and retain the existing persistence, trade-session, broadcast, routing, and missing-player error behavior. The other six trade receiver systems remain outside this 7-item slice.

RED: `/tmp/claude/game-server/trade-offer-drain-red.log`. The initial GREEN harness-timing failure is recorded at `/tmp/claude/game-server/trade-offer-drain-green-attempt1.log`; final GREEN is `/tmp/claude/game-server/trade-offer-drain-green.log` (**6 passed**). Formatting/readability artifacts are recorded alongside the final proof. No live CPU savings claim is made.

### Who Query Roster Construction

Before `a4e7aeb`, `who::process_query_requests` materialized the full online roster before inspecting `QueryWho` receivers, so repeated idle `Update`s cloned names and allocated snapshot fields without a request. The fix drains every eligible receiver first and keeps only its latest request. If none are pending, the system returns without materializing the roster; when requests exist, it builds one shared roster and clones that snapshot per response.

The protocol contract remains unchanged: only `Authenticated` + `InWorld` connections are eligible; multiple connections are routed independently; multiple requests from one connection produce one response for the latest request; filtering remains case-insensitive substring matching after trimming; results remain sorted case-insensitively by name; responses retain the existing snapshot, `who: N result(s)` message, and error fields. The focused tests also prove ineligible connections produce neither responses nor roster materialization.

RED artifacts: `/tmp/claude/game-server/who-idle-roster-red.log` and `/tmp/claude/game-server/who-request-contract-red.log`. GREEN artifact: `/tmp/claude/game-server/who-drain-first-green.log`. Formatting/readability artifacts: `/tmp/claude/game-server/who-drain-first-fmt.log`, `/tmp/claude/game-server/who-drain-first-readability-audit.json`, and `/tmp/claude/game-server/who-drain-first-readability-metrics.json`. No live CPU savings claim is made.

### Remaining Definite Idle Work

Ten recurring systems still perform source-proven work on idle ticks:

- Six trade request handlers resolve `PlayerLink` → `Player` before checking `InitiateTrade`, `AcceptTrade`, `DeclineTrade`, `CancelTrade`, `SetTradeItem`, or `ConfirmTrade` receivers.
- `collect_chat_messages` resolves sender name and guild metadata before checking `ChatMessage`.
- `collect_emote_messages` resolves sender name before checking `EmoteIntent`.
- Auction and rest initialization remain in `Update` after their one-time load, guarded only by initialized/resource checks.

The initialization guards are cleanup candidates with low steady-state work. Moving them requires explicit startup ordering after database and world-data initialization.

## Correctness-Dependent Polls

These loops are real, but removing them without replacing their transition model would break behavior:

| System | Current scope | Safe direction |
|---|---|---|
| `apply_terrain_gravity` | Every player every fixed step; terrain lookup even when stationary and grounded | Track grounded/sleeping versus airborne/dirty players; horizontal moves and teleports must wake grounding |
| `tick_auto_afk` | Every player every `Update` for a 300-second deadline | Store activity timestamps/deadlines and process due players at a lower cadence |
| `stream_terrain_tiles` | Every connection every fixed step; only the 3×3 work is tile-gated | Wake the system from player tile-coordinate changes instead of probing every connection |
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
| `stream_terrain_tiles` | All connections; same-tile early return after position lookup and tile computation | **PARTIAL / OPTIMIZE**; downstream sends are gated, recurring probes remain |
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
| `process_player_death_transitions` | Drains lethal transitions and resolves only referenced players | **KEEP**; `18c936b` removed the broad player scan |
| `update_interest` | Dirty interest/observer/target queues; unconditional fixed-tick invocation | **PARTIAL** against the literal position-change-only target; expensive visibility work is dirty-driven |
| `expire_auctions` | External one-second run condition; scans auctions only when due | **KEEP**; measure auction-map size later |
| `enforce_duel_outcomes` | Active duel list | **KEEP** |
| `enforce_trade_range` | Active trade sessions | **KEEP** |
| `save_dirty_characters` | Internal 300-tick early return plus `Changed` query | **BUG**; change window is lost |

## Update Matrix

Most `Update` systems are request-driven. The table lists all module groups covering the 104 expanded systems.

| Module | Registered work | Idle shape / verdict |
|---|---|---|
| Achievement | initial snapshot; level achievement update | `Added<InWorld>` / `Changed<Level>` — **KEEP** |
| Admin IPC | `poll_admin_ipc` | Empty channel poll; unbounded drain on bursts — **MEASURE** |
| Auction house | one-time init; open/query/inventory/owned/bid/mail/create/buyout/cancel handlers | Receivers are request-driven; recurring initialized/resource check remains **REMOVE/GATE** cleanup |
| Barber shop | initial snapshot; query/apply handlers | Added + receivers — **KEEP** |
| Calendar | initial snapshot; query; schedule/signup processing | Added + receivers; query snapshot is drain-first — **KEEP**; initial snapshot and mutation order unchanged |
| Collections | initial state; summon/dismiss mount/pet | Added + receivers — **KEEP** |
| Currency | initial state; earn/spend handlers | Added + receivers — **KEEP** |
| Death service | initial state; query/release/resurrect/healer/stuck handlers | Added + receivers — **KEEP** |
| Duel service | challenge/accept/decline handlers | Receivers — **KEEP** |
| Durability | initial/query; appearance sync; persistence; changed snapshot | Added/Changed; changed snapshot now query-gated — **KEEP** |
| Friends | initial/query/add/remove/presence; auto-AFK; changed/removed presence; dirty snapshots | Idle rebuild is removed; friend-list dirty-owner semantics remain **PARTIAL**; auto-AFK is **OPTIMIZE** |
| Guild | initial state; query/profile/officer-note handlers | Added + receivers — **KEEP** |
| Ignore list | initial/query/add/remove | Added + receivers — **KEEP** |
| Inspect | query handler | Receiver — **KEEP** |
| LFG | status/queue/dequeue/role response; matchmaking advance | Receivers + queue-state check — **KEEP/MEASURE** |
| Networking/auth | chat/history/guild/target; login/register/create/delete/select; auth timeout; forced disconnect | Gold scan is removed; chat/emote metadata lookup before receiver checks remains **REMOVE/GATE** |
| Professions | query/craft/gather; pending gather casts | Receivers + active cast entities — **KEEP** |
| PvP | initial state; query/queue/rated/dequeue | Added + receivers — **KEEP** |
| Reputation | initial snapshot | `Added<InWorld>` — **KEEP** |
| Rest | registry init; player rest-state update | Player evaluation is change-gated — **KEEP**; recurring one-time registry guard remains **REMOVE/GATE** cleanup |
| Talents | query/apply/reset | Receivers — **KEEP** |
| Trade service | initiate/accept/decline/cancel/item/money/confirm handlers | Offer-update helper is gated; six other handlers still resolve players before receiver checks — **REMOVE/GATE** |
| Who | query handler | Drains requests before roster materialization — **KEEP** |
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

Who and calendar are drain-first. The shared trade-offer helper is gated for `ClearTradeItem` and `SetTradeMoney`; six other trade handlers plus chat and emote still have source-proven pre-message work. The 74 typed receiver polls themselves remain **KEEP/MEASURE**.

## PostUpdate and Last

| System | Gate | Verdict |
|---|---|---|
| `forward_position_changes_to_replication` | Dirty queue drain before Lightyear buffer | **KEEP** |
| `despawn_pending` | `PendingDespawn` entities only | **KEEP** |
| `process_npc_kill_reputation` | `Added<DeathState>` | **KEEP** |
| `apply_gold_updates` | `GoldUpdateQueue` drain; returns when empty | **KEEP**; mutation-triggered replacement for the removed redb scan |
| `flush_all_characters_on_exit` | Returns unless `AppExit` exists | **KEEP**; exit-only full player scan |

## Lightyear and Framework Loops

Required recurring plumbing:

- UDP/link receive and send per connected transport
- transport channel buffering
- message receive, send, and cleanup
- ACK/NACK and sender-timer maintenance
- replication dirty-candidate fanout to connected senders

The steady-state all-root candidate traversal is gone. Initial sync still visits all roots once for a new/reset sender. The dirty queue is entity-only: each dirty entity still enters Lightyear's replicated-component traversal, so the literal entity/component-queue requirement remains **PARTIAL**.

Interest no longer performs a recurring entities×players scan. Dirty queues and spatial buckets bound the expensive work, but `update_interest` still invokes every `FixedUpdate`; the literal position-change-only execution requirement remains **PARTIAL**.

The vendored replication buffer still performs connection-scaled sender passes, dirty fanout, component checks for dirty entities, and `new_spawns` metadata completion. These are not full-world root scans and are not proven removable; measure sender counts and idle cost before redesign.

## Implemented Reductions and Eliminations

- Auction expiration: 20 Hz body → one-second cadence — **COMPLETED**
- Terrain streaming: repeated 3×3 send work → same-tile early return; per-connection tile probe remains — **PARTIAL**
- Friends snapshots: all-owner idle rebuilds removed; friend-list mutations bypass dirty-owner marking — **PARTIAL**
- Interest: entities×players scan → dirty queues plus spatial buckets; fixed-tick invocation remains — **PARTIAL**
- Lightyear replication: all-root candidate traversal → dirty entity candidates; component-aware queue remains absent — **PARTIAL**
- Who requests: roster materialization before receiver inspection → drain-first shared roster — **COMPLETED**
- Gold synchronization: per-player redb scan → mutation-triggered `GoldUpdateQueue` — **COMPLETED**
- NPC death: living-NPC health scan → authoritative damage transition — **COMPLETED**
- Player death: all-player health scan → authoritative combat/fall/login lethal transitions (`18c936b`) — **COMPLETED**
- Equal replicated assignments: guarded across proven writers — **COMPLETED for audited writers**

These reductions do not establish completion of the four preserved primary requirements or the remaining audit.

## Recommended Order

1. Fix periodic persistence without losing the 300-tick `Changed<T>` window; add a production-order regression.
2. Gate the ten definite idle-work systems: six trade handlers, chat, emote, auction initialization, and rest initialization.
3. Complete the four preserved requirements: component-aware Lightyear dirtiness, position-change-only interest execution, friend-list dirty-owner updates, and tile-change-only terrain execution.
4. Add explicit fixed phase ordering for movement, combat, lifecycle, visibility, and persistence.
5. Measure receiver fan-out, gravity, auto-AFK, creature AI, Lightyear sender work, database latency, and fixed catch-up before broader redesign.

## Proof Limits

This is a static source audit plus a focused Bevy 0.18 change-detection reproduction. It proves registration coverage, query/gate shape, the persistence bug, and possible scheduler-order-dependent outcomes. It does not prove whole-server CPU cost, bandwidth reduction, production tick overruns, or the benefit of any proposed optimization.

## Sources

- [crates/server/src/main.rs](../../../crates/server/src/main.rs) — runner cadence and server plugin assembly
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — input, gravity, terrain, Update handlers, and schedule registration
- [crates/server/src/gold.rs](../../../crates/server/src/gold.rs) — mutation-triggered gold queueing and `PostUpdate` application
- [crates/server/src/who.rs](../../../crates/server/src/who.rs) — drain-first QueryWho handling, shared roster construction, filtering, sorting, and response routing
- `game-server` commit `a4e7aeb` — drain Who requests before building roster
- [crates/server/src/rest.rs](../../../crates/server/src/rest.rs) — initial and change-filtered player/innkeeper evaluation
- `game-server` commit `fffa86a` — rest-state evaluation gate and transition proof
- [crates/server/src/durability.rs](../../../crates/server/src/durability.rs) — initial/query durability handling, persistence, appearance sync, and changed-player snapshots
- `game-server` commit `c46b322` — query-level durability change gating with added-component exclusion
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — attack participation, combat state, and respawn
- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — five fixed AI passes and state branches
- [crates/server/src/death.rs](../../../crates/server/src/death.rs) — player death-transition consumer and lifecycle side effects
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — combat lethal-transition emission
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — fall lethal-transition emission
- [crates/server/src/auth_character.rs](../../../crates/server/src/auth_character.rs) — persisted zero-health transition queue
- `game-server` commit `18c936b` — replace player death polling with lethal transitions
- [crates/server/src/friends.rs](../../../crates/server/src/friends.rs) — auto-AFK and dirty friend snapshots
- [crates/server/src/calendar.rs](../../../crates/server/src/calendar.rs) — drain-first calendar query handling, initial snapshot, and schedule/signup mutation ordering
- `game-server` commit `18be416` — drain calendar queries before building a shared snapshot
- [calendar tests](../../../crates/server/src/calendar.rs) — idle, shared-snapshot, routing, and eligibility proof
- [crates/server/src/trade.rs](../../../crates/server/src/trade.rs) — completed generic offer-update gate plus six remaining pre-message player lookups
- [crates/server/src/messaging.rs](../../../crates/server/src/messaging.rs) — remaining chat/emote metadata work before receiver checks
- [vendor/lightyear_replication/src/send/dirty.rs](../../../vendor/lightyear_replication/src/send/dirty.rs) — entity-only global/per-sender dirty queue
- [vendor/lightyear_replication/src/send/buffer.rs](../../../vendor/lightyear_replication/src/send/buffer.rs) — remaining per-dirty-entity component traversal
- [crates/server/src/trade_tests.rs](../../../crates/server/src/trade_tests.rs) — trade-offer idle, batch, routing, and error proof
- `game-server` commit `969f2e0` — skip idle trade-offer player lookups
- [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs) — periodic save gate and `Changed` filters
- [crates/server/src/interest.rs](../../../crates/server/src/interest.rs) — dirty interest and replication forwarding
- [crates/server/src/admin_ipc.rs](../../../crates/server/src/admin_ipc.rs), [auth.rs](../../../crates/server/src/auth.rs), and [main.rs](../../../crates/server/src/main.rs) — admin, connection lifecycle, runner, and tick-log registrations
- [crates/server/src/achievement.rs](../../../crates/server/src/achievement.rs), [barber_shop.rs](../../../crates/server/src/barber_shop.rs), [collection.rs](../../../crates/server/src/collection.rs), and [currency.rs](../../../crates/server/src/currency.rs) — progression/economy `Update` registrations
- [crates/server/src/auction_house.rs](../../../crates/server/src/auction_house.rs), [duel.rs](../../../crates/server/src/duel.rs), [guild.rs](../../../crates/server/src/guild.rs), and [ignore_list.rs](../../../crates/server/src/ignore_list.rs) — service and fixed-enforcement registrations
- [crates/server/src/inspect.rs](../../../crates/server/src/inspect.rs), [lfg.rs](../../../crates/server/src/lfg.rs), [pvp.rs](../../../crates/server/src/pvp.rs), and [profession.rs](../../../crates/server/src/profession.rs) — request and active-state registrations
- [crates/server/src/reputation.rs](../../../crates/server/src/reputation.rs), [talent.rs](../../../crates/server/src/talent.rs), [world_map.rs](../../../crates/server/src/world_map.rs), and [stats.rs](../../../crates/server/src/stats.rs) — snapshot, changed-state, and fixed-stat registrations
- [crates/server/src/persistence.rs](../../../crates/server/src/persistence.rs) — periodic and exit persistence registration
- [Cargo.lock](../../../Cargo.lock) — locked Bevy 0.18.0 and Lightyear 0.26.4 versions
- `bevy_ecs 0.18.0 src/system/function_system.rs:445-451,680-684` — system `last_run` advances on execution
- `bevy_ecs 0.18.0 src/query/filter.rs:1083-1094` — `Changed<T>` compares against system `last_run`

## See Also

- [[tick-loop-design]] — proposed fixed phase and cadence contract
- [[tick-model]] — observed schedule topology
- [[recurring-loop-elimination]] — implemented eliminations and literal partial reductions
- [[semantic-noop-replication]] — writer-specific no-op replication proof
- [[networking]] — Lightyear, interest, and terrain behavior
