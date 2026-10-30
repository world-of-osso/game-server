# Dev Commands

Development workflow commands for the game-server project.

## Native development

[Native workflow](../../remote-builds.md) owns build/run/check/test commands, host selection, watched restarts, data separation, and runtime proof limits. [Build-host contract](../../specs/build-host.md) owns requirements and test mapping. Normal runs use the selected host; raw Cargo and the standalone `run-tests.sh` remain local.

## Admin CLI

```bash
ssh sakuin GAME_SERVER_ADMIN_SOCKET=/run/game-server/admin.sock /var/lib/game-server/bin/game-server-admin <command>
```

Runs on the `sakuin` host. Communicates with the server via a Unix IPC socket (`admin_ipc.rs`, `admin_ipc_server.rs`). Source for the admin binary: `crates/server/src/bin/admin.rs`. [Deployment](../../deploy.md) owns the shared test-realm unit, state paths and bookworm build requirement (`0dcaa23`, `6bfd7b6`); desktop/local helpers below are private development, not the shared UDP5000 realm.

### Timeout

`GAME_SERVER_ADMIN_SOCKET` selects the socket (default `/tmp/game-server-admin.sock`).
The client has one **10-second total IPC deadline**, covering connect, request write,
and the complete reply. Override it with `GAME_SERVER_ADMIN_TIMEOUT_MS`, a positive
integer number of milliseconds; zero, invalid values, and overflow are errors.

```bash
GAME_SERVER_ADMIN_TIMEOUT_MS=60000 game-server-admin list-characters
```

Timeout exits with status 1 and names the socket, command (without credentials or
other arguments), and deadline. Existing connection errors and successful output
are unchanged. A timeout does **not** cancel a command already received by the
server; inspect its result before retrying a mutation.

No commands are exempt. Admin handlers perform local queries, password hashing,
or bounded character/equipment updates with redb commits; there is no admin import
or full-world save command (world import is a separate script below). Ten seconds
allows headroom for hashing and disk commits, not an unlimited stalled-server wait.
Large account/character lists, account moves, or unusually slow storage can use an
explicitly extended deadline as above.

Process/socket regression tests: `crates/server/tests/admin_timeout.rs` (silent
listener, normal Pong, missing socket, invalid timeout configuration).

Trusted physical equipment manifests use a plain JSON array, not an envelope:

```bash
game-server-admin grant-equipment <character> <manifest.json>
game-server-admin list-equipment <character>
```

`grant-equipment` requires exactly `item_id`, `name`, `quality`, `required_level`, `inventory_type`, `vendor_sell_price`, `allowable_class`, and `allowable_race` for every array entry; unknown fields are rejected. See [the contract](../../specs/admin-equipment.md).

### Live GameObject lookup

```bash
game-server-admin game-object-info <map_id> <instance_id> <entry>
game-server-admin game-object-info 0 0 197134
```

Read-only admin IPC lookup, following `npc-info`'s runtime resource-query path. Returns every matching live GameObject's entity bits, entry, display ID, name, map copy, and WoW world `(x, y, z)` coordinates. When present, it also reports authoritative numeric GO state, replicated collision, and visible sender IDs. No matching object returns an explicit not-found error.

Use the returned entity bits for `game-cli mail open`; entry IDs and database spawn GUIDs are not runtime entity IDs. The reported coordinates are the object's world origin, suitable for positioning within interaction reach. Lookup does not open or claim mail. See [[game-objects]] for interaction reach validation.

## World Data

```bash
scripts/import-azerothcore.sh
```

Regenerates `data/world.db` (132MB SQLite) from AzerothCore SQL dumps and retail DB2 CSVs. Run this when updating world data from source. See [[reference-repos]] for source locations.

## Rust Configuration

| Setting | Value |
|---------|-------|
| Edition | 2024 |
| Minimum rust-version | 1.95 |

## Key Dependency Versions

| Crate | Version |
|-------|---------|
| bevy | 0.19 |
| lightyear | 0.28.0 |
| bevy_replicon | 0.41.1 (vendored) |
| redb | 3 |
| bitcode | 0.6 |
| rusqlite | 0.35 |

## Data Files

| File | Description |
|------|-------------|
| `data/world.db` | SQLite, read-only static world data (creatures, spells, spawns). 132MB. |
| `data/game.redb` | redb, mutable persistence (accounts, auth, characters). |

## Sources

- [Cargo.toml](../../../Cargo.toml) and [Cargo.lock](../../../Cargo.lock) — Rust and locked dependency versions
- [vendor/lightyear_replication/UPSTREAM.md](../../../vendor/lightyear_replication/UPSTREAM.md) — Lightyear 0.28.0 vendored provenance
- [vendor/bevy_replicon/UPSTREAM.md](../../../vendor/bevy_replicon/UPSTREAM.md) — bevy_replicon 0.41.1 vendored provenance
- [AGENTS.md](../../../AGENTS.md) — development commands and world data
- [Native workflow](../../remote-builds.md) — selected-host development

## See Also

- [[crate-structure]] — what each crate contains
- [[reference-repos]] — where world data comes from
- [[code-health]] — test coverage hotspots
