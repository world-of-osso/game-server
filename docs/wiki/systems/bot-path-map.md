# Stockade bot path map

Manual trace renderer, independent of server runtime systems. Real bounded failed-run capture/render proof recorded below; no Stockade clear claim.

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

`spawn_state` rows identify a source GUID and actual observed alive/dead state. Known boss death rows originate only from the exact native entity death already captured by the harness. Renderer uses red DEAD, green ALIVE and gray UNKNOWN, with explicit legend labels; current producer does not invent ALIVE rows for survivors.

## Real Stockade proof — 2026-10-19

Capture revision `011ef565b`, native extension/CLI Rust `0a960bcf6` (unchanged Rust trees), server/renderer `48538fa49`, protocol `f9a9175`. All own artifacts built successfully; SHA receipts and exact commands in `/home/osso/.worktrees/logs/bot-path-map-2026-10-19/proof-ledger.json`. Native five-identity capture reproducer observed0→5 after the UnitPick omission fix; original pose/combat/wipe/boss and SVG fixtures pass. Default-disabled harness/report controls35 pass; actual private admin/provision subprocess tests2 pass. Independent retained planner/check gate remains lead-owned.

Authentic private5981 refresh captured52.365s, two acknowledged Heroic Throw openings,354 owner samples and290 samples for each of Garrick, Crenna, Shuja and Meredy; all five have facing.37 already-read roster samples,296 combat events, two observed combat starts/one end and one mob death. No boss completed and no party death: controller stopped on existing `Sourced route exhausted before goal; no direct-line fallback`. No retry, alternate route or fabricated completion. Actual walkability uses baked map34 polygons;143 source spawns use the shared formation identity. Unknown trash end-state remains gray UNKNOWN.

Inspected SVG/rsvg PNG pair: `/syncthing/AgentShared/2026-10-19/bot-path-map/stockade-five-bots-failed-52s.{svg,png}`. Visible heading was annotated FAILED52s/no bosses with the exact failure; floor/path geometry unchanged. The earlier63.222s failure pair `stockade-failed-63s-no-bosses.{svg,png}` retains three named missing visual tracks. It ended on a5308 outcome deadline without repeating the request; later server work stalled/disconnected and panicked inserting UnitThreatList into despawned227883v0 in `followers::party::advance_runs`. That server root cause belongs to separate owner work, not this capture fix. Neither record is a clear, death-free dungeon acceptance, clean exit or packet-loss diagnosis.

Private JSONL/logs/hashes/cleanup: `stockade-refresh-1/final-proof.json` and `stockade-real-1/` under the proof root above. All owned controller/client/server processes stopped, UDP5981 empty, both lead windows cleared. Only requested SVG/PNG files were published.
