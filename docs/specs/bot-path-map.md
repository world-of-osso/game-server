# Stockade bot path map

Manual `bot-map` binary renders an explicitly captured Stockade (map 34) harness trace to SVG. [Workflow and schema](../wiki/systems/bot-path-map.md).

## What it must do

- [ ] Read JSONL and render observed paths with periodic wall-time ticks, combat/death markers, legend, run ID/revision/duration.
- [ ] Render baked walkability with existing navigation artifact readers; flatten all levels explicitly.
- [ ] Read map34 creature rows from world.db; share the tank planner's pure formation identity rule, never geometric grouping.
- [ ] Mark observed end-state honestly; unknown identity/census data remains unknown.
- [ ] Preserve existing pull-source/route behavior and tests unchanged.
- [ ] Produce one real Stockade SVG and inspected PNG.

## How it works

- [Manual usage](../wiki/systems/bot-path-map.md).

## Implementation inventory

- `crates/server/src/bin/bot-map.rs`: manual CLI.
- `crates/server/src/bot_map/`: trace parsing, source scene and SVG.
- `crates/server/src/followers/pack_identity.rs`: shared pure source pack ID rule.
- `crates/navigation/src/polygons.rs`: geometry export using the query reader's validated packet codec.

## Tests asserting this spec

- `crates/server/src/bin/bot-map.rs`: concrete trace points,10s tick, death marker and title fixture.
- Existing `followers::pull_sources_tests` and `pull_route_tests`: unchanged planner behavior.

## Known gaps (current cycle)

- [ ] Targeted GREEN, native capture and real SVG/PNG acceptance pending.
- [ ] Source GUID↔combat entity mapping missing for trash; per-spawn end-state unknown, not inferred alive.

## Out of scope

Automatic capture/rendering; other maps; server-tick or hazard-worker instrumentation; fabricated death/overlap flags; geometric pack clustering.
