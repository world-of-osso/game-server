# Installed-data regression receipts

## Unpopulated world maps

The old test called Outland530 unhosted, contradicting `maps/instances.rs::HOSTED_WORLD_MAPS` and `world_population_expansion_maps_admit_players_without_dungeon_copies`. Read-only canonical `world.db` has 30,619 `content_creature` rows on530. This predates the DB2 refresh; the Map row-count increase is unrelated.

Replacement: installed `data/db2/12.1.5.70391/Map.csv`, ID449, Directory `AlliancePVPBarracks`, MapName `Alliance PVP Barracks`, InstanceType0. Canonical `content_creature` has zero rows on449. Test asserts that premise before retaining MapNotAllowed and Kalimdor admission assertions. No map-hosting code changed.

Evidence: engine `data/diagnostics/master-reds-70391-2026-10-14/`; targeted logs `/home/osso/.worktrees/logs/master-reds-70391-*.log`.

## Stormwind rat walkability

Production regression, not DB2 value drift: 6e85d3a7 changed every AI ground probe to unfiltered creature collision height. Direct random wandering has no routed walkability filter; a steep WMO face became supporting ground. Reproduced rat20313784 tick28 at WoW(-8785.112,838.06696,98.29562), player-walkable floor86.20752. Restrict only direct wander admission to walkable support; routed chase/home still sample unfiltered exact creature collision. Original two-minute/20Hz real-geometry test and all floor/distance assertions stay unchanged.

Source: TrinityCore a352b1fa `src/server/game/Movement/MovementGenerators/RandomMovementGenerator.cpp:105-127` requires collision/LOS and successful PathGenerator routing, rejecting NOPATH/SHORTCUT. Existing direct wander must reject non-walkable support rather than treating collision faces as floor. No pathfinding expansion or blanket chase slope change.

## Mount payment fixture

All three failures occur before payment: Arthur33307 opens gossip with no vendor choice because current and pre-import canonical `content_creature_template.npcflag=1`, matching TDB1210 `tdb_creature_template`; only legacy AzerothCore `creature_template.npcflag=129`. `content_policy.py`/`retail_content.py` promote Retail template flags. Neither DB2 import nor the build change rewrote these tables. Full pre-import/canonical fingerprint diff confirms no creature table changes.

Use TDB1210/canonical Samamba33554 (npcflag129), Swift Purple Raptor46760 / ExtendedCost2671 (same5000000 copper +5 Champion seals241, Darkspear530 Exalted), and Darkspear Raptor45593 / ExtendedCost2600. Installed ChrRaces Troll8 supplies the shopper faction. Gold, currency, inventory, rollback, durable-reopen and fake-currency5 assertions retain their original values. No production merchant or canonical-data changes.

Startup mount-cost WARN is unrelated: canonical metadata is absent for currencies392/3252 and item124669. Cost loader deliberately omits those affected offers rather than making them free. Champion seal241 metadata exists; none of the three payment fixtures uses the unavailable IDs. The existing omission assertion for vendor13216/item19030 remains unchanged.
