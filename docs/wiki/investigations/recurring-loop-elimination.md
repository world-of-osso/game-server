# Recurring Loop Elimination

The server performance pass is replacing broad recurring scans with state-, event-, or cadence-bounded work while preserving gameplay and protocol behavior. Each loop is handled as an independent tests-first change.

## Status

| Loop | Previous cadence/work | Target | Status |
|---|---|---|---|
| Auction expiration | Scan every auction at 20 Hz | Scan at most once per second | Completed in `e02ce0a` |
| Terrain streaming | Recompute each connection's 3×3 neighborhood at 20 Hz | Evaluate on initial delivery or tile transition | Completed in `3c5ceca` |
| Friends online snapshots | Rebuild all online/friend snapshots every app frame | Rebuild only dirty affected owners | Completed in `914d570` |
| Interest management | Scan every visible entity against every player every 500 ms | Dirty position/lifecycle triggers plus spatial buckets | Completed in `4523373`, tests split in `a85b035` |
| Lightyear replication | Traverse every replicated root/component per client every 50 ms | Dirty global/per-sender candidates with initial full sync and ACK retention | Implemented in `269655a`, `30b5078`; strict-Empty CPU gate pending |

## Auction Expiration

`AuctionHousePlugin` still schedules expiration in `FixedUpdate`, but a local run condition now executes `expire_auctions` immediately once and then at most once per accumulated second. Sub-second fixed ticks do not borrow the auction state or scan its entries.

Expiration semantics are unchanged: expired auctions are removed from persistence, completed sales deliver their existing proceeds/items, and unsold items return through auction mail. A deadline heap was intentionally not added because the requested one-second cadence is the smaller complete solution.

### Behavioral proof

The test `scheduled_auction_expiration_runs_at_most_once_per_second` exercises the actual `FixedUpdate` registration:

1. The first due auction expires immediately and produces seller mail.
2. A second due auction remains through five 50 ms ticks.
3. The second auction expires when accumulated time reaches one second and produces the second mail entry.

The pre-fix test failed when the second auction expired during the sub-second ticks. The focused GREEN test passed after `e02ce0a`.

## Terrain Streaming

`SentTiles` now records the connection's last evaluated tile separately from its set of already-sent tiles. `stream_terrain_tiles` still resolves the player's current tile each fixed tick, but it skips the 3×3 neighborhood loop and sender lookups when that tile is unchanged.

A fresh connection starts with no last tile, so initial world entry and reconnect force evaluation. Movement within one tile preserves the `SentTiles` value and Bevy change tick. Boundary crossings and teleports update the last tile and evaluate the destination neighborhood; the existing tile set still suppresses duplicate `LoadTerrain` messages.

### Behavioral proof

The terrain tests exercise the real streaming system:

1. Initial streaming records the expected 3×3 neighborhood.
2. Same-tile movement preserves both the tile set and `SentTiles::last_changed()` tick.
3. Crossing a tile boundary adds destination tiles and advances the component tick.
4. A fresh connection linked to the same player performs its own initial evaluation.

The pre-fix same-tile test failed because repeated duplicate `HashSet::insert` attempts dirtied `SentTiles`. All three focused tests passed after `3c5ceca`.

## Friends Online Snapshots

`FriendSnapshotIndex` caches the current online presence map and maintains a reverse index from normalized friend name to owners who list that friend. Presence, login/logout, zone, and level changes mark only affected owners dirty. Friend-list mutations replace that owner's reverse-index links; initial `InWorld` snapshots and explicit query requests remain immediate and owner-scoped.

`sync_changed_friend_presence` updates the cached online data from `Changed<Player>`, `Changed<Level>`, `Changed<Zone>`, and `Changed<PresenceStatus>`. `sync_removed_friend_presence` handles logout/despawn. `send_dirty_friends_snapshots` rebuilds and compares snapshots only for drained dirty owners, so unchanged frames no longer rebuild every owner's snapshot or perform offline-character database lookups. Existing equality suppression remains as a final output guard.

Privacy and ignore state are not inputs to friend snapshots in the current protocol and remain unchanged; ignore-list mutations continue to affect only the ignore-list subsystem.

### Behavioral proof

The added friends tests cover the real `Update` chain and index behavior:

- `unchanged_frame_does_not_rebuild_offline_friend_snapshot` changes persisted offline data between unchanged frames and verifies the tracked snapshot and change tick stay unchanged.
- `presence_change_updates_only_owners_listing_that_player` updates only owners listing the changed friend; an unrelated owner's tracker stays unchanged.
- `friend_login_and_logout_update_the_listed_owner` proves online/offline transitions update the listed owner.
- `friend_zone_and_level_changes_refresh_the_listed_owner` proves both live metadata changes refresh the listed owner.
- `friend_list_changes_replace_reverse_owner_links` proves reverse ownership is removed from the old friend and added to the new friend.

## Interest Management

Before `4523373`, `update_interest` ran every 500 ms and evaluated every `NetworkVisibility` entity against every connected player, even when no relevant position or lifecycle state had changed. The replacement keeps a 100-yard X/Z spatial index and marks targets or observers dirty when authoritative position writers, `NetworkVisibility`, or `PlayerLink` lifecycle boundaries change. A fixed-tick system drains the dirty queue, updates bucket membership, evaluates only dirty observers and targets, and checks the exact 3D squared-distance rule against candidates from the observer's 3×3 bucket neighborhood plus previously visible entities.

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

Before `30b5078`, the vendored Lightyear sender traversed every cached replicated root for every sender whenever its 50 ms timer fired, then inspected every replicated component and child. `269655a` vendored `lightyear_replication` `0.26.4` so the sender boundary could change without replacing Lightyear's existing encoder. `30b5078` removes the per-sender all-root/all-component traversal from the steady-state path: a global `ReplicationDirtyQueue` marks entities for all senders, while `mark_for_sender` records connection-specific candidates. The queue is drained into each sender's dirty candidate map before the existing `replicate_entity` semantic pipeline runs.

A new or reset sender performs one initial full sync by queueing every current replication root. After that, unchanged roots are not candidates. Dirty candidates remain queued while their group has pending actions/updates or unacknowledged `SinceLastAck` state, so ACK advancement and NACK retry behavior remain authoritative. Visibility gain/loss, room visibility changes, component inserts/configuration, `NetworkVisibility` removal, hierarchy children, spawn/despawn actions, grouping, priority, delta compression, entity mapping, and wire encoding continue through Lightyear's existing paths.

Authoritative server writers mark real changes rather than relying on a broad `Changed<T>` scan. The marks cover player movement/gravity, NPC movement and leash health restoration, combat status/health transitions, derived stats, gold synchronization, barber appearance changes, friend/presence activity, chat/emote activity, and interest/visibility transitions. The implementation does not claim that every replicated writer is a new feature; it routes existing state boundaries into the sender's candidate queue.

### Behavioral proof

The integration tests use the in-process crossbeam client/server stepper and test-only atomic diagnostics in `lightyear_replication::test_diagnostics`; the counters are not production metrics. The RED idle case measured exactly 160 root visits during five idle send windows over 32 unchanged roots. GREEN reduced both `root_candidates` and `replicate_entity_visits` to zero after initial sync.

Coverage includes initial full replication, changed-position traversal of only the changed root, retention of an unacknowledged dirty root until ACK, sender-targeted visibility gain/loss without visiting another sender, replicated component insert/remove actions, unchanged-root idle behavior, client connection, input delivery, and multi-client routing. The proof covers candidate selection and preserved observable replication behavior; it is not a byte-level wire-count benchmark.

The vendored provenance is recorded in [`vendor/lightyear_replication/UPSTREAM.md`](../../../vendor/lightyear_replication/UPSTREAM.md): version `0.26.4`, crates.io checksum `a451536b73e618035472548e8d7f1af51e193649f556318f7836ddc70e6f73d2`, upstream commit `7082c5eb09463fc110461f1cc928994460611e98`, and MIT OR Apache-2.0 licensing. The remaining strict-Empty gate is live client verification that CPU usage is at or below 10% with zero terrain/world rendering; no `<=10%` result is claimed here.

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
- [friends tests](../../../crates/server/src/friends_tests.rs) — unchanged, affected-owner, lifecycle, metadata, and reverse-index proof
- `game-server` commit `914d570` — dirty-owner friends snapshot implementation and proof
- [interest system](../../../crates/server/src/interest.rs) — dirty position queue, lifecycle observers, spatial buckets, and exact visibility transitions
- [interest integration tests](../../../crates/server/src/interest_tests.rs) — fixed-tick, radius, multi-player, lifecycle, and reconnect proof
- [position writers](../../../crates/server/src/networking.rs) — player movement and replication dirty routing
- [movement implementation](../../../crates/server/src/networking_movement.rs) — position-only interest dirtiness plus rotation/speed replication changes
- [movement tests](../../../crates/server/src/networking_tests/physics.rs) — zero-translation rotation/speed regression proof
- [AI position writers](../../../crates/server/src/creature_ai.rs) — NPC movement dirty marking
- `game-server` commit `4523373` — dirty-position interest implementation
- `game-server` commit `a85b035` — interest integration-test split
- [vendored Lightyear provenance](../../../vendor/lightyear_replication/UPSTREAM.md) — version, checksum, upstream commit, and license
- [dirty candidate queue](../../../vendor/lightyear_replication/src/send/dirty.rs) — global/per-sender marks and lifecycle observers
- [candidate traversal](../../../vendor/lightyear_replication/src/send/candidate.rs) — initial sync, candidate processing, and retry retention
- [sender state](../../../vendor/lightyear_replication/src/send/sender.rs) — ACK/NACK and dirty-candidate retention
- [replication integration tests](../../../crates/integration-tests/src/tests.rs) — diagnostic RED/GREEN and behavior coverage
- `game-server` commits `269655a` and `30b5078` — vendoring and dirty-candidate implementation
- `game-server` commit `aa5dddb` — rotation/speed dirty-candidate follow-up and movement extraction

## See Also

- [[tick-model]] — server fixed-update cadence and lower-frequency gates
- [[persistence]] — auction, inventory, and mail storage
- [[semantic-noop-replication]] — earlier elimination of downstream no-op replication work
