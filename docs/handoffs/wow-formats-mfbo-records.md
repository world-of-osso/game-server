# MFBO signed-grid consolidation

Approved scope: shared minimum-grid projection in `wow-adt`, replacing only server `ground.rs::load_terrain`'s inline signed-grid loop. Client full bounds reuse the same grid decoder. Keep first-MFBO server versus last-MFBO client, exact error labels/order, unused tails, plane geometry/float order/coordinates/defaults/falling/MH2O untouched. No BLP/assets/deployment.

## Before/after contract

Test-only `ground::mfbo_receipts_tests::` invokes actual filesystem `load_terrain` and the unchanged full client loader. Required roots779167/778027/776382 and existing texture-shadow companions779170/778030/776385 are present; using the client's existing companions adds no prerequisite to the new record projection.51 real-root-derived synthetic cases cover absence, every0–35B short record, exact36B, unused tails, signed extrema, distinguishable duplicate records and combined-invalid root/MCVT order.

`WOW_FORMATS_MFBO_MODE=capture|replay`, `WOW_FORMATS_MFBO_OUTPUT=<empty dir>`, replay `WOW_FORMATS_MFBO_REFERENCE=<old dir>` and `GAME_SERVER_GROUND_DIR=<engine data>` select receipt I/O only. Replay reads frozen input copies/XY bits, never regenerates coordinates. Default mode writes no persistent receipts.

12B magic/version/count envelopes. MFSR: fdid u32/presence u8/9i16 minima(23B/record;3real roots81B). MFCR: fdid/presence/18i16 max-then-min(41B;3roots135B). MFBE holds the exact length-prefixed old error. MFBR stores selected raw payload presence/length/bytes independently. MFBQ input is mapID+WoWXY float bits12B/record; MFBO query outcome appends explicit-tagged Option<f32>,17B/record. None payload is zeroed; no min-height sentinel or float tolerance.

Queries call actual `GroundMap::min_height` on a legitimate warm cache populated with actual file-loader terrain+MFBO. It has no lazy neighbour/WMO source because those collision faces do not determine minimum planes; inside/outside/unknown-map outcomes remain real method results. Eight triangle interiors, nine vertices, diagonals and ±1/512 edge probes are frozen once. Retained real-map/fall tests separately cover the full canonical cache boundary. Full inventories/counts/sizes/SHA-256/bytes and unchanged input hashes gate acceptance; repeat current terrain/MH2O receipts and Stockade4LOS+4NAV as pin guards, not substitute MFBO proof.

## Exact local gate commands

Builds run through `agent-run wow_formats` with `CAPACITY_OWNER=wow_formats` after normal capacity admission. Receipt environment is above; `GAME_SERVER_GROUND_DIR` is canonical engine data, protocol is exactf9a91753.

```text
cargo test --locked -p server --bin game-server ground::mfbo_receipts_tests:: -- --nocapture
cargo test --locked -p los --features bake --test mh2o_receipts -- --nocapture
cargo test --locked -p server --bin game-server terrain_height:: -- --nocapture
```

Each retained control runs `cargo test --locked -p server --bin game-server <FQN> -- --exact --nocapture`:

- `ground::tests::mfbo_planes_bound_the_fall_on_a_continent_tile`
- `ground::tests::map_min_height_follows_the_mfbo_plane_under_the_feet`
- `ground::tests::falling_out_of_the_stockade_kills_below_minus_500`
- `ground::tests::cached_stockade_has_every_ground_file_and_a_missing_cache_lacks_the_wdt`

Engine `python3 scripts/depot-build.py --root <engine slot>` helper arguments, with `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol`:

```text
--test -p game-engine-core --test adt_authored --test zephras_terrain --test terrain_water_height -- --nocapture
--test -p game-engine-godot --lib terrain:: -- --nocapture
--test -p game-engine-godot --lib ground:: -- --nocapture
```

Fresh Stockade: `cargo run --locked -p navigation --features bake --bin nav-bake -- --data <engine data> --world-db <server data>/world.db --out <empty output> --map 34`. Server `cargo check --locked -p server` and workspace `cargo fmt --all --check`; engine helper-native core/Godot check plus Godot workspace fmt. Full receipt/inventory comparison: `/home/osso/.worktrees/logs/wow_formats-phase6-receipt.py after`.

## Ledger

OLD test-only sourcec66700750: MFBO2/0,3roots71 frozen queries each,51synthetic cases/196MFBO files; retained MH2O2/0,terrain-height23/0,two exact real MFBO/fall controls2/0,fresh4LOS+4NAV0. Complete376-file inventory50,037,281B with120 source hashes and Map34WDT lookup791060 frozen at `/home/osso/.worktrees/logs/wow_formats-phase6-reference-receipt.json`. Each consumer compares only to its own baseline. Serverbasecd4553fd5,enginec177af21d,library2a91b224,protocolf9a91753bbbc15ad7ff7392955b52c4da989d22e. All old outputs captured before production edits. Library882b2fc748c0365dad315375d0827e1f066f733f:3 new API tests REDmissingimport→GREEN;default253/0/0,no-default173/0/0,check/fmt0. Published before both consumer pins; no features/dependencies changed. Server after-source d95669932: MFBO2/0,retained MH2O2/0,height23/0,four exact real MFBO/global-WMO/missing controls4/0,fresh4LOS+4NAV0,check/fmt0. `wow_formats-phase6-receipt.py after` proves376/376 files50,037,281B byte-identical,120/120 receipt-source hashes and Map34lookup791060 unchanged. Actual normal-edge server runtime cargo tree320→320 with identical features, differing only in four revision pins. Engine sourcec0b9adaef: authored/fullADT5 +direct positive-MFBO Zephras1 +retained water-height4 =10/0; native terrain45/0 and ground18/0; helper core/Godot check/fmt0. No source changes after these proofs; later ledger commits are docs-only. Independent paired lead integration remains pending.
