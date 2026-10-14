# Server Tick Lag

The :5000 dev server ran its 20 Hz simulation at 55–70% of real time while players were online (2026-09-28/29). Cause: outer frames of the debug build took longer than the 50 ms tick, and under machine load they passed Bevy's 250 ms catch-up clamp, which discards the excess time ([[tick-loop-design]]). Frame cost was dominated by per-tick scans over all ~184 000 spawned creatures and by debug (opt-level 0) codegen of the server crate. Branch `ticklag` removes the scans; the debug build then holds 20 ticks/s with three clients at 15–29 ms per frame on a loaded machine, where the baseline reached 44–64 ms with up to 122 of 200 frames over budget per 10 s.

## Content

### Symptom

`game-server/data/logs-server-20260928d.log` (debug build, :5000): with no player online the `Tick` log advances 200 ticks per 10 s of wall time; with 1–4 players it drops to 0.37–0.78 of real time, for example 00:33:38–00:39:04, 4 000 ticks in 326 s. Players online and machine load (parallel agent builds, Godot clients) varied together, so the log alone does not separate them. Server-time systems slow with the clock: breath timers ([[swimming-and-breath]]), movement banks ([[networking]]), creature chase.

### Instrumentation

- `frame_stats.rs` (always on): every 10 s of wall time, `Frames: <ticks/s> (<steps> steps, <frames> frames in <wall>s), work mean/max ms, <n> over budget`. Work is `First` start to `Last` end of one `App::update`; over budget is work > 50 ms. `ticks/s` below 20 means lost simulation time.
- `system-timings` cargo feature (diagnostic): enables Bevy `trace`, sums wall time inside each `system` span and logs the top 15 every 10 s (`system_timings.rs`). Span bookkeeping itself costs a few ms per frame in debug.
- `eu-stack` sampling of the running server works (perf is blocked); the server's executor is single-threaded (`MinimalPlugins`), so all systems run on the main thread.

Private instances ran on ports 5058/5059 with copies of `game.redb` (`GAME_SERVER_DB_PATH`, `GAME_SERVER_PORT`, `GAME_SERVER_ADMIN_SOCKET`; the socket path must be shorter than `SUN_LEN`, so it lives under `/tmp`). Load: headless Godot clients (`cage`, a scratch fixture extending `tests/world_units_flow.gd`) logged in as `fb_fps` (Stormwind), `fb_worldmap` (Northshire) and `fb_stockade` (Stormwind), walking in circles. A/B pairs ran concurrently on both ports so both saw the same machine load.

### Hot paths found (debug, per frame)

| System | Cost | Cause | Fix |
|---|---:|---|---|
| `sync_unit_powers` | 2.6–3.9 ms | Scanned every creature with `PowerPools`, allocating a `Vec` each | `Without<GridDormant>`; compare in place, allocate only on change |
| `update_auras` | 2.3–5.5 ms | Rebuilt its `QueryState` every frame and ran six world lookups per active unit, most with no auras | Cached `QueryState`; units without auras only advance combat/spec tracking |
| `unit_target::sync_creature_unit_target` | 1.2–2.0 ms | `Changed<CreatureState>` checks every spawned creature | `Without<GridDormant>`, plus `RemovedComponents<GridDormant>` re-derives on wake |
| `regenerate_powers`, `regenerate_creature_mana` | 1.0–1.8 ms each | Scanned dormant creatures | `Without<GridDormant>` (AzerothCore does not update creatures outside active cells) |
| `aggro_on_sight_system` | 0.9–3.7 ms | Faction reaction and level radius computed for every creature × player pair before the distance test | Player positions collected once; map and `MAX_AGGRO_RADIUS` (45 yd) tested first |
| `save_dirty_characters` | ~335 ms every 15 s | Three redb write transactions (map, character, corpse) per character, each a disk sync on the nearly full btrfs volume | One transaction for all characters of a save |
| `update_interest` | 2.2–2.8 ms | SipHash `std` maps keyed by `Entity` | `EntityHashMap`/`EntityHashSet`, foldhash for buckets (→ 1.2–1.8 ms) |
| `send_world_server_info` | 0.3–0.7 ms | Rebuilt its `QueryState` every frame | Cached `QueryState` |

Remaining top costs with three clients (debug, after fixes): `bevy_replicon::server::collect_changes` 1.8–2.4 ms, `wander_system` 1.5–1.9 ms, `update_interest` 1.2–1.8 ms, `update_auras` 0.6–0.9 ms, and ~4 ms spread over ~300 small systems (message receivers). These scale with active (non-dormant) moving creatures in the 3×3 grids around each player ([[creature-ai]]); release codegen makes them small.

### Measurements

Work mean per frame, ms (ticks/s was 20.0 in every steady window unless noted). Machine load came from other agents' builds plus the local Godot clients (three per server; six during concurrent A/B runs).

| Build | Idle | 3 clients, load avg ~6 | 3 clients, load avg 10–14 (concurrent A/B) |
|---|---:|---:|---:|
| Debug, master `8dc7d03` | 8.0–9.4 | 29–44 (up to 20 frames over budget / 10 s) | 36–64 (up to 122 over budget); client entry window 8.7 ticks/s at 576 ms/frame |
| Debug, `ticklag` | 2.4–3.3 | 15–19 (`system-timings` build) | 15–29 (0 over budget); client entry window 113 ms/frame, 20 ticks/s |
| Debug, `ticklag` + `[profile.dev.package.server] opt-level = 1` | 0.9 | — | 7–11 |
| Release, master | 1.7–2.2 | 5.6–6.6 | 9–12 |
| Release, `ticklag` | 0.8 | — | 7–8.5 |

One Stormwind client on the debug baseline cost 17–18 ms against 9 ms idle, so each client in its own area added about 9 ms. Three clients under load 10–14 put the baseline at 64 ms mean; above 50 ms mean the catch-up can no longer keep up, and single frames above 250 ms lose time outright.

Incremental server-crate rebuild (one file touched, loaded machine): debug opt-level 0 1–2.4 min, debug opt-level 1 4.4 min, release 2.2 min (release deps must be built once, ~8 min at `-j 2`).

### Behavior changes

- Dormant creatures (`GridDormant`) no longer regenerate power or mana; they resume on reactivation without catch-up, matching AzerothCore's unupdated cells.
- A dormant creature's `UnitTarget` may be stale until its grid wakes; no player is within interest range of a dormant creature.
- Periodic, shutdown and logout saves write all their characters atomically in one transaction.

### Tests

- `spell_cast::tests::dormant_creature_holds_its_mana_until_its_grid_is_active_again`
- `auras::tests::combat_entered_without_auras_does_not_interrupt_a_later_aura`
- `unit_target::tests::state_changed_while_dormant_reaches_the_target_on_wake`
- `creature_aggro::tests::long_detection_range_is_capped_at_max_aggro_radius`
- `persistence::tests::periodic_save_writes_every_changed_player_with_map_and_corpse`
- `frame_stats::tests::*` — window reset and lost ticks past the clamp

## Release catalog-copy regression (2026-10-12)

Release master `cb1a89e` on private UDP5882 with a SQLite backup of canonical world.db reproduced idle work means 20.9–23.4 ms, maxima 50.7–64.8 ms and 1–2 overruns per 10 seconds after startup. Existing `system-timings` profiling measured `pvp_objective::update_battleground_objectives` at 11.97 ms/run: it cloned the complete `GameObjectSpawns` resource every fixed tick, including when no battleground existed. `perf` was not installed; tracing used a diagnostic release snapshot only.

The objective tick now reads only the two flag homes for each mapped match. No catalog clone occurs when there are no matches; each active match retains the same authored flag positions, roster start, drop and capture logic. Creature activation, respawn clocks and script activation are unchanged. Behavioral regression: `pvp::tests::objective_tick_without_matches_needs_no_world_object_catalog`; existing `maps::tests::ten_warsong_players_accept_over_the_channel_then_ack_one_reserved_map` covers the real roster/transfer/three-capture lifecycle.

Identical harness: restored private redb seed, same SQLite backup, release binaries without tracing, 25-second warm-up, 60-second idle period, three real NetworkBridge peers, 25-second entry warm-up, 60-second online period. Only reporting windows wholly inside a measurement period count (five complete windows each). Peers: level90 warriors at Stormwind map0 (-8833,628,94), Orgrimmar map1 (1631,-4378,30) and authored Valdrakken map2444 (273.76,-1018.95,870.875). Auth/world-entry logs confirm all three connected. Host load varied; these are not sakuin measurements.

| Scope | Before mean / max ms | After mean / max ms | Before / after overruns (50s) |
|---|---:|---:|---:|
| Idle | 9.44 / 31.5 | 3.06 / 12.7 | 0 / 0 |
| Three city peers | 14.70 / 60.5 | 11.84 / 42.5 | 1 / 0 |

The earlier loaded-host reproduction was 20.9–23.4 ms idle; do not present that as the paired baseline or claim production improvement without deploying. Local startup hosted 115356 creatures and 23560 objects; the cloned catalog contained 75205 object spawns, including other map copies. This is the requested canonical local data, not a demonstrated exact production census. Cold entry still spikes (before 553.3 ms, after 677.9 ms), separate from steady simulation; this fix does not bound login/startup latency. Diagnostic after-profile removed objectives from the top15: online leaders were replication change collection ~1.92 ms/run and active wander ~1.80 ms/run, not global idle catalog copies.

Durable logs and harness: `game-engine/data/diagnostics/server-tick-perf-2026-10-12/`; branch/proof ledger in `/home/osso/.worktrees/handoff-server-tick-perf.md`. The peer fixture is [native_tick_perf_peers.rs](../../../../game-engine/godot/network/examples/native_tick_perf_peers.rs).

## Login/catalog/admission investigation (verified 2026-10-13)

Private UDP5831 uses a reflinked world.db, isolated redb and `fb_perf1`/PerfOne; no production operations. The debug diagnostic scenario restores the same `paired-seed.redb`, warms through four frame-report windows, then runs `crates/client/examples/tick_perf_login.rs` for60s. Host contention varies; these sequential runs are not a controlled CPU-load comparison or a release/deployed proof.

| Boundary | Root cause / change | Behavioral proof |
|---|---|---|
| Achievement publication | Multiple final snapshots and Adventure Guide refreshes per frame; deferred final construction preserves every toast | Coalescing, final counters, toast order and reopened contributions |
| Observed spell facts | Full grant passes per simultaneous fact, including unrelated empty criterion trees; batch facts and index counter/dependency candidates |131 imported facts1.0003s before;15.656ms after;71 achievement/account/cascade tests |
| Health aura watchers | Changed filters still traversed idle units; maintain watcher membership structurally |250k units/five passes152.85ms before;7.525µs after; authentic threshold/remove/reapply |
| Admission geometry | Cold WDT/ADT/WMO/M2/LOS/nav IO ran on tick thread; worker prepares surrounding activated NPC/home/waypoint regions, installs one nav tile/frame | Blocked actualWDT FIFO keeps updates running; absent assets admit, corruption fails;22 auth/admission/death/reconnect tests |
| Connection replication | Default-visible bootstrap and disabled diagnostic census scanned hidden world population; default-hidden whitelist, spatial candidates and sparse full sync | Real UDP pre-world privacy, nearby replication, movement and unlink/re-entry;100k hidden NPCs max1.672ms |
| Credentials | Password hash ran synchronously; worker verifies, tick completion rechecks ban/hash before token | Async roster/token and credential-rotation/ban rejection |
| Toy presentation | Source-only availability classification repeated per owner; source revision cache is prewarmed, owner flags remain uncached |8-owner production SpellStore burst85.898ms before;1.812ms after; same-tick source replacement/restoration |
| Sleeping detection | Geometry changes rescanned dormant units; initialize at spawn and refresh on wake |32 interest tests, including source ShadowSight/Prowl facing, dormant mutation, wake and expiry |

Shared protocol `2559d19` adds owned LOS preload/merge and shallow immutable WMO collision cloning, with no wire changes. Two LOS tests prove prepared queries survive later disk corruption and absence differs from parse failure. Two navigation tests prove prepared route endpoints after disk corruption and missing/empty-bake/corrupt distinctions. Empty navigation artifacts are intentional bake outputs, not corrupt headers. Admission never uses a timeout to bypass geometry readiness.

Budget remains **unmet**. Initial debug baseline `11988b33`: admission5.9ticks/s, work mean848.7/max3261.7ms; eventual steady11.0ticks/s, mean455.5/max566.2ms. At `fb2ea6ac`, private entry2.687s and worker1.618s, but online means50–107ms and admission max675.2ms remained over budget. At `05b6b965` (Toy cache), entry7.747s and worker5.743s; online means100–124ms, maxima265–1197ms. Three achievement messages were observed over60s, not three messages in one frame. No claim that host contention or code alone explains the timing difference.

Latest tick-thread maxima: `save_account_progress`223ms, `save_monthly_contribution`120ms, achievement flush376ms, initial Toy publication252ms, periodic character save204ms and economy update967ms. Source-only caching does not remove those durable transactions. Neither reduced durability, a larger tick budget nor altered admission timeouts is implemented. Durable evidence: `game-engine/data/diagnostics/server-tick-perf-2026-10-13/`; `toy-cache-server.log`, `toy-cache-client.log`, `auth-admission-regressions.log`, `interest-final-tests-corrected.log`, `protocol-preparation-tests-corrected.log`, `prepared-navigation-tests.log`. Earlier zero-test filters are not evidence.

### Release confirmation and FIFO work (verified 2026-10-13)

Lead requires private release builds for acceptance; debug timings above are diagnostics only. Bookworm release `a43247f6` private5831: entry13.136s;69-cell worker10.621s; cold entry max1176.1ms. Last four online windows20Hz, mean15.1–17.0ms/max38.9ms. Dedicated commit spans confirm actual tick-thread account commit212.926ms and monthly313.132ms. Logs: `release-current-build.log`, `release-current-server.log`, `release-current-client.log`. Host load is not controlled; no paired release improvement claim yet.

Approved persistence follows pinned TrinityCore `DatabaseWorkerPool::AsyncCommitTransaction` (`src/server/database/Database/DatabaseWorkerPool.h:151–156`, a352b1fa). `GameDatabase::new` now owns an Arc database and one FIFO writer. Account/monthly working contributions are cached per database so later tick reads see enqueued state; owned character/account byte snapshots commit atomically on the worker. Failures log and increment counters; writer drop drains. Pet journal reads no longer create/commit a table during a read.At `f412aa37`,7 targeted FIFO tests pass: real cross-account/monthly contribution ordering, actual blocked-redb monthly enqueue (RED250.629ms), FIFO withholding/drain, failed-job counters, owned periodic saves, subprocess abrupt-exit77 and real HostClient character-delete receipt gating (RED251.878ms).2 token/hash/ban tests plus reopened-account migration and periodic stop-moving/death/corpse fixtures pass. Verified login tokens now commit on the same FIFO before authentication/roster publication. Runtime reports queued/successful/failed/pending job metrics every10s. Trading Post settlement, purchase/refund/freeze and query publication are now queued; busy-writer RED255.980ms initially persisted because `load_achievements` created an absent row synchronously during Guide initialization. Missing inventory/achievement reads are now pure, and legacy achievement normalization persists with the next owned contribution snapshot, not inside a reader. Trading Post account reads are also pure. Trading Post GREEN:6 real settlement/refund/vendor/query cases plus a blocked-reader test pass. Release ea145122 private5831: entry1.516s,69-cell geometry0.993s;20Hz online means9.2–14.4ms, but admission/online maxima120.2/132.3ms still fail50ms. Writer14submitted/14successful/0failed/0pending. Debug codegen is unchanged.

Departing character snapshots now enqueue on FIFO; reentry waits for the older snapshot/fence before loading a new player, preserving failure propagation. Actual blocked-redb reentry RED255.451ms becomes nonwaiting GREEN (`reentry-fifo-green-final.log`). Mail send/actions/expiry run on that writer; `MailSent`, postage criteria, gold and mailbox publications follow receipts. Real send RED251.870ms becomes GREEN and all12 mail regressions pass, including COD, expiry, return, gold cap and reopened postage facts. Accepted trades capture final offers, ignore subsequent edits/cancels/retries while their commit is pending, and publish gold/completion only after success. Real acceptance RED251.070ms becomes GREEN; all27 trade/source/flow cases pass (`trade-fifo-regressions.log`). Auctions now enqueue create/bid/buyout/cancel/expiry and publish the book, gold, criteria and acknowledgements through an ordered completion queue. Workers read the current durable book before each operation. Real create RED252.749ms becomes GREEN (`auction-fifo-green.log`);64 auction/source/atomic/restart/cadence cases pass, with the explicit300k fixture ignored. Commit errors propagate separately from domain refusals into writer error logs/metrics; atomic stale/concurrent callers retain their original refusal outcomes. Economy initialization and cycles share that market queue: a real blocked cycle RED251.548ms becomes nonwaiting GREEN (`economy-fifo-green.log`). Merchant buy/sell/junk/buyback/personal and guild repair now execute owned inputs on the writer; the tick queues requests, polls one ordered merchant receipt and publishes money, buyback, stock, wallet, criteria and durability only after completion. Blocked purchase RED251.249ms becomes GREEN at1d216801; at5347beed,48 of51 merchant tests pass. The three mount currency fixtures fail before purchase: canonical NPC33307 has npcflag1 and gossip_menu_id0, while its TDB menu is10417; the returned gossip has no vendor option. No canonical data changed. Character creation now enqueues creation and starting-kit writes and publishes the roster entry only after success, rechecking the authenticated account at publication. Actual HostClient RED257.818ms becomes GREEN at4739035f. Bank deposits/withdrawals/tab purchase/money/auto-deposit/settings now execute against fresh FIFO rows and publish owner contents and credit after receipts. Real tab purchase RED254.800ms becomes GREEN at2964976a; the bank/guild scope initially had27/28 passes, with the bank credit producer restored independently of AchievementPlugin at8bc770ef and its criterion regression passing. Guild bank operations are being converted through the same owned worker boundary; blocked tab purchase reproduces RED252.918ms. Other synchronous actors and cross-actor publication ordering still require audit; no global FIFO completion, new release acceptance or landable claim.

Seething39 requested delay correlation: score1000 serialized `2026-10-14T00:17:04.988237Z`, native relay `00:17:42.877Z` (37.889s). Server continued9.7–15.5ticks/s in that interval; report maxima400–941ms, not a continuous38s freeze. Native received ordered PvP IDs41/42 at42.859/42.872, remote ticks11941/11477 respectively. This is consistent with an earlier-message/retry head-of-line delay, not proof of the underlying packet-loss/retry cause. Serialization is not UDP-send proof. Evidence: `seething39-tick-correlation.json`, source runtime logs in the bg-coverage slot. No network behavior or timeout change made.

Contract: [server tick budget](../../specs/server-tick-budget.md). Existing [persistence](../systems/persistence.md) boundaries remain authoritative.

## Sources

- [crates/server/src/frame_stats.rs](../../../crates/server/src/frame_stats.rs) — frame work and fixed-step report
- [crates/server/src/system_timings.rs](../../../crates/server/src/system_timings.rs) — per-system timing layer
- [crates/server/src/spell_cast/powers.rs](../../../crates/server/src/spell_cast/powers.rs), [crates/server/src/auras/mod.rs](../../../crates/server/src/auras/mod.rs), [crates/server/src/unit_target.rs](../../../crates/server/src/unit_target.rs), [crates/server/src/creature_aggro.rs](../../../crates/server/src/creature_aggro.rs), [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs), [crates/server/src/interest.rs](../../../crates/server/src/interest.rs), [crates/server/src/maps/difficulty.rs](../../../crates/server/src/maps/difficulty.rs) — fixes
- `game-server/data/logs-server-20260928c.log`, `logs-server-20260928d.log` — :5000 tick logs

## See Also

- [[tick-loop-design]] — 250 ms clamp and bounded catch-up
- [[tick-model]] — schedules and ordering
- [[swimming-and-breath]] — breath timers count fixed-step time
- [[recurring-loop-elimination]] — earlier recurring-scan reductions
- [[persistence]] — save boundaries
