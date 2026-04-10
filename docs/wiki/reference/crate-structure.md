# Crate Structure

The project is a Cargo workspace with four crates: `shared`, `server`, `client`, and `integration-tests`.

## shared

**Purpose**: Components, game types, and formulas. Compiled into both the server and client binaries, enabling shared game logic and client-side prediction.

**Key modules:**
- `components` / `components/component_stats` — Bevy components (Position, Health, Mana, etc.)
- `formulas/` — melee, spell, rating calculations; base HP/mana data tables
- `protocol/` — Lightyear message definitions (channels, core messages, gameplay messages, registration)
- `types` — newtype IDs (ItemId, SpellId, CreatureId, MapId, ZoneId)
- `aura`, `casting`, `movement`, `threat` — core gameplay mechanics
- `combat`, `death`, `loot`, `xp` — combat pipeline pieces
- `quest`, `reputation`, `talent`, `class_spec`, `profession` — progression systems
- `item_data`, `spell_data`, `spell_catalog` — data-layer types for static content
- `instance`, `instance_lockout`, `instance_teleport` — dungeon/raid instance logic
- `battleground`, `battleground_objectives`, `arena`, `pvp` — PvP systems
- `group`, `guild_creation`, `guild_bank` — social systems
- `trade`, `mail`, `auction_house` (via server) — economy types
- `navmesh`, `game_object`, `game_object_handlers` — world interaction
- `pet_battle`, `transmog`, `holiday`, `motd`, `warbank`, `currency`, `dungeon_finder` — additional features

## server

**Purpose**: Server binary. Headless Bevy app with Lightyear server plugin, running at 20Hz FixedUpdate.

**Key modules:**
- `main` — app setup, FixedUpdate schedule at 20Hz
- `networking` — Lightyear server plugin configuration; channel setup
- `persistence` / `persistence_accounts` / `persistence_character` / `persistence_runtime` / `persistence_types` — redb save/load systems using `Changed<T>` filters
- `world_data` — `WorldDataPlugin`, `WorldData` resource (creatures from SQLite)
- `spell_db` — `SpellDatabasePlugin`, `SpellDatabase` resource (spells from SQLite)
- `zones` / `world_map` — zone system sets, map loading, zone transitions
- `creature_ai` — patrol, aggro, chase, leash, wander, waypoint behaviors
- `combat` / `stats` / `death` / `interest` — server-side combat pipeline
- `auth` / `auth_character` / `character_data` — authentication and character management
- `admin_ipc` / `admin_ipc_server` / `admin_handlers` / `admin_output` / `admin_types` — Unix socket admin CLI
- `auction_house` / `trade` / `guild` / `friends` / `ignore_list` — economy and social
- `talent` / `profession` / `reputation` / `achievement` / `collection` / `currency` — progression
- `pvp` / `pvp_support` / `duel` / `barber_shop` / `inspect` / `who` — misc gameplay
- `lfg` / `rest` / `messaging` / `calendar` / `terrain_height` / `durability` — supporting systems
- `bin/admin` — standalone admin CLI binary

## client

**Purpose**: Client stub. Will link to `wow-engine` for rendering. Currently implements auth flow and message handling.

**Key modules:**
- `auth_flow` — login/character selection flow
- `response_handlers` — handles server messages
- `chat_actions` — chat input processing
- `bin/game_cli` — CLI game client binary

## integration-tests

Separate crate for cross-crate integration tests that need both server and client behavior.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — Crate Structure section
- [CLAUDE.md](../../../CLAUDE.md) — structure overview
- Actual file listings from `crates/server/src/`, `crates/shared/src/`, `crates/client/src/`

## See Also

- [[stack-decisions]] — why Bevy ECS, Lightyear, redb were chosen
- [[dev-commands]] — how to build and run each crate
- [[code-health]] — test coverage breakdown by module
