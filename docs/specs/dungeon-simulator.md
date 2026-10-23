# Dungeon simulator

Transport-free Stockade qualification runs real gameplay systems in a plain Bevy App in the server test executable. The game-server entry point is never run. [Implementation](../wiki/systems/dungeon-simulator.md).

Simulator party: Garrick (Protection), Crenna (Restoration), Shuja (Elemental), Meredy (Fire), Austin (Beast Mastery) as the in-process leader using the same kit controller. This user-approved party replaces the prior live Fury72 owner; no Fury APL is recreated.

## What it must do

- [x] Freeze map-34 Normal spawns grouped by the tank planner's source formation identity into `content/dungeons/stockade-34.json`; retain source GUIDs, entries and 2D positions.
- [ ] Extract inter-pack walk distances once using the existing nav query. Runtime movement consumes only the frozen graph; freeze native entrance/node heights and preserve full3D nearest-node, range and aggro distances for every dungeon. No runtime geometry services.
- [x] Spawn through the production map-population path, preserving its Normal difficulty, formation, pool and source-identity semantics; a hand-built population must not bypass those tags.
- [ ] Run real spell casting, auras, damage, threat, creature AI, full follower kits and tank pull planning in-process, with manually stepped time and direct intents/callable handlers, without networking plugins or transport.
- [ ] Clear every Stockade boss in seconds of CPU, targeting at least 100 simulated seconds per wall-second. Report ticks, setup/update/control/observation wall costs, simulation ratio, live entity/event counts and process memory in periodic checkpoints. Report cleared/wiped, failed pack, HP/mana traces and pull decisions; terminal source-unit state and complete native combat-event order must make stalls diagnosable. Keep all admitted members in samples after native dead-summon despawn: record absence and unavailable HP/mana/position explicitly, never invent values or abort the batch. Unrelated NPC fights/regen and unexecuted movement requests do not establish party progress. Actual party HP or mana gains do establish progress, including OOC rest; capped resources alone do not. Flag a stall only after60s without party damage, actual movement or resource gain.
- [x] Use optimized `sim` profile (release inheritance with debug assertions), and reuse loaded immutable catalogs across fresh seeded worlds without per-run catalog clones.
- [ ] Batch N seeded runs and summarize per-pack wipe rate. Report runs reaching each pack as the denominator, wipes separately from all failed clears (including stalls/blocks/timeouts), and keep each run's HP/mana/pull trace. A batch that executed successfully is not a clear unless all runs cleared.
- [ ] Enforce `SCRIPTED_BOSS_PULL_WHEN_READY` and `CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK` from the [follower policy](follower-dungeons.md): no numeric forecast may strand a cleared, full-ready party before its next single pack. No overlap or chained adds on that admission.
- [ ] Preserve approved tank policy: ten active-combat seconds of current-run DPS/healing measurements, retained through OOC travel/recovery/holds and reset at dungeon start or full wipe; separate five-second health stability remains wall-clock. Preserve 30% HP reserve, first single pack, affordable sourced heal, every member above 80% across five seconds before overlap, static spawns until observed dead, caster collection and hazard reaction.

## Generic source construction

- [x] Prepare new-map source topology without a compiled map registry: retain exact Normal actor GUIDs, formation identity, native float poses, follower205 MapDifficulty tuning and unique authored Entrance. Order explicit `raid_encounter_creature` bindings by DungeonEncounter then source JournalEncounterCreature order.
- [x] Missing/ambiguous entrance, follower difficulty, encounter boss binding, source boss spawn, nav bake or complete native boss walk progression must produce a named `FOLLOWER_GRAPH_*` refusal; never fabricate an actor, route, radius or walk connection.
- [ ] Ruby2521 and Halls2527 must qualify native edges and corridor-input radii, then run the ignored source-map terminal fixture. Halls currently refuses missing final encounter2618 binding; Ruby refuses the measured native Melidrussa→Kokia walk disconnection. Ruby joint-actor completeness remains explicitly limited to published raid bindings.
- [ ] Existing34/2515 graph bytes and behavior must remain unchanged. New-map source preparation never rewrites their files or substitutes their radii.

Implementation: `scripts/follower_simulator_sources.py`; behavioral native-source fixtures: `scripts/tests/test_follower_simulator_sources.py`; ignored terminal proof: `crates/server/src/dungeon_simulator/generic_map_tests.rs`. Generic dispatch/metadata remains the separate existing `graph_selection.rs` interface; Uldaman capture remains separately owned.

## Pet diagnostic evidence

Each regular party sample also records actual controlled pets, their owner/entity/template/level provenance, HP, position, CombatRefs, threat and cast/motion state. Record related opponents even when initial acquisition has zero threat. Retain those diagnostic identities after pet Death, enemy evade/home and despawn so reset transitions and unavailable state remain observable; never fabricate missing HP/position. Terminal unit snapshots include those pets. These observations do not change party admission, threat, AI or healer policy.

## Abort evidence

A gameplay panic or initialization failure must return an explicit `aborted` result with its original reason instead of losing the run's evidence. Final output always includes `run-SEED.json` and `combat-events-SEED.json` with status/reason and the ordered native events captured before termination. Horizon completion keeps its `timeout` reason and exports the same trace. Initialization failure before a retained gameplay world is identified explicitly; never invent combat events, unit state or boss credit. Filesystem write errors remain explicit failures. This covers recoverable Rust unwind/error boundaries, not process kill, hardware failure or abort-on-panic builds.

## Azure qualification

- [x] Freeze the staged2515 native graph, complete Normal source population and an authored Leymor→Azureblade→Telash→Umbrelskul route. Difficulty205 retains its native Normal-population alias.
- [ ] Execute the same [Azure translocation policy](azure-translocation.md) as production when the authored route crosses a disconnected component; freeze directed teleport edges, never invented walk connectors.
- [ ] Run Azure seeds1–10 with `--jobs`, unchanged Garrick/Crenna/Shuja/Meredy/Austin roster and native205 ContentTuning2155 maximum level70; report every clear/wipe/stall/block and root cause.
- [x] Rerun Stockade seeds1–10 after generic source-height preservation: native-height batch12 all10 clear/30boss Deaths/0failures.

## Data-driven map dispatch

- [ ] Accept any selected map with exactly one valid `content/dungeons/*-<map_id>.json` graph and explicit source difficulty/boss order in `route-requirements.json`. Keep the existing map34 CLI default; invalid, missing, ambiguous or mismatched selections fail explicitly instead of selecting another map.
- [ ] Preserve existing frozen graph bytes and route order. Difficulty and required bosses come from data, never map-specific Rust branches; all actors in shared encounters remain required. Native corridor capture uses this same dispatch without changing its eligibility/radius calculation.

## Uldaman frozen source-route packet

- [x] Freeze map2451's same-map Normal population and native formation identities/heights in `content/dungeons/uldaman-2451.json`, using the existing Stockade/Azure graph and `FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR` format. Native NPC patrols are not party progression routes.
- [x] Preserve authenticated encounter order LostDwarves→Bromach→Talondras→Emberon→Deios. Within the shared first encounter, retain all three actors in authenticated journal order Baelog184581→Eric184582→Olaf184580; all must be cleared before Bromach184018, then184124/184422/184125. No representative or centroid substitutes for an actor.
- [ ] Qualify pull selection with actual native eligibility/acquisition radii for the unchanged default party at sourced map ContentTuning2938 maximum level70; every Normal source GUID remains represented. Never infer radii from faction proxies or another dungeon. The [authored route](#authored-route) fixes order, permits only pace variation, always advances from cleared state and skips route packs already observed dead after accidental combat.
- [ ] Deliver source/nav/output hashes and qualification receipt with the frozen data. Generic map2451 simulator dispatch/capture is agent14305-owned separate scope; retain its explicit gap until supported. A topology-only or empty-route capture input is not a qualified frozen pull profile.

## Clearable opponents

- [ ] Use the ordinary native enemy-target relation for every party member: an alive source that no living same-copy member can attack must not hold the history or next-pull controller forever. Do not mark it dead or invent a per-entry passive exclusion.
- [ ] Preserve pre-impact combat holds for valid native combat references, even before the first damage event. Unattackable-only references do not veto clear readiness.
- [ ] Preserve actual damage/healing measurements, fatal-hit output and observed Death receipts while filtering clearable-opponent identity.

## Boss-path corridor authoring

`FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR` (user,2026-10-21): freeze a deterministic route per map. Visit Azure bosses in Leymor→Azureblade→Telash→Umbrelskul order and use the qualified books for translocation hops. Preserve Stockade's qualified Moloch→Hogger→Overheat encounter order and completion equivalence.

- [ ] Compute shortest paths on the frozen native-height graph, with directed zero-distance teleport edges; a teleport exposes no intervening physical corridor.
- [ ] Plan only packs whose native acquisition sphere intersects a walked path segment, plus sourced boss-gate packs. Skip other packs. Qualification uses the map's fixed baseline party/level, never a per-seed route recomputation.
- [ ] Source Leymor's three191164 tenders/formation29001816 as an explicit prerequisite before the boss; do not clear every post-boss wing first.
- [ ] Keep the1h diagnostic bound. Prove Stockade seeds1–10 and report Azure seeds1–10 after generic route integration.

## Authored route

Stockade's committed `route` lists a unique59-pull boss-path corridor and full3D anchor for every pull (friendly guards, prisoners and noncombat stalkers remain topology nodes, not pulls). Azure's route lists66 pulls. Deterministic authoring uses shortest frozen boss paths, native acquisition spheres and sourced gates; runtime never chooses the nearest pack or regenerates order. Only pull pace/chaining varies. Handle accidental combat first, then skip already-dead packs in the route. Within a source formation, a dead or unselectable leader must not strand remaining living opponents: approach the selected living member's actual position, retaining source formation identity and the initial leader's frozen corridor anchor. The subsequent pull footprint contains only living formation members: an observed-dead leader must not be resubmitted as a missing source GUID after approach. Opening qualification must prove actual accidental engagement, positive damage, native Death and route skipping, not forbid accidental aggro. If no pullable pack remains while a required boss is alive but temporarily unattackable, hold until it becomes attackable again; never invent boss credit. Other pullable packs still take precedence. A required boss permanently absent without an observed Death remains an explicit error. Anchors remain available if the tank despawns; no native opener is fabricated for an absent tank. The autonomous DPS leader obeys the same non-tank native idle-sight approach hold as its companions, even though it carries `LeaderProfile` rather than `Follower`. Tank exemption follows the actual follower role or active specialization, not the presence of that component. Non-tank sight qualification follows the actual complete movement path, not its endpoint chord: frozen graph legs in the simulator, authored navigation in production. Pending or unavailable path geometry holds movement; it never declares an unsafe or unknown approach clear.

## Mid-edge movement

Moving or following a target partway along a frozen walk edge must use only the remaining native walk distance. Neither the movement path nor its sight/admission qualification may overshoot the target through its original pack anchor, or send a mid-edge mover back to a newly nearest pack node. Qualification and movement must describe the same walked legs and retain native3D heights. Existing disconnected components and directed book hops remain disconnected for ordinary walking; never invent inter-component walk links.

## Native evade movement

An NPC chase without a walking path must hold its actual position and use the existing server cannot-reach/evade/home-arrival lifecycle, not abort the process or cross a disconnected component. Preserve the native timeout and map/state conditions; do not invent a Rune-specific exception or a new clock.

Evading actors must travel frozen edges at their native speed to the native home destination (explicit HomePosition, then escort home, then spawn). Arrival must leave Returning, stop motion, restore home facing and run the same native mana/health/addon/pose/JustReachedHome behavior as production; regenerating actors do not receive an invented instant full heal. No runtime physical-navigation system may be installed to satisfy arrival.

## Native pet utility

Production `HunterPetPlugin` and `PetControlPlugin` load native pet family/specialization/learned-spell definitions and run normal pet lifecycle/actions/AI. Immutable catalogs move between cached seeded worlds; stables, summoned pets, commands, owners and cast state never do. A transport-free leader retains the ordinary runtime-only stable initialized while it has follower identity; never invent a persistent character or transport connection to summon its pet. Native source templates remain required. Native Felguard30146 must expose learned Axe Toss89766 at the selected level in both first and cached worlds. No player spell grant or invented Hunter pet is permitted. Harness initialization omissions cannot qualify production utility coverage.

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
