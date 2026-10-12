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

The objective tick now reads only the two flag homes for each mapped match. No catalog clone occurs when there are no matches; each active match retains the same authored flag positions, roster start, drop and capture logic. Creature activation, respawn clocks and script activation are unchanged. Behavioral regression: `pvp::tests::objective_tick_without_matches_needs_no_world_object_catalog`; release measurements exercise the full production-population boundary. Durable logs and harness: `game-engine/data/diagnostics/server-tick-perf-2026-10-12/`; final numbers in `/home/osso/.worktrees/handoff-server-tick-perf.md`.

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
