# World-static AreaTriggers

Server-owned static AreaTriggers from TDB `areatrigger`, `areatrigger_create_properties` and `areatrigger_template`, distinct from DB2 portal triggers and spell-owned fields. Source: TC a352b1fa `AreaTrigger.cpp363–376`. [Runtime](../wiki/systems/world-area-triggers.md).

## What it must do

- [x] Ordinary instance transfer and login content creation spawn each map's authored static AreaTriggers in that exact copy, preserving the spawn/properties/template identities and both custom namespaces.
- [x] Preserve authored position, facing, shape and visual-spell identity. Do not cast the visual spell as an invented gameplay effect or manufacture placements, statistics or difficulty rows.
- [x] Use the existing source-population difficulty policy, including Follower205→Normal1 projection without changing TDB rows.
- [x] Static spawns live indefinitely, belong to their map rather than a caster/aura and disappear when their map copy unloads. Occupied copies remain isolated; a new copy gets fresh entities.
- [x] The authentic Orator bridge barrier98/property84(custom)/template35803(native) exists after ordinary NewWorld/WorldPortAck in Normal1 and Follower205. Its source sphere radius20 and visual449122 remain intact.

## How it works

- [World-static AreaTriggers](../wiki/systems/world-area-triggers.md).
- [Map copies](../wiki/systems/zone-architecture.md).

## Implementation inventory

- `crates/server/src/world_area_triggers.rs`: static identities and map content.
- `crates/server/src/world_area_triggers/catalog.rs`: composite source joins and static-mode validation.
- `crates/server/src/maps/store.rs`: source catalog.
- `crates/server/src/maps/transfer.rs`: transfer/login content path.
- `crates/server/src/maps/mod.rs`: hosted world creation and map teardown.

## Tests asserting this spec

- `crates/server/src/maps/tests_world_area_triggers.rs`: real Orator barrier map entry, copy isolation, indefinite lifetime, manager-driven unload and fresh reentry in Normal1/Follower205.

## Known gaps (current cycle)

- [ ] Orator's source death hook removes the actual nearby35803 barrier after generic spawning lands; separate encounter integration.
- [ ] Generic conversation ingress for source spawns266–268 follows the static-spawn gate as a second slice.

## Out of scope

Client AreaTrigger visual rendering, unrelated spell-field redesign and fabricated portal entrances. The barrier fixture transfers to its authored TDB pose; it does not claim a Retail entrance traversal.
