# Maps, map transfers and dungeon instances (server)

Dungeons phase 1: several maps in one world, far teleports between them and dungeon copies per group. Code: `crates/server/src/maps/`. Protocol: shared-protocol `protocol/transfer_messages.rs` (`NewWorld`, `WorldPortAck`, `TransferAborted`). Client: game-engine `docs/specs/instances.md`. Behavior reference: TrinityCore master a352b1fa (functions cited per rule).

## What it must do

### Maps in one world
- [x] Every world entity carries `InMap { map_id, instance_id }` (`WorldObject::GetMapId` / `GetInstanceId`); continents are instance 0.
- [x] Eastern Kingdoms (0) and Kalimdor (1) are hosted (`maps::HOSTED_CONTINENTS`): both spawn their creatures and game objects at startup (44,378 + 49,408 creatures). Other world maps are refused (`MapNotAllowed`). Creatures spawn only on maps the client has NPC appearance data for (`maps::NPC_APPEARANCE_MAPS`: 0, 1, 34).
- [x] Visibility, creature aggro (`IsInMap` in `_IsTargetAcceptable`), spell area and explicit targets (`ObjectAccessor::GetUnit` searches the caster's map), grid activation, say/yell/emote range and group XP, quest and loot credit (`IsAtGroupRewardDistance`) only relate entities of one map copy. A creature whose victim leaves its map loses it and evades.
- [x] Player ground is per map: the map's `Map.db2` WDT either names one global WMO (MPHD 0x1; the Stockade, WMO 108631 placed from the world origin) or ADT tiles.
- [x] The character's map is saved (`character_maps`) with its position; a player in a transfer is saved at its destination (`m_teleport_dest`). Characters without a row are on map 0.

### Teleports (`Player::TeleportTo`, Player.cpp:1225-1466)
- [x] Same map: the player moves at once (`MovementControl` epoch).
- [x] Other map: `Map::PlayerCannotEnter` (Map.cpp:1807-1852) is checked first; a refusal sends `TransferAborted` and the player stays. Otherwise its selection and combat stop, its cast is cancelled, its taxi flight ends, it leaves its map (no `InMap`, `MapTransfer`) and gets `NewWorld { map_id, map_directory, position, facing }`.
- [x] `WorldPortAck` (`HandleMoveWorldportAck`, MovementHandler.cpp:189-384) adds the player to the copy `MapManager::CreateMap` gives it, after that copy's `CannotEnter`; a refused arrival sends `TransferAborted` and returns the player where it left from (TrinityCore uses the homebind, which the server has no data for). `TELE_REVIVE_AT_TELEPORT` resurrects on arrival, and so does entering the dungeon that holds the player's corpse.
- [x] A player in a transfer does not move, fall or fight.

### Dungeon copies (`MapManager::CreateMap` MapManager.cpp:154-235, `FindInstanceIdForPlayer` :278-319)
- [x] Five-player dungeons (`InstanceType` 1) at `DIFFICULTY_NORMAL` without reset schedule: a group enters its most recent copy of the map (`Group::GetRecentInstanceId`), a player without a group its own (`Player::GetRecentInstanceId`); without one a copy is created and remembered. Raids, scenarios, battlegrounds and dungeons with a reset schedule are refused (`MapNotAllowed`).
- [x] `InstanceMap::CannotEnter` (Map.cpp:2908-2946): a full copy (`MapDifficulty.MaxPlayers`, 5 for the Stockade) or one owned by another group answers `TRANSFER_ABORT_MAX_PLAYERS`.
- [x] A copy starts its unload timer when created and when its last player leaves (`InstanceMap::RemovePlayerFromMap`); after `Instance.UnloadDelay` (30 min) it is removed with its entities, and the next entry recreates it fresh under the same id.
- [x] Each Stockade copy spawns its own Retail creatures (143, TDB1210 Normal; [retail world content](retail-world-content.md)) and game objects of the spawned types when it is created, with the generic creature AI (boss scripts: phase 3); they respawn inside their copy and go when the copy unloads. Other dungeons open empty: their displays are not in the client's NPC appearance data.

### Area triggers (`HandleAreaTriggerOpcode`, MiscHandler.cpp:479-640)
- [x] `AreaTrigger.db2` sphere and box triggers with an `areatrigger_teleport` row (`world_safe_locs` target) teleport a player who walks into them (`Player::IsInAreaTrigger`: `IsInDist`, `Position::IsWithinBox`). A ghost uses only triggers whose `AreaTriggerActionSet` allows ghosts; a taxi passenger none. The first position on a map records the trigger stood in without firing.
- [x] Stockade: trigger 101 in Stormwind → safe loc 3599 (56.68, 0.62, -19.27) on map 34; trigger 503 inside → safe loc 3618 (-8766.11, 845.50, 88.00) on map 0.

### Dungeon deaths (`Player::RepopAtGraveyard`, Player.cpp:4720-4764)
- [x] Releasing in a dungeon turns the corpse to bones and teleports the player, revived at half health (`TELE_REVIVE_AT_TELEPORT`), to the graveyard `ObjectMgr::GetClosestGraveyardInZone` (ObjectMgr.cpp:6868-6970) picks for the dungeon's zone: same or parent map by 3D distance, else on `CorpseMapID` closest to `Corpse`, else any. A WMO-only dungeon's zone is its `Map.db2` AreaTableID (`TerrainInfo::GetAreaId`). Stockade (zone 717) → graveyard 1854 (-8786.88, 829.08, 97.65) in Stormwind.
- [x] Continent deaths keep the spirit-healer graveyards on the player's own map (Kalimdor deaths release to Kalimdor healers).

### Login (`Player::LoadFromDB`, Player.cpp:18480-18545)
- [x] A character enters its saved map's copy (`CreateMap`); a dungeon copy that refuses it, or cannot be created, sends it to the go-back trigger target (`GetGoBackTrigger`); a newly created dungeon copy puts it at the entrance (`GetMapEntranceTrigger`). `LoadTerrain` names the map's directory.

### Meeting stones across maps
- [x] An accepted summon teleports the target to the summoner's map and position (`TeleportTo(m_summon_location)`), with a map transfer when they differ.

### Admin
- [x] `game-server-admin teleport <name> <map> <x> <y> <z>` (`.go xyz`): online characters teleport (transfer across maps), offline ones are saved there.
- [x] `game-server-admin kill <name>` (`.die`).

## Gaps
- Instance locks, reset schedules, difficulties other than Normal, raids and `access_requirement` level checks (phase 4); Dungeon Finder entry (phase 5).
- No instance boot when a player leaves the owning group inside (`Player::CheckInstanceValidity` timer).
- `graveyard_zone` conditions (team, phase) and AreaTable parent-zone fallback are not imported; polygon and cylinder area triggers are not loaded (no teleport trigger uses them).
- TrinityCore's homebind fallbacks: no homebind data.
- Players carry no `Zone`; the zone of a continent position is not computed.

## Tests asserting this spec
`cargo test -p server --bin game-server maps::` (map store on real world.db, instance ids, max players, unload, Stockade entrance/exit/group/refusal, per-copy Stockade creatures and their unload, Kalimdor teleport, Stockade release), `interest::` (other-map targets hidden, transfer visibility), `grid_activity::`, `creature_aggro::` (other-map player not attacked), `meeting_stone::` (Kalimdor target summoned to Eastern Kingdoms, portal in the caster's map), `auth_character::` (Kalimdor login terrain), `admin_ipc::` (teleport, kill).

## Live evidence (2026-09-26)
Own server :5081 (copied world.db/game.redb), headless clients Fixshout, Fixpartner, Stormlight (level 20); screenshots and logs in game-engine `data/diagnostics/instances-20260926/`:
- Walking into trigger 101 transferred Fixshout to map 34 copy 1 at (56.68, -19.27, -0.62) (`02`, `03`); trigger 503 returned it to (-8766.1, 845.5) Stormwind through the loading screen (`04`, `05`).
- Fixshout and Fixpartner (one group) entered copy 2 and see each other; Stormlight (solo) got copy 3 at the same coordinates and sees neither (`06`, `07`, `instances-group-vs-solo.log`).
- After a server restart Fixshout, saved in map 34, logged into a new copy 1 at the entrance; `kill` then release teleported it to graveyard 1854 in Stormwind, alive at 2120/4240 (`08`-`10`).
- Stormlight, offered a summon at the Westfall stone and moved to Ratchet (Kalimdor), accepted there and was transferred to Fixshout in Westfall (`11`-`13`, `summon-kalimdor-to-westfall.log`). The reverse direction (Tanaris stone, target moved to Stormwind) was not completed live: the headless UI script could not time its click around the loading screen.

## Live evidence, Stockade content (2026-09-26)
Own server :5083 (worktree world.db = `world-stockade-20260926.db`, copied game.redb), headless clients on `npc_appearance-retail-stockade-20260926.sqlite`; screenshots, scene dumps and logs in game-engine `data/diagnostics/stockadecontent-20260926/`:
- Fixshout entered copy 1: Stockade guards, prisoners, Petty Criminals, Riverpaw gnolls, Rumbling Earth and Hogger render with their Retail models and customizations; 0 client panics in three client runs (`01`-`05`, `scene-05-hogger.txt`).
- Fixshout killed a Petty Criminal (no loot table in TDB1210) and Bruegal Ironknuckle, whose corpse opened 2s 69c and Jimmied Handcuffs (3228) (`06`-`08`).
- Stormlight (solo) got copy 2, where Bruegal was alive while dead in copy 1 (`09`, `scene-09-copy2.txt`).
