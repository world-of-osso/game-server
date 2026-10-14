# Creature navigation

Phase-1 baked obstacle-aware creature movement in `crates/navigation` and server creature AI. Reuses the [LOS](line-of-sight.md)/[ground](ground-assets.md) offline asset collector, local-CASC pipeline, coordinates and tile layout. [Implementation](../wiki/systems/navigation.md).

## What it must do

- [x] Produce reproducible per-tile navigation beside `<ground dir>/los`, covering existing LOS maps/tiles starting with map 0; include Stockade global-WMO geometry for acceptance.
- [x] Voxelize walkable surfaces deterministically and exclude slopes exceeding the shared player slope limit.
- [x] Include collision-bearing WMO floors, walls and open doorways, terrain holes and M2 doodad collision hulls, with agent clearance/radius/climb; mark authored terrain water as a distinct area. M2 hull faces use their transformed authored collision normals: upward support faces within the shared slope limit remain walkable; underside/steep faces remain obstacles. Never classify an entire support model as a nonwalkable obstruction.
- [x] Query connected tiles and return string-pulled corners; reject incomplete/unreachable routes instead of crossing obstacles. Use pinned TC a352b1fa PathGenerator.cpp:141's nominal5-yard vertical nearest-poly extent; retain current0.8-yard horizontal bounds, without its larger-search fallback or shortcuts.
- [x] Chase, return-home and waypoint legs follow paths; chase replans after its target moves more than one yard. Creature/follower height correction uses collidable WMO map heights without the player walkable-slope filter, following TC a352b1fa Object.cpp:709–746/3131–3136 and Map.h:329. Player height sampling and its slope filter remain unchanged.
- [x] Continuous failed chase reachability holds movement, then enters the existing evade lifecycle after five seconds outside raids (threat/combat cleared, return-home). A reachable path or new victim resets the timeout; raid maps do not use this timeout.
- [x] Never substitute straight movement on a baked map with a missing tile, invalid artifact or unreachable endpoint. Entirely unbaked maps log that boundary explicitly and retain current movement.
- [x] Stockade chase and home return detour around actual walls; every path segment is clear under the existing LOS query. Stormwind ground fixtures and unreachable-target evade have behavioral proof.

## How it works

- [Bake choice, profile, command and artifact layout](../wiki/systems/navigation.md)
- [LOS geometry contract](line-of-sight.md)

## Implementation inventory

- `crates/los/src/bake/` — shared asset and placement collector, collision selection and global-WMO placement.
- `crates/los/src/liquid.rs` — shared runtime/bake MH2O parser (server re-exports it).
- `crates/navigation/` — tiled Recast bake and path-query runtime.
- `crates/server/src/navigation.rs`, `creature_ai.rs`, `main.rs` — cached route resource, movement integration and startup plugin.

## Tests asserting this spec

- `crates/navigation/src/bake.rs` — deterministic walkable floor and steep-slope rejection.
- `crates/navigation/src/query.rs` — real Stockade obstacle detours with LOS-clear path segments (opt-in); actual Ruby WMO4391189/MODD753 metalplate4496145 doorway re-collected and baked from local assets, then crossed without LOS obstruction (opt-in).
- `crates/server/src/creature_ai_tests/stockade_chase.rs` — real chase/home/waypoint/repath/unreachable-evade fixtures (opt-in), including the supported narrow arch's exact five-second boundary, reachability/victim resets and raid exemption.
- `crates/los/src/liquid.rs` — real modern ocean LiquidObject42 regression (opt-in).
- `crates/server/src/navigation_tests.rs` — real Stormwind auction-house floor, per-tick stair support/LOS, cross-tile terrain portals, and missing/unbaked/empty/corrupt artifact boundaries.

## Known gaps (current cycle)

- [ ] Lead-owned release/publication; local full LOS-tile bake coverage is complete.
- [ ] Lead must preserve vendored native build/config inputs in the release snapshot; see [deployment boundary](../wiki/systems/navigation.md#lead-owned-shipping).
- [ ] Full workspace formatting has an unchanged inherited module-order failure; only changed-file formatting is passing.

## Out of scope

Dynamic doors/game objects, flying, transports, crowds, ordinary pet/wander/formation movement, per-display agent profiles and swim-capability policy. Explicit dungeon-follower follow/chase requirements now live in [follower dungeons](follower-dungeons.md); they require baked routes without unbaked compatibility. Lead owns shipping data and deploying to sakuin.

### Diagnostic route export
`cargo run -p navigation --example follower_route -- NAV_DIRECTORY START_X START_Y START_Z END_X END_Y END_Z` prints the complete baked route as Bevy-coordinate x,y,z CSV. It uses the same strict query and fails on an incomplete corridor; output is input for physical walking, not a movement or teleport operation. Concrete Stockade coordinate parsing is tested.
