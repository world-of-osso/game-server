# Installed-data regression receipts

## Unpopulated world maps

The old test called Outland530 unhosted, contradicting `maps/instances.rs::HOSTED_WORLD_MAPS` and `world_population_expansion_maps_admit_players_without_dungeon_copies`. Read-only canonical `world.db` has 30,619 `content_creature` rows on530. This predates the DB2 refresh; the Map row-count increase is unrelated.

Replacement: installed `data/db2/12.1.5.70391/Map.csv`, ID449, Directory `AlliancePVPBarracks`, MapName `Alliance PVP Barracks`, InstanceType0. Canonical `content_creature` has zero rows on449. Test asserts that premise before retaining MapNotAllowed and Kalimdor admission assertions. No map-hosting code changed.

Evidence: engine `data/diagnostics/master-reds-70391-2026-10-14/`; targeted logs `/home/osso/.worktrees/logs/master-reds-70391-*.log`.

## Stormwind rat walkability

Production regression, not DB2 value drift: 6e85d3a7 changed every AI ground probe to unfiltered creature collision height. Direct random wandering has no routed walkability filter; a steep WMO face became supporting ground. Reproduced rat20313784 tick28 at WoW(-8785.112,838.06696,98.29562), player-walkable floor86.20752. Restrict only direct wander admission to walkable support; routed chase/home still sample unfiltered exact creature collision. Original two-minute/20Hz real-geometry test and all floor/distance assertions stay unchanged.

Source: TrinityCore a352b1fa `src/server/game/Movement/MovementGenerators/RandomMovementGenerator.cpp:105-127` requires collision/LOS and successful PathGenerator routing, rejecting NOPATH/SHORTCUT. Existing direct wander must reject non-walkable support rather than treating collision faces as floor. No pathfinding expansion or blanket chase slope change.
