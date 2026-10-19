# game-server

Bevy 0.19 authoritative game server for the WoW project (Lightyear 0.28 networking, redb auth/characters, SQLite world data).

## Wiki

LLM-maintained knowledge base at `docs/wiki/`. See `docs/wiki/SCHEMA.md` for conventions and workflows. Read `docs/wiki/index.md` first when answering questions about the project.

### Wiki Maintenance

After completing work that produces knowledge worth preserving, update the wiki:

- **Investigations/debugging**: Create or update a page in `investigations/` with root cause, symptoms, and fix.
- **New systems or major changes**: Create or update a page in `systems/` describing how the system works.
- **Architecture decisions**: Record rationale in `design/`.

Follow the workflow in `SCHEMA.md`: check existing pages first (update > create), maintain cross-references, update `index.md` and `log.md`. Skip wiki updates for routine bug fixes, config tweaks, and cosmetic changes.

## Shared format readers

Runtime ground metadata, LOS/nav bake adapters and vehicle seat lookup use git-rev-pinned `world-of-osso/wow-formats` crates with default features disabled. LOS enables only M2 `animation`/`materials`; vehicles enable static `attachments`. Keep server geometry/posing policies local, never duplicate binary readers. See [shared formats](docs/wiki/systems/wow-formats.md).

## Development build/run

- `python3 scripts/build-host.py --build-host desktop --run` builds and runs the server in desktop WSL; `--build-host local` does both here. Bare Cargo remains local.
- `--bin game-server-admin|game-cli --run -- <args>` selects a tool. `--release` exports container-built binaries on desktop/local; it rejects native and test/check/features/run.
- `python3 scripts/build-host.py --test -p server FILTER -- --nocapture` runs filtered tests; `--test` must be last. `python3 scripts/build-host.py --check -p server` checks without launching.
- `./run-dev.sh --build-host desktop|local` watches crates/Cargo.toml and rebuilds/runs on that host.
- `--save-build-host desktop|local|native` changes the shared default at `~/.config/game-engine/build-host`. Host failures never select another host. Native tool selection: desktop pinned Rust 1.98.1 via rustup; local installed Arch Cargo/rustc 1.98.1, no rustup.
- Shared runner: `../game-engine/scripts/`; `BUILD_HOST_SCRIPTS` overrides its location. Desktop runtime assets and player storage are separate from local data; see [native workflow](docs/remote-builds.md).

- `--build-host native` runs host Cargo under the engine's shared native slots with checkout-local `target/` and real `data/`; agents use `agent-run`, no container build lock. Cargo sizes jobs from the agents.slice quota.

## Data

- `data/world.db` (SQLite) stores world/reference data loaded at runtime (creature templates, creature spawns, waypoint data).
- `scripts/world_schema.sql` defines the SQLite schema used for `data/world.db`.
- `data/game.redb` stores account/auth/character persistence.

## Race Metadata

- Character race metadata should be stored in `data/world.db` in a `chr_races` table when available.
- Source of truth for race rows: Wago DB2 `ChrRaces` export.
- Seed script: `scripts/seed_chr_races.sql` (imports current playable races into `chr_races`).
