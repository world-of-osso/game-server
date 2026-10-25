# game-server

Bevy 0.19 authoritative game server for the WoW project (Lightyear 0.28 networking, redb auth/characters, SQLite world data).

## Speed overrides (this project overrides global rules)

- Fix = one behavioral test failing before and passing after, then `cargo check --locked --workspace --all-targets`, `cargo fmt --check`, the touched tests, commit. No proof ledgers, receipt JSON, census files, control batteries or fidelity escalation.
- DPS parity: SimC (pinned in `docs/specs/dps-parity.md`) decides behavior; native data supplies values. Match SimC without asking; cite the SimC line in a code comment. Measure with `scripts/dps_parity_measure.py` after every fix.
- Fix adjacent issues on the critical path directly; mention them in the report. No approval round-trips.
- Before submitting, agents run `CARGO_TARGET_DIR=target-check /home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run <name> cargo check --locked --workspace --all-targets` and `cargo fmt` in their worktree (no capacity admission; check-only, a few GiB). Agents run no other cargo command, no capacity admission and no pushes. Run `cargo fmt`, commit, then `python3 /home/osso/.worktrees/train/merge_train.py submit --commit <sha> --test '<cargo test args>' [--land] [--measure spec=N:world_db=PATH:output=DIR:actor=player:targets=1,3]` and `... wait <id>`. The train merges every queued commit onto `origin/master` in one tree, runs one check, fmt and each entry's tests, pushes `--land` entries that pass, and runs measurements from one executable. Without `--land` the entry is a probe (RED proof, unlanded measurement): it is tested but never pushed; a RED proof shows as `rejected: tests failed` with its log. Submit several tests in one entry. Full workspace suites are suspended until every DPS spec is within parity.

## Wiki

LLM-maintained knowledge base at `docs/wiki/`. See `docs/wiki/SCHEMA.md` for conventions and workflows. Read `docs/wiki/index.md` first when answering questions about the project.

### Wiki Maintenance

After completing work that produces knowledge worth preserving, update the wiki:

- **Investigations/debugging**: Create or update a page in `investigations/` with root cause, symptoms, and fix.
- **New systems or major changes**: Create or update a page in `systems/` describing how the system works.
- **Architecture decisions**: Record rationale in `design/`.

Follow the workflow in `SCHEMA.md`: check existing pages first (update > create), maintain cross-references, update `index.md` and `log.md`. Skip wiki updates for routine bug fixes, config tweaks, and cosmetic changes.

## Shared format readers

Runtime ground/terrain-height and raw MFBO/MH2O metadata, LOS/nav bake adapters and vehicle seat lookup use git-rev-pinned `world-of-osso/wow-formats` crates with default features disabled. LOS enables only M2 `animation`/`materials`; vehicles enable static `attachments`. Keep server geometry/posing policies local, never duplicate binary readers. See [shared formats](docs/wiki/systems/wow-formats.md).

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
