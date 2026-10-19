# Creature navigation

Phase-1 baked obstacle-aware creature movement in `crates/navigation` and server creature AI. Reuses the [LOS](line-of-sight.md)/[ground](ground-assets.md) offline asset collector, local-CASC pipeline, coordinates and tile layout. [Implementation](../wiki/systems/navigation.md).

## What it must do

- [x] Produce reproducible per-tile navigation beside `<ground dir>/los`, covering existing LOS maps/tiles starting with map 0; include Stockade global-WMO geometry for acceptance.
- [x] Voxelize walkable surfaces deterministically and exclude slopes exceeding the shared player slope limit.
- [x] Include collision-bearing WMO floors, walls and open doorways, terrain holes and M2 doodad collision hulls, with agent clearance/radius/climb; mark authored terrain water as a distinct area. M2 hull faces use their transformed authored collision normals: upward support faces within the shared slope limit remain walkable; underside/steep faces remain obstacles. Never classify an entire support model as a nonwalkable obstruction.
- [x] Query connected tiles and return string-pulled corners; reject incomplete/unreachable routes instead of crossing obstacles. Use pinned TC a352b1fa PathGenerator.cpp:141's nominal5-yard vertical nearest-poly extent; retain current0.8-yard horizontal bounds, without its larger-search fallback or shortcuts.
- After a complete A* query, remove tile-corner portal-midpoint detours only through a complete native mesh-surface corridor reaching the same destination polygon/floor. Retain every resulting polygon crossing/detail projection and lower-cost-area preference; require an actually shorter projected route. Obstructed rays retain the valid around-obstacle corridor; query failures/buffer exhaustion remain errors. Never substitute an unchecked world-space line or modify canonical mesh data for a query defect.
- Melee chase stop and final-leg navigation arrival use one shared3Dreach predicate around the actual victim; retain the existing2yd value or smaller follower reach and independent actual LOS requirement. Ground travel solves its final intersection with that same sphere while retaining horizontal speed budgets and exact floor correction. A victim about0.5yd below and2yd away horizontally must cause movement into real reach, not a stationary pursuing loop.
- [x] Chase, return-home and waypoint legs follow paths; chase replans after its target moves more than one yard. Creature/follower height correction uses collidable WMO map heights without the player walkable-slope filter, following TC a352b1fa Object.cpp:709–746/3131–3136 and Map.h:329. Player height sampling and its slope filter remain unchanged.
- [x] Continuous failed chase reachability holds movement, then enters the existing evade lifecycle after five seconds outside raids (threat/combat cleared, return-home). A reachable path or new victim resets the timeout; raid maps do not use this timeout.
- [x] Never substitute straight movement on a baked map with a missing tile, invalid artifact or unreachable endpoint. Entirely unbaked maps log that boundary explicitly and retain current movement.
- [x] Stockade chase and home return detour around actual walls; every path segment is clear under the existing LOS query. Stormwind ground fixtures and unreachable-target evade have behavioral proof.

## Nonblocking server queries

- [x] Server fixed systems never perform navigation directory/tile reads, decoding or Detour queries; a worker exclusively owns its NavigationMaps.
- [x] Pending requests retain the current route/movement, or hold position without a route; Pending is neither Blocked nor Unbaked and never permits invented/direct movement.
- [x] Latest request per entity wins; discard stale IDs, publish a completion set in `(entity bits, request ID)` order at the start of the next fixed tick. Request ownership uses the requesting entity's Position, not an AI-type component; despawn or loss of that component cancels its route. InMap metadata remains optional, with map/teleport invalidation unchanged.
- [x] Bound queued work and retained completions to one live request/result per entity plus one executing worker job; no unbounded wake channel.
- [x] Preserve ordinary query success/unbaked/error results. Log worker panic/exit, explicitly fail outstanding and future requests, and never leave them Pending forever or restart silently.
- [x] A test-only drain supports synchronous fixtures; production never drains or joins the worker during ticks. A held actual tile read cannot stop ticks; after release the real path publishes.

Design/caller inventory: [navigation worker](../wiki/design/navigation-worker.md). Worker/types and Navigation::advance have held-read/route GREEN proof. Follower approach probes and eight-candidate escape batches use deferred queries; a pending probe retains prior actor state and cannot become a clear approach or replace a current route. Capture escape candidates until completion, then revalidate against current fields so a growing aura395 cannot starve its batch. Fear captures one random intent, awaits collision, runs existing main-thread clipping/LOS, then awaits its path; interruption/removal invalidates that continuation and retries keep the existing timing. Normal ground script waypoints defer smooth-path controls and commit node/start state only after controls are ready; preemption/replacement cancels their pending identity. Exact/flying paths remain query-free. No production synchronous NavigationMap access remains. Test-only direct-query inspections submit to the same worker and explicitly drain. [Scoped GREEN proof and lead-owned gate](../wiki/systems/navigation.md#navigation-worker-migration--2026-10-18).

## FOLLOWER_HAZARD_REACTION_POLICY

User-approved project policy, not Retail reference data. Followers detect hostile field contact using its authoritative radius at simulation `now + 50ms`, evaluated by the existing authored catalog curve and native combat reach. Constant-radius fields retain that radius; no extra prediction safety margin is added.

Detection immediately submits asynchronous escape planning and starts a250ms simulation-time reaction window. The follower holds position throughout that window even if its route publishes early. On the first tick at or after detection+250ms with a published valid corridor, escape movement starts. Selected candidate corridors are used directly, without a second worker roundtrip. Candidate refreshes during the same contact do not restart the reaction clock; leaving predicted contact or changing field/map starts a new detection.

If planning is still Pending at the deadline, the follower continues standing, even inside the hazard. It starts on the first tick after publication. No synchronous query, production wait/drain, invented movement or fallback is allowed. Growth may overtake a holding follower during reaction or worker delay; that is accepted, not a navigation failure. Once ready after reaction, it must escape; ordinary follow/combat may not move it back into the field. [Follower contract](follower-dungeons.md#follower_hazard_reaction_policy).

## C++ script ground waypoints

- [x] Normal ground `MotionMaster::MovePath` uses TC a352b1fa default `PathGenerator::FindSmoothPath` surface stepping (4yd step, 0.3yd steering slop, 74-point/polygon limit), then exact creature collision-height normalization. Generic chase/home/follow/hazard queries retain their existing portal-crossing output.
- [x] Preserve spline's 0.1yd interior-segment rejection, actual owner start, exact/flying controls and failed-route no-arrival behavior. Missing bake/collision support, incomplete routes, exhausted buffers and unsupported off-mesh links fail explicitly; no direct fallback or corner deletion.
- [x] Authored Wailing path367801/node2 launches from the production-observed boundary and reaches its exact collision floor; generated controls for all49 nodes of paths367800–367802 remain collision-clear under the existing LOS query.
- [x] Physical source escort traverses all three authored paths and both real camps without injected arrivals or escort relocation, reaching the exact stopped endpoint under the movement checks below.
- [ ] Natural activation, ordinary balance and both-skin native clears remain open; source-faction/highHP/control-death fixtures do not certify them.

Source and implementation: [smooth script waypoints](../wiki/systems/navigation.md#cpp-script-smooth-surface-controls--2026-10-14).

## C++ waypoint motion ownership

- [x] An active chase/home generator preempts the default-slot C++ waypoint; only the active owner advances position and lower waypoint time does not run in parallel.
- [x] Reactivation relaunches the remaining authored target from the actor's real position; already-visited exact nodes are not replayed. Keep route generation and strict collision/segment thresholds unchanged.
- [x] A same-tick chase/home handoff initializes without a second position step, including activation from the final path callback. Finalized spline ownership survives that callback; the end event is emitted once. Geometry loss stops both position and replicated gait without an arrival.
- [x] Normal ground C++ moving feet use the same exact creature-collision sampler as generated controls; exact/flying paths are unchanged. Missing support stops without synthetic arrival.
- [x] Physical Wailing tick2136 RED becomes a complete collision-clear, non-relocating three-path/two-camp walk. Verify source horizontal speed × tick (native integer-ms precision), exact floor within existing0.05yd tolerance, baked walkable surface and no vertical-only snap—not an unsourced fixed3D displacement threshold. Source faction/highHP fixture limits remain explicit; no natural/native-clear claim.

## How it works

- [Bake choice, profile, command and artifact layout](../wiki/systems/navigation.md)
- [LOS geometry contract](line-of-sight.md)

## Implementation inventory

- `crates/los/src/bake/` — shared asset and placement collector, collision selection and global-WMO placement.
- `crates/los/src/liquid.rs` — shared runtime/bake MH2O parser (server re-exports it).
- `crates/navigation/` — tiled Recast bake and path-query runtime.
- `crates/navigation/src/worker.rs` — bounded owned-query worker, coalescing and explicit failure results.
- `crates/server/src/navigation.rs`, `navigation/requests.rs`, `creature_ai.rs`, `main.rs` — cached route resource, current-ID fixed publication, movement integration and startup plugin.

## Tests asserting this spec

- `crates/navigation/src/worker_tests.rs` — worker coalescing/cancel/error/death and real query variants; current dispatch recheck passes all six controls.
- `crates/server/src/navigation_blocking_io_tests.rs` — actual FIFO tile-read boundary; synchronous tick-stall RED and production-worker GREEN.
- `crates/server/src/navigation_request_tests.rs` — pending hold/continuation, latest pose/ID, completion-set ordering and old-map invalidation controls.
- `crates/navigation/src/bake.rs` — deterministic walkable floor and steep-slope rejection.
- `crates/navigation/src/query.rs` — real Stockade obstacle detours with LOS-clear path segments (opt-in); actual Ruby WMO4391189/MODD753 metalplate4496145 doorway re-collected and baked from local assets, then crossed without LOS obstruction (opt-in).
- `crates/server/src/creature_ai_tests/stockade_chase.rs` — real chase/home/waypoint/repath/unreachable-evade fixtures (opt-in), including the supported narrow arch's exact five-second boundary, reachability/victim resets and raid exemption.
- `crates/los/src/liquid.rs` — real modern ocean LiquidObject42 regression (opt-in).
- `crates/server/src/navigation_tests.rs` — real Stormwind auction-house floor, per-tick stair support/LOS, cross-tile terrain portals, and missing/unbaked/empty/corrupt artifact boundaries.
- `crates/server/src/scripted_ai/waypoint_path_tests.rs` — authored node2, all49 source controls, slot handoff/initialization ordering, exact-node resume, completion/no duplicate arrival, and support-loss stopped pose.
- `crates/server/src/scripted_ai/tests/wailing_physical.rs` — full three-path/two-camp walk with per-step source speed/floor/WALK-corridor/vertical-only/LOS checks; declared controlled-fixture limits.

## Known gaps (current cycle)

- [ ] Lead-owned release/publication; local full LOS-tile bake coverage is complete.
- [ ] Lead must preserve vendored native build/config inputs in the release snapshot; see [deployment boundary](../wiki/systems/navigation.md#lead-owned-shipping).
- [ ] Full workspace formatting has an unchanged inherited module-order failure; only changed-file formatting is passing.

## Out of scope

Dynamic doors/game objects, flying, transports, crowds, ordinary pet/wander/formation movement, per-display agent profiles and swim-capability policy. Explicit dungeon-follower follow/chase requirements now live in [follower dungeons](follower-dungeons.md); they require baked routes without unbaked compatibility. Lead owns shipping data and deploying to sakuin.

### Diagnostic route export
`cargo run -p navigation --example follower_route -- NAV_DIRECTORY START_X START_Y START_Z END_X END_Y END_Z` prints the complete baked route as Bevy-coordinate x,y,z CSV. It uses the same strict query and fails on an incomplete corridor; output is input for physical walking, not a movement or teleport operation. Concrete Stockade coordinate parsing is tested.
