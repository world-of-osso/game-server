# Native build host

`scripts/build-host.py` selects desktop/local or host-native development compilation and execution; `run-dev.sh` selects the same workflow for watched runs. Details and evidence live in the [workflow guide](../remote-builds.md).

## What it must do

- [ ] Host `native` runs Cargo in the server checkout, uses checkout-local target/data, clears inherited job limits, and shares the engine's host-wide slots. Native releases fail; desktop/local release exports use bookworm via the engine server exporter.

- [ ] Explicit host overrides the shared saved default; failures never fall back to another host. Raw Cargo stays local.
- [ ] Build selects requested binaries or all server/admin/client binaries; run defaults to server. Check/test preserve package selection, features, filters, separators, and exit status; failed builds do not run.
- [ ] Desktop source snapshots include working edits and shared-protocol; synchronization prunes only supplied files and notices content changes. Running apps do not block independent builds.
- [ ] Desktop read-only assets and checkout-keyed player/socket state stay separate from local data; explicit runtime environment overrides remain effective, without automatic player database migration.
- [ ] Watched runs observe crates/Cargo.toml, retry exits after 20 seconds, and clean controlled process groups/SSH leases on shutdown.

## How it works

- [Native development guide](../remote-builds.md)

## Implementation inventory

- `scripts/build-host.py` — native adapter, source selection, and runtime defaults.
- `run-dev.sh` — Python cargo-watch entry point.
- `scripts/dev-loop.py` — retry delay and shutdown lifecycle.
- Canonical game-engine `scripts/native_build_hosts.py`, `depot-build.py`, `build_hosts.py` — native execution, snapshots, and shared selection; location overridable with `BUILD_HOST_SCRIPTS`.

## Tests asserting this spec

- `scripts/tests/test_build_host.py` — inputs, selected modes/binaries, environment, data separation, status, and running-app boundary.
- `scripts/tests/test_native_host.py` — saved/explicit native selection, Cargo arguments, real data/target paths, exit status and release rejection.
- `scripts/tests/test_dev_loop.py` — watcher argument preservation, retry timing, failure, signals, and process cleanup.
- Shared game-engine native-runner tests cover transport/lifetime and source synchronization separately.

Tests are inventoried, not rerun by this documentation audit; requirements remain unchecked rather than borrowing historical reports as a fresh gate.

## Known gaps (current cycle)

- [x] Bounded native desktop acceptance: actual build, isolated admin `pong`, disposable-account authenticated UDP login, and owned-process cleanup observed by main; see guide.
- [ ] User approved host-native dev builds on October 9, 2026; current native workspace proof lives in the server-native handoff. Local runtime acceptance remains outside this task.
- [ ] Bounded corrected source gate pending agent 36 after seven I001 findings and two stale rustup fixtures were fixed; see guide for commits. Existing 93 fixture passes do not complete workflow acceptance.
- [ ] Normal desktop simulator GUI remains unresolved; source-gate success is not whole-workflow completion.

## Out of scope

CI/release Docker or deployment changes, distro migration, ICU bundles, host fallback, player database migration, and full server parity certification.
