# Creature AI

Creature behavior is driven by a state machine in `server/src/creature_ai.rs`. Each creature has a `CreatureState` (Idle / Pursuing / Returning) that gates which movement subsystem runs.

## Movement Types

Determined by `creature.movement_type` in SQLite (see [[world-data]]):

| Type | Component | Behavior |
|------|-----------|----------|
| 0 | none | Idle — stands at spawn point |
| 1 | `WanderState` | Random wander within `wander_distance` of spawn |
| 2 | `WaypointState` | Follows the spawn's creature-addon `path_id` in `waypoint_data` |

## Wander System

`WanderState` is a two-phase state machine (`Idle` / `Walking`):

- **Idle phase**: counts down `idle_remaining` (2–5s random); on expiry, picks a random target point using an LCG seeded with the spawn guid (the state advances per pick; advancing the seed by one picked nearly the same point every time). A target with no ground, or more than `STEP_UP_HEIGHT` (1.6 yd) below the creature, is refused and retried 100 ms later (`RandomMovementGenerator::SetRandomLocation` path failure, `_timer.Reset(100)`).
- **Walking phase**: moves toward target at `speed_walk * BASE_WALK_SPEED (2.5 m/s)`; each step stands on the ground under it — the player rule ([[player-ground]] in game-engine: terrain and WMO floors, highest within 1.6 yd above the feet) through `GroundMap` and the creature's `InMap` (`UpdateAllowedPositionZ`). A step onto no support or a drop deeper than 1.6 yd ends the walk there. Without ground data (`Unloaded`) the height is kept. Ground loads from `GAME_SERVER_GROUND_DIR`, else `game-engine/data` beside the checkout (not the working directory), and the server refuses to start unless map 0's WDT opens there and every map with creature spawns has every ground file it reads (`open_ground_map`, `check_spawned_ground`; the error names each map and file). `(map, file)` rows of `crates/server/src/ground_gaps.tsv` (files `scripts/extract_ground.py` could not extract, and transports' `no WDT FileDataID`) are logged at startup as known gaps with a count instead. `scripts/extract_ground.py` extracts the missing files from local CASC (resumable). `deploy.sh` ships the WDTs, ADTs and WMOs of every map with creature spawns (`scripts/ground_files.py`, 27,641 files, 5.43 GB on 2026-10-01) to `/opt/game-engine/data` and sets the variable in a systemd drop-in. Before, ground came from `../game-engine/data` relative to the working directory: a server started elsewhere ran with no ground and every creature kept its spawn height (the Northshire Blackrock Spy chased and wandered polymorphed 5 yd above the ridge it spawned on; `tests/ground_data.rs`). On arrival, transitions back to Idle with a new random pause

Before this (2026-09-27) wanderers kept the spawn height: Stormwind Rat 20314034 walked through the wall by the Stockade entrance and hung 11 yd over the canal (86.7), 20314037 stayed 0.37 yd above the floor under its TDB z. Current chase/home ground and path behavior is documented in [navigation](navigation.md).

Wander respects `SpawnPoint.wander_distance` — targets are always within that radius of the spawn.

## Waypoint System

`waypoint_data.id` is a **path ID**, not a creature guid. A MovementType 2 spawn walks `path_id` from its `creature_addon` row, else its `creature_template_addon` row (AzerothCore `Creature::GetCreatureAddon` → `LoadCreaturesAddon` sets `m_path_id`; `WaypointMovementGenerator::LoadPath` reads `GetWaypointPath()`). With no addon path the spawn idles: AzerothCore never derives `guid*10` (`ObjectMgr::LoadCreatureAddons` resets such spawns to idle). Of 6,554 MovementType 2 spawns, 6,543 resolve a path this way.

Per point (`WaypointMovementGenerator::StartMove`/`OnArrived`): walk to the point, pause `delay` **milliseconds**, then start the next leg; the path wraps after the last point. `move_type` 0 walks (`speed_walk * 2.5`), 1 runs (`speed_run * 7.0`); 2/3 (land/takeoff) also use run speed since flight is not modelled. A point's `orientation` is faced (`Rotation`) on arrival only when it has a delay. `waypoint_data.action` (waypoint_scripts) is not run. Waypoint coordinates are WoW space, converted via `wow_to_bevy()`. Each leg uses [navigation](navigation.md) without changing arrival/delay/orientation/wrap semantics.

## Formations

`creature_formations` members with `GROUP_AI_FLAG_FOLLOW_LEADER` (0x200) get `FormationMember { leader, follow_dist, follow_angle }` when their leader is spawned; groups whose leader has no own row never move (AzerothCore `CreatureGroup::AddMember`). `formation_system` runs after `waypoint_system` (`CreatureGroup::LeaderMoveTo`): each tick the leader moves, a member heads for the leader's current point offset by `follow_dist` at `follow_angle` (degrees in the table) from the leg direction, closing its distance in the leader's proportion so both arrive together. Members hold while the leader pauses or fights. Dead members, and members of a dead leader, stay where they are (`LeaderMoveTo` skips dead members; a dead leader never reaches `StartMove`; `setDeathState` dismisses the group with `FormationReset(true, false)`). AzerothCore mirrors the angle when the leader turns more than 90° from its facing; the server always uses the travel-direction offset. Assist/evade group AI flags are not implemented.

## Death and Respawn

Wander, waypoint and formation systems skip entities with `DeathState`. Every spawn carries a `SpawnPoint` and a `RespawnDelay` of `creature.spawntimesecs` + corpse decay by rank (60 s normal, 300 s elite/rare/rare elite, 600 s world boss; AzerothCore `Creature::setDeathState` `m_respawnDelay + m_corpseDelay`, default `Corpse.Decay.*`). `combat::respawn_system` restores the creature at its spawn when the timer ends; a patroller restarts its path from the first point (`WaypointState::restart`; AzerothCore `Creature::Respawn` → `Motion_Initialize`). A respawned escort, or the escorts of a respawned leader, rejoin their slots on the leader's next leg.

## Chase and Aggro

`CreatureState::Pursuing { target }` overrides wander/waypoint AI. `chase_system` runs at `speed_run × 7` yd/s (`ChaseMovementGenerator`), stopping `CHASE_REACH` (2 yd) short of the actual victim on the terminal route leg, with an earlier current-victim range/LOS stop before cached intermediate corners. Chase, home and waypoint legs share the [baked navigation resource](navigation.md), with ground support rather than target-height interpolation. That page defines repathing, failed-route evade and the explicit unbaked-map boundary.

Who enters `Pursuing`, threat and target selection are [[creature-aggro]].

## Confused movement

A creature under MOD_CONFUSE 5 (Polymorph) carries `auras::Confused`, anchored where it was confused. `confused_movement_system` replaces chase, wander, waypoint, formation and leash for it (`MoveConfused` on `MOTION_SLOT_CONTROLLED`): it walks at `speed_walk × 2.5` × the aura speed multiplier to a random point up to 2 yd either way from the anchor (`4 × frand − 2` yd at a random angle), pauses 0.8-1.5 s, and picks the next; it holds still while its speed is 0; a point without ground is retried after 100 ms. See [[auras]].

Every movement system writes the creature's replicated `CreatureMotion` (Still, Walk, Run) and, while it moves, `MovementSpeed`: the yards per second of that gait after auras (wander `speed_walk × 2.5`, chase `speed_run × 7 ×` the aura multiplier, confused walk, run home, waypoints and formations by leg type, scripted point moves). TrinityCore sends the same to viewers (`SMSG_MOVE_SET_WALK_SPEED`/`RUN_SPEED`); the client paces the walk and run clips by it. A Frostbolt-chilled Blackrock Spy replicates 3.0 while it runs, its sheep 2.5 while it walks (`polymorph_tests.rs`).

## Leash and Evade

`leash_check_system` runs after chase each tick. Party/raid/scenario maps skip it, matching TC `Creature::CanCreatureAttack`'s dungeon home-distance exemption; script-authored boundaries remain active. Open-world behavior is unchanged. [Contract and superseded Stockade proof](../../specs/creature-aggro.md). Outside dungeon maps, a pursuing creature more than **40 yards** (2D distance) from its `SpawnPoint` evades through `creature_aggro::enter_evade_mode` (threat cleared, `combat_stop`, health unchanged, `CreatureState` → `Returning`; see [[creature-aggro]]).

`return_to_spawn_system` follows the [navigation route](navigation.md) back to its spawn point at `speed_run × 7` yd/s (`MoveTargetedHome` runs by default). On arrival, `CreatureState` → `Idle` and normal AI resumes.

## Creature addon mount and auras

Spawn-time `creature_addon`/`creature_template_addon` fields (`zones.rs`): `mount` → replicated `Mounted { mount_display_id }`; `auras` → self-cast through the aura runtime on the next FixedUpdate (`apply_addon_auras`; spells missing from `SpellStore` are logged). `bytes1` byte 0 (stand state), `bytes2` byte 0 (sheath state) and `emote` (`UnitData::EmoteState`) → replicated `UnitPose` (TrinityCore `Creature::LoadCreaturesAddon`; out-of-range stand/sheath values are truncated to 0 with an error, as `ObjectMgr::LoadCreatureTemplateAddons`). A spawn without an addon row stands with melee weapons drawn (`Creature::UpdateEntry` `SHEATH_STATE_MELEE`). A creature that engages stands up and drops its emote state (`Unit::Attack`); `SpawnPose` restores the addon pose when it reaches home (`HomeMovementGenerator::DoFinalize` → `LoadCreaturesAddon`). `visibilityDistanceType` is imported but not applied.

Equipment: `creature.equipment_id` picks the entry's `content_creature_equip_template` row (`Creature::LoadEquipment`; 0 none, -1 a random ID, summons ID 1). Its items become a replicated `EquipmentAppearance` (MainHand, OffHand, Ranged) with the ItemDisplayInfo of each (ItemID, AppearanceModID) `item_modified_appearance` row and the item's InventoryType; an item without either is left out with an error.

## SpawnPoint Component

Every spawned creature carries a `SpawnPoint`:
- `x, y, z` — original spawn coordinates
- `wander_distance` — radius for random wander
- `speed_walk` — creature's walk speed multiplier (from `creature_template.speed_walk`)
- `speed_run` — run speed multiplier for chase and evade (from `creature_template.speed_run`)

Also used by `combat.rs` to reset position on respawn after death.

## Grid Activation

`grid_activity.rs` follows AzerothCore grid activation (`Map::Update` → `MarkNearbyCellsOf`, `Creature::IsUpdateNeeded`): map 0 is bucketed into 533.3333 yd grids (`SIZE_OF_GRIDS`) and a creature runs its per-tick systems only while one of its home grids is within one grid (3x3) of a player's grid. Home grids are the spawn grid plus every grid the creature's patrol path, or its formation leader's path, crosses. Other creatures carry `GridDormant`; wander, waypoint, formation, chase, leash, return-to-spawn, `update_auras` and `sync_unit_auras` skip them. The active set is recomputed in `FixedPreUpdate` only when a player changes grid or enters/leaves the world, and only creatures in grids that changed are toggled. Unlike AzerothCore, patrollers and formation members are not updated everywhere; covering their path grids keeps them active wherever a player can see them.

Active SmartAI escorts carry `ScriptedPath` even while paused at an ambush. Path start removes `GridDormant`; grid registration and sleeping transitions do not put active paths to sleep. This keeps combat cleanup, timed continuation and out-of-range failure running beyond the original spawn grid. Feero976 reproduced the failure at second camp3756.44 (spawn grid5, acceptor grid7); same-coordinate lifecycle regression and both-skin earned native completion cover the correction. See [escort contract](../../specs/escort-quests.md).

Timers: dormant state resumes where it stopped. Aura expiry and periodic tick times are absolute, so on reactivation expired auras end and missed ticks run at once. `respawn_system` is not gated (it only visits dead creatures): a dormant creature respawns at its spawn point on its normal timer.

With all 24,521 map 0 creatures and no players, an idle dev server uses about 8% of one core (81% without grid activation).

## Coordinate System

Creature data is stored in WoW coordinate space. The AI operates in Bevy space (X-right, Y-up, Z-back). The `wow_to_bevy()` function in `terrain_height.rs` handles the conversion at waypoint load time.

## Sources

- [crates/server/src/creature_ai.rs](../../../crates/server/src/creature_ai.rs) — CreatureAiPlugin, all AI systems
- [crates/server/src/zones.rs](../../../crates/server/src/zones.rs) — AI component assignment on creature spawn
- [CLAUDE.md](../../../CLAUDE.md) — movement type values (0/1/2)

## See Also

- [[world-data]] — `creature`, `creature_template`, `waypoint_data`, `creature_addon`, `creature_formations` tables
- [[creature-aggro]] — aggro on sight/damage, threat, evade
- [[combat]] — the creature's `CombatTarget` drives its auto-attack
- [[navigation]] — shared offline geometry, baked routes and unreachable chase behavior
- [[tick-model]] — AI systems run at step 2 of the 20Hz pipeline


## Current-victim stop contract — 2026-10-17

Run22's final pull initially engaged three units through real Avenger's Shield chain hits. Three further units attacked Garrick before receiving party damage as his chase entered their proximity radii. No matching formation links exist for those six source spawns; the final approach ran at20ticks/s. Exact per-tick server victim/path state was not recorded, so this does not establish which cached corridor caused that run's first extra acquisition.

A production RED over the real Stockade bake proves a bounded correctness gap independently: chaser `(121.1,-25.6,-3.9)`, current victim `(122.66,-25.61,-3.91)` (1.560yd, server LOS clear), cached destination `(123.6,-25.61,-4.14)` (0.968yd drift, still reusable). `chase_system` advanced to x121.49999 in50ms because `Corners::advance` deferred reach checks until the corridor's final leg. The cached destination is constructed from retained geometry, not relabeled as an actual recorded route.

Before advancing a cached route, shared chase now validates the current victim and stops in the existing2yd region with authoritative server body-ray LOS; smaller follower reaches stay smaller. Stopping drops the stale corridor and publishes Still/facing through the existing movement queues. Dead/despawned/ghost/wrong-copy victims and explicit attack-victim changes stop the old chase; ordinary AI retains responsibility for reselecting an already engaged live alternate or following its owner when none remains. Aggro, selection, pull policy, kits and stats are unchanged. A nearby target behind a wall is not a successful in-range stop. The new pre-stop requires an actual `LineOfSight` resource; the existing no-resource test-world visibility convention must not certify it.

Pinned TC a352b1fa sources: `ChaseMovementGenerator.cpp:28–30,44–55,99–110,125–140` (lost target/range+LOS stops), `CreatureAI.cpp:244–261` and `ScriptedFollowerAI.cpp:145–147` (valid-victim selection), `Creature.cpp:2165–2206` (unchanged aggro distance). Source SHA receipts and original production RED live under `/home/osso/.worktrees/bot-harness-2026-10-15/` and engine `data/diagnostics/bot-predicate-2026-10-17/tc-a352b1fa-chase/receipt.json`. New controls cover actual follower death/reselection ordering and real-nav invalid-target, movement-resumption and blocked-LOS behavior. Native run23 and full clear remain separate acceptance, not established by this RED.


### Run23 diagnostic frames

Enable `game_server::chase_trace=debug,game_server::creature_sight=debug,game_server::creature_aggro=debug` in `RUST_LOG` for the bounded private run. The selective follower query logs each tank's actual entity, `CombatTarget` victim, `CreatureState`, position and health before chase decisions on every fixed tick. A same-actor/same-`simulation_secs` event logs the borrowed cached route: map/goal, destination, expected origin, complete corners, next-corner index and terminal leg (or explicit absent cache). Player/creature proximity acquisitions carry the same fixed simulation clock, their real target and positions. This captures the formerly missing run22 boundary without changing target selection, route progress, motion or aggro. Trace is disabled unless its dedicated debug target is enabled; it does not scan the whole world for tank identities.
