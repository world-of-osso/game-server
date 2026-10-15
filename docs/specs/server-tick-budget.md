# Server tick budget

Keep login, catalog publication and admission from stalling the server's 20 Hz simulation. Sources: `frame_stats.rs`, `system_timings.rs`, `achievement.rs`, `admission_geometry.rs`. [Investigation and measurements](../wiki/investigations/server-tick-lag.md).

## What it must do

- [ ] Keep steady simulation within the existing 50 ms frame budget; login/catalog/admission work must not occupy a frame beyond one budget period. Preserve frame-overrun reporting and tick rate.
- [ ] Attribute slow frames to individual systems, serialized message types/bytes, and tick-thread database operations. With `system-timings` enabled, each system exceeding50ms logs its name and elapsed work immediately, not only in a later aggregate window. Enclosing schedules and pre-execution preparation also report named stalls, covering costs outside system bodies. Deferred command work exceeding50ms names its producing system separately from system execution; do not count it twice in execution aggregates. Slow Main runs emit measured inclusive phase rows (`phase`, `preparation`, `work_ms`) without changing frame accounting; nested parent/child durations overlap and must not be summed.
- [x] Evaluate simultaneously observed achievement facts without rescanning grants for each fact; preserve final counters, earned IDs and every toast.
- [x] Publish one final achievement snapshot and one Adventure Guide refresh per connection per frame; retain every toast and persisted contribution, with no idle replay.
- [x] Health-percent aura observation must not scan idle world populations; preserve threshold crossing, removal, reapplication and apply-below-threshold behavior.
- [x] Keep admission pending while surrounding ground, M2 floors, LOS and navigation are loaded off the tick thread. No successful entry or player activation before preparation completes.
- [x] Missing tile files count as prepared/no geometry; read or parse errors log explicitly and fail admission. No timeout admits unready geometry.
- [x] Connection bootstrap must not enumerate every hidden world entity; visibility changes, unlink/re-entry and component filters must retain the same network results.
- [x] Password verification runs off the tick thread; successful completion rechecks bans and credential rotation before minting a token and authenticating.
- [x] Grant evaluation avoids walking unrelated empty criterion trees, retaining zero-threshold and earned-achievement dependency semantics.
- [x] Toy presentation preflight is shared across owner snapshots and refreshed when source catalogs change; ownership/favourites remain account-specific.
- [x] Sleeping-unit detection geometry is initialized once and refreshed on activation, not rescanned every tick.
- Owned FIFO item-catalog snapshots must not deep-copy the full catalog on the tick. A later catalog mutation must leave each captured request's definitions unchanged.
- Initial creature target synchronization must not rebuild the whole population solely to materialize empty targets; pursuit, confusion/fear and dormant-wake target changes remain authoritative.
- Dormant world populations must not initialize aura state or apply pending spawn-addon auras during cold ticks. Grid activation initializes them in the same fixed step, retaining pending addons and never resetting an already applied aura.

## Release acceptance (lead 2026-10-14)

- [ ] A private release scripted session covering login, combat/loot/quests, trade/mail/AH, guild/currency/achievements and logout records zero synchronous redb commit spans on the tick thread and frame p99 below 50 ms.
- [ ] Land converted writer families in slices with durability-gated acknowledgement proof and release measurements; a slice is not global completion.
- Frame logs report nearest-rank p99 and `tick_redb_commits` for each completed 10-second window, alongside mean, maximum, overruns and tick rate. Each tick-thread commit logs its source location and measured commit work, including commits below50ms. Worker/startup commits do not count. Label per-window percentiles explicitly; do not average percentiles.

## Persistence (approved 2026-10-13)

- [ ] One serial FIFO writer persists owned snapshots off the tick thread, preserving enqueue/commit order across accounts and monthly contributions. Actor polling order must not regress an already committed runtime snapshot.
- [ ] Tick systems enqueue and poll completions without waiting. Logout completion, character deletion, trade, mail/AH money transfers and purchases acknowledge only after their required commit succeeds.
- [ ] World-map discovery writes run on the same FIFO; discovery snapshots wait for their commit and overlapping zone discoveries preserve their union. Missing world-map reads do not create rows or wait for a writer.
- [ ] Changed profession skill/spell state enqueues owned snapshots on FIFO; owner `ProfessionSnapshot` publication waits for its commit. Trainer/crafting transaction conversion is separately required.
- [ ] Friends and ignore-list add/remove requests validate the latest FIFO state; successful owner replies and live list changes follow durability, retaining request order. Friend-presence subscriptions change only with committed list publication.
- [ ] Missing PvP, friends, ignore and collection reads return their existing defaults without creating rows or waiting for the writer; actual mutations persist them. Pet combat-state reads likewise return existing injuries/abilities or the existing empty-state semantics without initializing a table on the tick. Catalog-derived owned pet defaults are initialized in memory only; reads do not persist them. Ability changes and battle outcomes remain responsible for durable mutation.
- [ ] Mount favourite mutations validate fresh durable ownership on FIFO; account caches and success/error snapshots follow receipts. Dependent random selection reads ownership at its FIFO position, then validates/casts after the receipt rather than rejecting an earlier pending favourite. Spell-derived mount ownership also merges on FIFO, coalesces in-flight account/mount grants and publishes after receipts without resetting favourites. Mount teaching-item learning must atomically consume matching fresh FIFO inventory with account ownership; inventory/error replies follow the same-session receipt, while successful account ownership refreshes current account peers even if the learner leaves.
- Housing decor collection from bag use and authenticated redemption must atomically consume fresh durable inventory and merge storage on FIFO; inventory, storage and error publication follow receipts bound to the same account/character/player. Plot purchase likewise queues owned authenticated plot/context, atomically charges fresh FIFO wallet state, and gates owner gold/success/error publication on its receipt. Layout, dyes, access and location writes remain separately required.
- [ ] Appearance refresh grants merge fresh durable account state on FIFO; complete owner snapshots and caches follow receipts. Existing atomic inventory/equipment/disposal grants remain unchanged.
- [ ] Toy favourite changes read fresh durable ownership on FIFO; successful results and account-wide broadcasts follow commit receipts. Toy learning likewise loads fresh durable inventory on FIFO and atomically consumes the matching item with ownership; learning results, inventory publication and ownership credit wait for receipts. Other item consumption writers remain separately required.
- [ ] NPC-kill reputation gains read fresh durable state and commit on FIFO, preserving successive kills and spillover; live reputation and owner snapshots change only after durability. Quest-reward writers remain separately required.
- [ ] Writer errors log and increment an observable failure metric; shutdown drains queued work.
- [ ] Process-crash proof loses only work not acknowledged as durable.

Reference: TrinityCore a352b1fa `src/server/database/Database/DatabaseWorkerPool.h`, `CommitTransaction`/`AsyncCommitTransaction` (lines151–156). Budget acceptance uses only private **release** server builds; debug timings are diagnostics, not acceptance. Dev profile remains unchanged.

## How it works

- [Tick-lag investigation](../wiki/investigations/server-tick-lag.md)
- [Ground assets](ground-assets.md)
- [Achievement publication](dungeon-achievements.md)

## Implementation inventory

- `crates/server/src/frame_stats.rs`, `system_timings.rs` — frame and system diagnostics.
- `crates/server/src/achievement.rs`, `achievement_catalog.rs` — fact evaluation and coalesced publication.
- `crates/server/src/auras/health_pct_trigger.rs` — indexed health watchers.
- `crates/server/src/admission_geometry.rs`, `auth_character.rs` — pending admission and geometry completion.
- `crates/server/src/ground/preparation.rs`, `line_of_sight.rs`, `navigation.rs` — worker-prepared cache installation.
- `crates/navigation/src/query.rs` — staged navigation tile reads and installation.
- Shared-protocol `src/los/map.rs`, `src/ground.rs` — prepared LOS cache merge and immutable WMO placement sharing.

## Tests asserting this spec

- `crates/server/src/achievement_perf_tests.rs` — concrete imported spell facts, persistence, final snapshot and toast delivery.
- `crates/server/src/auras/health_pct_trigger.rs`, `auras/tests.rs` — 250k-unit catch-up budget and authentic aura lifecycle.
- `crates/server/src/admission_geometry_tests.rs` — actual blocked WDT FIFO, absent assets and corrupt asset admission.
- Shared-protocol `src/los/map_tests.rs` — prepared tiles stay usable without rereading changed disk files; missing/corrupt distinction.
- `crates/navigation/src/query.rs` — prepared routes remain usable after disk mutation; absent, valid empty bake and corrupt artifact behavior.
- `crates/server/src/interest_tests.rs` — source stealth/facing/expiry and dormant-mutation/wake visibility.
- `crates/server/src/toys_tests.rs` — production-catalog owner burst and source revision invalidation.

## Known gaps (current cycle)

- [ ] Paired live before/after frame-budget proof on private UDP5831.
- [ ] Remaining synchronous account/catalog hot spots must be measured after geometry preparation.

## Out of scope

Production mutation, tick-rate/budget changes, timeout-based admission and removal/suppression of performance logs.
