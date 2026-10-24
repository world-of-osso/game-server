# Server test performance

The whole server test suite must retain every discovered case and existing ignore while completing within15minutes and approximately8GiB RSS without swap. The three-gate union is an intermediate measurement, not full-suite completion. Shared fixture code lives in `crates/server/src/test_data/`; [measurement and implementation notes](../wiki/investigations/server-test-performance.md).

## What it must do

### Execution and coverage

- [x] `--full-suite` executes all discovered tests from the supplied complete Cargo artifact inventory, with no union-only filter or implicit skip.
- [x] Report exact known and unexpected failing identities without converting failures to ignores, dropping assertions, or rewriting raw failure status.
- [x] Crawth's native Peck damage qualification uses a per-difficulty test-local seed through the existing native RNG seam; native hit rules and all damage/aura assertions remain unchanged, proven by repeated isolation and the original module order.
- [ ] Qualify the whole-suite command against900s/~8GiB with a complete all-case/ignore inventory and separate compile timing.

- [x] Execute the union of the `fu913`, `sm894` and `rd912` gate filters within900s wall at approximately8GiB peak RSS with no swap;706.402s/6.77GiB/zero swap qualifies performance, with one lead-accepted known master environment/data failure reported explicitly.
- [x] Preserve test cases, existing ignore status, assertions and native simulation steps; the complete native union accounts for2856 enabled/87 ignored without missing/extra identities.
- [x] Module-process batching preserves the exact2856enabled/87ignored inventory and reports per-module CPU/wall plus aggregate peak RSS, using at most six concurrent test threads.
- [x] Retain debug assertions and overflow checks in optimized server tests; the compiler artifact and same-source controlled route receipt assert these settings.

### Immutable fixture inputs

- [x] Disk-backed class, Wild Imp and encounter fixture directories use PID-prefixed names. Before creating a directory, reclaim sibling directories only when their PID has no live `/proc/<pid>` entry; preserve live owners without age-based guessing.

- [x] Reuse decoded immutable definitions without querying SQLite again for the same database path and content.
- [x] Permit one source reader to supply concurrent Apps; do not serialize repeated database decoding behind a shared global lock.
- [x] Prepared-input keys initialize independently: a gated initializer must not block another key, while concurrent readers of one key publish exactly one completed value and install independent App resources. Snapshot bytes and native fixture steps remain unchanged.
- [x] Keep prepared paths distinct from canonical authority, even when initial bytes match; changed content at the same path must load the changed definitions.
- [x] App-local catalog mutations must leave sibling Apps and retained input definitions unchanged.
- [x] Deleted prepared source files must not keep otherwise unused decoded catalogs alive after the next catalog load; existing App copies remain valid and independent, while existing canonical input stays reusable.
- [x] Production catalog behavior and owned collection representation must remain unchanged; fixture sharing is test-only.

## How it works

- [Server test performance investigation](../wiki/investigations/server-test-performance.md).
- [Existing native fixture caches](../wiki/investigations/slow-native-test-fixtures.md).

## Implementation inventory

- `Cargo.toml` — server test optimization profile.
- `crates/server/src/scripted_ai/tests/prepared_inputs.rs` — short-held key registry and per-key initializer slots; snapshot/import/decode paths are unchanged.
- `crates/server/src/test_data/catalog_cache.rs` — immutable typed definitions keyed by source path/content.
- `crates/server/src/catalog_data.rs` — production-owned/test-shared collection backing; live edits detach. Spell entries share independently so prepared server-side additions do not copy unrelated definitions.
- Spell, class, follower and large world collections use this backing in tests; world path and read-only spell/follower/map loaders reuse complete decoded inputs. Writable and in-memory connections continue reading their local SQL state.
- `crates/server/src/test_data/catalog_cache_tests.rs` — real SQLite read capability, concurrency and source isolation fixtures, included by the lightweight library test target.

## Tests asserting this spec

- `crates/server/src/test_data/catalog_cache_tests.rs` — immutable input reuse, concurrent read capability and path/content isolation.
- `crates/server/src/test_data.rs` — existing live-App catalog/creature/prepared-source isolation.
- `crates/server/src/test_data/fixture_dir.rs` — dead-PID database reclamation, live-PID preservation and current-PID ownership in the server library test target.
- `crates/server/src/scripted_ai/tests/prepared_input_concurrency_tests.rs` — distinct-key progress, same-key once initialization, warm reuse and independent installed resources.
- `crates/server/src/test_data/catalog_data_tests.rs` — local edits/iteration and serialized collection values.
- `scripts/tests/test_native_test_batches.py` — real process-cache lifetime, bounded parallelism, ignored-case accounting and failure propagation contracts.
- Native gate discovery/execution inventories and compiler artifact receipts linked from the investigation.

## Known gaps (current cycle)

- [x] Module-process execution meets900s/no-swap performance; raw functional status remains2855PASS/1known master environment/data FAIL, not all green.
- [x] Native loader/storage integration, targeted live-App isolation and compilation pass; the matching19-case workspace-graph before/after measurements are recorded.
- [x] Two process slots×three threads qualify the standard union below8GiB with no swap.
- [ ] Whole-workspace900s/no-swap qualification and default adoption remain open; [latest complete measurements and cost bounds](../wiki/investigations/server-test-performance.md#fixed-source-whole-suite-overlap-and-costs) retain raw failures and exact inventories.

## Out of scope

- Production caching, SQLite safety-mode changes, gameplay timing changes, or weaker native assertions: this work optimizes the test path, not game behavior.
- LTO/codegen-units experiments: the approved profile experiment changes only optimization level/debug information.
