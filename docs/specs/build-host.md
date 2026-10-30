# Native build host

`scripts/build-host.py` selects local or host-native development compilation and execution; `run-dev.sh` selects the same workflow for watched runs. Details and evidence live in the [workflow guide](../remote-builds.md).

## What it must do

- [ ] Host `native` runs Cargo in the server checkout, uses checkout-local target/data, clears inherited job limits, and shares the engine's host-wide slots. Native releases fail; local release exports use bookworm via the engine server exporter.

- [ ] Explicit host overrides the shared saved default; failures never fall back to another host. Raw Cargo stays local.
- [ ] Build selects requested binaries or all server/admin/client binaries; run defaults to server. Check/test preserve package selection, features, filters, separators, and exit status; failed builds do not run.
- [ ] Local builds and runs use the original checkout and data; explicit runtime environment overrides remain effective. Running apps do not block independent builds.
- [ ] Watched runs observe crates/Cargo.toml, retry exits after 20 seconds, and clean controlled process groups/SSH leases on shutdown.

## How it works

- [Native development guide](../remote-builds.md)

## Implementation inventory

- `scripts/build-host.py` — native adapter, source selection, and runtime defaults.
- `run-dev.sh` — Python cargo-watch entry point.
- `scripts/dev-loop.py` — retry delay and shutdown lifecycle.
- Canonical game-engine `scripts/native_build_hosts.py`, `engine-build.py`, `siblings.py`, `build_hosts.py` — native execution, snapshots, and shared selection; location overridable with `BUILD_HOST_SCRIPTS`.

## Tests asserting this spec

- `scripts/tests/test_build_host.py` — inputs, selected modes/binaries, environment, status, and running-app boundary.
- `scripts/tests/test_native_host.py` — saved/explicit native selection, Cargo arguments, real data/target paths, exit status and release rejection.
- `scripts/tests/test_dev_loop.py` — watcher argument preservation, retry timing, failure, signals, and process cleanup.
- Shared game-engine native-runner tests cover transport/lifetime and source synchronization separately.

Tests are inventoried, not rerun by this documentation audit; requirements remain unchecked rather than borrowing historical reports as a fresh gate.

## Out of scope

CI/release Docker or deployment changes, distro migration, ICU bundles, host fallback, player database migration, and full server parity certification.
