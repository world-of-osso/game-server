# WoW-Inspired Game Server — Architecture

## Overview

Native Rust game server using headless Bevy as the ECS runtime. Server and client share
the same component types and game logic. Server is authoritative; clients predict locally
and reconcile with server state.

```
┌─────────────────────┐            ┌─────────────────────┐
│  Client (Bevy)      │            │  Server (Bevy)      │
│  Rendering          │◄────UDP────►│  Headless           │
│  Input capture      │            │  Game logic          │
│  Client prediction  │            │  Authority           │
│  Interpolation      │            │  Persistence         │
│  UI (wow-ui-sim)    │            │  AI / Scripting      │
└─────────────────────┘            └─────────────────────┘
         Shared crate: components, systems, game types
```

## Why Not SpacetimeDB

SpacetimeDB was evaluated (see ~/Repos/SpacetimeDB/ARCHITECTURE.md) and rejected for this
use case because:

- **No tick loop control** — SpacetimeDB processes reducers as they arrive. Games need
  deterministic server ticks (20Hz for WoW-like). You cannot batch and order updates.
- **WASM overhead** — even Rust modules compiled to WASM have 10-30% overhead vs native
  (sandbox bounds checking, no SIMD, indirect calls). For a combat tick processing 2000
  entities, this matters.
- **Write lock during game logic** — a reducer holds the global DB write lock for its entire
  execution. A 50ms combat tick = 50ms of blocked reads and writes.
- **Protocol mismatch** — forces WebSocket + BSATN. Game networking needs UDP with
  unreliable channels for movement and reliable channels for chat/inventory.
- **Solves the wrong problems** — persistence is trivial (periodic save to embedded DB).
  State sync has better game-specific solutions (Lightyear) that understand spatial relevance,
  client prediction, and rollback.

## Stack

| Layer | Crate | Purpose |
|-------|-------|---------|
| ECS | `bevy` (headless) | Game state, systems, scheduling. Shared with client. |
| Networking | `lightyear` | Server-authoritative multiplayer for Bevy. Prediction, interpolation, interest management, delta compression. |
| Transport | Lightyear UDP + netcode | UDP transport with reliable and unreliable game channels. |
| Persistence | `redb` | Embedded, pure Rust, ACID, append-only B-tree. Character saves, inventory, quest state. |
| Static data | `ron` files or SQLite | Item templates, spell definitions, creature templates. Loaded once at startup. |
| Serialization | `bitcode` | Fast binary serialization for networking. ~10x faster than serde-json, smaller than bincode. |
| Spatial | `big_space` or custom | Floating-point precision for large worlds. Grid-based interest management. |

## Crate Structure

```
game-server/
  crates/
    shared/          ← Components, game types, formulas. Used by both client and server.
      src/
        components/  ← Position, Health, Mana, Inventory, SpellBook, ...
        combat/      ← Damage formulas, spell resolution, stat calculations
        movement/    ← Movement validation, speed calculations
        types/       ← ItemId, SpellId, CreatureId, MapId, ZoneId, ...

    server/          ← Server binary. Headless Bevy app.
      src/
        main.rs      ← App setup, headless Bevy with FixedUpdate at 20Hz
        zones/       ← Per-zone system sets, map loading
        persistence/ ← redb save/load systems
        ai/          ← Creature AI, patrol paths, aggro
        networking/  ← Lightyear server plugin config
        scripting/   ← Lua or Rhai for encounter scripts (optional)

    client/          ← Client binary. Links to wow-engine for rendering.
      src/
        main.rs      ← Bevy app with rendering + lightyear client
        prediction/  ← Client-side prediction + reconciliation
        interpolation/ ← Entity interpolation for remote players
```

## Server Tick Model

The current runtime and proposed ordering contract are maintained separately:

- [Current tick model](docs/wiki/systems/tick-model.md) — observed schedules, cadence, and explicit ordering
- [Tick-loop audit](docs/wiki/investigations/tick-loop-audit.md) — exhaustive recurring work and correctness findings
- [Tick-loop design](docs/wiki/design/tick-loop-design.md) — draft normative phase, catch-up, replication, and maintenance contract

The server targets a 50ms outer runner and passes the same duration to Lightyear, which sets
Bevy's `Time<Fixed>` timestep to 50ms. Fixed simulation, request handling, and replication
span `FixedUpdate`, `Update`, and `PostUpdate`; current code does not implement one global
input → AI → movement → combat → zone → replication → persistence chain.

The draft design retains one 20Hz authoritative fixed simulation and adds explicit phase sets.
Per-zone system sets and cross-zone transfer events remain proposed rather than implemented.

## Bevy ECS as In-Memory Storage

Bevy's ECS is not just a game framework — it is the storage engine. It functions as a
columnar in-memory database with zero abstraction overhead.

### How Bevy stores data

When you spawn an entity:

```rust
world.spawn((Position(x, y, z), Health(100), Mana(500), Player { name }));
```

Bevy creates an **archetype** — a table where all entities sharing the same component set
are stored together. Components are packed into contiguous arrays:

```
Archetype: (Position, Health, Mana, Player)
  Position array: [pos_0, pos_1, pos_2, ...]   ← contiguous in memory
  Health array:   [hp_0,  hp_1,  hp_2,  ...]   ← contiguous in memory
  Mana array:     [mp_0,  mp_1,  mp_2,  ...]   ← contiguous in memory
  Player array:   [pl_0,  pl_1,  pl_2,  ...]   ← contiguous in memory
```

This is a **columnar layout** — identical to how analytical databases (DuckDB, ClickHouse)
store data for fast scans. The difference: Bevy has zero query overhead. There is no SQL
parsing, no query plan, no index lookup. A query is a direct pointer to a contiguous array.

### Queries are direct memory access

```rust
fn heal_all_players(mut query: Query<&mut Health, With<Player>>) {
    for mut hp in &mut query {
        hp.current = hp.current.min(hp.max);
    }
}
```

This compiles to a loop over `&mut [Health]` — a contiguous slice in memory. No hash
lookups, no pointer chasing, no serialization boundaries. It's the fastest possible way
to iterate game state on modern hardware because it maximizes CPU cache line utilization.

### Comparison with SpacetimeDB's storage

| Aspect | Bevy ECS | SpacetimeDB |
|--------|----------|-------------|
| Layout | Columnar (archetype tables) | Row-based (64 KiB pages) |
| Query | Direct array iteration | Scan pages + deserialize rows |
| Access overhead | Zero — pointer to contiguous memory | WASM boundary + BSATN decode |
| Change detection | Built-in `Changed<T>` filter (bitflags) | WAL diff after commit |
| Threading | Parallel system scheduling (automatic) | Single writer per database |
| Index | Archetype-based filtering | BTree / Hash / Direct indexes |

Bevy's archetype system is essentially a hash index on component composition. Asking
"give me all entities with Position and Health but not Dead" is O(1) archetype lookup +
O(n) array scan where n is exactly the matching entities. No false positives, no filtering.

### Built-in change detection

Bevy tracks mutations per-component per-entity automatically:

```rust
// Only iterates entities whose Health changed since last tick
fn log_damage(query: Query<(&Player, &Health), Changed<Health>>) {
    for (player, hp) in &query {
        info!("{} now at {} HP", player.name, hp.current);
    }
}
```

This powers the persistence layer: only save what actually changed. No dirty-flag boilerplate,
no manual tracking. The ECS does it at the component level with bitflags — zero runtime cost
when nothing changes.

## Persistence Strategy

Game state lives in the ECS during runtime. Persistence is a background save, not a hot path.
The ECS IS the database while the server runs. redb is just the durable backup.

### Save system (the entire persistence layer)

```rust
fn save_dirty_characters(
    query: Query<
        (&CharacterId, &Position, &Health, &Inventory),
        (With<Player>, Changed<Health>.or(Changed<Position>).or(Changed<Inventory>))
    >,
    db: Res<Database>,
) {
    if query.is_empty() { return; }
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(CHARACTERS).unwrap();
        for (id, pos, hp, inv) in &query {
            table.insert(id, &serialize(pos, hp, inv)).unwrap();
        }
    }
    txn.commit().unwrap();
}
```

This is the intended shape, but the current implementation's 300-tick internal early return
advances Bevy's `Changed<T>` window on intervening executions. At 20 Hz, the nominal periodic
interval is 15 seconds; the current due query can miss changes made earlier in that interval.
Logout/disconnect and shutdown saves remain separate boundaries. The ECS is the live source of
truth between durable writes.

Compare this to SpacetimeDB's persistence: WAL with 50ms sync interval, snapshot every 1M
transactions, crash recovery via log replay. All that machinery exists because SpacetimeDB's
storage IS the database. In our design, the ECS is the storage and redb is just a periodic
checkpoint — fundamentally simpler.

### What gets persisted (redb)

Character data saved on:
- Logout / disconnect
- Periodic interval (every 300 fixed ticks, nominally 15 seconds at 20 Hz)
- Server shutdown

```
Tables in redb:
  characters:    CharacterId → CharacterData (position, level, stats, ...)
  inventories:   CharacterId → Vec<ItemInstance>
  quest_states:  CharacterId → Vec<QuestProgress>
  social:        CharacterId → FriendsList, IgnoreList
  mail:          MailId → MailData
  auctions:      AuctionId → AuctionData
  guilds:        GuildId → GuildData
```

### What does NOT get persisted

- Transient combat state (current target, spell cooldowns in progress, active buffs)
- Movement interpolation state
- NPC runtime state (aggro tables, patrol progress)
- These are reconstructed from base state on login or respawn

### Static data (read-only, loaded at startup)

```
data/
  items.ron          ← ItemTemplate definitions (id, name, stats, icon, ...)
  spells.ron         ← SpellTemplate definitions
  creatures.ron      ← CreatureTemplate definitions
  maps/
    eastern_kingdoms.ron  ← Spawn points, zone boundaries, terrain refs
    kalimdor.ron
```

These can also be SQLite if the dataset is large (AzerothCore's world DB is ~500MB).
RON is simpler for smaller datasets and version-controllable.

## Networking with Lightyear

Lightyear handles the hard parts of game networking:

### Server → Client (replication)

```rust
// Server registers which components to replicate
app.replicate::<Position>();
app.replicate::<Health>();
app.replicate::<Mana>();
app.replicate::<CharacterAppearance>();
app.replicate::<CastingSpell>();
```

Lightyear automatically:
- Tracks which entities are near each client (interest management / relevance)
- Delta-compresses changed components (only sends diffs)
- Handles entity spawn/despawn as entities enter/leave client's area
- Serializes with configurable codec

### Client → Server (inputs)

```rust
// Client sends inputs, not state
#[derive(Serialize, Deserialize)]
enum PlayerInput {
    Move { direction: Vec3 },
    CastSpell { spell_id: SpellId, target: EntityId },
    Interact { target: EntityId },
    Jump,
    StopMove,
}
```

Server validates and applies. Client predicts locally, reconciles on server correction.

### Channels

| Channel | Reliability | Use |
|---------|------------|-----|
| Movement | Unreliable | Position updates, interpolation data |
| Combat | Reliable ordered | Spell casts, damage events, health changes |
| Chat | Reliable ordered | Chat messages, system notifications |
| Inventory | Reliable ordered | Item moves, loot, trade |

Lightyear layers reliable and unreliable game channels over its UDP transport.

## Zone Architecture

Zone partitioning is a proposed design, not current runtime behavior. The server currently
spawns every map 0 creature during `PostStartup`; it does not register per-zone system sets or
implement runtime zone-transfer events. See [zone architecture](docs/wiki/systems/zone-architecture.md)
for current implementation, proposed phase placement, and instance-isolation options.

## Comparison with AzerothCore

| Aspect | AzerothCore (C++) | This design (Rust/Bevy) |
|--------|-------------------|------------------------|
| Language | C++17 | Rust |
| ECS | Custom OOP hierarchy | Bevy ECS (data-oriented) |
| Tick rate | ~1ms main loop | 20Hz FixedUpdate (50ms) |
| Map threading | Thread pool (MapUpdater) | Bevy parallel system scheduling |
| Networking | Boost ASIO + custom WoW protocol | UDP + netcode via Lightyear |
| Persistence | MySQL (async queries) | redb (embedded, no server) |
| Static data | MySQL (loaded at startup) | RON/SQLite (loaded at startup) |
| State sync | Manual packet construction | Automatic via Lightyear replication |
| Client prediction | None (server authoritative, client waits) | Lightyear prediction + rollback |
| Scripting | C++ + Eluna Lua | Rhai or Lua (optional) |
| Build | CMake + vcpkg | Cargo |

## Reference Repos

- **AzerothCore** (`~/Repos/azerothcore`) — C++ WoW 3.3.5a emulator. Reference for zone threading
  (MapUpdater), combat formulas, spell system, creature AI, and the general server architecture.
- **Rustbolt Core** (`~/Repos/rustbolt-core`) — Rust WoW server attempt (early stage). Reference for
  Rust-specific patterns and WoW protocol handling.
- **SpacetimeDB** (`~/Repos/SpacetimeDB`) — Evaluated and rejected. See "Why Not SpacetimeDB" above.

## Open Questions

- **Tick rate**: WoW uses ~20Hz for combat, higher for movement validation. Single
  FixedUpdate at 20Hz or separate schedules at different rates?
- **World size**: floating-point precision at WoW scale (32km maps). Use `big_space`
  crate or origin-shifting?
- **Instance management**: dungeons/raids as separate Bevy worlds or same world with
  isolated system sets?
- **Hot-reloading**: encounter scripts need iteration without server restart. Lua/Rhai
  scripting layer vs dynamic Bevy plugins?
