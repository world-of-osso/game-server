# Dungeon simulator

Transport-free Stockade qualification runs real gameplay systems in a plain Bevy App in the server test executable. The game-server entry point is never run. [Implementation](../wiki/systems/dungeon-simulator.md).

Simulator party: Garrick (Protection), Crenna (Restoration), Shuja (Elemental), Meredy (Fire), Austin (Beast Mastery) as the in-process leader using the same kit controller. This user-approved party replaces the prior live Fury72 owner; no Fury APL is recreated.

## What it must do

- [x] Freeze map-34 Normal spawns grouped by the tank planner's source formation identity into `content/dungeons/stockade-34.json`; retain source GUIDs, entries and 2D positions.
- [ ] Extract inter-pack walk distances once using the existing nav query. Runtime movement consumes only the frozen graph; aggro and range use 2D positions.
- [x] Spawn through the production map-population path, preserving its Normal difficulty, formation, pool and source-identity semantics; a hand-built population must not bypass those tags.
- [ ] Run real spell casting, auras, damage, threat, creature AI, full follower kits and tank pull planning in-process, with manually stepped time and direct intents/callable handlers, without networking plugins or transport.
- [ ] Clear every Stockade boss in seconds of CPU, targeting at least 100 simulated seconds per wall-second. Report ticks, setup/update/control/observation wall costs, simulation ratio, live entity/event counts and process memory in periodic checkpoints. Report cleared/wiped, failed pack, HP/mana traces and pull decisions; terminal source-unit state and complete native combat-event order must make stalls diagnosable. Keep all admitted members in samples after native dead-summon despawn: record absence and unavailable HP/mana/position explicitly, never invent values or abort the batch. Unrelated NPC fights/regen and unexecuted movement requests do not establish party progress. Actual party HP or mana gains do establish progress, including OOC rest; capped resources alone do not. Flag a stall only after60s without party damage, actual movement or resource gain.
- [x] Use optimized `sim` profile (release inheritance with debug assertions), and reuse loaded immutable catalogs across fresh seeded worlds without per-run catalog clones.
- [ ] Batch N seeded runs and summarize per-pack wipe rate. Report runs reaching each pack as the denominator, wipes separately from all failed clears (including stalls/blocks/timeouts), and keep each run's HP/mana/pull trace. A batch that executed successfully is not a clear unless all runs cleared.
- [ ] Enforce `SCRIPTED_BOSS_PULL_WHEN_READY` and `CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK` from the [follower policy](follower-dungeons.md): no numeric forecast may strand a cleared, full-ready party before its next single pack. No overlap or chained adds on that admission.
- [ ] Preserve approved tank policy: ten active-combat seconds of current-run DPS/healing measurements, retained through OOC travel/recovery/holds and reset at dungeon start or full wipe; separate five-second health stability remains wall-clock. Preserve 30% HP reserve, first single pack, affordable sourced heal, every member above 80% across five seconds before overlap, static spawns until observed dead, caster collection and hazard reaction.

## Authored route

Stockade's committed `route` lists a unique99-hostile-pack sequence and graph-source 2D anchor for every pull (friendly guards, prisoners and noncombat stalkers remain topology nodes, not pulls). The authored order comes from a rooted walk of the frozen topology; runtime never chooses the nearest pack or regenerates order. Only pull pace/chaining varies. Handle accidental combat first, then skip already-dead packs in the route. Opening qualification must prove actual accidental engagement, positive damage, native Death and route skipping, not forbid accidental aggro. Anchors remain available if the tank despawns; no native opener is fabricated for an absent tank.

## Native pet utility

Production `PetControlPlugin` initializes real `PetLevelupSpells` and runs normal pet actions/AI. The immutable catalog moves between cached seeded worlds; pet commands, owners and cast state never do. Native Felguard30146 must expose learned Axe Toss89766 at the selected level in both first and cached worlds. No player spell grant or invented Hunter pet is permitted. Harness initialization omissions cannot qualify production utility coverage.

## How it works

- [Simulator](../wiki/systems/dungeon-simulator.md)

## Implementation inventory

- `crates/server/src/dungeon_simulator/mod.rs`: targeted qualification entry points.
- `crates/server/src/random.rs`: explicit test-only seeded scope over unchanged native rand distributions; normal production execution reexports rand unchanged. Gameplay RNG call sites share this scope, and simulator schedules run single-threaded.
- `crates/server/src/dungeon_simulator/runtime.rs`: plain manually clocked gameplay App and production `zones::spawn_map_creatures` population, with replication disabled.
- `crates/server/src/dungeon_simulator/campaign.rs`: full-population controller, real party-progress observation, periodic profiling checkpoints and HP/mana/pull/death report.
- `crates/server/src/dungeon_simulator/diagnostics.rs`: read-only source-unit snapshots and process memory observations.
- `crates/server/src/dungeon_simulator/movement.rs`: source-speed pursuit/follow/pet movement timed along frozen edges, never straight-line travel time.
- `Cargo.toml` `[profile.sim]`: optimized batch execution with native debug assertions.
- `scripts/run-dungeon-simulator.py`: execute the compiled filtered test from repository root, with explicit output directory, run count and starting seed.
- `crates/server/src/dungeon_simulator/batch.rs`: seeded collector, per-pack wipe/failure counts and summary JSON.
- `crates/server/src/followers/simulator.rs`: authentic party initialization and direct leader/kit/pull-admission seam.
- `crates/server/src/dungeon_simulator/graph.rs`: typed frozen graph and explicit one-time native-nav extraction; minimum spanning forest plus four nearest neighbours per pack.

## Tests asserting this spec

- `dungeon_simulator::frozen_stockade_graph_covers_native_formations`: exact Normal spawn GUID/formation census (GREEN).
- `dungeon_simulator::one_pack_runs_real_gameplay_without_transport`: isolated native pack death with real positive damage events (GREEN; execute from repository root).
- `dungeon_simulator::full_stockade_clears_all_three_bosses`: full-route completion and three boss deaths (RED).
- `dungeon_simulator::batch::batch_summary_counts_wipes_per_reached_pack_and_retains_stalls`: concrete aggregation with a clear, wipe and stall (GREEN).
- `dungeon_simulator::campaign::spawned_normal_population_allows_existing_fight_tank_movement`: native existing-fight consumer accepts spawned population metadata (missing-difficulty RED→GREEN50cc09c7e).
- `random::same_seed_replays_native_random_draws`: repeated native draw sequences and different-seed control (GREEN).
- `followers::pull_history_tests::tank_pull_history_preserves_active_combat_window_across_idle_and_resets_on_wipe`: real combat/death/OOC ordering, active-window rollover and wipe reset (GREEN; full history family5/5).

- `dungeon_simulator::catalogs::seeded_apps_reuse_catalog_allocations_but_reset_dynamic_state`: exact spawn/spell allocation reuse with fresh clocks, actors and event queues (RED→GREEN6.39s).
- `dungeon_simulator::campaign::sampling_retains_a_member_after_native_dead_summon_despawn`: actual kill→native corpse decay→absent actor sampling (RED→GREEN11.23s).

## Known gaps (current cycle)

- [x] Graph extraction and isolated one-pack RED/GREEN: pack20375759, 5.70 simulated seconds, 221 real combat events. This fixture limits the source population to that pack; it does not prove full-route admission.
- [ ] Complete route, seeded batches and low-wipe qualification.
- [ ] Final live acceptance after simulator qualification.

## Out of scope

- Runtime navmesh, terrain, LOS, ground, physics, Godot, server process, UDP/lightyear transport and client connections.
- Simulator success alone is not live 3D acceptance.

## Parallel seed runner

`scripts/run-dungeon-simulator.py --jobs N` starts one independent native test process per seed, with at mostN active. Default jobs is bounded by requested seeds and half the estimated idle CPU capacity, at least1. Every process receives the same explicit seed regardless of concurrency and writes into the shared output directory. Per-seed logs retain native output. After every worker exits, merge all reports into an ordered `summary.json` with native reached-pack/wipe/failure accounting and total batch wall time. Preserve failed-clear reports; failed workers or missing reports must fail the command, never reuse old successful receipts. Catalogs remain process-local; parallel execution does not claim cross-process reuse.

## Explicit simulator comparison overrides

Defaults stay level32 and roster209057/209072/209053/209059/209065. `DUNGEON_SIMULATOR_LEVEL` selects a native supported level. `DUNGEON_SIMULATOR_HEALER_ENTRY=215011` replaces Crenna with sourced Holy Henry. `DUNGEON_SIMULATOR_PARTY_ENTRIES=tank,healer,dps,dps,leader-dps` selects five existing follower profile entries in that order; lastDPS uses the leader's existing native-kit seam. Explicit healer override replaces the list's healer slot. Reject unknown profiles, wrong roles, malformed/incomplete lists and invalid levels; do not invent profiles or stats. All resources and class/spec progression still come from the unchanged production initializer. Overrides are inherited by every isolated seed process.
