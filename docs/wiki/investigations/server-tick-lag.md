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

## Current full-population profile (2026-10-16)

Private UDP5831 uses a SQLite backup of canonical world data, isolated redb, and no tick/timer/profile changes. Debug `ce4b18cb` (includes official `7f7c8875` and achievement fix `790ce401`) reproduces idle catch-up collapse: slow windows mean280–430ms/frame, 11.6–15.2ticks/s. Seven complete diagnostic windows rank health-percent aura observation23.96ms/fixed-step, detection geometry12.51ms, NPC resize8.42ms, collision sync5.37ms and threat replication2.96ms. Each outer frame can repeat fixed work five times; these are per-step costs, not outer-frame totals.

Release `c55b5e2c` with system-timings and eleven genuine UDP peers instead held19.7–20.3ticks/s for eight complete windows, means18.1–29.4ms/frame, max174.9ms. All eleven entered, then stayed90s. Server CPU39.76s over89.04s sampled wall. Source/profile/host conditions differ, so this is not a paired optimization speedup or plain-release budget acceptance. Use plain release for production and live timing acceptance; debug and trace-enabled release are attribution tools.

Health-percent aura468 observation still scanned every unit and allocated empty watcher vectors. Index watcher membership from `auras_changed`, the shared apply/remove/refresh boundary, and keep a cached filtered query. Removing the last matching aura removes its observations, so a new already-below-threshold aura still fires on application. Crossing direction, thresholds and actual triggered casts remain unchanged. The 250000-unit/five-catch-up regression reproduced 118.707 ms before indexing, then 10.53 µs on merged `6fb65361`; all three targeted controls pass, including actual native 171393→171397/171404 crossings, removal and reapplication. Runtime check passes without warnings. Post-fix debug/system-timings (same private world, no peers) holds 20.0–20.1 ticks/s across eight complete windows, mean work 29.3–59.6 ms and maximum 238.5 ms. Health observation drops out of the top 15; detection geometry (8.91 ms/step), NPC resize (6.50 ms) and collision synchronization (3.95 ms) remain. Variable host load and intervening official changes preclude a controlled speedup ratio; the bounded isolated RED/GREEN demonstrates the specific scan removal.

Evidence: `game-engine/data/diagnostics/server-live-frame-cost-2026-10-16/`, including `debug-idle-*`, `release-independent-*` and `health-watch-red.*`. The peer example is [tick_perf_peers.rs](../../../crates/client/examples/tick_perf_peers.rs). Replicon client receive resources are app-global: eleven connections require eleven independent Apps; sharing one App reproduced a mutate-tick panic, not a server performance failure. No rendered clients or saved auth tokens are used.

Plain release `f4d9164c` (no system-timings), ELF SHA-256 `21434919291142f21e56122a54f94b8228b055b9e9ec0252f6097cdb8ef81764`, confirmed all eleven peers for 90 seconds: eight complete windows at 19.9–20.1 ticks/s, work means 15.7–21.5 ms, maximum 143.7 ms, 18 frames above 50 ms. CPU 31.25 s / 89.04 s sampled wall. This sustains real-time simulation but does not meet an every-frame 50 ms bound. First cold startup exceeded harness readiness; one diagnosed retry completed readiness in 46.6 seconds. Stack capture was denied by ptrace policy; no host protection was changed. Evidence `release-plain-retry-*`; UDP5831 absent after cleanup.

## Remaining geometry scans (2026-10-16)

Follow-up scope covers the measured detection, NPC resize and live-object collision leaders only. Two independent 250000-unit/five-step regressions reproduce 67.268 ms (detection) and 56.780 ms (resize). Excluding `GridDormant` archetypes avoids checking changed flags on the dormant population. A removal observer immediately rebuilds detection geometry and model-derived spatial reach from current components on grid wake, even when dormant changes settled several ticks earlier. Existing notify semantics remain; no simulation timer is changed. Both regressions then take 13.405 µs and 10.2 µs respectively, and all 43 interest/creature-sight controls pass, including reciprocal invisibility after wake and native-model scaled neighbour search.

Live GO collision similarly scans unrelated world objects each fixed step: 250000 copies of an authored nontransport continent object reproduce 99.772 ms for five updates. `CollisionIndexPlugin` classifies relevant objects when their `GameObjectInfo` or `InMap` components are installed/replaced: transport type15 anywhere, arena types0/5/6/35 on catalog arena maps. A component marker restricts the hot query; enabled/visible state, placement, map-copy hull publication and cache cleanup remain live. Metadata/map membership currently change through component insertion, not in-place field mutation. Movement remains a per-step input for actual indexed objects. Standalone collision fixtures install the same index plugin as `GameObjectPlugin`; there is no alternate scanning path. Merged `49f0d64f` passes 50 targeted controls in 55.91 seconds, with the existing explicit microbenchmark ignored. The same 250000-object/five-step regression falls to 9.368 µs; detection and resize are 8.376 µs and 10.53 µs. Actual authored hull tests cover moving transport floors, visibility toggles, copy changes, metadata replacement, despawn, arena waterfall/door states, trapdoor gravity and caster-ground destinations. A finished physics fixture installs the identical observer directly because Bevy forbids adding plugins after `App::finish`; runtime installation remains through `CollisionIndexPlugin`. Runtime check passes without warnings (137.27 seconds). Final debug/system-timings ELF `59451cd21347175d04f02c17843c99f8de34cb4ab4b6c0e185a3f5694619b19b` with protocol `5405246` completes a 90-second idle run at the unchanged 187770-creature census: eight complete windows at 19.9–20.0 ticks/s, work means 17.5–42.9 ms, maximum 91.0 ms. NPC resize and live collision leave the top 15; detection falls to 1.891 ms/step. Remaining leaders are threat replication 3.332 ms/step, aura initialization 1.628 ms and creature health regeneration 1.453 ms. CPU is 56.49 seconds over 89.05 sampled wall seconds. Host load and official source changes vary, so these windows are not a controlled whole-server speedup, nor an every-frame budget pass.

The first final-profile attempt exposed a stale private snapshot lacking the newly required world-quest area `z` column; canonical published data already contained it. A read-only SQLite backup refreshed only the private world snapshot, with schema and `quick_check` receipts; canonical data and private characters were unchanged. Evidence `geometry-private-world-refresh.json`, `geometry-final-{tests,check,debug-build}.*` and `debug-geometry-published-{server.log,provenance.json,ranking.json,cpu-pressure.json}` under the same diagnostics root. All owned runtime processes exited and UDP5831 was absent after cleanup. Plain-release eleven-peer proof above predates these three geometry optimizations; do not relabel it as proof of this later binary.

## Residual idle indexing (2026-10-16)

Follow-up starts on official `d89a3dbc`, protocol master `b701f32`, and preserves gameplay/timer/profile settings. Each of five systems first received a full-population benchmark (250000 entities, 4096 optional-component layouts, warm-up then five updates, fastest of three samples). Layout diversity models the cost of discovering matching archetypes as well as entity iteration; this is not an exact production archetype census. The final isolated-maintenance guard is 100 µs for five empty updates, not a new server tick policy. Initial 1 ms calibration passed three cases, so those three were rerun against the tighter guard before production edits.

| System | RED / GREEN for five idle updates | Boundary |
|---|---:|---|
| Threat membership replication | 15.884 ms / 7.364 µs | unchanged full-population threat tables |
| Detection geometry | 57.796 ms / 6.833 µs | unchanged active entities without observers |
| Aura initialization | 371.826 µs / 7.424 µs | all units already initialized, diverse layouts |
| Creature health regeneration | 610.949 µs / 6.713 µs | dormant timers, diverse layouts |
| JustAppeared | 165.404 µs / 6.152 µs | no pending hooks, diverse layouts |

Aura initialization, health regeneration and JustAppeared rebuilt exclusive-system `QueryState`s each update, rediscovering world archetypes even with no eligible entities. Persist their existing filtered queries as exclusive-system parameters: Bevy updates matched archetypes when topology changes. The actual initialization, timer subtraction/interval/cast/health rules and deferred JustAppeared dispatch ordering are unchanged. The five GREEN benchmarks and 81 additional controls pass on merged `cc6eb7d4` with protocol `b701f32`: 86/86 tests in 273.04 seconds, compilation separate. Controls include active and woken threat reset/re-add, native polymorph and ordinary regeneration/evade timing, aura initialization/passive/cancel behavior, interest and reciprocal visibility, native Kystia/Minion of Zul JustAppeared and mirror respawn-versus-reset effects. Runtime check passes without warnings (73.43 seconds); matching peer build and plain release build pass. The eleven-peer acceptance below uses the same runtime code and pinned protocol.

Threat membership change checks now use the active-grid index instead of every dormant threat table. Grid wake publishes the current table before ordinary visibility/replication, even when its edits occurred several dormant ticks earlier; active direct component edits still use change detection and preserve entry order, empty-list clearing and dirty publication. No threat authority, selection, damage or healing calculations change. A concrete wake/clear/re-add regression accompanies existing aggro and threat controls.

Detection no longer asks `Changed` filters about every active world entity: `observer_sees` consumes full range/facing geometry only for registered observers, and target geometry only for its unit-role flag. Read current components for actual observers and compare values before dirtying visibility; this preserves direct in-place rotation, level and scale edits. Role/network-visibility additions and existing grid wake publish current target classification. Existing remove/readd cleanup remains. The production geometry rows, stealth/invisibility predicates and distance rules are unchanged; all original interest controls remain required.

### Plain release acceptance

Runtime code `cc6eb7d4`, protocol `b701f328d27d485372dc6af48df04ca505e0f707`, bookworm optimized release **without** `system-timings`; server SHA-256 `484b26519d32b31926f4306f1c51a56a2d84bc1bb5f959f2cb62e1e75b3512b6`. Private UDP5831 loads the full published snapshot: 187770 spawned creatures and 45674 game objects. Eleven genuine UDP apps all enter and remain connected for a 90-second measurement; all eleven complete without disconnect/panic. Eight complete reporting windows wholly inside that interval report **20.0–20.1 ticks/s**, window mean work **12.0–23.2 ms**, weighted overall mean **19.93 ms**. This meets the requested 20 Hz / mean-under-25-ms acceptance. Maximum frame work is 92.0 ms (59 frames above 50 ms): this is not an every-frame or p99 guarantee. Sampled server CPU is 31.21 seconds over 85.06 seconds wall.

The first same-binary run is retained, not discarded: overall mean 22.72 ms passed, but a window fell to 17.8 Hz with a 1167.5 ms frame, so it was not called complete. That interval coincided with host full-I/O pressure avg10 above 53% (peaking 60.64%) and full-memory pressure up to 9.82%; these measurements support a resource-pressure correlation, not a proven attribution of every stall. One bounded retry began only after full-memory avg10 ≤1% and full-I/O avg10 ≤5%, alongside ordinary capacity admission, without reserving a quiet window or changing host policy. The retry's preflight was 0.18% / 1.4%; concurrent pressure subsequently rose and all eight window means still met the target. No further source/profile/timer/data reductions were made between runs.

Evidence under the existing diagnostics root: `idle-five-{red,green-controls-corrected,check,peer-build,release-build}.*`, `idle-layout-red.*`, `idle-five-private-world.json`, and both `idle-five-release-{acceptance,retry}-{server.log,peers.log,provenance.json,cpu-pressure.json,summary.json}`. Summaries exclude whole windows touching admission/teardown boundaries rather than selecting fast windows. Private world input was a consistent read-only SQLite backup with `quick_check`; canonical data was untouched. Both runs cleaned their PIDs and left UDP5831 unbound. Later commits change documentation only. The lead's targeted-filter/check/fmt gate is the independent verification; no second verifier is required. Production deployment and broader gameplay/FIFO goals are separate.

## Combat workload staging (2026-10-16, not yet measured)

`tick_perf_peers <private-loopback-endpoint> combat <npc-entry> <spell-id,spell-id>` keeps eleven independent UDP apps, selects each owner's nearest replicated NPC of the explicit entry, approaches with ordinary movement, auto-attacks, then alternates explicitly supplied learned spells every two seconds during the ninety-second all-entered window. No token files or server-stat overrides. Completion requires each owner's received damage and successful-cast events, an aura application/refresh, and authoritative position changes; connection readiness alone is insufficient. The direct Replicon dev dependency reads the owner's entity mapping, not a guessed entity ID.

This is a diagnostic request workload, not a class rotation or balance oracle. A dummy run does not prove hostile AI/Stockade combat. Stockade staging references the maintained engine bot-harness wiki and `/home/osso/.worktrees/bot-harness-2026-10-15/native-stockade-fury15b-release-workflow.pyrun.py`; do not execute its other agent's runtime/port unchanged. Server base4d1facdc/protocol604ea60, private UDP5831. No combat profile or performance improvement claimed yet.
