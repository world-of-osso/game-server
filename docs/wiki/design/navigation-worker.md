# Navigation worker

Approved design; worker/types and Navigation::advance group implemented; other query callers/integrated GREEN pending. Initial base server `83a8ce176`, protocol `029a927`; server `c2bd6e38f` merged and protocol `a2b07a6` selected before route integration. Goal: remove navigation disk/Detour work from fixed ticks while preserving movement and query outcomes. [Contract](../../specs/navigation.md), [current implementation](../systems/navigation.md), [separate CPU gap](../investigations/server-tick-lag.md#sight_faction_main_thread_cpu).

## Ownership and messages

One worker exclusively owns `HashMap<map_id, Option<NavigationMap>>`, including lazy directory discovery, tile reads, decoding, Detour insertion and query scratch. `NavigationMap` already implements `Send`; neither maps nor their mutex remain accessible to simulation callers. Main-thread `Navigation` owns entity route progress, current request IDs, pending continuations and completed results. No worker touches ECS, movement, ground/LOS resources, logging decisions or replication.

`Request { entity, id, map_id, query }`: monotonically increasing request ID; `query` is ordinary path, smooth waypoint path, surface collision ray, or a batch of candidate paths. Each variant captures input coordinates; a candidate batch is one entity request, not eight requests that supersede each other. `Response { entity, id, result }` carries owned points/collision/candidate results, retaining `Result<Option<_>, String>`: success, genuinely unbaked map, or the unchanged query error. Pending is separate from all three. The surface-ray variant includes today's map-initialization query; no synchronous shortcut remains.

Submission never blocks. The queue coalesces to at most one live queued request and one retained completion per entity, plus the worker's single executing job; obsolete in-flight work is not another live intent. There is no unbounded notification/job channel. Repeated polling of an unchanged pending intent does not allocate another ID. New entity intent supersedes the older ID; cancellation, map/goal changes and existing route-invalidating conditions invalidate the ID. The worker coalesces queued obsolete requests; responses for noncurrent IDs or retired entities are dropped. Existing replan thresholds, route geometry and error text remain unchanged.

## Publication and pending movement

At the start of each fixed tick, before any movement consumer, nonblocking collection snapshots a completion set and sorts it by `(entity bits, request ID)` before applying current results. Identical completion sets produce identical application order. Worker arrival timing may change which tick receives a result; determinism is not claimed across different completion sets. Existing routes and actor state remain on the simulation thread.

While pending, continue the current route when present; otherwise hold position. Do not classify pending as blocked/unbaked, start a no-path timeout, invent a destination, emit arrival, or substitute direct movement. Ready paths enter the existing corner/spline followers; preserve route ownership and real-position validity. Query errors take the same caller error/log/hold branch as today, only after completion. Genuine unbaked results remain distinct from pending; do not expand or silently alter the existing contract's unbaked-map behavior.

Fear captures one random destination, awaits collision, performs existing main-thread LOS clipping, then requests its path. Hazard selection awaits a candidate batch and retains existing choice policy; a pending probe is not a false/clear decision. Waypoints retain their current spline/hold and do not advance node/start/arrival state merely because work was queued. Production never drains or joins the worker during a tick. Tests alone get `drain_until_idle`, followed by normal ordered publication.

## Complete synchronous caller inventory (base83a8)

| Production caller | Required conversion |
|---|---|
| `navigation.rs::query_map`, `query_path`, `query_waypoint_path`, `first_collision` (including its `query_path(from,from)`), `query_route` | Move map ownership/query execution to worker; typed request/poll boundary |
| `creature_ai.rs`: waypoint556, chase840, companion1225 (follow-owner1188/pet1366,1386), home1651; `scripted_ai/kings_rest/gold_follow.rs::advance_path`139; `followers/hazards.rs::advance_escape`254 | Existing `Navigation::advance` callers: pending continues old route/holds, not existing Unbaked/Blocked branches |
| `followers/hazards.rs::path_crosses_hostile_field`106, `query_escape_candidate`202 | Deferred path probe; eight-candidate batch; preserve ground/field checks |
| `auras/fleeing.rs::choose_path`193 and `start_path`234 | Deferred collision→LOS clip→path continuation; retain fear state while pending |
| `scripted_ai/waypoint_path.rs::query_ground_waypoints`436 via `query_path_controls`/`start_move`/`path_movement_system` | Deferred smooth path; exact/flying authored splines remain query-free |

Tests directly using server queries: `scripted_ai/waypoint_path_tests.rs`252/271 and `scripted_ai/tests/wailing_physical.rs`72; server route tests also need explicit drain/publication where synchronous results are asserted. Standalone `navigation` crate tests and `examples/follower_route.rs` own maps outside the server and remain synchronous; no production fixed-tick caller uses those entry points.

## Worker failure

Catch and log a worker panic; detect worker exit/disconnection as well. Fail every outstanding/current request with an explicit worker-unavailable error, and reject future requests with that error rather than leaving Pending forever. Do not restart silently or add synchronous fallback. A worker-death control checks all pending and subsequent requests terminate explicitly.

## Proof and exclusions

First RED: real baked tile read held at its actual IO boundary while an App advances fixed ticks. Assert ticks advance and no path publishes while held; release, drain only in the test, run the next fixed tick, and assert the real path publishes and movement resumes. Before the fix, a bounded rescue releases the read and demonstrates stalled tick advancement rather than hanging the suite. Add behavioral controls for existing-route continuation, latest-request wins, stale completion rejection, stable publication order, unchanged errors, fear/hazard/waypoint continuations and drain isolation.

No timeout, bake, Detour algorithm, geometry, LOS/ground IO, unrelated actor, protocol or sight/faction CPU changes. `SIGHT_FACTION_MAIN_THREAD_CPU` remains a separately measured next candidate. No claim this slice alone cures historical ten-second gaps.

## Sources

- `crates/server/src/navigation.rs` and production/test callers listed above — synchronous ownership and consumers at the requested base.
- `crates/navigation/src/query.rs` — exclusively owned Send mesh/query and actual tile-read boundary.
- Lead-approved pending-route decisions and joint capture receipts under `game-engine/data/diagnostics/server-live-frame-cost-2026-10-16/`.
