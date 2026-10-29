# Offline floor resolver

The floor resolver supplies WoW-coordinate heights for log positions without assuming that an XY column has only one floor.

## What it must do

- [ ] Enumerate terrain, WMO and M2 walkable surfaces; report missing required geometry instead of resolving from incomplete data.
- [ ] Retain only surfaces near a baked navigation polygon within the agent step height.
- [ ] Resolve a single candidate; for several candidates, use connected neighbor floors within plausible travel distance or return all candidates as a named gap.
- [ ] Treat `ui_map_id` only as metadata; do not filter candidates by UI-map Z bounds.
- [ ] Read a JSON request array and write one result per input row in the same order.

## How it works

See [ground assets](../wiki/systems/ground-assets.md).

## Implementation inventory

- `crates/server-combat/src/ground.rs`: strict candidate geometry.
- `crates/server-combat/src/floor_resolver.rs`: filtering and hint resolution.
- `crates/server-combat/src/bin/floor-resolver.rs`: batch JSON interface.
- `crates/navigation/src/query.rs`: nearby polygon projection.

## Tests asserting this spec

- `crates/server-combat/src/floor_resolver_tests.rs`.
- Shared-protocol LOS tile tests cover M2 column enumeration.

## Known gaps (current cycle)

- [ ] Qualify stacked Neltharus floors, missing tiles and hint selection on real data.

## Out of scope

- UI-map assignment bounds: these do not distinguish the floors.
