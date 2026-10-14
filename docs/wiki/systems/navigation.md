# Creature navigation

Contract: [navigation](../../specs/navigation.md). Phase 1 uses the same offline `los::bake::Baker` asset/placement collector as LOS, with collision selection: terrain fans/holes, shared ground MOPY/MPY2 WMO collision faces, M2 bounding hulls instead of render batches. No new geometry extractor or CDN access. Shared MH2O parsing lives in `los::liquid` and is re-exported by server ground. Modern LiquidObject IDs (>=42) resolve LVF from the exact authored vertex block size; real ocean root776382 stores81depth bytes, not81float heights.

## Choice and profile

Evaluated pure-Rust landmark/waymark 0.1.0 (wowemulation-dev/recast-rs, Recast/Detour port; published initial release with three-mesh C++ comparisons) against recastnavigation-sys 1.0.3. Choose pinned sys: upstream Recast/Detour's established tiled pipeline, not a new port's multi-layer/seam implementation. Tradeoff: native C++/CMake/libclang build dependencies and a small audited unsafe ownership boundary. No host packages installed. Crate builds bundled pinned source (`RECAST_VENDOR=true`); Vendored sys patch raises CMake's declared minimum3.1→3.5 and bindgen0.69→0.72.1: the host clang otherwise emits an opaque query with no methods. Algorithms remain unchanged; [provenance/retirement](../../../vendor/recastnavigation-sys/UPSTREAM.md). Initial native build succeeded; timing includes unrelated workspace dependencies, not a comparable pure-Rust benchmark. Determinism is byte-equality tested on fixed input; geometry/placement order and build settings are fixed. Recast allocation lifetimes are owned and freed on errors.

Profile: radius **0.388** from shared LOS `BODY_RADIUS` (not an invented 0.3889 constant), climb **1.6** from shared ground `STEP_UP_HEIGHT`, slope **50°** from shared movement `MAX_SLOPE_ANGLE`. Height **2.03128** is the server's existing `mirror_timers::DEFAULT_COLLISION_HEIGHT` (TrinityCore Object.h), used by server liquid/head-clearance logic without per-display model data. Larger displays require future profiles, not silently scaled clearance. Voxel XZ size is 533.3333/2048 yd, Y size 0.2 yd, radius erosion rounds up to two cells, height eleven cells, climb eight cells. Each WoW tile packages sixteen 512-cell Detour subtiles with a five-cell neighbor border. This bounds per-build memory while preserving cross-WoW-tile connectivity. Ground's two-sided walkable-normal rule is preserved. Water marks submerged terrain spans as area 2; movement-mode policy is not added.

## Geometry correction from real Stockade proof

Historical phase1: `GroundMap` supported only terrain/WMO floors; allowing a1.2-yard Stockade crate top in navigation could put actual feet through its side when runtime ground chose the floor beneath it. M2 collision support has since been added through dedicated authored-normal floor bakes. The follower Ruby investigation found the blanket obstacle policy also removed the real doorway's supporting metalplate4496145. Navigation now classifies transformed authored M2 face normals: support faces within the shared slope budget can be walked, underside/steep faces stay obstacles. The exact Ruby collector→bake→query RED/GREEN is documented below; this does not claim every old shipping artifact has been rebaked.

## Runtime query

`NavigationMap` owns the Detour mesh/query scratch memory and lazily reads tiles under endpoint bounds plus one neighbor ring. Complete endpoint artifacts are mandatory. Query snaps to nearby polygons (XZ0.8,Y5.0 yd; the pinned TC nominal vertical extent only, no larger-search fallback), finds a complete corridor, and rejects partial paths, exhausted nodes/buffers and missing endpoints. String pulling retains polygon crossings and projects each corner onto its polygon's detail surface (portal interpolation can land just outside a polygon). Recast spans/contours are an approximation: a real Stormwind stair portal lies0.78yard above the exact collision floor. Existing server ground sampling puts actual chase/home feet on that floor; the stair fixture checks every movement tick within0.05yard and every movement segment with LOS, rather than treating a nav contour as exact point support. Maps are serialized by the server resource; no mutable-query sharing across threads. Native `.nav` packets are trusted offline outputs, not a network input format; envelope/header lengths are validated before Detour loads them.

## Creature movement

Server `Navigation` caches routes per entity/map/goal, replanning when target displacement exceeds1yard or the creature was repositioned by more than2horizontal yards. Chase stops on the actual victim's two-yard reach circle on the final funnel leg (including intermediate polygon crossings), not two yards from a snapped nav endpoint. Home and waypoint legs use the same corner follower. Ground corrects every routed movement mode, including waypoint travel and final home arrival, onto exact collision support. Direct unbaked waypoint legs retain authored arrival Z; route-following does not overwrite grounded feet with approximate nav or authored Z. Arrival delays/orientation/wrap remain unchanged.

A failed chase holds still and records `CannotReachTarget { target, elapsed }`. The fixed20Hz no-path system waits five seconds outside raids before invoking existing `creature_aggro::enter_evade_mode`: threat, target and combat clear, state becomes Returning, then home movement completes normal reset. Reachable paths, changed victims and evade clear/reset the timer. Raid metadata comes from `MapStore`; no-path timeout does not run on raid maps. This follows TrinityCore a352b1fa `Creature::Update`, `SetCannotReachTarget` and `CREATURE_NOPATH_EVADE_TIME`; health behavior remains unchanged. Failed home/patrol paths hold and log rather than teleporting or crossing obstacles. Missing tiles, corrupt artifacts and incomplete paths are errors on any baked map. An entirely unbaked map logs once and explicitly retains old movement; this is the only compatibility boundary. Despawned creatures' route caches are pruned.

## Dungeon follower extension — 2026-10-13

The explicit follower-dungeon goal extends phase1 follow/chase for units carrying `followers::Follower`, not every pet/formation consumer. Existing `Goal::Follow` route progress now drives their real follow point; repeated idle AI ticks retain `FollowOwner.moving` until arrival. Missing/blocked navigation never substitutes direct follower movement. Existing ordinary unbaked compatibility remains unchanged. [Exact real Stockade RED/GREEN and private Ruby package](../investigations/follower-data.md#native-class-level-and-follow-navigation--2026-10-13) owns proof; whole-dungeon/live acceptance remains open.

## Ruby authored support and vertical search — 2026-10-13

The old12-tile bake split the entrance's210,538-polygon component from the boss floor's1,236-polygon component. Even65,535 query nodes returned a partial/out-of-nodes result; exhaustive loaded-link traversal independently proved no connection. Source WMO4391189/MODD753 places metalplate4496145 across the nearest2.565-yard component gap. Its upward authored collision normals were incorrectly tagged as obstacles. Real tile32,29 re-collection/bake reproduces failed doorway traversal before the face-policy repair and passes after it; entrance→collected boss floor has119 LOS-clear corners. Twelve NAV tiles and twelve M2 support tiles were rebaked privately, zero missing inputs. No canonical publication or source-coordinate changes.

A production-ordered follower fixture then failed at tick43 at `(1405.7883,127.84611,173.86066)`: WMO4391460 face1016 has collision height128.1038 and world-up normal0.564409, below the player slope limit. Creature movement now uses the same collidable WMO map surfaces without that player slope filter, preserving the player sampler unchanged. An exact same-point test keeps the player Unsupported before and after creature sampling; the creature gets the authored128.1038 surface. Native1500-tick follower regroup now crosses the actual approach/metalplate, remains supported and collision-clear, and finishes at3.9999883 yards from the grounded owner. These are native fixed-order fixtures, not live dungeon completion.

Source citations: TC a352b1fa `Movement/PathGenerator.cpp:141` has nominal `{3,5,3}`; only its Y5 is adopted, retaining XZ0.8 and refusing the larger Y50 retry at152. `Entities/Object/Object.cpp:709–746` routes non-flying/non-swimming units through GetMapHeight; `3131–3136` delegates to Map::GetHeight. `Maps/Map.h:329` combines static/dynamic map height; `Maps/TerrainMgr.cpp:678–719` selects terrain/VMAP collision height, not a player walkable-normal filter. Existing shared step/search-column bounds remain. Authored boss Z129.782 is never rewritten; nominal vertical lookup resolves its126.0143 support within a complete corridor.

Proof ledger: private `/home/osso/.worktrees/follower-dungeons-2026-10-13/` contains `ruby-metalplate-doorway-red.log`, `ruby-metalplate-approach-green.log`, `ruby-nominal-vertical-red.log`, `ruby-native-height-green.log` (native2/2 at server6e85d3a7/protocol5584762), three real navigation query regressions, three M2-floor regressions, rebake/check/fmt logs and pinned TC source copies. Original diagnostics without assertions are preserved outside tracked tests and are not acceptance gates. Whole-instance and live both-skin acceptance remain open.

## Stationary arch and leash acceptance

Verified2026-10-10: Stockade narrow arch supports stationary player feet at WoW(125,46,-30.936882), while the baked query rejects its disconnected corridor. The old immediate evade failed the Retail five-second grace boundary. Real-geometry opt-in tests now cover4.95s held chase with threat intact,5.0s evade/threat clear, routed return-home, a fresh timeout after a reachable path/new victim, and raid exemption. At that revision, distance leash was the existing40-yard creature/home radius, independent of no-path timing. **Superseded2026-10-13:** dungeon maps now skip that spawn leash (TC `Creature::CanCreatureAttack`, Creature.cpp:2731); open-world40yd behavior is unchanged ([contract](../../specs/creature-aggro.md)). The five-second no-path policy is unchanged.

**Historical divergence evidence; Stockade39.888yd reversal no longer describes current behavior.** Private UDP5094 rebuilt live proof at6c96656 (production fix c8a516c): Slayer4294855900 at home WoW(150.557,45.8164,-34.85624) chased the stationary player(103,76,-34.85623), reversed at sampled39.888yard radius and returned home with the player alive. A separate chase into the cell stopped at Bevy(129.82007,-33.939564,-50.49754) when the player stood on the arch; home movement first appeared5.103s after teleport, then completed while the player stayed exactly125,-30.936882,-46 for the remainder. The five-second grace never crossed geometry; the existing LOS-routed return remained in use. Other nearby creatures can remain in combat; selected Slayer's home return is not evidence that all combat globally ended. At home, repeated on-sight engagements can retry the still-unreachable target in five-second cycles without moving through geometry.

Native targeted56/56 creature-AI tests passed,0 ignored (including9 real navigation fixtures). User media:12 inspected PNGs and two real-time/VFR frame-capture MP4s (22.04s leash,30.32s perch) under `/syncthing/AgentShared/2026-10-10/navmesh-leash/`. Screenshot intervals preserve measured timing, not continuous high-FPS video. Durable logs, scene positions, timestamps and artifact hashes: engine `data/diagnostics/navmesh-leash-2026-10-10/`; [handoff](/home/osso/.worktrees/handoff-navmesh-leash.md). This is bounded Stockade live acceptance, not general raid-navigation/collision certification or shared-realm deployment.

## Reproduce

From the server checkout (through `scripts/agent/agent-run navmesh` for agent commands):

```text
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 0 --los-tiles /home/osso/Projects/world-of-osso/game-engine/data/los/0
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 2991 --los-tiles /home/osso/Projects/world-of-osso/game-engine/data/los/2991
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out data/ground --map 34
```

Continents select exactly the existing LOS tile filenames; globals select the WDT global WMO bounds. Global collection uses shared ground's origin-relative placement rather than the continent corner offset, and also writes LOS tiles for the existing LOS test oracle. Tool logs missing assets in `report.txt` and exits nonzero on geometry gaps; missing input is not success.

Artifacts: `data/ground/navmesh/<map>/<row>_<col>.nav` (versioned envelope of native little-endian, 64-bit-polyref Detour subtiles), per-map `report.txt`; Stockade additionally `data/ground/los/34/*.los`. Zero-byte `.nav` means baked but no walkable region, not absent data. Exact output inventory/size and proof ledger: `/home/osso/.worktrees/handoff-navmesh.md`. Deploy these directories beside existing server ground/LOS; do not commit them.

## Final local bake inventory

Verified2026-10-10, profile `OSSONAV3`; every full bake exited0 and reports zero missing/unreadable inputs. Map0/2991 filenames exactly match their existing LOS tile sets. Data is uncommitted under canonical server `data/ground/`.

| Files relative to `data/ground` | Count | Exact bytes |
|---|---:|---:|
| `navmesh/0/*.nav` |1176|6,192,886,016|
| `navmesh/2991/*.nav` |72|128,093,016|
| `navmesh/34/*.nav` |4|2,054,988|
| `navmesh/{0,2991,34}/report.txt` |3|30,777|
| `los/34/*.los` |4|5,140,924|

Navigation payload total **6,323,034,020bytes**; full new shipping set including reports and Stockade LOS **6,328,205,721bytes**. Map2991 has39 authored empty/nonwalkable tile files (zero bytes), which remain baked/blocked, not legacy maps. Exact filenames, sizes and SHA256: engine `data/diagnostics/navmesh-2026-10-10/bake-files.tsv`; summarized counts/profile: `bake-inventory.json` there. Nine representative rebakes are byte-identical across the collector refactor.

## Lead-owned shipping

`deploy.sh` now incrementally syncs `data/ground/navmesh/` with `--delete` restricted to the destination navigation directory, before ownership and restart. LOS still comes from the engine cache; its Stockade map34 supplement must be promoted first. See [deployment](../../deploy.md).

The engine `navmesh-release` branch preserves server `.cargo/config.toml` and the full tracked `vendor/recastnavigation-sys/` release snapshot, including CMake/templates and inline C++. The existing Dockerfile supplies clang/libclang/CMake; no host installs are needed.

On2026-10-10 the four Stockade LOS files (5,140,924bytes) were copied into canonical engine `data/los/34/`, preserving eye tables and other maps. Exact hashes and integration/release proof are in `/home/osso/.worktrees/handoff-navmesh.md`. Rebased navigation uses normal sibling shared-protocol master75fe6a4; frozen proof overrides are no longer required. Whole-workspace formatting passes on both current master and navigation; no formatting-only fix needed.

No deploy or server restart authorized/performed. Native/release proof is not full CI or live gameplay certification.

## Sources

- recastnavigation-sys 1.0.3 README and bundled upstream source, Cargo.lock pins the dependency.
- wowemulation-dev/recast-rs README/CHANGELOG (landmark/waymark 0.1.0); evaluated, not linked.
- shared-protocol `src/los/body.rs`, `src/ground.rs`, `src/movement.rs` and `src/ground/wmo.rs`.
- [TrinityCore a352b1fa Creature.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Creature/Creature.cpp#L917-L923): non-raid no-path evade; `SetCannotReachTarget` at3503 resets elapsed time on reachability changes.
- [CreatureData.h](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Creature/CreatureData.h#L424): `CREATURE_NOPATH_EVADE_TIME = 5 * IN_MILLISECONDS`.
