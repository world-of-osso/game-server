# Dev Commands

Development workflow commands for the game-server project.

## Running the Server

```bash
cargo run -p server --bin game-server
```

Binds to `0.0.0.0:5000`.

## Tests

```bash
cargo test          # Run all tests across all crates
./run-tests.sh      # cargo test + clippy (use this before committing)
```

The `run-tests.sh` script runs both the test suite and Clippy lints in one pass.

## Admin CLI

```bash
ssh sakuin /opt/game-server/game-server-admin <command>
```

Runs on the `sakuin` host. Communicates with the server via a Unix IPC socket (`admin_ipc.rs`, `admin_ipc_server.rs`). Source for the admin binary: `crates/server/src/bin/admin.rs`.

## World Data

```bash
scripts/import-azerothcore.sh
```

Regenerates `data/world.db` (132MB SQLite) from AzerothCore SQL dumps and retail DB2 CSVs. Run this when updating world data from source. See [[reference-repos]] for source locations.

## Rust Configuration

| Setting | Value |
|---------|-------|
| Edition | 2024 |
| Minimum rust-version | 1.89 |

## Key Dependency Versions

| Crate | Version |
|-------|---------|
| bevy | 0.18 |
| lightyear | 0.26 |
| redb | 3 |
| bitcode | 0.6 |
| rusqlite | 0.35 |

## Data Files

| File | Description |
|------|-------------|
| `data/world.db` | SQLite, read-only static world data (creatures, spells, spawns). 132MB. |
| `data/game.redb` | redb, mutable persistence (accounts, auth, characters). |

## Sources

- [CLAUDE.md](../../../CLAUDE.md) — Dev section, dependency versions, world data section

## See Also

- [[crate-structure]] — what each crate contains
- [[reference-repos]] — where world data comes from
- [[code-health]] — test coverage hotspots (run `run-tests.sh` before pushing)
