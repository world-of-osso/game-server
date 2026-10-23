# Server crate boundaries

Verified: 2026-10-23. Stable catalog code has one implementation in `server-catalog`; gameplay modules and their tests now belong to the existing `server` library, with a thin executable.

## Current ownership

- `crates/server-catalog`: generated character/spell/XP tables, pure faction-template reactions, and DB2 source-build validation/reading. Its only dependency is the existing rusqlite version/connection type; no Bevy plugin or runtime initialization.
- `crates/server/src/lib.rs`: existing standalone modules plus the former binary's gameplay module declarations and App assembly. `configure_app(&mut App)` preserves runtime→gameplay→resource→system registration order. One `extern crate self as server` keeps existing standalone-library imports canonical without editing hot modules.
- `crates/server/src/main.rs`: creates App, calls `server::configure_app`, runs it. No gameplay source files move for this consolidation. `item_instance::enchants` imports the canonical catalog spell table instead of compiling a second source copy.
- Existing shared protocol, LOS and navigation crates retain their types and responsibilities.

Catalog extraction changes visibility, not values or algorithms: 24 `pub(crate)` declarations become public. Three generators now write to `crates/server-catalog/src/` and emit public constants. Generated numeric tables and faction/provenance logic otherwise remain unchanged. `level_scaling` remains in server because combat/cast tests consume its private fixture helpers.

Test opt2/debug1 overrides apply explicitly to both server packages. No tests move in the catalog-only commit. Library consolidation moves the former `game-server` test target's ownership to `server` lib, retaining existing library tests too; test functions/module paths remain unchanged. The thin bin has zero tests. Select `--lib` / nextest `binary(server)`, not the obsolete `--bin game-server` test filter. This small leaf cut is a crate-wiring proof, not a claimed material parity-iteration speedup.

## Test workflow and pending proof

```text
python3 scripts/build-host.py --test -p server --lib --no-run
python3 scripts/build-host.py --test -p server --lib dungeon_simulator
```

`scripts/dps_parity_native.py` accepts this checkout's executable `target/.../server-*` test artifacts; the real bubblewrap/source-view fixture exercises the new library artifact through the CLI. The batch runner consumes the Cargo-emitted executable path; its seed/test names remain unchanged.

A alone does **not** isolate parity edits from the large library test target. Separate simulator and area-test crates remain the next steps. No full-suite or paired rebuild-speed claim is made here: the exact unsplit primer safety-aborted below6GiB host availability; the interim a28 opt2/-j2 comment-edit receipt (114.96s wall/69CPU-s) is a different source revision, not paired attribution. Full before/after test-census reconciliation and parent full-suite acceptance remain required.

## Sources

- `crates/server-catalog/{Cargo.toml,src/lib.rs}` — canonical module/dependency ownership.
- `Cargo.toml`, `crates/server/Cargo.toml`, `crates/server/src/{main,lib}.rs`, `crates/server/src/item_instance/enchants.rs` — profile and consumer wiring.
- `scripts/gen_character_stat_tables.py`, `scripts/gen_spell_scaling_table.py`, `scripts/gen_xp_tables.py` — reproducible generated-table destinations and visibility.

## See Also

- [[dungeon-simulator]] — headless gameplay qualification.
- [[server-test-performance]] — measured profile/runtime tradeoffs; extraction alone is not a benchmark result.
