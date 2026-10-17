//! Dungeon difficulty selection and the saved-instance messages (TrinityCore
//! `WorldSession::HandleSetDungeonDifficultyOpcode`, MiscHandler.cpp:936-996;
//! `HandleRequestRaidInfoOpcode`, `HandleResetInstancesOpcode`,
//! `HandleSetSavedInstanceExtend`; `Player::SendDungeonDifficulty`, `SendRaidInfo`).
//!
//! A player's dungeon difficulty (`DungeonDifficulty`, saved per character) is the
//! difficulty it creates dungeon copies at; in a group the group's difficulty
//! (`GroupRegistry::dungeon_difficulty_of`) applies and only the leader changes it,
//! for every member. A change is refused inside an instance; it resets (and makes the
//! group or player forget) its unlocked copies (`ResetInstances(OnChangeDifficulty)`).

use bevy::prelude::*;
use lightyear::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};
use shared::components::Player;
use shared::protocol::{
    DungeonDifficultySet, InstanceChannel, InstanceInfo, InstanceReset, InstanceResetFailed,
    InstanceResetFailedReason, InstanceSaveCreated, RaidInstanceMessage, RaidInstanceMessageType,
    RequestRaidInfo, ResetInstances, SetDungeonDifficulty, SetSavedInstanceExtend, WorldServerInfo,
};

use super::instances::{InstanceResetResult, Now};
use super::store::{DIFFICULTY_NORMAL, InstanceType, MapStore};
use super::{InMap, InstanceManager};
use crate::auth::InWorld;
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};

/// character_id (u64) -> `Difficulty.db2` ID of its dungeon difficulty.
pub(crate) const CHARACTER_DUNGEON_DIFFICULTY: TableDefinition<u64, u32> =
    TableDefinition::new("character_dungeon_difficulty");

/// `Player::m_dungeonDifficulty`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonDifficulty(pub u32);

impl Default for DungeonDifficulty {
    fn default() -> Self {
        Self(DIFFICULTY_NORMAL)
    }
}

/// Characters whose dungeon difficulty their group set (`Group::AddMember`,
/// `Group::SetDungeonDifficultyID`), applied by [`apply_difficulty_changes`].
#[derive(Resource, Debug, Default)]
pub struct DifficultyChanges(pub Vec<(u64, u32)>);

/// The wall clock of instance locks and auction expiry (`GameTime::GetSystemTime`), shifted by
/// `offset_secs` (`game-server-admin clock-advance`) or pinned by tests.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServerClock {
    pub offset_secs: i64,
    pub fixed_unix: Option<i64>,
}

impl ServerClock {
    pub fn unix(&self) -> i64 {
        let system = self.fixed_unix.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after 1970")
                .as_secs() as i64
        });
        system + self.offset_secs
    }
}

/// Both clocks `InstanceManager` needs.
pub(crate) fn now(world: &World) -> Now {
    Now {
        elapsed: world.resource::<Time>().elapsed_secs_f64(),
        unix: world
            .get_resource::<ServerClock>()
            .copied()
            .unwrap_or_default()
            .unix(),
    }
}

pub(crate) fn load_dungeon_difficulty(db: &Database, character_id: u64) -> DungeonDifficulty {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(CHARACTER_DUNGEON_DIFFICULTY) else {
        return DungeonDifficulty::default();
    };
    table
        .get(character_id)
        .expect("Failed to read dungeon difficulty")
        .map_or_else(DungeonDifficulty::default, |guard| {
            DungeonDifficulty(guard.value())
        })
}

pub(crate) fn save_dungeon_difficulty(db: &Database, character_id: u64, difficulty_id: u32) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_DUNGEON_DIFFICULTY)
            .expect("Failed to open dungeon difficulty table");
        table
            .insert(character_id, difficulty_id)
            .expect("Failed to save dungeon difficulty");
    }
    txn.commit().expect("Failed to commit dungeon difficulty");
}

/// The connection of `player` in the world.
fn connection_of(world: &mut World, player: Entity) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &PlayerLink), With<InWorld>>()
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

pub(crate) fn send_to_player<M: lightyear::prelude::Message>(
    world: &mut World,
    player: Entity,
    message: M,
) {
    let Some(connection) = connection_of(world, player) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<M>>(connection) {
        sender.send::<InstanceChannel>(message);
    }
}

/// `(player, character, message)` of every message `M` the in-world players sent.
pub(crate) fn drain<M: lightyear::prelude::Message>(world: &mut World) -> Vec<(Entity, u64, M)> {
    let mut receivers =
        world.query_filtered::<(&PlayerLink, &mut MessageReceiver<M>), With<InWorld>>();
    let received: Vec<(Entity, M)> = receivers
        .iter_mut(world)
        .flat_map(|(link, mut receiver)| {
            let player = link.0;
            receiver
                .receive()
                .map(move |message| (player, message))
                .collect::<Vec<_>>()
        })
        .collect();
    received
        .into_iter()
        .filter_map(|(player, message)| {
            let character = world.get::<ClientId>(player)?.0;
            Some((player, character, message))
        })
        .collect()
}

/// `HandleSetDungeonDifficultyOpcode`.
pub(crate) fn receive_set_dungeon_difficulty(world: &mut World) {
    for (player, character, request) in drain::<SetDungeonDifficulty>(world) {
        set_dungeon_difficulty(world, player, character, request.difficulty_id);
    }
}

fn set_dungeon_difficulty(world: &mut World, player: Entity, character: u64, difficulty_id: u32) {
    let store = world.resource::<MapStore>();
    let Some(entry) = store.difficulty_entries.get(&difficulty_id) else {
        warn!("{player} sent an invalid dungeon difficulty {difficulty_id}");
        return;
    };
    if entry.instance_type != InstanceType::Party || !entry.can_select() {
        warn!("{player} sent a non-selectable dungeon difficulty {difficulty_id}");
        return;
    }
    // Cannot reset while in an instance.
    let in_instance = world.get::<InMap>(player).is_some_and(|map| {
        store
            .map(map.map_id)
            .is_some_and(|entry| entry.is_dungeon())
    });
    if in_instance {
        info!("{player} tried to change the dungeon difficulty inside an instance");
        return;
    }
    let entrant = super::transfer::entrant_of(world, player);
    let now = now(world);
    let grouped = world
        .get_resource::<GroupRegistry>()
        .is_some_and(|groups| groups.group_id_of(character).is_some());
    if grouped {
        let Some(effects) = world
            .resource_mut::<GroupRegistry>()
            .set_dungeon_difficulty(character, difficulty_id)
        else {
            return;
        };
        if effects.is_empty() {
            return;
        }
        // The difficulty is set even if the instances can't be reset.
        let results = world
            .resource_mut::<InstanceManager>()
            .reset_instances(entrant, true, now);
        send_reset_successes(world, player, &results);
        let changes = effects.into_iter().filter_map(|effect| match effect {
            crate::group::GroupEffect::DungeonDifficulty { to, difficulty_id } => {
                Some((to, difficulty_id))
            }
            _ => None,
        });
        world.resource_mut::<DifficultyChanges>().0.extend(changes);
        return;
    }
    if world.get::<DungeonDifficulty>(player).map(|d| d.0) == Some(difficulty_id) {
        return;
    }
    let results = world
        .resource_mut::<InstanceManager>()
        .reset_instances(entrant, true, now);
    send_reset_successes(world, player, &results);
    world
        .resource_mut::<DifficultyChanges>()
        .0
        .push((character, difficulty_id));
}

/// `SendResetInstanceSuccess` for each copy that was reset.
fn send_reset_successes(world: &mut World, player: Entity, results: &[(u32, InstanceResetResult)]) {
    for &(map_id, result) in results {
        if result == InstanceResetResult::Success {
            send_to_player(world, player, InstanceReset { map_id });
        }
    }
}

/// `Player::SetDungeonDifficultyID` + `SendDungeonDifficulty` for every change.
pub(crate) fn apply_difficulty_changes(world: &mut World) {
    let changes = std::mem::take(&mut world.resource_mut::<DifficultyChanges>().0);
    for (character, difficulty_id) in changes {
        let player = world
            .query::<(Entity, &ClientId)>()
            .iter(world)
            .find_map(|(entity, id)| (id.0 == character).then_some(entity));
        let Some(player) = player else {
            continue;
        };
        world
            .entity_mut(player)
            .insert(DungeonDifficulty(difficulty_id));
        if let Some(db) = world.get_resource::<GameDatabase>() {
            save_dungeon_difficulty(&db.db, character, difficulty_id);
        }
        info!("Character {character} dungeon difficulty set to {difficulty_id}");
    }
}

/// `SMSG_SET_DUNGEON_DIFFICULTY` whenever a player's difficulty is set (login, change).
pub(crate) fn send_dungeon_difficulty(world: &mut World) {
    let changed: Vec<(Entity, u32)> = world
        .query_filtered::<(Entity, &DungeonDifficulty), (With<Player>, Changed<DungeonDifficulty>)>(
        )
        .iter(world)
        .map(|(player, difficulty)| (player, difficulty.0))
        .collect();
    for (player, difficulty_id) in changed {
        send_to_player(world, player, DungeonDifficultySet { difficulty_id });
    }
}

/// `SMSG_WORLD_SERVER_INFO` when a player is added to a map: the copy's difficulty.
pub(crate) fn send_world_server_info(
    world: &mut World,
    entered: &mut QueryState<(Entity, &InMap), (With<Player>, Changed<InMap>)>,
) {
    let entered: Vec<(Entity, InMap)> = entered
        .iter(world)
        .map(|(player, map)| (player, *map))
        .collect();
    for (player, map) in entered {
        let difficulty_id = world
            .resource::<InstanceManager>()
            .copy_difficulty(map)
            .map_or(0, |difficulty| difficulty.difficulty_id);
        send_to_player(
            world,
            player,
            WorldServerInfo {
                map_id: map.map_id,
                difficulty_id,
            },
        );
    }
}

/// `HandleRequestRaidInfoOpcode` → `Player::SendRaidInfo`.
pub(crate) fn receive_request_raid_info(world: &mut World) {
    for (player, character, _) in drain::<RequestRaidInfo>(world) {
        send_raid_info(world, player, character);
    }
}

pub(crate) fn send_raid_info(world: &mut World, player: Entity, character: u64) {
    let unix = now(world).unix;
    let store = world.resource::<MapStore>();
    let locks = world.resource::<InstanceManager>().locks.raid_info(
        character,
        |lock| {
            store
                .map_difficulty(lock.map_id, lock.difficulty_id)
                .copied()
        },
        unix,
    );
    send_to_player(world, player, InstanceInfo { locks });
}

/// `HandleResetInstancesOpcode`: a group's leader resets the group's copies, a player
/// outside a group its own.
pub(crate) fn receive_reset_instances(world: &mut World) {
    for (player, character, _) in drain::<ResetInstances>(world) {
        let groups = world.get_resource::<GroupRegistry>();
        if groups
            .and_then(|groups| groups.leader_of(character))
            .is_some_and(|leader| leader != character)
        {
            continue;
        }
        let entrant = super::transfer::entrant_of(world, player);
        let now = now(world);
        let results = world
            .resource_mut::<InstanceManager>()
            .reset_instances(entrant, false, now);
        for (map_id, result) in results {
            match result {
                InstanceResetResult::Success => {
                    send_to_player(world, player, InstanceReset { map_id })
                }
                InstanceResetResult::NotEmpty => send_to_player(
                    world,
                    player,
                    InstanceResetFailed {
                        map_id,
                        reason: InstanceResetFailedReason::PlayersInside,
                    },
                ),
                InstanceResetResult::CannotReset => {}
            }
        }
    }
}

/// `HandleSetSavedInstanceExtend` (CalendarHandler.cpp): extend or unextend a lock of
/// an extendable difficulty, then the refreshed Raid Info.
pub(crate) fn receive_set_saved_instance_extend(world: &mut World) {
    for (player, character, request) in drain::<SetSavedInstanceExtend>(world) {
        let Some(entries) = world
            .resource::<MapStore>()
            .map_difficulty(request.map_id, request.difficulty_id)
            .copied()
        else {
            continue;
        };
        if !entries.is_extendable() {
            continue;
        }
        let unix = now(world).unix;
        let mut instances = world.resource_mut::<InstanceManager>();
        if instances
            .locks
            .set_extended(character, &entries, request.extend, unix)
            .is_some()
        {
            instances.locks_dirty = true;
        }
        send_raid_info(world, player, character);
    }
}

/// `InstanceMap::Update`'s expire event: occupied copies whose lock reset passed tell
/// their players (`RAID_INSTANCE_EXPIRED`); empty ones unload.
pub(crate) fn expire_instance_locks(world: &mut World) {
    let now = now(world);
    let occupied = world
        .resource_mut::<InstanceManager>()
        .expire_locked_copies(now);
    for map in occupied {
        let difficulty_id = world
            .resource::<InstanceManager>()
            .copy_difficulty(map)
            .map_or(0, |difficulty| difficulty.difficulty_id);
        let players = crate::scripted_ai::instance_script::players_in_map(world, map);
        for player in players {
            send_to_player(
                world,
                player,
                RaidInstanceMessage {
                    kind: RaidInstanceMessageType::Expired,
                    map_id: map.map_id,
                    difficulty_id,
                },
            );
        }
    }
}

/// `InstanceSaveCreated` to each newly saved player.
pub(crate) fn send_instance_saved(world: &mut World, players: &[Entity]) {
    for &player in players {
        send_to_player(world, player, InstanceSaveCreated);
    }
}

#[cfg(test)]
mod admin_game_object_tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use shared::{components::Position, protocol::GameObjectInfo};

    fn spawn_mailbox(world: &mut World, map: InMap, entry: u32) -> Entity {
        world
            .spawn((
                GameObjectInfo {
                    entry,
                    go_type: shared::protocol::GAMEOBJECT_TYPE_MAILBOX,
                    display_id: 1907,
                    name: "Mailbox".into(),
                    scale: 1.0,
                },
                // Runtime Position uses Bevy axes, not the reported WoW axes.
                Position {
                    x: -8810.13,
                    y: 94.875,
                    z: -646.57,
                },
                map,
            ))
            .id()
    }

    fn lookup(world: &mut World, entry: u32) -> crate::admin_types::AdminResponse {
        let request = serde_json::from_value(serde_json::json!({
            "GameObjectInfo": { "map_id": 0, "instance_id": 0, "entry": entry }
        }))
        .expect("admin IPC accepts game-object-info");
        let db = GameDatabase::new(crate::persistence::temp_database());
        let mut state = SystemState::<AdminInstances>::new(world);
        state
            .get_mut(world)
            .unwrap()
            .dispatch(&request, &db)
            .expect("lookup handled")
    }

    #[test]
    fn game_object_info_returns_live_entity_bits_and_world_coordinates() {
        let mut world = World::new();
        spawn_mailbox(&mut world, InMap::continent(1), 197134);
        spawn_mailbox(
            &mut world,
            InMap {
                map_id: 0,
                instance_id: 7,
            },
            197134,
        );
        spawn_mailbox(&mut world, InMap::continent(0), 999);
        let object = spawn_mailbox(&mut world, InMap::continent(0), 197134);
        let response = lookup(&mut world, 197134);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({
                "GameObjectInfoList": { "lines": [format!(
                    "Mailbox (entity {}) entry 197134 display 1907 map 0 instance 0 world (-8810.13, 646.57, 94.875)",
                    object.to_bits()
                )] }
            })
        );
        assert_eq!(world.get::<Position>(object).unwrap().y, 94.875);
        assert_eq!(world.query::<&GameObjectInfo>().iter(&world).count(), 4);
    }

    #[test]
    fn game_object_info_reports_actual_visibility_sender_ids() {
        let mut world = World::new();
        let object = spawn_mailbox(&mut world, InMap::continent(0), 197134);
        let sender = world.spawn_empty().id();
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(sender);
        world.entity_mut(object).insert(visibility);
        let response = serde_json::to_value(lookup(&mut world, 197134)).unwrap();
        let line = response["GameObjectInfoList"]["lines"][0].as_str().unwrap();
        assert!(line.ends_with(&format!("visible_senders=[{}]", sender.to_bits())));
    }

    #[test]
    fn game_object_info_reports_missing_object_in_requested_map_copy() {
        let mut world = World::new();
        spawn_mailbox(&mut world, InMap::continent(1), 197134);
        assert_eq!(
            lookup(&mut world, 197134),
            crate::admin_types::AdminResponse::Error(
                "game object entry 197134 not found in map 0 instance 0".into()
            )
        );
    }
}

/// Admin commands on the server clock and instance locks.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct AdminInstances<'w, 's> {
    clock: Option<ResMut<'w, ServerClock>>,
    instances: Option<ResMut<'w, InstanceManager>>,
    store: Option<Res<'w, MapStore>>,
    game_objects: Query<
        'w,
        's,
        (
            Entity,
            &'static shared::protocol::GameObjectInfo,
            &'static shared::components::Position,
            &'static InMap,
            Option<&'static crate::interest::NetworkVisibility>,
        ),
        Without<crate::persistence::ClientId>,
    >,
    creatures: Query<
        'w,
        's,
        (
            Entity,
            &'static shared::components::Npc,
            &'static shared::components::Health,
            &'static InMap,
            Option<&'static shared::components::UnitLevel>,
            Option<&'static crate::zones::CreatureDifficulty>,
            Option<&'static crate::stats::CharacterStats>,
            Option<&'static crate::level_scaling::CreatureArmor>,
            Option<&'static crate::networking::CombatTarget>,
            Option<&'static shared::components::ThreatTable>,
        ),
    >,
}

impl AdminInstances<'_, '_> {
    pub(crate) fn dispatch(
        &mut self,
        request: &crate::admin_types::AdminRequest,
        db: &GameDatabase,
    ) -> Option<crate::admin_types::AdminResponse> {
        use crate::admin_types::{AdminRequest, AdminResponse};
        let character = |name: &str| {
            crate::persistence::find_character_id_by_name(&db.db, name)
                .ok_or_else(|| AdminResponse::Error(format!("character '{name}' not found")))
        };
        Some(match request {
            AdminRequest::ClockAdvance { seconds } => {
                let clock = self.clock.as_mut()?;
                clock.offset_secs += seconds;
                info!(
                    "Instance lock and auction expiry clock advanced {seconds} s (offset {} s)",
                    clock.offset_secs
                );
                AdminResponse::ClockAdvanced {
                    offset_secs: clock.offset_secs,
                    unix: clock.unix(),
                }
            }
            AdminRequest::InstanceLocks { character_name } => {
                let id = match character(character_name) {
                    Ok(id) => id,
                    Err(error) => return Some(error),
                };
                let unix = self.clock.as_deref().copied().unwrap_or_default().unix();
                let (instances, store) = (self.instances.as_deref()?, self.store.as_deref()?);
                let locks = instances
                    .locks
                    .raid_info(
                        id,
                        |lock| {
                            store
                                .map_difficulty(lock.map_id, lock.difficulty_id)
                                .copied()
                        },
                        unix,
                    )
                    .into_iter()
                    .map(|lock| format!("{lock:?}"))
                    .collect();
                AdminResponse::InstanceLockList {
                    character_name: character_name.clone(),
                    locks,
                }
            }
            AdminRequest::InstanceUnbind {
                character_name,
                map_id,
            } => {
                let id = match character(character_name) {
                    Ok(id) => id,
                    Err(error) => return Some(error),
                };
                let unix = self.clock.as_deref().copied().unwrap_or_default().unix();
                let store = self.store.as_deref()?;
                let instances = self.instances.as_mut()?;
                let in_use = instances.lock_copies_in_use();
                let reset = instances.locks.reset_for_player(
                    id,
                    *map_id,
                    |lock| in_use.contains(&(lock.map_id, lock.instance_id)),
                    |lock| {
                        store
                            .map_difficulty(lock.map_id, lock.difficulty_id)
                            .copied()
                    },
                    unix,
                );
                instances.locks_dirty |= !reset.is_empty();
                AdminResponse::InstanceUnbound {
                    character_name: character_name.clone(),
                    count: reset.len(),
                }
            }
            AdminRequest::NpcInfo {
                map_id,
                instance_id,
                entry,
            } => {
                let map = InMap {
                    map_id: *map_id,
                    instance_id: *instance_id,
                };
                let lines = self
                    .creatures
                    .iter()
                    .filter(|(_, npc, _, in_map, ..)| **in_map == map && npc.template_id == *entry)
                    .map(
                        |(
                            entity,
                            npc,
                            health,
                            _,
                            level,
                            difficulty,
                            stats,
                            armor,
                            target,
                            threat,
                        )| {
                            format!(
                                "{} (entity {}) level {} health {}/{} difficulty {} combat={}",
                                npc.name,
                                entity.to_bits(),
                                level.map_or(0, |level| level.0),
                                health.current,
                                health.max,
                                difficulty.map_or(0, |difficulty| difficulty.0),
                                npc_combat_metrics(stats, armor, target, threat)
                            )
                        },
                    )
                    .collect();
                AdminResponse::NpcInfoList { lines }
            }
            AdminRequest::GameObjectInfo {
                map_id,
                instance_id,
                entry,
            } => {
                let map = InMap {
                    map_id: *map_id,
                    instance_id: *instance_id,
                };
                let lines: Vec<_> = self
                    .game_objects
                    .iter()
                    .filter(|(_, info, _, in_map, _)| **in_map == map && info.entry == *entry)
                    .map(|(entity, info, position, _, visibility)| {
                        let world = crate::game_objects::wow_position(position);
                        let mut line = format!(
                            "{} (entity {}) entry {} display {} map {} instance {} world ({}, {}, {})",
                            info.name, entity.to_bits(), info.entry, info.display_id,
                            map_id, instance_id, world.x, world.y, world.z
                        );
                        if let Some(visibility) = visibility {
                            let senders: Vec<_> = visibility.visible_senders().map(Entity::to_bits).collect();
                            line.push_str(&format!(" visible_senders={senders:?}"));
                        }
                        line
                    })
                    .collect();
                if lines.is_empty() {
                    AdminResponse::Error(format!(
                        "game object entry {entry} not found in map {map_id} instance {instance_id}"
                    ))
                } else {
                    AdminResponse::GameObjectInfoList { lines }
                }
            }
            _ => return None,
        })
    }
}

fn npc_combat_metrics(
    stats: Option<&crate::stats::CharacterStats>,
    armor: Option<&crate::level_scaling::CreatureArmor>,
    target: Option<&crate::networking::CombatTarget>,
    threat: Option<&shared::components::ThreatTable>,
) -> serde_json::Value {
    let stats = stats.map(|stats| {
        serde_json::json!({
            "strength": stats.primary.strength,
            "intellect": stats.primary.intellect,
            "spell_power": stats.spell_power,
            "healing_power": stats.healing_power,
            "attack_power": stats.attack_power,
            "crit_pct": stats.crit_pct,
            "haste_pct": stats.haste_pct,
            "versatility_pct": stats.versatility_pct,
        })
    });
    let threat = threat.map(|table| {
        table
            .entries()
            .iter()
            .map(|entry| serde_json::json!({"entity": entry.entity, "amount": entry.threat}))
            .collect::<Vec<_>>()
    });
    serde_json::json!({
        "stats": stats,
        "creature_armor": armor.map(|armor| armor.0),
        "victim": target.and_then(|target| target.0).map(Entity::to_bits),
        "threat": threat,
    })
}

#[cfg(test)]
mod admin_npc_combat_tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use shared::components::{Health, Npc, ThreatTable};

    fn inspect(world: &mut World) -> serde_json::Value {
        let db = GameDatabase::new(crate::persistence::temp_database());
        let mut state = SystemState::<AdminInstances>::new(world);
        let response = state
            .get_mut(world)
            .unwrap()
            .dispatch(
                &crate::admin_types::AdminRequest::NpcInfo {
                    map_id: 34,
                    instance_id: 1,
                    entry: 209057,
                },
                &db,
            )
            .unwrap();
        let crate::admin_types::AdminResponse::NpcInfoList { lines } = response else {
            panic!("npc-info did not return its live rows");
        };
        assert_eq!(lines.len(), 1);
        let (_, metrics) = lines[0]
            .split_once(" combat=")
            .expect("native combat metrics");
        serde_json::from_str(metrics).unwrap()
    }

    fn spawn_npc(world: &mut World) -> Entity {
        world
            .spawn((
                Npc {
                    template_id: 209057,
                    name: "Captain Garrick".into(),
                },
                Health {
                    current: 7500.0,
                    max: 10584.0,
                },
                InMap {
                    map_id: 34,
                    instance_id: 1,
                },
            ))
            .id()
    }

    #[test]
    fn npc_info_exposes_native_stats_victim_threat_without_mutating_them() {
        let mut world = World::new();
        let victim = world.spawn_empty().id();
        let npc = spawn_npc(&mut world);
        let mut table = ThreatTable::default();
        table.add_threat(victim.to_bits(), 450.0);
        let stats = crate::stats::CharacterStats {
            spell_power: 73.0,
            healing_power: 75.0,
            attack_power: 214.0,
            haste_pct: 4.0,
            ..Default::default()
        };
        world.entity_mut(npc).insert((
            stats,
            crate::networking::CombatTarget(Some(victim)),
            table,
            crate::level_scaling::CreatureArmor(1234.0),
        ));
        let metrics = inspect(&mut world);
        assert_eq!(metrics["stats"]["spell_power"], 73.0);
        assert_eq!(metrics["stats"]["healing_power"], 75.0);
        assert_eq!(metrics["stats"]["attack_power"], 214.0);
        assert_eq!(metrics["stats"]["haste_pct"], 4.0);
        assert_eq!(metrics["creature_armor"], 1234.0);
        assert_eq!(metrics["victim"], victim.to_bits());
        assert_eq!(
            metrics["threat"],
            serde_json::json!([
                {"entity": victim.to_bits(), "amount": 450.0}
            ])
        );
        assert_eq!(
            *world.get::<crate::stats::CharacterStats>(npc).unwrap(),
            stats
        );
        assert_eq!(
            world
                .get::<ThreatTable>(npc)
                .unwrap()
                .threat_for(victim.to_bits()),
            450.0
        );
        assert_eq!(world.get::<Health>(npc).unwrap().current, 7500.0);
    }

    #[test]
    fn npc_info_does_not_fabricate_missing_player_stats_or_threat() {
        let mut world = World::new();
        spawn_npc(&mut world);
        assert_eq!(
            inspect(&mut world),
            serde_json::json!({
                "stats": null, "creature_armor": null, "victim": null, "threat": null
            })
        );
    }
}
