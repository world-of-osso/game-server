# Installed-data regression receipts

## Unpopulated world maps

The old test called Outland530 unhosted, contradicting `maps/instances.rs::HOSTED_WORLD_MAPS` and `world_population_expansion_maps_admit_players_without_dungeon_copies`. Read-only canonical `world.db` has 30,619 `content_creature` rows on530. This predates the DB2 refresh; the Map row-count increase is unrelated.

Replacement: installed `data/db2/12.1.5.70391/Map.csv`, ID449, Directory `AlliancePVPBarracks`, MapName `Alliance PVP Barracks`, InstanceType0. Canonical `content_creature` has zero rows on449. Test asserts that premise before retaining MapNotAllowed and Kalimdor admission assertions. No map-hosting code changed.

Evidence: engine `data/diagnostics/master-reds-70391-2026-10-14/`; targeted logs `/home/osso/.worktrees/logs/master-reds-70391-*.log`.
