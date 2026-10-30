# Swimming and breath

Terrain water on the server: MH2O liquid per ADT chunk, the swim height bounds that let the server follow a swimming client, and TrinityCore's breath and fatigue mirror timers with drowning. Requirements: [player height](../../specs/player-height.md), [breath and drowning](../../specs/breath-and-drowning.md).

## Content

### Liquid data

`terrain_height::parse_adt_heights` also reads the root ADT's top-level `MH2O` (`liquid.rs`): for each of the 256 chunks, in MCNK order, its liquid instances (`LiquidType` ID, min height, x/y offset, width/height, exists bitmap one byte per row, vertex heights by vertex format: 1 height+UV, 2 depth only, 3 height+UV+depth, else a float array) and the attributes' `deep` mask. `TerrainHeightMap::liquid_at(wow_x, wow_y)` returns the highest covering layer's bilinear surface (`LiquidSample { level, depth_level, liquid_type, deep }`, `depth_level` the terrain height); `GroundMap::liquid_at(map, feet)` loads the tile. The sampling copies the Godot client (game-engine `adt_tex_water.rs`, `terrain_height_data.rs`) so both agree on the surface; TrinityCore's extractor reads the exists bitmap as packed bits, which differs from one byte per row only for layers narrower than 8. A global-WMO map has no terrain liquid. Northshire lake (Azeroth 32,48, WoW -8558, -490): seabed 140.04, surface 143.99 (liquid type 5); the Godot client floats at 142.74. None of the 81 cached root ADTs under `../game-engine/data/terrain` has a `deep` mask, so no cached tile fatigues yet.

`los::wmo_liquid` parses active MLIQ tiles into liquid triangles, resolves the group liquid type, and applies each WMO placement transform. `GroundMap::liquid_at` samples those surfaces for global WMOs and nearby ADT placements, and uses the collision floor below the swimmer for depth, so a higher overlapping floor does not hide deep water. For overlapping terrain and WMO surfaces it selects the surface nearest the feet. The client does not yet use WMO liquid for swim prediction.

### Swim height

`networking_movement::swim_bounds`: water where `shared::movement::is_swimming(floor, floor, level)` (1.25 yd over the ground) gives `floor` (ground probe, else `depth_level`) and `top` = `swim_top(level)`. A `swimming` input over such water adds a vertical time (`|Δy| / (SWIM_SPEED × aura speed)`) to its bank request; the moved height is held to `[floor, top]` at the new x/z. `apply_terrain_gravity` skips gravity for a player where `is_swimming` holds and resets its `FallTracker`, so falling into water does no damage and an idle swimmer keeps its depth. Land bounds: [[networking]].

### Mirror timers

`mirror_timers.rs` ports `Player::HandleDrowning` literally: `MirrorTimers { value[3], flags, flags_last }` per player (server-only, inserted at world entry), `UNDERWATER_INWATER` / `UNDERWATER_INDARKWATER` from `exposure_at` each `FixedUpdate` after gravity, and `handle_drowning` returning the `MirrorTimerStart`/`MirrorTimerStop` to send and the environmental damage to deal.

- Under water: `liquid_status(level, depth_level, feet, DEFAULT_COLLISION_HEIGHT)` is `UnderWater` (surface more than 2.03128 over the feet, `GridMap::GetLiquidStatus`). Dark water: the chunk's `deep` mask with any status but `NoWater`.
- Breath max 180 000 ms × `water_breathing_mult` (aura 155), disabled by `water_breathing` (aura 82) or death; fatigue 60 000 ms.
- In: first tick starts full (scale -1); later ticks subtract the tick; below 0, +1 000 and one damage (`environmental_damage`: 20% max health + small `urand`). Resend only when the flag was off last tick.
- Out: +10 × tick; resend once (scale 10) the first tick out; stop when full.
- Water breathing gained or lost desyncs `flags_last` (`UpdateMirrorTimers`), so a running breath bar stops.
- Dead: `stop_mirror_timers` stops every running timer. Damage marks replication dirty and a lethal hit writes `PlayerDeathTransition`.
- Messages go to the connection whose `PlayerLink` is the player, on `MirrorTimerChannel`.

Live check (2026-09-29, private server on :5057 with a copy of game.redb, the Godot `swimming_live.gd` fixture with `SWIM_DROWN=1`): the server dealt drowning hits a second apart (292 → 233 → 174 → 115 → 57) and the client's replicated health followed. The :5000 failure the same day came from the server clock falling behind: 4 000 ticks in 326 s of wall time, so the server's 180 s of breath had not run out when the client's bar reached 0. Logs from 2026-09-28 before these timers existed show the same lag, at 23.4 s of wall time per 10 s of ticks on average. The timers count fixed-step time; TrinityCore passes real elapsed time to `HandleDrowning`. Cause and fix of the lag: [[server-tick-lag]].

Numbers differ from classic WoW (60 s breath, lower damage): TrinityCore master and AzerothCore (`WaterBreath.Timer` 180000) both use 180 s and a 1 s drowning cadence.

## Sources

- `~/Repos/TrinityCore/src/server/game/Entities/Player/Player.cpp` — `HandleDrowning`, `getMaxTimer`, `SendMirrorTimer`, `StopMirrorTimer(s)`, `UpdateLiquidMirrorTimerFlagsOnPositionChange`, `KillPlayer`
- `~/Repos/TrinityCore/src/server/game/Maps/GridMap.cpp` — `GetLiquidStatus`
- `~/Repos/TrinityCore/src/server/game/Server/Packets/MiscPackets.cpp` — mirror timer packets
- `~/Repos/TrinityCore/src/tools/map_extractor/System.cpp` — MH2O liquid flags, DarkWater
- `~/Repos/azerothcore/src/server/game/Entities/Player/Player.cpp`, `World/WorldConfig.cpp` — the same rules, `WaterBreath.Timer`
- game-engine `docs/specs/swimming.md` — client swim rules and mirror timer UI

## See Also

- [[networking]] — input adoption, land height bounds, gravity
- [[auras]] — water breathing aura fields
- [[combat]] — fall damage and player death transitions
