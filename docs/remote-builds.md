# Native desktop/local development

Normal builds **and runs** use `scripts/build-host.py`. `desktop` means native Rust and runtime in desktop WSL as `osso-test`; `local` means both on this machine. Choose local while gaming on desktop. Raw Cargo remains local.

## Commands

```sh
python3 scripts/build-host.py
python3 scripts/build-host.py --run
python3 scripts/build-host.py --build-host local --run
python3 scripts/build-host.py --bin game-server-admin --run -- --help
python3 scripts/build-host.py --bin game-cli --run -- --help
python3 scripts/build-host.py --check -p server
python3 scripts/build-host.py --test -p server FILTER -- --nocapture
./run-dev.sh --build-host desktop
./run-dev.sh --build-host local
```

Without `--bin`, build selects server, admin, and client binaries; `--run` defaults to `game-server`. `--release` selects release output; `--features CSV` forwards Cargo features. Test/check defaults to the workspace unless Cargo package selection is supplied. `--test` must be the last helper option: everything after it goes to Cargo, including filters and the test-executable separator.

`run-dev.sh` is a Python entry point invoking `cargo-watch` over `crates/` and `Cargo.toml`. It runs `scripts/dev-loop.py`, rebuilding/running on the selected host and waiting 20 seconds after an exit before restarting. Source changes restart the watched invocation. Shutdown signals stop the helper and its controlled process groups/SSH lease.

## Host selection and dependencies

`python3 scripts/build-host.py --save-build-host desktop` saves the shared default in `~/.config/game-engine/build-host`; use `local` to change it. The configured default is desktop. Explicit `--build-host` overrides it for one invocation. Host failure is an error, never a host fallback.

Shared selection, snapshot, and native execution helpers live in canonical `../game-engine/scripts/`; `BUILD_HOST_SCRIPTS` overrides that dependency. Desktop snapshots include `shared-protocol`. Desktop uses pinned Rust 1.98.1 through rustup; the in-progress local native-tool fix uses installed Arch Cargo/rustc 1.98.1 without rustup, not fallback. Native builds run in capped eight-job/16 GB scopes with same-ABI native runtime dependencies; no Docker runtime or ICU bundles are substituted. Existing CI/release Docker and deployment pipelines remain unchanged.

## Source and process lifetime

Desktop source/target directories persist under `/home/osso-test/.cache/native-builds/<key>/`. Snapshots include working edits. Owned pruning removes previously supplied files without deleting runtime-owned directories; content changes receive fresh mtimes, including A/B/A changes. Builds can proceed while an app is alive. SSH leases control cleanup; source SCP has a 60-second attempt limit and at most three transient attempts, with permanent failures reported immediately.

## Runtime data separation

Desktop default runtime working directory is `/home/osso-test/.local/share/game-server/assets`, containing read-only world/game-table/economy inputs. Ground defaults to `/home/osso-test/game-engine-trial/ground`.

Mutable player storage and admin socket default to `/home/osso-test/.local/share/game-server/<server-key>/game.redb` and `admin.sock`, isolated per checkout. Explicit `GAME_SERVER_DB_PATH`, `GAME_SERVER_ADMIN_SOCKET`, and `GAME_SERVER_GROUND_DIR` override those defaults; bind/port/reset environment settings are also forwarded. Local runs retain the checkout working directory and local data. No automatic player database migration occurs.

## Evidence and pending runtime proof

Main-observed evidence on October 3, 2026: actual native desktop server/tools build passed (6m34s). Isolated desktop admin `pong` and disposable-account authenticated UDP login probes passed; the owned server process was cleaned up. This is bounded runtime proof, not gameplay or full server parity acceptance.

Existing 93 shared workflow fixtures passed; lint fixes and actual local native acceptance remain pending. A reported sixteen focused tests/check exit 0 at a source ancestor is historical evidence, not a fresh current-tree gate. No broad-suite, warning-free, or whole-workflow completion claim follows.

## Related

- [Build-host contract](specs/build-host.md)
- [Agent commands](../AGENTS.md#development-buildrun)
- [Development command reference](wiki/reference/dev-commands.md)
