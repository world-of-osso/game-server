# Native build host

`scripts/build-host.py` selects native desktop/local compilation and execution; `run-dev.sh` selects the same workflow for watched runs. Details and evidence live in the [workflow guide](../remote-builds.md).

## What it must do

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
- `scripts/tests/test_dev_loop.py` — watcher argument preservation, retry timing, failure, signals, and process cleanup.
- Shared game-engine native-runner tests cover transport/lifetime and source synchronization separately.

Tests are inventoried, not rerun by this documentation audit; requirements remain unchecked rather than borrowing historical reports as a fresh gate.

## Known gaps (current cycle)

- [ ] New native server runtime acceptance remains pending; successful compilation alone is insufficient.

## Out of scope

CI/release Docker or deployment changes, distro migration, ICU bundles, host fallback, player database migration, and full server parity certification.
