# Persistence

## Physical Equipment

`grant-equipment <character> <manifest.json>` accepts a trusted JSON array with `item_id`, `name`, `quality`, `required_level`, `inventory_type`, `vendor_sell_price`, `allowable_class`, and `allowable_race`. Masks use one-based class/race bits; `-1` permits every class/race. The operation validates the complete batch under a single redb write transaction, then reserves item-instance GUIDs and writes both slot ownership and item-based appearance entries. Existing items, appearances (including hidden overrides), bags and gold are preserved; occupied slots are rejected, not replaced.

The separate `character_equipment` table maps character IDs to unique-slot records containing `InventoryItemData`. It does not change the encoded `CharacterData` structure. Equipped instances are not inserted into the auction/trade bag inventory. Normal character saves load the latest character appearance and do not overwrite this ownership table. `list-equipment <character>` reads persisted slot/GUID/item records. These operations do not add combat damage, armor calculations, or bonus stats. See [admin equipment contract](../../specs/admin-equipment.md) for test coverage and limits.


Dual-layer storage: the ECS is the live database, redb is the durable checkpoint. SQLite holds read-only static world data loaded once at startup.

## Content

## Design Principle

The ECS is the live storage engine while the server runs; redb is the durable checkpoint. Current redb reads and writes execute synchronously on the Bevy thread rather than through a background worker, so database latency can stall the game loop. This is still simpler than making every gameplay mutation a database transaction, but persistence must be cadence- and dirty-bounded.

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
| `character_progression` | `CharacterId` | Active spec ID and action bar ([[class-progression]]) |
| `character_traits` | `CharacterId` | Trait purchases per spec ([[talents]]) |
| `character_corpses` | `CharacterId` | `CorpseData` (map, Bevy-space position, death time) of a released ghost. Periodic, logout and shutdown saves (`save_live_character`) write it while `PlayerDeathState` is Ghost and delete it otherwise; a 0-health character with a row logs in as a Ghost with that corpse and the corpse's graveyard (`death.rs` `restore_persisted_ghosts`, AzerothCore `Player::LoadFromDB` `PLAYER_FLAGS_GHOST` + `Player::LoadCorpse`); without one it dies again on login (Dead at its saved position) |

File: `data/game.redb`

## Legacy Database Incident: Missing `pvp_states`

On August 7, 2026, an existing `data/game.redb` database lacked the `pvp_states` table because `create_tables()` did not open `PVP_STATES`. Character selection then panicked while loading PvP state with `TableDoesNotExist("pvp_states")`. The documented `run-dev.sh` loop restarted the server after the panic; the client repeatedly re-entered `InWorld`, eventually triggering repeated GPU resource allocation and a downstream GPU out-of-memory failure. The rendering failure was a consequence of the server restart loop, not the root cause.

Commit `ec5ff52` fixed the migration path by adding `PVP_STATES` to `create_tables()`. It also added a regression test that creates a legacy database without the table, runs `create_tables()`, and verifies `load_pvp()` returns the default state.

## Legacy Inventory Encoding Incident

Production inventory rows for characters 6 and 7 used the pre-April 6, 2026 `InventoryItemData` encoding, which lacks `unique_auction`. Commit `01eaf52` made startup eagerly rewrite those legacy bytes to the current encoding, setting `unique_auction` to `false`. If inventory bytes match neither the current encoding nor a known legacy encoding, startup now fails explicitly; it does not install a permanent load-time fallback.

## Character customization selections

`examples/read_character.rs` inspects a saved row, verifies its decode→current-storage encode→decode round trip, and prints `stored_bytes` without saving the re-encoded row.

Runtime `CharacterData` has no bitcode traits. All six production write paths (single save, creation/name save, inventory transaction, equipment grant, live save and startup migration) encode a private `StoredCharacterData` V7 DTO. Its equipment field and V3–V6 legacy fields use frozen pre-source entry fields and slot variant order. Shared `EquippedAppearanceEntry.definition_source` never enters character bytes. Current-layout startup detection reads the stored DTO, so valid V7 rows remain byte-identical without a new version or decoder path.

`load_character` reads physical slot ownership and global GUID markers in the same read transaction, replacing cached item sources before roster/login/inventory consumers receive the data. Slot/ItemID disagreement fails explicitly. Historical item caches without owned rows remain known Retail; display-only entries have `None`. Equipment grants hydrate existing cache entries inside their write transaction before returning them. Single/new-character and inventory writers reject Forever caches without matching owned GUID and Forever marker; creation persists the inventory record before saving character/name metadata. Source-bearing accidental wire layouts and unowned Forever cache entries are unsupported, not new compatibility schemas. See [[inventory]] for GUID marker ownership.

Stored character appearance carries the shared appearance contract: six core selectors plus `sex`, and ordered non-core `(option_id, choice_id)` pairs. Historic encodings are decoded through frozen schemas and rewritten once with an empty additional-selection vector; current encodings are written without an active legacy fallback. Creation, temporary reopen and login-roster tests preserve nonempty pairs unchanged. The server does not yet evaluate the engine's local customization catalog, requirements or effect support before accepting a pair; that remains engine/catalog integration work. See [character customization storage](../../specs/character-customization-storage.md).

## Save Triggers

Character data is written to redb on:
- **Logout / disconnect** — immediate flush for the departing player
- **Periodic interval** — every 300 fixed ticks, nominally 15 seconds at 20 Hz
- **Server shutdown** — final flush of all live characters

The periodic system currently executes on every fixed tick but returns for 299 of 300 executions. Because Bevy advances the system's change-detection window on each execution, its due `Changed<Position|Health|Player>` query does not preserve mutations from the full 300-tick interval. This is a confirmed correctness bug; see [[tick-loop-audit]].

## Inventory Gold Replication

Gold is `u64` copper (`InventoryData.gold`, replicated `Gold(u64)`), capped at `gold::MAX_MONEY` (Retail 9,999,999g 99s 99c). Inventories are stored in `character_inventories`; `create_tables` moves rows of the older `u32`-gold `inventories` table (either layout) into it in one transaction and drops the old table. Login/select still loads persisted inventory gold into the live player's replicated `Gold`. Runtime writers record the persisted value in `GoldUpdateQueue` after successful saves; `apply_gold_updates` drains same-frame records coalesced by character ID in `PostUpdate` before Lightyear buffering. Only matching online players are updated, equal values do not dirty `Gold`, and offline records are discarded because the next login reloads redb.

The covered mutation boundaries are barber payments, completed trades, auction creation/bids/refunds/buyouts, and mail claims. Profession inventory saves do not mutate gold, and auction sale proceeds remain in mail until claim. This replaced the unconditional `sync_player_gold_from_inventory` database/player scan across commits `4e4c73d`, `c3b3c04`, `9d2067c`, `91bbedd`, and `8920feb`.

## What Is NOT Persisted

These are transient and reconstructed on login or respawn:
- Active combat state (current target, in-progress cooldowns)
- Active auras / buffs (except those with explicit persistence)
- Movement interpolation state
- NPC runtime state (aggro tables, patrol progress, wander seed)

## SQLite (Static World Data)

Read-only at runtime. Loaded into `Res<WorldData>` and `Res<SpellStore>` at startup. See [[world-data]] for full table reference.

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Persistence Strategy" section
- [AGENTS.md](../../../AGENTS.md) — redb file path (`data/game.redb`)
- [crates/server/src/persistence.rs](../../../crates/server/src/persistence.rs) — table creation and save/load systems
- [crates/server/src/persistence_runtime.rs](../../../crates/server/src/persistence_runtime.rs) — periodic and shutdown save implementation
- [crates/server/src/persistence_tests.rs](../../../crates/server/src/persistence_tests.rs) — legacy-database migration regression test added by `ec5ff52`
- [crates/server/src/persistence_equipment.rs](../../../crates/server/src/persistence_equipment.rs) — atomic physical-equipment ownership persistence

## See Also

- [[ecs-storage]] — why the ECS makes change-tracking trivial
- [[world-data]] — the SQLite side (creature templates, spells, spawn data)
- [[tick-model]] — observed persistence schedule and fixed-tick cadence
- [[tick-loop-audit]] — confirmed periodic-save change-detection bug
- [Administrative physical equipment](../../specs/admin-equipment.md) — trusted manifest contract and boundaries
