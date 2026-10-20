# Dungeon simulator

Transport-free Stockade qualification runs real gameplay systems in a plain Bevy App in the server test executable. The game-server entry point is never run. [Implementation](../wiki/systems/dungeon-simulator.md).

## What it must do

- [ ] Freeze map-34 Normal spawns grouped by the tank planner's source formation identity into `content/dungeons/stockade-34.json`; retain source GUIDs, entries and 2D positions.
- [ ] Extract inter-pack walk distances once using the existing nav query. Runtime movement consumes only the frozen graph; aggro and range use 2D positions.
- [ ] Run real spell casting, auras, damage, threat, creature AI, full follower kits and tank pull planning in-process, with manually stepped time and direct intents/callable handlers, without networking plugins or transport.
- [ ] Clear every Stockade boss in seconds of wall time. Report cleared/wiped, failed pack, HP/mana traces and pull decisions.
- [ ] Batch N seeded runs and summarize per-pack wipe rate.
- [ ] Preserve approved tank policy: 10-second observation, 30% HP reserve, first single pack, affordable sourced heal, every member above 80% across five seconds before overlap, static spawns until observed dead, caster collection and hazard reaction.

## How it works

- [Simulator](../wiki/systems/dungeon-simulator.md)

## Implementation inventory

- `crates/server/src/dungeon_simulator/mod.rs`: source graph qualification; gameplay runner pending.

## Tests asserting this spec

- `dungeon_simulator::frozen_stockade_graph_covers_native_formations`: exact Normal spawn GUID/formation census (RED).

## Known gaps (current cycle)

- [ ] Graph extraction and one-pack RED/GREEN.
- [ ] Complete route, seeded batches and low-wipe qualification.
- [ ] Final live acceptance after simulator qualification.

## Out of scope

- Runtime navmesh, terrain, LOS, ground, physics, Godot, server process, UDP/lightyear transport and client connections.
- Simulator success alone is not live 3D acceptance.
