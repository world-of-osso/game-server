# Dungeon simulator

Transport-free Stockade qualification runs a plain Bevy App inside the server test executable. It registers real combat, spell/aura, threat, scripted creature AI and follower systems directly. The game-server entry point, ServerPlugins and ProtocolPlugin are not used. [Contract](../../specs/dungeon-simulator.md).

## Frozen geometry

`content/dungeons/stockade-34.json` contains 143 Normal source packs and 367 native-query walk-distance edges (70 KB). Pack identity matches source formations, never proximity clusters. Extraction retains a minimum spanning forest plus four nearest neighbours; every node is connected. Only the explicitly ignored extraction test opens the nav bake. Runtime positions are flattened to a plane; travel consumes source speeds and frozen edge distances, not the straight-line endpoint distance.

## Execution

Build the filtered server test executable through `scripts/build-host.py --test -p server --bin game-server --no-run`. Pass Cargo's printed executable to `scripts/run-dungeon-simulator.py --test-executable PATH --output DIRECTORY`, via `agent-run stockade` after capacity admission. The runner supplies repository-root working context because gameplay Startup loaders use `data/world.db`. It never launches a server or connects a client.

Five sourced kits: Garrick, Crenna, Shuja, Meredy, Austin. Austin is the in-process leader using the same kit controller, not a self-owned follower. This explicitly differs from the older live Fury72 owner.

## Proof and current limits

- Graph census passed against actual Normal spawn GUIDs and formation IDs.
- Isolated opening pack20375759 cleared at5.70 simulated seconds, with221 real combat events, before full-route work. That fixture deliberately narrows the source population to one pack; it is not dungeon-clear proof.
- Full-population runs produce actual HP/mana/position samples, native pull decisions and exact source-actor boss death receipts. No Stockade boss clear is qualified yet.
- Route3 stalled at160.05s on20375765: trailing wall-time history erased DPS during an OOC forecast/recovery hold. User clarified ten **active-combat** seconds, retained across OOC and reset at dungeon start/full wipe. Production correction is committed; GREEN pending. Five-second HP stability still uses wall time. Observed dead-spawn knowledge survives a same-copy wipe.
- Combat RNG repeatability and seeded batch reporting remain unfinished. Current diagnostic seeds cover spawn rolls only. No low-wipe-rate or live acceptance claim.

## Sources

- [Simulator spec](../../specs/dungeon-simulator.md)
- [Follower policy](../../specs/follower-dungeons.md#shared-tank-pull-project-policies)
- [Runtime](../../../crates/server/src/dungeon_simulator/runtime.rs)
- [Route/report](../../../crates/server/src/dungeon_simulator/campaign.rs)
- [Frozen graph](../../../content/dungeons/stockade-34.json)

## See Also

- [[follower-data]] — full native class/spec kits and tank policies.
