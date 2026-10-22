# Server test performance

The server's standard lead gate must retain its native coverage while completing within15minutes and approximately8GiB RSS without swap. Shared fixture code lives in `crates/server/src/test_data/`; [measurement and implementation notes](../wiki/investigations/server-test-performance.md).

## What it must do

### Execution and coverage

- [ ] Execute the union of the `fu913`, `sm894` and `rd912` gate filters within900s wall at approximately8GiB peak RSS with no swap; report compilation separately.
- [ ] Preserve test cases, existing ignore status, assertions and native simulation steps; scheduling may separate heavy tests without dropping coverage.
- [x] Retain debug assertions and overflow checks in optimized server tests; the compiler artifact and same-source controlled route receipt assert these settings.

### Immutable fixture inputs

- [x] Reuse decoded immutable definitions without querying SQLite again for the same database path and content.
- [x] Permit one source reader to supply concurrent Apps; do not serialize repeated database decoding behind a shared global lock.
- [x] Keep prepared paths distinct from canonical authority, even when initial bytes match; changed content at the same path must load the changed definitions.
- [ ] App-local catalog mutations must leave sibling Apps and retained input definitions unchanged.
- [ ] Production catalog behavior and owned collection representation must remain unchanged; fixture sharing is test-only.

## How it works

- [Server test performance investigation](../wiki/investigations/server-test-performance.md).
- [Existing native fixture caches](../wiki/investigations/slow-native-test-fixtures.md).

## Implementation inventory

- `Cargo.toml` — server test optimization profile.
- `crates/server/src/test_data/catalog_cache.rs` — immutable typed definitions keyed by source path/content.
- `crates/server/src/catalog_data.rs` — production-owned/test-shared collection backing; live edits detach. Spell entries share independently so prepared server-side additions do not copy unrelated definitions.
- Spell, class, follower and large world collections use this backing in tests; world path and read-only spell/follower/map loaders reuse complete decoded inputs. Writable and in-memory connections continue reading their local SQL state.
- `crates/server/src/test_data/catalog_cache_tests.rs` — real SQLite read capability, concurrency and source isolation fixtures, included by the lightweight library test target.

## Tests asserting this spec

- `crates/server/src/test_data/catalog_cache_tests.rs` — immutable input reuse, concurrent read capability and path/content isolation.
- `crates/server/src/test_data.rs` — existing live-App catalog/creature/prepared-source isolation.
- `crates/server/src/test_data/catalog_data_tests.rs` — local edits/iteration and serialized collection values.
- Native gate discovery/execution inventories and compiler artifact receipts linked from the investigation.

## Known gaps (current cycle)

- [ ] Current profile-only union takes37m58s and swaps; the performance goal is not met.
- [ ] Native loader/storage integration is implemented; targeted live-App isolation, compilation and matching before/after measurements remain open.
- [ ] Memory-weighted heavy-test scheduling and final complete gate proof remain open.

## Out of scope

- Production caching, SQLite safety-mode changes, gameplay timing changes, or weaker native assertions: this work optimizes the test path, not game behavior.
- LTO/codegen-units experiments: the approved profile experiment changes only optimization level/debug information.
