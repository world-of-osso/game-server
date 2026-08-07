# Persistence

Dual-layer storage: the ECS is the live database, redb is the durable checkpoint. SQLite holds read-only static world data loaded once at startup.

## Design Principle

The ECS IS the storage engine while the server runs. redb is a background save — not a hot path. This is fundamentally simpler than systems like SpacetimeDB, where the storage IS the database and every write requires WAL + snapshot machinery.

## redb (Mutable Player Data)

Pure Rust, ACID, embedded B-tree. No external server process. Tables:

| Table | Key | Value |
|-------|-----|-------|
| `characters` | `CharacterId` | Position, level, stats |
| `inventories` | `CharacterId` | `Vec<ItemInstance>` |
| `quest_states` | `CharacterId` | `Vec<QuestProgress>` |
| `social` | `CharacterId` | FriendsList, IgnoreList |
| `mail` | `MailId` | MailData |
| `auctions` | `AuctionId` | AuctionData |
| `guilds` | `GuildId` | GuildData |
| `pvp_states` | `CharacterId` | PlayerPvpProgress |

File: `data/game.redb`

## Legacy Database Incident: Missing `pvp_states`

On August 7, 2026, an existing `data/game.redb` database lacked the `pvp_states` table because `create_tables()` did not open `PVP_STATES`. Character selection then panicked while loading PvP state with `TableDoesNotExist("pvp_states")`. The documented `run-dev.sh` loop restarted the server after the panic; the client repeatedly re-entered `InWorld`, eventually triggering repeated GPU resource allocation and a downstream GPU out-of-memory failure. The rendering failure was a consequence of the server restart loop, not the root cause.

Commit `ec5ff52` fixed the migration path by adding `PVP_STATES` to `create_tables()`. It also added a regression test that creates a legacy database without the table, runs `create_tables()`, and verifies `load_pvp()` returns the default state.

## Save Triggers

Character data is written to redb on:
- **Logout / disconnect** — immediate flush for the departing player
- **Periodic interval** — every 5 minutes for all online players
- **Server shutdown** — final flush of all dirty entities

The save query uses `Changed<T>` filters so only entities that actually mutated in the period are serialized and written. Ticks where nothing changes cost nothing.

## What Is NOT Persisted

These are transient and reconstructed on login or respawn:
- Active combat state (current target, in-progress cooldowns)
- Active auras / buffs (except those with explicit persistence)
- Movement interpolation state
- NPC runtime state (aggro tables, patrol progress, wander seed)

## SQLite (Static World Data)

Read-only at runtime. Loaded into `Res<WorldData>` and `Res<SpellDatabase>` at startup. See [[world-data]] for full table reference.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Persistence Strategy" section
- [AGENTS.md](../../../AGENTS.md) — redb file path (`data/game.redb`)
- [crates/server/src/persistence.rs](../../../crates/server/src/persistence.rs) — table creation and save/load systems
- [crates/server/src/persistence_tests.rs](../../../crates/server/src/persistence_tests.rs) — legacy-database migration regression test added by `ec5ff52`

## See Also

- [[ecs-storage]] — why the ECS makes change-tracking trivial
- [[world-data]] — the SQLite side (creature templates, spells, spawn data)
- [[tick-model]] — persistence flush runs at the end of each tick pipeline
