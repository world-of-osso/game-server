# Recurring Loop Elimination

The server performance pass is replacing broad recurring scans with state-, event-, or cadence-bounded work while preserving gameplay and protocol behavior. Each loop is handled as an independent tests-first change. This page documents both complete eliminations and partial reductions among the named slices; it is not an exhaustive claim that all server recurring work is removed. See [[tick-loop-audit]] for the full 130-system inventory.

## Content

## Status

| Loop | Previous cadence/work | Target | Status |
|---|---|---|---|
| Auction expiration | Scan every auction at 20 Hz | Scan at most once per second | Completed in `e02ce0a` |
| Terrain streaming | Recompute each connection's 3×3 neighborhood at 20 Hz | Evaluate on initial delivery or tile transition | **Partial** in `3c5ceca`: repeated 3×3/send work is skipped, but every connection still probes its position and tile each `FixedUpdate` |
| Friends online snapshots | Rebuild all online/friend snapshots every app frame | Rebuild only dirty affected owners | **Partial** in `914d570`: idle rebuilds are removed, but friend-list mutations send immediately instead of marking the owner dirty, and logout lifecycle coverage is not fully explicit |
| Auto-AFK | Scan every player and accumulate idle seconds every `Update` | Activity deadlines; process due players at lower cadence | **Partial** in `16c821c`: all-player scans are removed, but the deadline heap is still polled every `Update` |
| Interest management | Scan every visible entity against every player every 500 ms | Dirty position/lifecycle triggers plus spatial buckets | **Partial** in `4523373`, tests split in `a85b035`: dirty spatial work replaces the broad scan, but the system still runs every `FixedUpdate` and has no movement-revision model |
| Lightyear replication | Traverse every replicated root/component per client every 50 ms | Dirty entity/component queue with initial full sync and ACK retention | **Partial** in `269655a`, `30b5078`: dirty entity candidates replace root traversal, but the queue is not component-aware, dirty entities still traverse replicated components, and the sender still wakes on cadence |
| Who roster materialization | Build the full online-player list before checking `QueryWho` receivers | Drain latest requests first; build one shared roster only when requests exist | Completed in `a4e7aeb`; no live CPU savings claim |
| Inactive auto-attacks | Fixed combat query retains every `AutoAttack { active: false }` after target loss | Remove `AutoAttack` when `CombatTarget` becomes `None`; active attackers remain fixed-step | Completed in `152ccbd`; live CPU gate pending |
| Rest-state evaluation | Every `Update`, every authenticated player, linear scan of all innkeepers | Evaluate initial state or only `Changed<Position>`/`Changed<Level>` players | Completed in `fffa86a`; live CPU gate pending |
| Durability snapshots | Every `Update`, every player with durability, then probe `Ref::is_changed()` in the body | Query-level `Changed<EquippedDurability>` gating while retaining the `is_added()` exclusion | Completed in `c46b322`; live CPU savings claim not made |
| Calendar query snapshots | Build and clone the full calendar snapshot before checking `QueryCalendar` receivers | Drain eligible requests first; build one shared snapshot only when requests exist | Completed in `18be416`; no live CPU savings claim |
| Trade offer lookups | Resolve `PlayerLink` → `Player` before checking `ClearTradeItem`/`SetTradeMoney` receivers | Gate the generic offer-update helper with `has_messages`; resolve once per non-empty batch | Completed in `969f2e0`; other six trade handlers remain outside this slice; no live CPU savings claim |
| Gold persistence synchronization | Every `Update`, every online player, synchronous redb inventory read | Record persisted mutation values and apply them in `PostUpdate` before replication buffering | Completed in `4e4c73d`, `c3b3c04`, `9d2067c`, `91bbedd`, `8920feb`; live CPU gate pending |

## Auction Expiration

`AuctionHousePlugin` still schedules expiration in `FixedUpdate`, but a local run condition now executes `expire_auctions` immediately once and then at most once per accumulated second. Sub-second fixed ticks do not borrow the auction state or scan its entries.

Expiration semantics are unchanged: expired auctions are removed from persistence, completed sales deliver their existing proceeds/items, and unsold items return through auction mail. A deadline heap was intentionally not added because the requested one-second cadence is the smaller complete solution.

### Behavioral proof

The test `scheduled_auction_expiration_runs_at_most_once_per_second` exercises the actual `FixedUpdate` registration:

1. The first due auction expires immediately and produces seller mail.
2. A second due auction remains through five 50 ms ticks.
3. The second auction expires when accumulated time reaches one second and produces the second mail entry.

The pre-fix test failed when the second auction expired during the sub-second ticks. The focused GREEN test passed after `e02ce0a`.

## Who Query Roster Materialization

Before `a4e7aeb`, `who::process_query_requests` built and cloned the full online-player roster on every `Update`, including repeated frames with no `QueryWho` request. Query receivers now drain first; idle Updates return with zero roster materializations, while request-bearing Updates build one shared roster for all pending connections.

The existing contract is preserved: `Authenticated` + `InWorld` eligibility, independent multi-connection routing, latest-request-per-connection semantics, case-insensitive trimmed substring filtering, case-insensitive name sorting, and the existing snapshot, `who: N result(s)` message, and error fields. Ineligible connections receive no response and do not trigger roster construction.

RED artifacts: `/tmp/claude/game-server/who-idle-roster-red.log` and `/tmp/claude/game-server/who-request-contract-red.log`. GREEN artifact: `/tmp/claude/game-server/who-drain-first-green.log`. Formatting/readability artifacts: `/tmp/claude/game-server/who-drain-first-fmt.log`, `/tmp/claude/game-server/who-drain-first-readability-audit.json`, and `/tmp/claude/game-server/who-drain-first-readability-metrics.json`. No live CPU savings claim is made.

## Inactive Auto-Attack Participation

Before `152ccbd`, `start_attack_system` replaced a cleared target's `AutoAttack` with `AutoAttack { active: false }`. The entity therefore remained in the `auto_attack_system` query on every `FixedUpdate`; the inactive branch was semantically empty but still paid ECS query validation and iteration. The fix removes `AutoAttack` when `CombatTarget` becomes `None`, making component presence the participation boundary.

Target acquisition still inserts an active component with the existing zero-second timer. Active swing cadence, target-loss combat grace, damage/death transitions, respawn, and reliable `CombatEvent` broadcast remain unchanged.

The behavioral RED test at `/tmp/claude/game-server/autoattack-inactive-removal-red.log` failed on the retained component. The focused GREEN test at `/tmp/claude/game-server/autoattack-inactive-removal-green.log` passed after `152ccbd`; `cargo fmt` and Rust readability evidence were also recorded. This is a server-side loop elimination; no live CPU or Character-stage claim is made before measurement.

## Rest-State Evaluation

Before `fffa86a`, `update_player_rest_state` visited every authenticated player on every `Update`. Each visit recomputed the innkeeper-radius check and mutated `PlayerRestState` even when `Position` and `Level` were unchanged. The query now admits only players whose `Position` or `Level` changed, plus players without an initial `PlayerRestState`; the registry initialization guard remains unchanged.

Initial world entry still creates and sends the first snapshot. Position changes preserve enter/leave transitions, and level changes recompute rested-XP maximums. Unchanged authenticated players no longer scan the innkeeper list or dirty their rest state.

The behavioral RED test at `/tmp/claude/game-server/rest-dirty-evaluation-red.log` failed because an unchanged update advanced `PlayerRestState::last_changed()`. The focused GREEN test at `/tmp/claude/game-server/rest-dirty-evaluation-green.log` passed after `fffa86a`; cargo fmt and Rust readability evidence were recorded in `/tmp/claude/game-server/rest-dirty-evaluation-readability`. This is a server-side loop elimination; no live CPU or Character-stage claim is made before measurement.

## Durability Snapshot Synchronization

Before `c46b322`, `send_changed_durability_snapshot` ran on every `Update`, visited every player with `EquippedDurability`, and only then called `Ref::is_changed()` in the system body. Idle updates therefore paid the per-player ECS iteration and change-tick probe even when no durability changed.

The query now uses `Changed<EquippedDurability>`, so unchanged players do not enter the system. The existing `is_added()` exclusion remains inside the body: newly added durability is still excluded from the changed snapshot path, while a later mutation sends the existing `DurabilityStateUpdate` with unchanged routing and snapshot content.

The six-test proof covers unchanged players, newly added durability, changed durability, routing, repair-cost snapshot content, and existing durability behavior. RED: `/tmp/claude/game-server/durability-changed-filter-red.log`. GREEN: `/tmp/claude/game-server/durability-changed-filter-green.log` (**6 passed**). Formatting and readability artifacts: `/tmp/claude/game-server/durability-changed-filter-fmt.log` and `/tmp/claude/game-server/durability-changed-filter-readability.json`. This is a server-side loop gate; no live CPU savings claim is made.

## Calendar Query Snapshot Materialization

Before `18be416`, `calendar::process_query_requests` built, cloned, and sorted a complete calendar snapshot before inspecting eligible `QueryCalendar` receivers. Idle `Update`s therefore paid snapshot materialization even when no calendar query was queued.

The system now drains authenticated + `InWorld` receivers first and returns when no requests were queued. When requests exist, it builds one shared snapshot and sends one refresh response per requesting connection. All queued empty-payload queries are drained, preserving the prior one-response-per-connection behavior. Schedule/signup mutation ordering and the initial `Added<InWorld>` snapshot path remain unchanged.

The six-test proof covers idle eligible connections, one shared sorted snapshot across multiple connections and queued requests, and ineligible receivers. RED: `/tmp/claude/game-server/calendar-query-drain-red.log`. GREEN: `/tmp/claude/game-server/calendar-query-drain-green.log` (**6 passed**). Formatting and readability artifacts: `/tmp/claude/game-server/calendar-query-drain-fmt.log` and `/tmp/claude/game-server/calendar-query-drain-readability.json`. No live CPU savings claim is made.

## Trade Offer Lookup

Before `969f2e0`, the generic `process_trade_offer_updates` helper resolved each authenticated connection's `PlayerLink` to a `Player` before checking whether its typed receiver had messages. Idle `Update`s therefore performed unnecessary ECS player-query work for `ClearTradeItem` and `SetTradeMoney`.

The helper now uses `MessageReceiver::has_messages()` first. Empty receivers skip the `PlayerLink` → `Player` lookup; non-empty receivers drain and apply every queued request in order, resolve the player once per batch, and preserve existing persistence, trade-session mutation, broadcast, routing, and missing-player error behavior. The six other trade receiver systems remain outside this named 7-item slice.

RED: `/tmp/claude/game-server/trade-offer-drain-red.log`. An initial GREEN harness-timing failure is retained at `/tmp/claude/game-server/trade-offer-drain-green-attempt1.log`; final GREEN is `/tmp/claude/game-server/trade-offer-drain-green.log` (**6 passed**). Formatting and readability artifacts are recorded alongside the final proof. This completes the narrow seven-item application slice; it does not complete the broader audit or the six other trade handlers. No live CPU savings claim is made.

## Gold Persistence Synchronization

Before `8920feb`, `sync_player_gold_from_inventory` scanned every online player every `Update` and synchronously loaded each inventory from redb. The replacement uses `GoldUpdateQueue`: successful persistence writers record the exact persisted value, same-frame records coalesce by character, and `apply_gold_updates` drains the queue in `PostUpdate` within `ReplicationBufferSystems::BeforeBuffer`. It updates only a matching online replicated `Gold`; equal values do not dirty the component, and offline records are discarded because the next login loads persistence again.

Login/select still initializes `Gold` from persisted character state. The mutation boundaries are explicit: barber payments, completed trades, auction creation/bids/refunds/buyouts, and mail claims record values only after their inventory saves succeed. Profession inventory saves do not mutate gold, and auction sale proceeds remain mail until claimed.

The RED/GREEN artifacts are `/tmp/claude/game-server/gold-update-queue-red.log` and `gold-update-queue-green.log`, `/tmp/claude/game-server/barber-gold-update-red.log` and `barber-gold-update-green.log`, `/tmp/claude/game-server/trade-gold-update-red.log` and `trade-gold-update-green.log`, `/tmp/claude/game-server/auction-gold-writers-red.log` and `auction-gold-writers-green.log`, plus the auction focused pair `auction-gold-update-red.log` and `auction-gold-update-green.log`. Readability artifacts are under `gold-update-queue-readability`, `barber-gold-update-readability`, `trade-gold-update-readability`, `auction-gold-writers-readability`, and `gold-poll-removal-readability`. These proofs establish behavior and removal of the unconditional scan; no live CPU or Character-stage claim is made before measurement.

## Terrain Streaming

`SentTiles` now records the connection's last evaluated tile separately from its set of already-sent tiles. `stream_terrain_tiles` still resolves the player's current tile each fixed tick, but it skips the 3×3 neighborhood loop and sender lookups when that tile is unchanged. This is a **PARTIAL** reduction: duplicate neighborhood/send work is gated, but tile-change-only system execution is not implemented.

A fresh connection starts with no last tile, so initial world entry and reconnect force evaluation. Movement within one tile preserves the `SentTiles` value and Bevy change tick. Boundary crossings and teleports update the last tile and evaluate the destination neighborhood; the existing tile set still suppresses duplicate `LoadTerrain` messages.

### Behavioral proof

The terrain tests exercise the real streaming system:

1. Initial streaming records the expected 3×3 neighborhood.
2. Same-tile movement preserves both the tile set and `SentTiles::last_changed()` tick.
3. Crossing a tile boundary adds destination tiles and advances the component tick.
4. A fresh connection linked to the same player performs its own initial evaluation.

The pre-fix same-tile test failed because repeated duplicate `HashSet::insert` attempts dirtied `SentTiles`. All three focused tests passed after `3c5ceca`.

## Friends Online Snapshots

`FriendSnapshotIndex` caches the current online presence map and maintains a reverse index from normalized friend name to owners who list that friend. Presence, login/logout, zone, and level changes mark only affected owners dirty. Friend-list mutations replace that owner's reverse-index links and immediately rebuild/send instead of marking the owner dirty; initial `InWorld` snapshots and explicit query requests remain immediate and owner-scoped. The idle rebuild is removed, but the literal all-trigger dirty-owner requirement remains **PARTIAL**.

`sync_changed_friend_presence` updates the cached online data from `Changed<Player>`, `Changed<Level>`, `Changed<Zone>`, and `Changed<PresenceStatus>`. `sync_removed_friend_presence` handles logout/despawn. `send_dirty_friends_snapshots` rebuilds and compares snapshots only for drained dirty owners, so unchanged frames no longer rebuild every owner's snapshot or perform offline-character database lookups. Existing equality suppression remains as a final output guard.

Privacy and ignore state are not inputs to friend snapshots in the current protocol and remain unchanged; ignore-list mutations continue to affect only the ignore-list subsystem.

### Behavioral proof

The added friends tests cover the real `Update` chain and index behavior:

- `unchanged_frame_does_not_rebuild_offline_friend_snapshot` changes persisted offline data between unchanged frames and verifies the tracked snapshot and change tick stay unchanged.
- `presence_change_updates_only_owners_listing_that_player` updates only owners listing the changed friend; an unrelated owner's tracker stays unchanged.
- `friend_login_and_logout_update_the_listed_owner` proves online/offline transitions update the listed owner.
- `friend_zone_and_level_changes_refresh_the_listed_owner` proves both live metadata changes refresh the listed owner.
- `friend_list_changes_replace_reverse_owner_links` proves reverse ownership is removed from the old friend and added to the new friend.

## Auto-AFK Deadlines

Before `16c821c`, `tick_auto_afk` visited every player each `Update`, incremented per-player idle seconds, and recomputed the desired presence state. `AutoAfkScheduler` now stores generation-tagged deadlines in a min-ordered heap. Login/presence initialization schedules a deadline; player input and chat/emote activity reschedule it; manual AFK/DND invalidates automatic deadlines; stale generations and despawned players are ignored.

`process_due_auto_afk` peeks the next deadline and queries only players whose deadline is due. Idle ticks no longer scan players or increment timers. The system still invokes every `Update`, so the original lower-cadence target remains **PARTIAL** rather than complete.

Tests cover the 300-second boundary, no early transition, activity invalidation, manual DND, online/offline scheduling, initial presence, despawn safety, zero idle player visits, and duplicate-transition suppression. No live CPU savings claim is made.

## Interest Management

Before `4523373`, `update_interest` ran every 500 ms and evaluated every `NetworkVisibility` entity against every connected player, even when no relevant position or lifecycle state had changed. The replacement keeps a 100-yard X/Z spatial index and marks targets or observers dirty when authoritative position writers, `NetworkVisibility`, or `PlayerLink` lifecycle boundaries change. A fixed-tick system drains the dirty queue, updates bucket membership, evaluates only dirty observers and targets, and checks the exact 3D squared-distance rule against candidates from the observer's 3×3 bucket neighborhood plus previously visible entities. `update_interest` still invokes every `FixedUpdate`, and the queue stores dirty entities rather than movement revisions; the literal requirement remains **PARTIAL**.

Initial `NetworkVisibility` and `PlayerLink` additions register and dirty their state; removal observers clear bucket, observer, and visibility bookkeeping. Player movement, NPC movement, teleports, respawns, and other authoritative position changes call the shared dirty marker. Bucket membership narrows candidates only; it does not replace the exact `distance_squared <= 100²` boundary. Existing per-connection visibility transitions remain intact for multiple players, initial nearby visibility, reconnects, spawns, despawns, radius crossings, teleports, corpse visibility, and stale-state removal. No recurring full entities×players fallback remains.

### Behavioral proof

The integration tests in `interest_tests.rs` cover the production `FixedUpdate` plugin with Lightyear `ReplicationState`:

- `new_nearby_entity_is_visible_on_next_fixed_tick` proves new targets are delivered without waiting for a 500 ms global scan.
- `unchanged_interest_ticks_do_not_dirty_visibility` proves ten unchanged fixed ticks preserve the visibility change tick.
- `dirty_target_crosses_exact_radius_within_same_bucket` proves exact radius gain/loss while bucket membership stays unchanged.
- `dirty_observer_movement_updates_only_its_visibility` proves independent multi-player updates.
- `visibility_removal_and_readd_do_not_leave_stale_range_state` and `reconnect_uses_fresh_observer_visibility` cover lifecycle cleanup and reconnect.
- Existing tests retain exact boundary, teleport, NPC spawn/despawn, two-player movement, corpse, phase, instance, stealth, and large-count behavior.

The implementation landed in `4523373`; `a85b035` moved the integration tests into `interest_tests.rs` without changing the runtime behavior. Focused interest tests passed after both revisions.

## Lightyear Replication

Before `30b5078`, the vendored Lightyear sender traversed every cached replicated root for every sender whenever its 50 ms timer fired, then inspected every replicated component and child. `269655a` vendored `lightyear_replication` `0.26.4` so the sender boundary could change without replacing Lightyear's existing encoder. `30b5078` removes the per-sender all-root candidate traversal from the steady-state path: a global `ReplicationDirtyQueue` marks entities for all senders, while `mark_for_sender` records connection-specific candidates. The queue is entity-only, and each dirty entity still enters the existing replicated-component traversal inside `replicate_entity`; the literal entity/component queue requirement remains **PARTIAL**.

A new or reset sender performs one initial full sync by queueing every current replication root. After that, unchanged roots are not candidates. Dirty candidates remain queued while their group has pending actions/updates or unacknowledged `SinceLastAck` state, so ACK advancement and NACK retry behavior remain authoritative. Visibility gain/loss, room visibility changes, component inserts/configuration, `NetworkVisibility` removal, hierarchy children, spawn/despawn actions, grouping, priority, delta compression, entity mapping, and wire encoding continue through Lightyear's existing paths.

Authoritative server writers mark real changes rather than relying on a broad `Changed<T>` scan. The marks cover player movement/gravity, NPC movement and leash health restoration, combat status/health transitions, derived stats, gold synchronization, barber appearance changes, friend/presence activity, chat/emote activity, and interest/visibility transitions. The implementation does not claim that every replicated writer is a new feature; it routes existing state boundaries into the sender's candidate queue.

### Behavioral proof

The integration tests use the in-process crossbeam client/server stepper and test-only atomic diagnostics in `lightyear_replication::test_diagnostics`; the counters are not production metrics. The RED idle case measured exactly 160 root visits during five idle send windows over 32 unchanged roots. GREEN reduced both `root_candidates` and `replicate_entity_visits` to zero after initial sync.

Coverage includes initial full replication, changed-position traversal of only the changed root, retention of an unacknowledged dirty root until ACK, sender-targeted visibility gain/loss without visiting another sender, replicated component insert/remove actions, unchanged-root idle behavior, client connection, input delivery, and multi-client routing. The proof covers candidate selection and preserved observable replication behavior; it is not a byte-level wire-count benchmark.

The vendored provenance is recorded in [`vendor/lightyear_replication/UPSTREAM.md`](../../../vendor/lightyear_replication/UPSTREAM.md): version `0.26.4`, crates.io checksum `a451536b73e618035472548e8d7f1af51e193649f556318f7836ddc70e6f73d2`, upstream commit `7082c5eb09463fc110461f1cc928994460611e98`, and MIT OR Apache-2.0 licensing. Remaining gates are component-aware dirty entries, exhaustive writer coverage, and live client verification with zero terrain/world rendering; no `<=10%` CPU result is claimed here.

Follow-up `aa5dddb` closed a replication wake-up gap in movement input handling. Turn-in-place and other zero-translation inputs can change replicated `Rotation` or `MovementSpeed`; those changes now return `replicated_state_changed` and enqueue the entity for Lightyear even when `position_changed` is false. `InterestDirtyQueue` remains position-only: it is not dirtied by rotation or speed changes. Movement logic moved into `networking_movement.rs` so the two dirty domains stay explicit. The initial no-op regression test was RED, then the full focused server slice passed with 43 tests GREEN. This follow-up changes candidate coverage, not the interest radius or wire format.

## Sources

- [auction plugin](../../../crates/server/src/auction_house.rs) — one-second expiration run condition
- [auction handlers](../../../crates/server/src/auction_house/handlers.rs) — unchanged expiration side effects
- [auction lifecycle tests](../../../crates/server/src/auction_house_tests/lifecycle.rs) — scheduler-boundary RED/GREEN coverage
- `game-server` commit `e02ce0a` — auction cadence implementation and proof
- [terrain streaming](../../../crates/server/src/networking.rs) — last-tile transition gate
- [terrain connection state](../../../crates/server/src/auth.rs) — sent and last-evaluated tile state
- [terrain streaming tests](../../../crates/server/src/networking_tests/terrain.rs) — initial, unchanged, boundary, and reconnect proof
- `game-server` commit `3c5ceca` — terrain transition implementation and proof
- [friends system](../../../crates/server/src/friends.rs) — cached online data, reverse owner index, dirty fanout, and snapshot sending
- [friends tests](../../../crates/server/src/friends_tests.rs) — unchanged snapshots, affected-owner isolation, lifecycle, metadata, reverse-index, and auto-AFK deadline proof
- `game-server` commit `914d570` — dirty-owner friends snapshot implementation and proof
- `game-server` commit `16c821c` — replace per-player auto-AFK accumulation with deadline scheduling
- [interest system](../../../crates/server/src/interest.rs) — dirty position queue, lifecycle observers, spatial buckets, and exact visibility transitions
- [interest integration tests](../../../crates/server/src/interest_tests.rs) — fixed-tick, radius, multi-player, lifecycle, and reconnect proof
- [position writers](../../../crates/server/src/networking.rs) — player movement and replication dirty routing
- [movement implementation](../../../crates/server/src/networking_movement.rs) — position-only interest dirtiness plus rotation/speed replication changes
- [movement tests](../../../crates/server/src/networking_tests/physics.rs) — zero-translation rotation/speed regression proof
- [AI position writers](../../../crates/server/src/creature_ai.rs) — NPC movement dirty marking
- `game-server` commit `4523373` — dirty-position interest implementation
- `game-server` commit `a85b035` — interest integration-test split
- [combat system](../../../crates/server/src/combat.rs) — `AutoAttack` lifecycle and fixed-step participation
- [combat tests](../../../crates/server/src/combat_tests.rs) — target-loss removal and preserved combat-grace behavior
- `game-server` commit `152ccbd` — remove inactive auto-attack components
- [rest system](../../../crates/server/src/rest.rs) — initial and change-filtered rest-state evaluation
- `game-server` commit `fffa86a` — avoid unchanged player rest-state scans
- [durability system](../../../crates/server/src/durability.rs) — changed-player snapshot gating and added-component exclusion
- `game-server` commit `c46b322` — gate durability snapshots on changed players
- [calendar system](../../../crates/server/src/calendar.rs) — initial snapshot, drain-first query handling, and schedule/signup mutation order
- [calendar tests](../../../crates/server/src/calendar.rs) — idle, shared-snapshot, routing, and eligibility proof
- `game-server` commit `18be416` — drain calendar queries before building a shared snapshot
- [Who system](../../../crates/server/src/who.rs) — drain-first request handling, shared roster, filtering, sorting, and response routing
- `game-server` commit `a4e7aeb` — drain Who requests before building roster
- [vendored Lightyear provenance](../../../vendor/lightyear_replication/UPSTREAM.md) — version, checksum, upstream commit, and license
- [dirty candidate queue](../../../vendor/lightyear_replication/src/send/dirty.rs) — global/per-sender marks and lifecycle observers
- [candidate traversal](../../../vendor/lightyear_replication/src/send/candidate.rs) — initial sync, candidate processing, and retry retention
- [sender state](../../../vendor/lightyear_replication/src/send/sender.rs) — ACK/NACK and dirty-candidate retention
- [replication integration tests](../../../crates/integration-tests/src/tests.rs) — diagnostic RED/GREEN and behavior coverage
- `game-server` commits `269655a` and `30b5078` — vendoring and dirty-candidate implementation
- `game-server` commit `aa5dddb` — rotation/speed dirty-candidate follow-up and movement extraction

## See Also

- [[tick-loop-audit]] — exhaustive remaining-loop and correctness audit
- [[tick-model]] — server fixed-update cadence and lower-frequency gates
- [[persistence]] — auction, inventory, and mail storage
- [[semantic-noop-replication]] — earlier elimination of downstream no-op replication work
