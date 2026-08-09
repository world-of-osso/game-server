# Tick Model

The server's core simulation runs at 20 Hz using Bevy's `FixedUpdate` schedule — one tick every 50ms. Systems that do not require tick-rate work use explicit lower-frequency or state-driven gates.

## Tick Pipeline

Each tick runs these stages in order:

1. **Receive & apply client inputs** — `PlayerInput` messages (Move, CastSpell, Interact, Jump, StopMove) are dequeued and applied to player entities
2. **AI systems** — `wander_system`, `waypoint_system`, `chase_system`, `leash_check_system`, `return_to_spawn_system` run in parallel where independent
3. **Movement validation** — server-authoritative position update; applies gravity, terrain snapping, speed limits
4. **Combat resolution** — `start_attack_system`, `auto_attack_system` (including the direct NPC death transition), `respawn_system`
5. **Zone transition checks** — entities crossing zone boundaries are handed off via events
6. **Lightyear replication** — delta-compressed changed components sent to clients
7. **Persistence flush** — every N ticks (every 5 minutes), dirty entities written to redb

NPC death is not a separate all-living-NPC health poll. The authoritative melee damage boundary handles the lethal transition when `try_swing` reduces an NPC to zero health; respawn remains a state-bounded timer query.

## Lower-Frequency Gates

Auction expiration is registered in `FixedUpdate` but runs immediately once and then at most once per second. Sub-second ticks skip `expire_auctions` entirely; expiration, sale completion, persistence, and auction-mail behavior remain unchanged.

Terrain streaming remains in `FixedUpdate` so position changes are observed promptly, but unchanged player tiles return before the 3×3 neighborhood loop. Initial entry, reconnect, boundary crossings, and teleports still evaluate terrain tiles. See [[recurring-loop-elimination]].

## Zone-Parallel Scheduling

Each zone is a separate system set. Bevy's parallel scheduler runs independent zones on different threads simultaneously. Zones that need to interact (entity crossing a boundary) synchronize via events at tick boundaries — the same model as AzerothCore's `MapUpdater` thread pool.

## FixedUpdate vs. AzerothCore

AzerothCore uses a ~1ms main loop and manually batches work. Bevy's `FixedUpdate` at 20Hz gives a clean, deterministic cadence that naturally batches updates and matches WoW's actual combat tick rate.

## Open Questions

- Single 20Hz schedule vs. separate schedules at different rates (movement may want higher frequency)
- Instance isolation: dungeons/raids as separate `World` instances or same world with isolated system sets

## Sources

- [ARCHITECTURE.md](../../../ARCHITECTURE.md) — "Server Tick Model" section
- [crates/server/src/main.rs](../../../crates/server/src/main.rs) — `FixedUpdate` app setup
- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — direct NPC death transition and state-bounded respawn
- [crates/server/src/auction_house.rs](../../../crates/server/src/auction_house.rs) — one-second auction expiration gate
- [crates/server/src/networking.rs](../../../crates/server/src/networking.rs) — terrain tile-transition gate

## See Also

- [[zone-architecture]] — parallel zone system sets within the tick
- [[combat]] — combat systems run in step 4 of the pipeline
- [[creature-ai]] — AI systems run in step 2
- [[persistence]] — flush runs at step 7
- [[ecs-storage]] — `Changed<T>` makes step 7 cheap
- [[recurring-loop-elimination]] — recurring scans replaced by cadence, state, or event boundaries
