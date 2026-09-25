# Flight Masters

Server-side taxi: known flight points, the flight map, and server-driven flights. The contract is shared-protocol `protocol/taxi_messages.rs` (`TaxiChannel`) and the replicated `MovementControl`. The client is game-engine `docs/specs/flight-master.md`.

## What it must do

- [x] The network is world.db `taxi_nodes`, `taxi_path`, `taxi_path_node` (Retail 12.1.0.69933 DB2). Paths with a `Teleport` point (`TaxiPathNode.Flags` 0x1) are left out.
- [x] A flight master's node is the nearest node of its continent that the player's team sees (`TaxiNodes.Flags` `ShowOnAllianceMap` 0x1 / `ShowOnHordeMap` 0x2), flies from (`MountCreatureID_1` Alliance, `_0` Horde) and that has a path (`ObjectMgr::GetNearestTaxiNode`). Dungar Longdrink resolves to Stormwind (2).
- [x] Known nodes are stored per character in redb `known_taxi_nodes`.
- [x] Opening the flight master role learns its node the first time (`TaxiNodeDiscovered`, `ERR_NEWTAXIPATH`) and sends `TaxiMap`. Retail opens the map at once; AzerothCore shows it only on the next talk.
- [x] `TaxiMap` lists every node of the continent the team sees: the current node `Current`; a known node reachable over known nodes `Reachable` with its cost and route; anything else `Unreachable`.
- [x] Nodes with a `ConditionID` need PlayerCondition, which is not evaluated: they are `Unreachable` and never flown to (e.g. Lakeshire, 20068).
- [x] Routes: shortest chain of paths by flown distance over known usable nodes; cost is the sum of `TaxiPath.Cost` copper (Stormwind → Sentinel Hill 5).
- [x] `ActivateTaxi { npc, destination }` needs the flight master's map open; refused with `TaxiFailed`: `NoVendorNearby`, `Busy` (already flying), `InCombat`, `SameNode`, `NotVisited` (destination unknown), `NoPath`, `NotEnoughMoney`. The whole route is paid at take-off (Retail; AzerothCore pays per hop).
- [x] Take-off closes the interaction, mounts the node's taxi mount (Riding Gryphon, display 6852, from `content_creature_template_model`), sets `MovementControl.controlled`, and flies the concatenated `TaxiPathNode` points at `PLAYER_FLIGHT_SPEED` 32 yd/s facing the flight direction. Player input and gravity leave a controlled player alone.
- [x] Landing: the player is on the last point, dismounted, `controlled` cleared and `MovementControl.epoch` bumped.
- [x] Every other server repositioning bumps the epoch too (`teleport::teleported`): spirit release, both resurrections, stuck escape, admin `set-position`.
- [x] Admin `taxi-learn <character> <node>` adds a known node.
- [ ] `TaxiPathNode.Delay` stops, arrival/departure events, early landing, flying while offline or disconnected (the flight state is not saved), flight master reputation discounts.
- [ ] A player's own mount is replaced by the taxi mount and not restored.

## Tests asserting this spec

- `crates/server/src/taxi/network_tests.rs`: Dungar at node 2, one hop for 5 copper and 28 points, routes only through known nodes, ConditionID nodes unusable, Horde visibility, gryphon display.
- `crates/server/src/taxi_tests.rs`: first talk discovers Stormwind and sends the map, a known Sentinel Hill is reachable for 5, a flight pays, mounts, flies and lands on the path end with the epoch bumped, Retail refusals, flight stepping.
- `crates/server/src/networking_tests/{physics,receiver_rate}.rs`: a controlled player gets no gravity and no input movement.
- `crates/server/src/death_tests.rs`, `admin_ipc.rs` tests: spirit release and `set-position` bump the epoch; `taxi-learn`.
- Live evidence: game-engine `data/diagnostics/npcloot-20260925/`.
