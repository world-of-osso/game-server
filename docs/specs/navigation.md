# Creature navigation

Phase-1 baked obstacle-aware creature movement in `crates/navigation` and server creature AI. Reuses the [LOS](line-of-sight.md)/[ground](ground-assets.md) offline asset collector, local-CASC pipeline, coordinates and tile layout. [Implementation](../wiki/systems/navigation.md).

## What it must do

- [ ] Produce reproducible per-tile navigation beside `<ground dir>/los`, covering existing LOS maps/tiles starting with map 0; include Stockade global-WMO geometry for acceptance.
- [x] Voxelize walkable surfaces deterministically and exclude slopes exceeding the shared player slope limit.
- [ ] Include collision-bearing WMO floors, walls and open doorways, terrain holes and M2 doodad collision hulls, with agent clearance/radius/climb; mark authored terrain water as a distinct area.
- [ ] Query connected tiles and return string-pulled corners; reject incomplete/unreachable routes instead of crossing obstacles.
- [ ] Chase, return-home and waypoint legs follow paths; chase replans after its target moves more than one yard.
- [ ] Failed chase reachability enters the existing evade lifecycle (threat/combat cleared, return-home).
- [ ] Never substitute straight movement on a baked map with a missing tile, invalid artifact or unreachable endpoint. Entirely unbaked maps log that boundary explicitly and retain current movement.
- [ ] Stockade chase and home return detour around actual walls; every path segment is clear under the existing LOS query. Stormwind ground fixtures and unreachable-target evade have behavioral proof.

## How it works

- [Bake choice, profile, command and artifact layout](../wiki/systems/navigation.md)
- [LOS geometry contract](line-of-sight.md)

## Implementation inventory

- `crates/los/src/bake/` — shared asset and placement collector, collision selection and global-WMO placement.
- `crates/los/src/liquid.rs` — shared runtime/bake MH2O parser (server re-exports it).
- `crates/navigation/` — tiled Recast bake and path-query runtime.

## Tests asserting this spec

- `crates/navigation/src/bake.rs` — deterministic walkable floor and steep-slope rejection.

## Known gaps (current cycle)

- [ ] Runtime query and AI integration.
- [ ] Real-geometry proof and complete existing LOS tile bake.

## Out of scope

Dynamic doors/game objects, flying, transports, crowds, wander/follow/formation movement, per-display agent profiles and swim-capability policy. Lead owns shipping data and deploying to sakuin.
