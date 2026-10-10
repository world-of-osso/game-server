# Creature navigation

Contract: [navigation](../../specs/navigation.md). Phase 1 uses the same offline `los::bake::Baker` asset/placement collector as LOS, with collision selection: terrain fans/holes, shared ground MOPY/MPY2 WMO collision faces, M2 bounding hulls instead of render batches. No new geometry extractor or CDN access. Shared MH2O parsing lives in `los::liquid` and is re-exported by server ground. Modern LiquidObject IDs (>=42) resolve LVF from the exact authored vertex block size; real ocean root776382 stores81depth bytes, not81float heights.

## Choice and profile

Evaluated pure-Rust landmark/waymark 0.1.0 (wowemulation-dev/recast-rs, Recast/Detour port; published initial release with three-mesh C++ comparisons) against recastnavigation-sys 1.0.3. Choose pinned sys: upstream Recast/Detour's established tiled pipeline, not a new port's multi-layer/seam implementation. Tradeoff: native C++/CMake/libclang build dependencies and a small audited unsafe ownership boundary. No host packages installed. Crate builds bundled pinned source (`RECAST_VENDOR=true`); Vendored sys patch raises CMake's declared minimum3.1→3.5 and bindgen0.69→0.72.1: the host clang otherwise emits an opaque query with no methods. Algorithms remain unchanged; [provenance/retirement](../../../vendor/recastnavigation-sys/UPSTREAM.md). Initial native build succeeded; timing includes unrelated workspace dependencies, not a comparable pure-Rust benchmark. Determinism is byte-equality tested on fixed input; geometry/placement order and build settings are fixed. Recast allocation lifetimes are owned and freed on errors.

Profile: radius **0.388** from shared LOS `BODY_RADIUS` (not an invented 0.3889 constant), climb **1.6** from shared ground `STEP_UP_HEIGHT`, slope **50°** from shared movement `MAX_SLOPE_ANGLE`. Height **2.0** is the selected phase-1 humanoid clearance profile: no universal creature collision-height constant was found. Larger displays require future profiles, not silently scaled clearance. Voxel XZ size is 533.3333/2048 yd, Y size 0.2 yd, radius erosion rounds up to two cells, height ten cells, climb eight cells. Each WoW tile packages sixteen 512-cell Detour subtiles with a five-cell neighbor border. This bounds per-build memory while preserving cross-WoW-tile connectivity. Ground's two-sided walkable-normal rule is preserved. Water marks submerged terrain spans as area 2; movement-mode policy is not added.

## Geometry correction from real Stockade proof

M2 bounding hulls are obstacles, not ground-support surfaces: `GroundMap` supports terrain/WMO floors only. Treating a1.2-yard Stockade M2 crate top as a floor produced a path through its side under the1.6-yard step budget. The collector now retains obstacle classification; Recast never promotes those hulls to walkable floors. Real alternate-cell query and movement paths are LOS-clear. Ground's walkable terrain/WMO surfaces still use the shared climb/slope budget.

## Runtime query

`NavigationMap` owns the Detour mesh/query scratch memory and lazily reads tiles under endpoint bounds plus one neighbor ring. Complete endpoint artifacts are mandatory. Query snaps to nearby polygons (XZ0.8,Y2.0 yd), finds a complete corridor, and rejects partial paths, exhausted nodes/buffers and missing endpoints. String pulling retains polygon crossings for terrain/stair height changes. Maps are serialized by the server resource; no mutable-query sharing across threads. Native `.nav` packets are trusted offline outputs, not a network input format; envelope/header lengths are validated before Detour loads them.

## Creature movement

Server `Navigation` caches routes per entity/map/goal, replanning when target displacement exceeds1yard or the creature was repositioned by more than2horizontal yards. Chase stops on the actual victim's two-yard reach circle on the final funnel leg (including intermediate polygon crossings), not two yards from a snapped nav endpoint. Home and waypoint legs use the same corner follower. Ground corrects chase/home floor height; waypoint arrival delays/orientation/wrap remain unchanged.

A failed chase invokes existing `creature_aggro::enter_evade_mode` (`CreatureAI::EnterEvadeMode` comment at that function): threat, target and combat clear, state becomes Returning, then home movement completes normal reset. No new evade timer/health behavior invented. Failed home/patrol paths hold and log rather than teleporting or crossing obstacles. Missing tiles, corrupt artifacts and incomplete paths are errors on any baked map. An entirely unbaked map logs once and explicitly retains old movement; this is the only compatibility boundary. Despawned creatures' route caches are pruned.

## Reproduce

From the server checkout (through `scripts/agent/agent-run navmesh` for agent commands):

```text
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 0 --los-tiles /home/osso/Projects/world-of-osso/game-engine/data/los/0
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 2991 --los-tiles /home/osso/Projects/world-of-osso/game-engine/data/los/2991
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 34
```

Continents select exactly the existing LOS tile filenames; globals select the WDT global WMO bounds. Global collection uses shared ground's origin-relative placement rather than the continent corner offset, and also writes LOS tiles for the existing LOS test oracle. Tool logs missing assets in `report.txt` and exits nonzero on geometry gaps; missing input is not success.

Artifacts: `data/ground/navmesh/<map>/<row>_<col>.nav` (versioned envelope of native little-endian, 64-bit-polyref Detour subtiles), per-map `report.txt`; Stockade additionally `data/ground/los/34/*.los`. Zero-byte `.nav` means baked but no walkable region, not absent data. Exact output inventory/size and proof ledger: `/home/osso/.worktrees/handoff-navmesh.md`. Deploy these directories beside existing server ground/LOS; do not commit them.

## Sources

- recastnavigation-sys 1.0.3 README and bundled upstream source, Cargo.lock pins the dependency.
- wowemulation-dev/recast-rs README/CHANGELOG (landmark/waymark 0.1.0); evaluated, not linked.
- shared-protocol `src/los/body.rs`, `src/ground.rs`, `src/movement.rs` and `src/ground/wmo.rs`.
