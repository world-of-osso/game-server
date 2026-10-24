# Server crate boundaries

Verified: 2026-10-23. Stable catalog code has one implementation in `server-catalog`; gameplay modules and their tests now belong to the existing `server` library, with a thin executable.

## Current ownership

- `crates/server-catalog`: generated character/spell/XP tables, pure faction-template reactions, and DB2 source-build validation/reading. Its only dependency is the existing rusqlite version/connection type; no Bevy plugin or runtime initialization.
- `crates/server/src/lib.rs`: existing standalone modules plus the former binary's gameplay module declarations and App assembly. `configure_app(&mut App)` preserves runtime→gameplay→resource→system registration order. One `extern crate self as server` keeps existing standalone-library imports canonical without editing hot modules.
- `crates/server/src/main.rs`: creates App, calls `server::configure_app`, runs it. No gameplay source files move for this consolidation. `item_instance::enchants` imports the canonical catalog spell table instead of compiling a second source copy.
- `crates/server-simulation`: test-only consumer of the server library; owns `dungeon_simulator` and nested DPS parity, with the original module prefix retained. Its normal library is empty; dependencies enabling `server/test-support` are dev-only.
- `server::test_support`: canonical frozen graph/Space, healer-intake receipts/Capture, and LiteralParityFixture marker used by both gameplay observation hooks and simulator. Existing shared protocol, LOS and navigation crates retain their types and responsibilities.

Catalog extraction changes visibility, not values or algorithms: 24 `pub(crate)` declarations become public. Three generators now write to `crates/server-catalog/src/` and emit public constants. Generated numeric tables and faction/provenance logic otherwise remain unchanged. `level_scaling` remains in server because combat/cast tests consume its private fixture helpers.

Test opt2/debug1 overrides apply explicitly to server, server-catalog and server-simulation. No tests move in the catalog-only commit. Library consolidation moves the former `game-server` test target's ownership to `server` lib, retaining existing library tests too; test functions/module paths remain unchanged. The thin bin has zero tests. Select `--lib` / nextest `binary(server)`, not the obsolete `--bin game-server` test filter. This small leaf cut is a crate-wiring proof, not a claimed material parity-iteration speedup.

## Test workflow and pending proof

```text
python3 scripts/build-host.py --test -p server --lib --no-run
python3 scripts/build-host.py --test -p server-simulation --lib --no-run
python3 scripts/build-host.py --test -p server-simulation --lib dungeon_simulator
```

`scripts/dps_parity_native.py` accepts this checkout's executable `target/.../server_simulation-*` test artifacts; the real bubblewrap/source-view fixture exercises this artifact-selection boundary through the CLI. The batch runner consumes the Cargo-emitted executable path; its seed/test names remain unchanged.

A alone does **not** isolate parity edits from the large library test target. B moves those edits to the simulator crate; per-area public-API test crates remain the next step. No full-suite or paired rebuild-speed claim is made here: the exact unsplit primer safety-aborted below6GiB host availability; the interim a28 opt2/-j2 comment-edit receipt (114.96s wall/69CPU-s) is a different source revision, not paired attribution. Full before/after test-census reconciliation and parent full-suite acceptance remain required.

## Fixture feature and test ownership

The explicit `server/test-support` feature preserves the original unit-test RNG scopes, immutable catalog/cache representation, test-only Clone implementations and observation hooks when server is a normal dependency of a test harness. Its optional dependencies are existing blake3, tempfile and pinned game-engine-core. Ordinary server/workspace builds do not enable this feature through the simulator package's normal dependency graph. Incoming native factor capture also needs this boundary at every observation call and armor-observation field/initializer, not just its public resource constructor/export: enabling the module alone compiles a capture that observes nothing. Unit-only test modules remain `cfg(test)`. No alternate gameplay implementation or duplicate component/resource definitions exist.

Source relocation keeps algorithms and numeric data unchanged: 46 tracked source/assets relocate after incorporating the landed haste and periodic-crit tests; four graph/receipt files belong to core support; the ignored corridor-input capture remains consumer-owned because it builds the simulator runtime. `../../data`, `../../content` and `../../target` paths retain their workspace location because both packages have the same manifest depth. Graph extraction-only modules/helpers and unrelated unit-only probes remain `cfg(test)` rather than pulling unit-test modules into a normal fixture-support dependency. Calendar/presence counters retain fixture-mode imports; their unit-test synchronization locks remain test-only.

Most case names retain `dungeon_simulator::...`. Six graph cases instead map to `test_support::graph::...` in server lib: four enabled route/graph cases and two ignored extraction cases. The ignored `dungeon_simulator::graph::corridor_inputs::freeze_native_corridor_inputs` case keeps its original name in the consumer; a graph re-export shares canonical types without reversing the runtime dependency. [Exact expected case/target mapping](../../handoffs/crate-split-test-mapping.json) records the exceptions and unchanged-name consumer move. Function bodies and ignore reasons are retained. The landed A suite at `8e7c7a685` supplies 123 exact subtree names: 117 consumer cases (30 enabled, 87 ignored) and six retained graph cases. B at `22aa67892` passes the simulator test-target compiler check and the ordinary `game-server` binary check without warnings. Incoming master `eec5b6bcf` adds four ignored single-spell cases to the consumer, making121 consumer cases (30 enabled/91 ignored) without changing original case identities. `14c74e6fa` also passes combined retained-core unit and simulator compilation without diagnostics. Actual post-split test inventories/full-suite reconciliation remain acceptance gates; compiler checks do not substitute for them.

## Next test-area pilot

[The C pilot and paired-measurement handoff](../../handoffs/crate-split-area-tests.md) identifies 106 public-behavior melee cases and the shared event collectors that must remain below both consumers. Private outcome-formula tests stay in core. The pilot is planned, not implemented.

## Sources

- `crates/server-catalog/{Cargo.toml,src/lib.rs}`, `crates/server-simulation/{Cargo.toml,src/lib.rs}` — package/dependency ownership.
- `crates/server/src/test_support/` and core observation call sites — shared fixture type identity and graph/receipt behavior.
- `Cargo.toml`, `crates/server/Cargo.toml`, `crates/server/src/{main,lib}.rs`, `crates/server/src/item_instance/enchants.rs` — profile and consumer wiring.
- `scripts/gen_character_stat_tables.py`, `scripts/gen_spell_scaling_table.py`, `scripts/gen_xp_tables.py` — reproducible generated-table destinations and visibility.

## See Also

- [[dungeon-simulator]] — headless gameplay qualification.
- [[server-test-performance]] — measured profile/runtime tradeoffs; extraction alone is not a benchmark result.
