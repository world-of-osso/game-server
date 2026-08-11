# Reference Repos

External repositories used as reference during development. None are dependencies — they are read for patterns, formulas, and architecture.

## AzerothCore — `~/Repos/azerothcore`

C++ WoW 3.3.5a emulator. The primary architecture reference.

**Used for:**
- Zone and map threading model (MapUpdater thread pool → adapted to Bevy system sets)
- Combat formulas and spell resolution logic
- Creature AI behavior (aggro, patrol, leash, chase)
- General server structure and the scope of systems needed for a WoW-like server
- SQL world data dumps — AzerothCore's `data/sql/base/db_world/` is the source for creature templates, spawn instances, and waypoint data loaded into `data/world.db`

**Not used for:** networking (custom WoW protocol over TCP, not applicable), persistence (MySQL-based, replaced by redb).

See [[azerothcore-comparison]] for a full side-by-side.

## Rustbolt Core — `~/Repos/rustbolt-core`

Early-stage Rust WoW server attempt.

**Used for:**
- Rust-specific patterns for WoW server concerns
- WoW protocol handling in Rust
- Structure reference for how to organize a Rust WoW server

**Maturity**: Early stage. Useful for Rust idioms, not for architectural decisions.

## SpacetimeDB — `~/Repos/SpacetimeDB`

Evaluated as an alternative persistence/state-sync backend. Rejected.

**Evaluated for:** replacing redb + Lightyear with a single database that handles both persistence and client state sync.

**Rejected because:**
- No tick loop control — reducers fire as they arrive, cannot batch to 20Hz
- WASM overhead — 10–30% vs native for all game logic
- Global write lock — one 50ms combat tick blocks all reads and writes
- Protocol mismatch — forces WebSocket + BSATN; game networking needs UDP with Lightyear's reliable and unreliable channels

See [[stack-decisions]] for the full rejection rationale.

## wow-engine — `~/Projects/wow/wow-engine`

Sibling project, not a third-party repo. The client renderer uses Bevy 0.19 and Lightyear 0.28.0 and shares component types through the shared protocol crate. Its renderer migration is coordinated separately from this server upgrade.

## Retail WoW DB2 Data — `../game-engine/data/`

DB2 CSVs extracted from the retail WoW CASC archive. Source for spell data loaded into `data/world.db`:
- `spell_name`, `spell`, `spell_misc`, `spell_effect`, `spell_power`, `spell_cast_times`, `spell_range`, `spell_cooldowns`, `spell_duration`

~1.4M rows total across spell tables.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — Reference Repos section, SpacetimeDB rejection
- [CLAUDE.md](../../../CLAUDE.md) — Reference Repos section, world.db sources
- [crates/server/Cargo.toml](../../../crates/server/Cargo.toml) — Lightyear UDP/netcode features

## See Also

- [[stack-decisions]] — how reference repo findings shaped stack choices
- [[azerothcore-comparison]] — detailed comparison with AzerothCore
- [[dev-commands]] — scripts that import from reference repos (`scripts/import-azerothcore.sh`)
