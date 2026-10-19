# Stockade bot path map

Manual trace renderer, independent of server runtime systems. Implementation in progress; real acceptance pending.

## Usage

From the game-server checkout:

```text
cargo run -p server --bin bot-map -- TRACE.jsonl -o out.svg --world-db PRIVATE/world.db --navmesh GROUND/navmesh/34
rsvg-convert -o out.png out.svg
```

`--world-db` defaults to `data/world.db`; `--navmesh` defaults to `data/navmesh/34`. Input/output positional order is shown above. Agent builds/tests still use the normal approved helper and capacity admission. Rendering is never automatic.

Produce input by setting `BOT_PATH_MAP_TRACE=1` on both the owned engine `bot_controller_live.gd` process and unchanged `scripts/run_stockade_bot.py` command. Existing `BOT_CONTROLLER_OUTPUT` must name a fresh existing run directory. Client writes `bot-path-samples.jsonl`; runner finish writes `bot-path-trace.jsonl`. Unset opt-in records no path-map files. Engine [capture contract](../../../../game-engine/docs/specs/bot-path-map.md) is authoritative for its schema.

## Geometry and grouping

Navigation exports walkable polygon outlines through the exact validated packet codec used by `NavigationMap`, without rebaking. All heights flatten to X/Z world yards; no claim of multi-floor separation. Map34 `content_creature` rows are read from world.db. Source formations require an authored leader self-row, matching WorldData admission. `followers::pull_sources::read_source` and the renderer call the same pure `pack_identity::pack_guid` rule: formation leader GUID if present, else source GUID. No distance clustering.

## Trace and qualifications

JSONL `run_start`, `pose`, observed `combat_start`/`combat_end`, `death`, `combat_event`, `run_end`. Title includes engine revision/run/duration. Time ticks use Unix `at_ms` differences, **not simulation time**. `simulation_s` is null because the harness exposes no simulation clock. Native `monotonic_ns` uses `godot_process_ticks`, distinct from host monotonic clocks; combat log events retain original wall timestamps and null monotonic values. Client timer cadence is100ms, not server ticks.

Unknown source-entity identity/end-health is labeled UNKNOWN, never alive from missing death events. Combat markers without observed positions cannot be placed. Trace gaps remain visible. Full trash status/pack-overlap proof is pending an existing owner mapping seam; no fabricated target-swap inference.

## Sources

- [Contract](../../specs/bot-path-map.md).
- `crates/server/src/bot_map/`, `crates/navigation/src/query.rs`, `crates/navigation/src/polygons.rs`.
- `crates/server/src/followers/pull_sources.rs`, `pack_identity.rs`, `world_data.rs`.

## See Also

- [Navigation](navigation.md).
