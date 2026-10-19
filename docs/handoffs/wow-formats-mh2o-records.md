# MH2O raw-record consolidation

Approved scope: shared12B chunk,24B instance,16B attribute projections and `wow-chunk` scalar reads. Client/server keep their differing bitmap and vertex layouts, LiquidObject interpretation, validation/error precedence and sampling/status policy. No mode flag, fallback, reconciliation, BLP or deployment.

## Frozen proof

Before production edits, `cargo test -p los --features bake --test mh2o_receipts` exercises both current consumers on required ADTs778027/778232/776382,25 concrete synthetic payloads and actual `Baker::collect_water` outputs. `WOW_FORMATS_MH2O_MODE=capture|replay`, `WOW_FORMATS_MH2O_OUTPUT=<empty dir>`, optional `WOW_FORMATS_MH2O_REFERENCE=<old dir>`, and `GAME_SERVER_GROUND_DIR=<engine data>` select test-only receipt I/O. Replay loads frozen synthetic inputs/query bits; it never regenerates queries.

Little-endian12B magic/version/count envelopes: WMSL server chunks (index/count/deep16B), layers(type/min bits/rectangle/exists/height-count22B +4B/height); WMCL client chunks(index/count/attributes-tag/fishable/deep25B), layers retain all type/object/min/max/rectangle/mask fields and length-prefixed heights/UVs/depth/raw-object bytes; WMER exact existing errors. WMRW inventories accessed source records on successful inputs. WMQI frozen chunk-index/XY-bit records12B; WMQO same inputs+tagged12B liquid outcomes24B. WMBN is24B per actual lower/upper bound pair. Client and server receipts compare only against their OWN baseline, never each other.

The existing terrain-height receipt harness also includes776382 for actual terrain/depth/area/Bevy query outcomes. Fresh Stockade4LOS+4NAV guards pin changes but does not prove water decoding. Require matching complete inventories/counts/sizes/SHA-256/actual bytes and unchanged source hashes. No missing-asset skips or zero-test probes count.

## Exact local gate commands

All Cargo builds below run through `agent-run wow_formats` with `CAPACITY_OWNER=wow_formats` after capacity admission; data root is canonical engine data. Engine helper uses `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol`.

```text
cargo test --locked -p los --features bake --test mh2o_receipts -- --nocapture
cargo test --locked -p server terrain_height:: -- --nocapture
cargo test --locked -p server --bin game-server liquid -- --nocapture
cargo test --locked -p server --bin game-server swim -- --nocapture
cargo test --locked -p los --features bake --lib liquid::tests::modern_ocean_liquid_object_42_is_depth_only -- --ignored --exact
cargo test --locked -p navigation --features bake --lib bake::tests::
```

Four separate `cargo test --locked -p server --bin game-server <FILTER> -- --exact` controls:

- `ground::tests::northshire_ground_under_the_blackrock_spy_is_the_terrain`
- `ground::tests::jasperlode_mouth_hole_has_no_terrain_and_its_ground_is_the_mine_floor`
- `ground::tests::stockade_fall_points_are_outside_the_wmo_and_its_floors_hold_beside_them`
- `ground::tests::cached_stockade_has_every_ground_file_and_a_missing_cache_lacks_the_wdt`

Engine helper `python3 scripts/depot-build.py --root <engine slot>` arguments:

```text
--test -p game-engine-core --test liquid_water --test terrain_water_height --test adt_water_geometry -- --nocapture
--test -p game-engine-godot --lib terrain:: -- --nocapture
--test -p game-engine-godot --lib ground:: -- --nocapture
```

Server `cargo check --locked -p server`, standalone `cargo check --locked -p los --no-default-features`, workspace `cargo fmt --all --check`; engine helper-native core/Godot check and Godot workspace fmt. Fresh Stockade command: `cargo run --locked -p navigation --features bake --bin nav-bake -- --data <engine data> --world-db <server data>/world.db --out <empty dir> --map 34`. The receipt capture/replay environment above is required for file proof, not parser behavior.

## Ledger

Baseline test-only source7064701bb: MH2O receipts2/0 (both consumers,25 synthetic cases), terrain_height23/0 and fresh Stockade4LOS+4NAV0. Complete180-file inventory28,034,481B/116 source hashes at `/home/osso/.worktrees/logs/wow_formats-phase5-reference-receipt.json`. Real server layers42/39/256; water queries6427/5968/39169; actual water bounds1181/1470/16384 records. Each ADT256 height chunks154636B and135428 frozen terrain/liquid queries4739992B. No production edits preceded capture. Server baseba74ef1df; engine basecc4858b97; protocolf9a91753bbbc15ad7ff7392955b52c4da989d22e; libraryed180ce. Test-only fixtures precede all production edits. Shared library2a91b2242dbb1cf0e0217958a973b527b80191f1 published first after four missing-API RED tests became GREEN: default250/0/0,no-default170/0/0,check/fmt0. `los` now requires raw ADT/chunk dependencies for its unconditional runtime liquid module; their default features remain disabled, M2/WMO stay bake-only. After-reader MH2O tests2/0; all142 MH2O receipt files byte-identical to their own frozen baseline, including client/server error strings. `cargo check -p los --no-default-features`0 proves its default runtime dependency boundary compiles. After terrain_height23/0 and fresh Stockade4LOS+4NAV0; comparison proves all180/180 outputs28,034,481B byte-identical,116/116 source inputs unchanged. Server check/fmt0. Scoped controls: server `liquid`1/0/1ignored (ignored Zephras not evidence), `swim`10/0, four exact Northshire/Jasperlode/Stockade controls4/0; explicit previously ignored real-ocean LOS test1/0 and navigation bake unit1/0. No production changes after source707e115e4; later ledger commits do not invalidate proof. Engine sourcef141dc100: helper water integrations20/0 (adt_water_geometry3,liquid_water13,terrain_water_height4), native terrain45/0 and ground18/0; helper check/fmt0. Raw helper logs saved separately. Independent lead integration remains pending; later docs-only commits do not invalidate these proofs.

Server runtime dependency proof: actual `cargo tree --locked -p server --edges normal --target x86_64-unknown-linux-gnu --prefix none --format '{p} features=[{f}]'` at beforeb359e7f8d versus after707e115e4 has320 distinct name/version pairs both sides,0 added/removed and0 enabled-feature changes. Tree differs only in four wow-formats revision pins after normalizing temporary baseline display paths. Baseline Cargo manifests/lock were read from git into a metadata-only fixture; no build/new worktree. Diff and raw outputs: `/home/osso/.worktrees/logs/wow_formats-phase5-server-tree{.diff,-before.txt,-after.txt,-summary.json}`.
