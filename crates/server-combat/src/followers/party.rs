use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, UnitLevel};
use shared::group::GroupRole;
use shared::protocol::{GroupChannel, LfgChannel, LfgSnapshot, LfgStateUpdate};

use super::catalog::{Dungeon, FOLLOWER_DIFFICULTY};
use super::{Follower, FollowerCatalog, Profile};
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::maps::{
    DungeonDifficulty, InMap, MapTransfer, TeleportOptions, WorldLocation, teleport_to,
};
use crate::persistence::ClientId;

#[path = "party_staging.rs"]
mod staging;
pub(crate) use staging::stage_follower_party;

#[derive(Resource, Default)]
pub(crate) struct DisconnectedRuns(std::collections::HashMap<u64, DisconnectedRun>);

struct DisconnectedRun {
    run: FollowerRun,
    lfg: crate::lfg::PlayerLfgState,
}

impl DisconnectedRuns {
    pub(crate) fn copy(&self, character: u64) -> Option<InMap> {
        self.0.get(&character)?.run.copy
    }

    pub(crate) fn contains_member(&self, member: Entity) -> bool {
        self.0.values().any(|run| run.run.contains_member(member))
    }

    pub(crate) fn member_difficulty(&self, member: Entity) -> Option<u32> {
        self.0
            .values()
            .any(|run| run.run.members.contains(&member))
            .then_some(FOLLOWER_DIFFICULTY)
    }
}

#[derive(Component, Clone)]
pub(crate) struct FollowerRun {
    dungeon: Dungeon,
    profiles: Vec<Profile>,
    members: Vec<Entity>,
    copy: Option<InMap>,
    previous_difficulty: u32,
    origin: WorldLocation,
    player_role: GroupRole,
}

impl FollowerRun {
    pub(crate) fn contains_member(&self, member: Entity) -> bool {
        self.copy.is_some() && self.members.contains(&member)
    }
}

/// Admitted copy, companion IDs (including missing entities), and owner's role.
/// Retail follower and explicit Stockade diagnostic admission are separate records.
pub(super) fn read_admission(
    world: &World,
    owner: Entity,
) -> Option<(InMap, &[Entity], GroupRole)> {
    let record = match (
        world.get::<FollowerRun>(owner),
        world.get::<super::stockade_diagnostic::DiagnosticParty>(owner),
    ) {
        (Some(run), None) => (run.copy?, run.members.as_slice(), run.player_role),
        (None, Some(diagnostic)) => (
            diagnostic.copy,
            diagnostic.members.as_slice(),
            diagnostic.player_role,
        ),
        _ => return None,
    };
    (world.get::<InMap>(owner) == Some(&record.0)).then_some(record)
}

pub(crate) fn queue_player(world: &mut World, player: Entity, role: GroupRole, dungeon_id: u32) {
    if let Err(error) = begin_run(world, player, role, dungeon_id) {
        send_update(world, player, None, Some(error));
    }
}

fn begin_run(
    world: &mut World,
    player: Entity,
    role: GroupRole,
    dungeon_id: u32,
) -> Result<(), String> {
    if world.get::<FollowerRun>(player).is_some() {
        return Err("already in a follower dungeon".into());
    }
    let id = world
        .get::<ClientId>(player)
        .ok_or("follower queue requires an authenticated character")?
        .0;
    if world.resource::<GroupRegistry>().group_id_of(id).is_some() {
        return Err("follower queue currently requires a solo player".into());
    }
    let catalog = world.resource::<FollowerCatalog>();
    let dungeon = catalog
        .dungeons
        .get(&dungeon_id)
        .cloned()
        .ok_or("follower dungeon lacks imported Normal spawns or an authored entrance")?;
    let profiles = super::select_party(&catalog.profiles, role)?;
    let position = *world
        .get::<shared::components::Position>(player)
        .ok_or("player has no position")?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let origin = WorldLocation {
        map_id: world
            .get::<InMap>(player)
            .ok_or("player already transferring")?
            .map_id,
        position: Vec3::new(x, y, z),
        orientation: 0.0,
    };
    let previous_difficulty = world
        .get::<DungeonDifficulty>(player)
        .ok_or("player has no dungeon difficulty")?
        .0;
    world
        .resource_mut::<crate::maps::InstanceManager>()
        .forget_recent_instance(id, dungeon.entrance.map_id);
    world
        .entity_mut(player)
        .insert(DungeonDifficulty(FOLLOWER_DIFFICULTY));
    if !teleport_to(world, player, dungeon.entrance, TeleportOptions::default()) {
        world
            .entity_mut(player)
            .insert(DungeonDifficulty(previous_difficulty));
        return Err("follower dungeon admission refused".into());
    }
    let mut roster = vec![crate::lfg::PlayerMatchMemberState {
        name: world
            .get::<Player>(player)
            .expect("queued player")
            .name
            .clone(),
        role,
    }];
    roster.extend(
        profiles
            .iter()
            .map(|profile| crate::lfg::PlayerMatchMemberState {
                name: profile.name.clone(),
                role: profile.role,
            }),
    );
    world.entity_mut(player).insert(crate::lfg::PlayerLfgState {
        selected_role: Some(role),
        dungeon_ids: vec![dungeon_id],
        role_check: None,
        match_found: Some(crate::lfg::PlayerMatchFoundState {
            dungeon_id,
            dungeon_name: dungeon.name.clone(),
            assigned_role: role,
            members: roster,
        }),
    });
    world.entity_mut(player).insert(FollowerRun {
        dungeon,
        profiles,
        members: Vec::new(),
        copy: None,
        previous_difficulty,
        origin,
        player_role: role,
    });
    send_update(
        world,
        player,
        Some("entering follower dungeon".into()),
        None,
    );
    Ok(())
}

pub(super) fn advance_runs(world: &mut World) {
    world.init_resource::<DisconnectedRuns>();
    let runs: Vec<_> = world
        .query::<(Entity, &FollowerRun)>()
        .iter(world)
        .map(|(player, run)| (player, run.clone()))
        .collect();
    for (player, run) in runs {
        if world.get::<MapTransfer>(player).is_some() {
            continue;
        }
        let player_map = world.get::<InMap>(player).copied();
        let at_destination =
            player_map.is_some_and(|map| map.map_id == run.dungeon.entrance.map_id);
        let in_run_copy = run.copy.is_none_or(|copy| player_map == Some(copy));
        if !at_destination || !in_run_copy {
            end_run(world, player, &run);
            continue;
        }
        if run.members.is_empty() {
            if let Err(error) = spawn_party(world, player, &run) {
                send_update(world, player, None, Some(error));
                end_run(world, player, &run);
                teleport_to(world, player, run.origin, TeleportOptions::default());
            }
        }
    }
    let expired: Vec<_> = world
        .resource::<DisconnectedRuns>()
        .0
        .iter()
        .filter_map(|(&id, run)| {
            let copy = run.run.copy?;
            world
                .resource::<crate::maps::InstanceManager>()
                .copy_difficulty(copy)
                .is_none()
                .then_some(id)
        })
        .collect();
    for id in expired {
        world.resource_mut::<DisconnectedRuns>().0.remove(&id);
        world.resource_mut::<GroupRegistry>().leave(id);
    }
    let suspended: std::collections::HashSet<_> = world
        .resource::<DisconnectedRuns>()
        .0
        .values()
        .flat_map(|run| run.run.members.iter().copied())
        .collect();
    let orphans: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .filter(|(unit, follower)| {
            let retail_run = world.get::<FollowerRun>(follower.owner).is_some();
            let diagnostic_run = world
                .get::<super::stockade_diagnostic::DiagnosticParty>(follower.owner)
                .is_some();
            !retail_run && !diagnostic_run && !suspended.contains(unit)
        })
        .map(|(unit, _)| unit)
        .collect();
    for unit in orphans {
        crate::zones::unsummon(world, unit);
    }
}

fn spawn_party(world: &mut World, owner: Entity, run: &FollowerRun) -> Result<(), String> {
    let members = super::spawning::spawn_equipped_members(world, owner, &run.profiles)?;
    if let Err(error) = join_party(world, owner, run, &members) {
        super::spawning::unsummon_members(world, &members);
        return Err(error);
    }
    let copy = *world
        .get::<InMap>(owner)
        .ok_or("follower owner left its admitted copy")?;
    {
        let mut state = world
            .get_mut::<FollowerRun>(owner)
            .expect("queued follower run");
        state.members = members;
        state.copy = Some(copy);
    }
    send_update(
        world,
        owner,
        Some(format!("follower party ready for {}", run.dungeon.name)),
        None,
    );
    Ok(())
}

fn join_party(
    world: &mut World,
    owner: Entity,
    run: &FollowerRun,
    members: &[Entity],
) -> Result<(), String> {
    join_members(world, owner, run.player_role, members)
}

pub(super) fn join_members(
    world: &mut World,
    owner: Entity,
    role: GroupRole,
    members: &[Entity],
) -> Result<(), String> {
    join_members_with_player(world, owner, role, members, None)
}

pub(super) fn join_members_with_player(
    world: &mut World,
    owner: Entity,
    role: GroupRole,
    members: &[Entity],
    player: Option<(Entity, GroupRole)>,
) -> Result<(), String> {
    let difficulty = world
        .get::<DungeonDifficulty>(owner)
        .ok_or("companion owner has no dungeon difficulty")?
        .0;
    let id = world.get::<ClientId>(owner).ok_or("owner logged out")?.0;
    let mut online = OnlineView::from([(id, online_player(world, owner, difficulty)?)]);
    let mut companion_roles: Vec<_> = members
        .iter()
        .map(|&unit| {
            let follower = world.get::<Follower>(unit).expect("spawned follower");
            online.insert(
                Follower::member_id(unit),
                follower.online(
                    unit,
                    world.get::<UnitLevel>(unit).expect("follower level").0,
                    difficulty,
                    world
                        .get::<shared::components::ModelDisplay>(unit)
                        .expect("spawned follower display")
                        .display_id,
                ),
            );
            (Follower::member_id(unit), follower.profile.role)
        })
        .collect();
    if let Some((player, role)) = player {
        let player_id = world
            .get::<ClientId>(player)
            .ok_or("party player logged out")?
            .0;
        online.insert(player_id, online_player(world, player, difficulty)?);
        companion_roles.push((player_id, role));
    }
    let mut registry = world.resource_mut::<GroupRegistry>();
    let group = registry.create_companion_party(id, role, &companion_roles, &online)?;
    let roster = registry
        .build_roster(group, &online)
        .expect("formed companion roster");
    let copy = *world
        .get::<InMap>(owner)
        .ok_or("companion party owner has no admitted copy")?;
    world
        .resource_mut::<crate::maps::InstanceManager>()
        .bind_group_instance(group, id, copy)?;
    if let Some(conn) = crate::spell_cast::owner_connection(world, owner) {
        if let Some(mut sender) =
            world.get_mut::<MessageSender<shared::protocol::GroupRosterSnapshot>>(conn)
        {
            sender.send::<GroupChannel>(roster);
        }
    }
    Ok(())
}

fn online_player(world: &World, entity: Entity, difficulty: u32) -> Result<OnlineMember, String> {
    let player = world
        .get::<Player>(entity)
        .ok_or("party member is not a player")?;
    Ok(OnlineMember {
        entity: entity.to_bits(),
        name: player.name.clone(),
        class: player.class,
        level: world
            .get::<UnitLevel>(entity)
            .ok_or("party player level absent")?
            .0,
        portrait: crate::group::portrait_appearance(
            player,
            world.get::<shared::components::EquipmentAppearance>(entity),
        ),
        dungeon_difficulty: difficulty,
        raid_difficulty: world
            .get::<crate::maps::RaidDifficulty>(entity)
            .copied()
            .unwrap_or_default(),
    })
}

pub(super) fn retire_owner(world: &mut World, player: Entity) {
    if let Some(run) = world.get::<FollowerRun>(player).cloned() {
        if let Some(id) = world.get::<ClientId>(player).map(|id| id.0)
            && run.copy.is_some()
        {
            let lfg = world
                .get::<crate::lfg::PlayerLfgState>(player)
                .cloned()
                .expect("follower owner has LFG state");
            world.init_resource::<DisconnectedRuns>();
            world
                .resource_mut::<DisconnectedRuns>()
                .0
                .insert(id, DisconnectedRun { run, lfg });
            world.entity_mut(player).remove::<FollowerRun>();
        } else {
            end_run(world, player, &run);
        }
    }
    if let Some(party) = world
        .get::<super::stockade_diagnostic::DiagnosticParty>(player)
        .cloned()
    {
        leave_members(world, player, &party.members);
        world
            .entity_mut(player)
            .remove::<super::stockade_diagnostic::DiagnosticParty>();
    }
}

pub(crate) fn reconnect(world: &mut World, player: Entity, character: u64) {
    let Some(DisconnectedRun { run, lfg }) = world
        .get_resource_mut::<DisconnectedRuns>()
        .and_then(|mut runs| runs.0.remove(&character))
    else {
        return;
    };
    if world.get::<InMap>(player).copied() != run.copy {
        world.resource_mut::<GroupRegistry>().leave(character);
        for member in run.members {
            if world.get_entity(member).is_ok() {
                crate::zones::unsummon(world, member);
            }
        }
        return;
    }
    for &member in &run.members {
        if let Some(mut follower) = world.get_mut::<Follower>(member) {
            follower.owner = player;
        }
        if let Some(mut summon) = world.get_mut::<crate::zones::TempSummon>(member) {
            summon.summoner = player;
        }
        if let Some(mut follow) = world.get_mut::<crate::creature_ai::FollowOwner>(member) {
            follow.owner = player;
        }
    }
    world
        .entity_mut(player)
        .insert((DungeonDifficulty(FOLLOWER_DIFFICULTY), run, lfg));
    send_update(world, player, None, None);
}

fn end_run(world: &mut World, player: Entity, run: &FollowerRun) {
    leave_members(world, player, &run.members);
    world.entity_mut(player).remove::<FollowerRun>().insert((
        DungeonDifficulty(run.previous_difficulty),
        crate::lfg::PlayerLfgState::default(),
    ));
}

pub(super) fn leave_members(world: &mut World, player: Entity, members: &[Entity]) {
    for &unit in members {
        if world.get_entity(unit).is_ok() {
            crate::zones::unsummon(world, unit);
        }
    }
    if let Some(id) = world.get::<ClientId>(player).map(|id| id.0) {
        world.resource_mut::<GroupRegistry>().leave(id);
    }
    if let Some(conn) = crate::spell_cast::owner_connection(world, player) {
        if let Some(mut sender) =
            world.get_mut::<MessageSender<shared::protocol::GroupRosterSnapshot>>(conn)
        {
            sender.send::<GroupChannel>(crate::group::empty_roster());
        }
    }
}

fn send_update(world: &mut World, player: Entity, message: Option<String>, error: Option<String>) {
    let Some(conn) = crate::spell_cast::owner_connection(world, player) else {
        return;
    };
    let snapshot = world
        .get::<FollowerRun>(player)
        .map(|run| snapshot(world, player, run));
    if let Some(mut sender) = world.get_mut::<MessageSender<LfgStateUpdate>>(conn) {
        sender.send::<LfgChannel>(LfgStateUpdate {
            snapshot,
            message,
            error,
        });
    }
}

fn snapshot(world: &World, player: Entity, run: &FollowerRun) -> LfgSnapshot {
    let player_name = world
        .get::<Player>(player)
        .expect("queued player")
        .name
        .clone();
    let mut members = vec![crate::lfg::PlayerMatchMemberState {
        name: player_name,
        role: run.player_role,
    }];
    members.extend(
        run.profiles
            .iter()
            .map(|profile| crate::lfg::PlayerMatchMemberState {
                name: profile.name.clone(),
                role: profile.role,
            }),
    );
    LfgSnapshot {
        queued: false,
        selected_role: Some(crate::lfg::map_group_role(run.player_role)),
        dungeon_ids: vec![run.dungeon.id],
        queue_size: 0,
        average_wait_secs: 0,
        in_demand_roles: Vec::new(),
        role_check: None,
        match_found: Some(shared::protocol::LfgMatchFoundSnapshot {
            dungeon_id: run.dungeon.id,
            dungeon_name: run.dungeon.name.clone(),
            assigned_role: crate::lfg::map_group_role(run.player_role),
            members: members
                .iter()
                .map(|member| shared::protocol::LfgMatchMemberSnapshot {
                    name: member.name.clone(),
                    role: crate::lfg::map_group_role(member.role),
                })
                .collect(),
        }),
    }
}

#[cfg(test)]
#[path = "party_staging_tests.rs"]
mod staging_tests;

#[cfg(test)]
#[path = "pull_frontal_tests.rs"]
mod frontal_tests;

#[cfg(test)]
mod copy_tests {
    use super::*;

    pub(super) fn fixture() -> (App, Entity, FollowerRun) {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let store = crate::maps::store::load_map_store(&db).unwrap();
        let catalog = super::super::catalog::load_catalog(&db).unwrap();
        let dungeon = catalog.dungeons.get(&2436).unwrap().clone();
        let profiles = super::super::select_party(&catalog.profiles, GroupRole::Dps).unwrap();
        let mut instances = crate::maps::InstanceManager::default();
        let copy = instances
            .create_map(
                &store,
                2521,
                crate::maps::instances::InstanceEntrant {
                    character_id: 30,
                    group: None,
                    leader: 30,
                    difficulty_id: 205,
                    raid_difficulty: Default::default(),
                    raid_group: false,
                },
                crate::maps::instances::Now {
                    elapsed: 0.0,
                    unix: 1_790_510_400,
                },
            )
            .unwrap()
            .map;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(store)
            .insert_resource(instances)
            .init_resource::<GroupRegistry>();
        let owner = app
            .world_mut()
            .spawn((
                Player {
                    name: "Followproof".into(),
                    race: 1,
                    class: 8,
                    appearance: Default::default(),
                },
                ClientId(30),
                shared::components::Level(12),
                UnitLevel(12),
                dungeon.entrance.bevy_position(),
                shared::components::Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                copy,
                DungeonDifficulty(205),
                crate::maps::RaidDifficulty {
                    primary: 15,
                    legacy: 4,
                },
            ))
            .id();
        let members = profiles
            .iter()
            .map(|profile| {
                let display_id = db
                    .query_row(
                        "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx LIMIT 1",
                        [profile.entry],
                        |row| row.get(0),
                    )
                    .unwrap();
                app.world_mut()
                    .spawn((
                        Follower {
                            owner,
                            profile: profile.clone(),
                        },
                        crate::zones::TempSummon { summoner: owner },
                        shared::components::Npc {
                            template_id: profile.entry,
                            name: profile.name.clone(),
                        },
                        UnitLevel(12),
                        shared::components::ModelDisplay { display_id },
                        shared::components::Health {
                            current: 1000.0,
                            max: 1000.0,
                        },
                        dungeon.entrance.bevy_position(),
                        copy,
                    ))
                    .id()
            })
            .collect();
        let run = FollowerRun {
            dungeon,
            profiles,
            members,
            copy: Some(copy),
            previous_difficulty: 1,
            origin: WorldLocation {
                map_id: 0,
                position: Vec3::ZERO,
                orientation: 0.0,
            },
            player_role: GroupRole::Dps,
        };
        (app, owner, run)
    }

    #[test]
    fn tank_pull_history_keeps_a_missing_admitted_member_in_health_coverage() {
        let (mut app, owner, run) = fixture();
        let missing = run.members[0];
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ));
        app.world_mut().entity_mut(owner).insert((
            run,
            shared::components::Health {
                current: 1000.0,
                max: 1000.0,
            },
        ));
        for _ in 0..102 {
            app.update();
            super::super::pull_history::sample_all(app.world_mut());
        }
        assert!(
            super::super::pull_history::snapshot(app.world(), owner)
                .unwrap()
                .party_stable
        );
        app.world_mut().despawn(missing);
        // Production ordering: remove the entity, then outlive both named history windows.
        for _ in 0..202 {
            app.update();
            super::super::pull_history::sample_all(app.world_mut());
        }
        let after = super::super::pull_history::snapshot(app.world(), owner).unwrap();
        assert!(
            !after.party_stable,
            "a known admitted member is missing, not healthy"
        );
        // This contract concerns health coverage; ordinary assist target enumeration is unchanged.
    }

    #[test]
    fn follower_disconnect_and_login_restore_follower_copy_and_party() {
        let (mut app, owner, run) = fixture();
        app.add_plugins((
            crate::creature_aggro::CreatureAggroPlugin,
            crate::group::GroupPlugin,
        ));
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        let map = run.copy.unwrap();
        crate::maps::install_occupancy_fixture(&mut app);
        app.world_mut()
            .entity_mut(owner)
            .remove::<InMap>()
            .insert(map);
        app.world_mut().flush();
        let database_file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(database_file.path()).unwrap();
        crate::persistence::create_tables(&db);
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.world_mut().entity_mut(owner).insert((
            shared::components::Health {
                current: 600.0,
                max: 1000.0,
            },
            shared::components::Position {
                x: 1403.56,
                y: 128.0,
                z: 173.39,
            },
        ));
        app.world_mut()
            .resource_mut::<crate::maps::InstanceManager>()
            .update_instance_lock(map, &[(owner, 30)], 1, Some(2), 1_790_510_400);
        let dead = run.members[0];
        let living = run.members[1];
        app.world_mut()
            .entity_mut(living)
            .insert(crate::creature_ai::FollowOwner::new(owner));
        app.world_mut().entity_mut(dead).insert((
            shared::components::Health {
                current: 0.0,
                max: 1000.0,
            },
            crate::combat::Corpse { remaining: 0.0 },
            crate::combat::DeathState { respawn_timer: 0.0 },
        ));
        let position = *app
            .world()
            .get::<shared::components::Position>(owner)
            .unwrap();
        let lfg = crate::lfg::PlayerLfgState {
            selected_role: Some(GroupRole::Dps),
            dungeon_ids: vec![run.dungeon.id],
            ..Default::default()
        };
        app.world_mut()
            .entity_mut(owner)
            .insert((run.clone(), lfg.clone()));
        crate::networking::install_disconnect_fixture(&mut app);
        let connection = app
            .world_mut()
            .spawn((
                lightyear::prelude::server::ClientOf,
                RemoteId(PeerId::Local(1)),
                Connected,
                crate::networking::PlayerLink(owner),
                crate::auth::InWorld { character_id: 30 },
            ))
            .id();
        app.world_mut().entity_mut(connection).remove::<Connected>();
        app.world_mut().flush();
        app.update();
        assert!(app.world().get_entity(owner).is_err());
        advance_runs(app.world_mut());
        super::super::update_ai(app.world_mut());
        assert!(
            app.world()
                .resource::<GroupRegistry>()
                .group_id_of(30)
                .is_some()
        );
        assert!(
            run.members
                .iter()
                .all(|&member| app.world().get_entity(member).is_ok())
        );

        let db = &app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let saved = crate::persistence::load_character(db, 30).unwrap();
        let saved_map = crate::maps::persistence::load_character_map(db, 30);
        let (x, y, z) = crate::terrain_height::wow_to_bevy(saved.x, saved.y, saved.z);
        let saved_position = shared::components::Position { x, y, z };
        assert_eq!(saved_map, map.map_id);
        assert_eq!(saved_position, position);
        assert_eq!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << 2
        );
        let mut state = bevy::ecs::system::SystemState::<(Commands, crate::maps::LoginMaps)>::new(
            app.world_mut(),
        );
        let (mut commands, mut maps) = state.get_mut(app.world_mut()).unwrap();
        let place = maps
            .resolve(
                &mut commands,
                30,
                1,
                Default::default(),
                saved_map,
                saved_position,
            )
            .unwrap();
        state.apply(app.world_mut());
        assert_eq!(place.map, map);
        assert_eq!(place.position, position);
        let reconnected = app
            .world_mut()
            .spawn((
                Player {
                    name: "Followproof".into(),
                    race: 1,
                    class: 8,
                    appearance: Default::default(),
                },
                ClientId(30),
                place.map,
                position,
            ))
            .id();
        reconnect(app.world_mut(), reconnected, 30);
        app.world_mut().flush();
        let elapsed = app.world().resource::<Time>().elapsed_secs_f64();
        assert!(
            app.world_mut()
                .resource_mut::<crate::maps::InstanceManager>()
                .take_expired(elapsed + 1800.0)
                .is_empty(),
            "reconnected player keeps the copy loaded"
        );
        assert_eq!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << 2
        );
        assert_eq!(
            app.world().get::<DungeonDifficulty>(reconnected).unwrap().0,
            205
        );
        assert_eq!(
            app.world().get::<crate::lfg::PlayerLfgState>(reconnected),
            Some(&lfg)
        );
        assert!(
            app.world()
                .resource::<GroupRegistry>()
                .group_id_of(30)
                .is_some()
        );
        assert!(run.members.iter().all(|&member| {
            app.world()
                .get::<Follower>(member)
                .is_some_and(|f| f.owner == reconnected)
                && app
                    .world()
                    .get::<crate::zones::TempSummon>(member)
                    .is_some_and(|s| s.summoner == reconnected)
        }));
        assert_eq!(
            app.world().get::<FollowerRun>(reconnected).unwrap().members,
            run.members
        );
        assert!(app.world().get::<crate::combat::Corpse>(dead).is_some());
        assert_eq!(
            app.world()
                .get::<shared::components::Health>(dead)
                .unwrap()
                .current,
            0.0
        );
        assert_eq!(
            app.world()
                .get::<crate::creature_ai::FollowOwner>(living)
                .unwrap()
                .owner,
            reconnected
        );
    }

    #[test]
    fn follower_disconnect_expires_copy_after_last_player_unload_delay() {
        let (mut app, owner, run) = fixture();
        let map = run.copy.unwrap();
        app.world_mut()
            .resource_mut::<crate::maps::InstanceManager>()
            .update_instance_lock(map, &[(owner, 30)], 1, Some(2), 1_790_510_400);
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        crate::maps::install_occupancy_fixture(&mut app);
        app.world_mut()
            .entity_mut(owner)
            .remove::<InMap>()
            .insert(map);
        app.world_mut()
            .entity_mut(owner)
            .insert((run.clone(), crate::lfg::PlayerLfgState::default()));
        crate::networking::install_disconnect_fixture(&mut app);
        let connection = app
            .world_mut()
            .spawn((
                lightyear::prelude::server::ClientOf,
                RemoteId(PeerId::Local(1)),
                Connected,
                crate::networking::PlayerLink(owner),
                crate::auth::InWorld { character_id: 30 },
            ))
            .id();
        app.world_mut().entity_mut(connection).remove::<Connected>();
        app.world_mut().flush();
        app.update();
        assert!(app.world().get_entity(owner).is_err());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1799));
        crate::maps::expire_instances_fixture(app.world_mut());
        advance_runs(app.world_mut());
        assert!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .copy_difficulty(map)
                .is_some()
        );
        assert!(
            run.members
                .iter()
                .all(|member| app.world().get_entity(*member).is_ok())
        );
        assert_eq!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << 2
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        crate::maps::expire_instances_fixture(app.world_mut());
        advance_runs(app.world_mut());
        assert!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .copy_difficulty(map)
                .is_none()
        );
        assert!(
            run.members
                .iter()
                .all(|member| app.world().get_entity(*member).is_err())
        );
        assert!(
            app.world()
                .resource::<GroupRegistry>()
                .group_id_of(30)
                .is_none()
        );
        assert!(
            app.world()
                .resource::<DisconnectedRuns>()
                .copy(30)
                .is_none()
        );
        let old_position = run.dungeon.entrance.bevy_position();
        let mut state = bevy::ecs::system::SystemState::<(Commands, crate::maps::LoginMaps)>::new(
            app.world_mut(),
        );
        let (mut commands, mut maps) = state.get_mut(app.world_mut()).unwrap();
        let place = maps
            .resolve(
                &mut commands,
                30,
                1,
                Default::default(),
                map.map_id,
                old_position,
            )
            .unwrap();
        state.apply(app.world_mut());
        assert_eq!(place.position, old_position);
        assert_eq!(
            app.world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(place.map),
            0
        );
    }

    #[test]
    fn follower_release_waits_for_combat_and_requires_a_complete_follow_path() {
        use shared::components::{CombatStatus, Health, Position};
        let (mut app, owner, run) = fixture();
        let member = run.members[0];
        let entrance = run.dungeon.entrance.bevy_position();
        app.world_mut()
            .insert_resource(crate::navigation::Navigation::new(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../data/ground/navmesh"),
            ));
        app.world_mut().entity_mut(owner).insert((
            run,
            CombatStatus(true),
            Position {
                x: 1403.56,
                y: 128.0,
                z: 173.39,
            },
        ));
        app.world_mut().entity_mut(member).insert((
            Health {
                current: 0.0,
                max: 1000.0,
            },
            crate::combat::Corpse { remaining: 0.0 },
            crate::combat::DeathState { respawn_timer: 0.0 },
        ));
        super::super::release::update(app.world_mut());
        assert!(
            app.world().get::<crate::combat::Corpse>(member).is_some(),
            "combat keeps the follower dead"
        );
        app.world_mut().get_mut::<CombatStatus>(owner).unwrap().0 = false;
        super::super::release::update(app.world_mut());
        app.world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
        super::super::release::update(app.world_mut());
        assert!(
            app.world().get::<crate::combat::Corpse>(member).is_none(),
            "complete route releases the follower"
        );
        assert_eq!(*app.world().get::<Position>(member).unwrap(), entrance);
        assert_eq!(app.world().get::<Health>(member).unwrap().current, 500.0);
        assert_eq!(
            app.world()
                .get::<crate::creature_ai::FollowOwner>(member)
                .unwrap()
                .owner,
            owner
        );
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        crate::creature_ai::install_follow_fixture(&mut app);
        for _ in 0..80 {
            app.update();
            app.world_mut()
                .resource_mut::<crate::navigation::Navigation>()
                .drain_until_idle();
        }
        let moved = *app.world().get::<Position>(member).unwrap();
        let target = *app.world().get::<Position>(owner).unwrap();
        let distance = |p: Position| (p.x - target.x).hypot(p.z - target.z);
        assert!(
            distance(moved) < distance(entrance) - 5.0,
            "normal Follow must walk toward the owner"
        );
    }

    #[test]
    fn follower_release_keeps_a_corpse_when_the_route_is_unreachable() {
        use shared::components::{Health, Position};
        let (mut app, owner, run) = fixture();
        let member = run.members[0];
        let corpse = Position {
            x: 1390.0,
            y: 131.0,
            z: 170.0,
        };
        app.world_mut()
            .insert_resource(crate::navigation::Navigation::new(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../data/ground/navmesh"),
            ));
        app.world_mut().entity_mut(owner).insert((
            run,
            Position {
                x: 1403.56,
                y: 10000.0,
                z: 173.39,
            },
        ));
        app.world_mut().entity_mut(member).insert((
            corpse,
            Health {
                current: 0.0,
                max: 1000.0,
            },
            crate::combat::Corpse { remaining: 0.0 },
            crate::combat::DeathState { respawn_timer: 0.0 },
        ));
        super::super::release::update(app.world_mut());
        app.world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
        super::super::release::update(app.world_mut());
        assert!(app.world().get::<crate::combat::Corpse>(member).is_some());
        assert_eq!(*app.world().get::<Position>(member).unwrap(), corpse);
        assert_eq!(app.world().get::<Health>(member).unwrap().current, 0.0);
    }

    #[test]
    fn killed_follower_waits_for_the_pull_to_end_then_releases() {
        use shared::components::{CombatStatus, Health, Position};
        let (fixture_app, _, mut run) = fixture();
        let store = fixture_app
            .world()
            .resource::<crate::maps::MapStore>()
            .clone();
        let copy = run.copy.unwrap();
        let entrance = run.dungeon.entrance.bevy_position();
        let (mut h, caster) = super::super::kit_tests::at_level_unengaged(209072, 32, |_| {});
        let target = super::super::utility_policy_tests::add_follower(&mut h, 215011);
        run.members = vec![caster, target];
        h.app.insert_resource(store);
        h.app.insert_resource(crate::navigation::Navigation::new(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
        ));
        h.app.world_mut().entity_mut(h.caster).insert((
            run,
            copy,
            Position {
                x: 1403.56,
                y: 128.0,
                z: 173.39,
            },
        ));
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert((copy, entrance));
        h.app.world_mut().entity_mut(target).insert((
            copy,
            entrance,
            crate::combat::CorpseDelay(60.0),
        ));
        h.app.world_mut().entity_mut(h.npc).insert(copy);
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), h.caster, h.npc);
        assert!(h.app.world().get::<CombatStatus>(h.caster).unwrap().0);
        crate::combat::kill_unit(h.app.world_mut(), h.npc, target);
        super::super::release::update(h.app.world_mut());
        assert_eq!(h.app.world().get::<Health>(target).unwrap().current, 0.0);
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_some());
        crate::combat::combat_stop(h.app.world_mut(), h.npc);
        assert!(!h.app.world().get::<CombatStatus>(h.caster).unwrap().0);
        super::super::release::update(h.app.world_mut());
        h.app
            .world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
        super::super::release::update(h.app.world_mut());
        assert!(h.app.world().get::<Health>(target).unwrap().current > 0.0);
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_none());
        assert_eq!(*h.app.world().get::<Position>(target).unwrap(), entrance);
        assert_eq!(
            h.app
                .world()
                .get::<crate::creature_ai::FollowOwner>(target)
                .unwrap()
                .owner,
            h.caster
        );
    }

    #[test]
    fn follower_release_distant_walkable_corpse_waits_for_real_resurrection() {
        use shared::components::{Health, Position};
        let (fixture_app, _, mut run) = fixture();
        let entrance = run.dungeon.entrance.bevy_position();
        let store = fixture_app
            .world()
            .resource::<crate::maps::MapStore>()
            .clone();
        let copy = run.copy.unwrap();
        let (mut h, caster) = super::super::kit_tests::level32(209072);
        let target = super::super::utility_policy_tests::add_follower(&mut h, 215011);
        let corpse = Position {
            x: 1390.0,
            y: 131.0,
            z: 170.0,
        };
        run.members = vec![caster, target];
        h.app.insert_resource(store);
        h.app.insert_resource(crate::navigation::Navigation::new(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
        ));
        h.app.world_mut().entity_mut(h.caster).insert((
            run,
            copy,
            Position {
                x: 1403.56,
                y: 128.0,
                z: 173.39,
            },
        ));
        h.app.world_mut().entity_mut(caster).insert((
            copy,
            Position {
                x: 1391.0,
                ..corpse
            },
        ));
        h.app.world_mut().entity_mut(target).insert((
            copy,
            corpse,
            crate::combat::CorpseDelay(60.0),
        ));
        crate::combat::kill_unit(h.app.world_mut(), h.npc, target);
        let mods = crate::auras::AuraStatMods {
            movement_speed: 0.01,
            ..Default::default()
        };
        h.app.world_mut().entity_mut(target).insert(mods);
        let start = Vec3::new(entrance.x, entrance.y, entrance.z);
        let end = Vec3::new(1403.56, 128.0, 173.39);
        let query = ::navigation::worker::Query::FollowPath {
            from: start,
            to: end,
        };
        {
            let mut nav = h
                .app
                .world_mut()
                .resource_mut::<crate::navigation::Navigation>();
            nav.query_deferred(
                target,
                copy.map_id,
                crate::navigation::Goal::FollowerRelease,
                query.clone(),
            );
            nav.drain_until_idle();
            let result = nav.query_deferred(
                target,
                copy.map_id,
                crate::navigation::Goal::FollowerRelease,
                query,
            );
            let Some(Ok(Some(::navigation::worker::Output::Path(path)))) = result else {
                panic!("expected a complete walkable path");
            };
            assert!(path.complete);
            let distance: f32 = path
                .points
                .windows(2)
                .map(|pair| pair[0].distance(pair[1]))
                .sum();
            assert!(distance > 7.0 * 0.01 * 120.0);
        }
        super::super::release::update(h.app.world_mut());
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_some());
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, 20484, Some(target), false)
            .unwrap();
        h.advance(80);
        assert!(h.app.world().get::<Health>(target).unwrap().current > 0.0);
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_none());
        assert_eq!(*h.app.world().get::<Position>(target).unwrap(), corpse);
    }

    #[test]
    fn active_follower_corpse_survives_decay_and_disconnect() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut app, owner, run) = fixture();
        let member = run.members[0];
        app.world_mut().entity_mut(owner).insert(run.clone());
        app.world_mut()
            .entity_mut(member)
            .insert(crate::combat::Corpse { remaining: 0.0 });
        app.world_mut()
            .run_system_once(crate::combat::corpse_decay_system)
            .unwrap();
        assert!(app.world().get::<crate::combat::Corpse>(member).is_some());
        app.world_mut()
            .entity_mut(owner)
            .insert(crate::lfg::PlayerLfgState::default());
        retire_owner(app.world_mut(), owner);
        app.world_mut()
            .run_system_once(crate::combat::corpse_decay_system)
            .unwrap();
        assert!(app.world().get::<crate::combat::Corpse>(member).is_some());
    }

    #[test]
    fn disconnected_owner_dormant_threat_orphans_unsummon_without_publication_panic() {
        let (mut app, owner, run) = fixture();
        app.add_plugins(crate::creature_aggro::CreatureAggroPlugin);
        for &member in &run.members {
            let mut threat = shared::components::ThreatTable::default();
            threat.add_threat(owner.to_bits(), 100.0);
            app.world_mut()
                .entity_mut(member)
                .insert((threat, crate::grid_activity::GridDormant));
        }
        app.world_mut().entity_mut(owner).insert(run.clone());
        // Native disconnect teardown removes the owner before orphan maintenance runs.
        app.world_mut().despawn(owner);
        advance_runs(app.world_mut());
        app.world_mut().flush();
        crate::creature_aggro::publish_woken_threat_lists(app.world_mut());
        assert!(
            run.members
                .iter()
                .all(|&member| app.world().get_entity(member).is_err()),
            "all owned summons must leave the world after their owner disconnects"
        );
    }

    #[test]
    fn party_follower_roster_keeps_actual_template_heads_after_interest_is_lost() {
        let (mut app, owner, run) = fixture();
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let expected: Vec<_> = run.members.iter().map(|&unit| {
            let profile = &app.world().get::<Follower>(unit).unwrap().profile;
            let display_id: u32 = db.query_row(
                "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx LIMIT 1",
                [profile.entry],
                |row| row.get(0),
            ).unwrap();
            let name = profile.name.clone();
            app.world_mut().entity_mut(unit).insert(shared::components::ModelDisplay { display_id });
            (name, display_id)
        }).collect();
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        let mut groups = app.world_mut().resource_mut::<GroupRegistry>();
        let group = groups.group_id_of(30).unwrap();
        let roster = groups.build_roster(group, &OnlineView::new()).unwrap();
        assert_eq!(roster.members.len(), 5);
        for (name, display_id) in expected {
            let member = roster
                .members
                .iter()
                .find(|member| member.name == name)
                .unwrap();
            assert_eq!(member.entity, None);
            let portrait = serde_json::to_value(&member.portrait).unwrap();
            assert_eq!(
                portrait["creature_display_id"], display_id,
                "real follower head: {name}"
            );
        }
    }

    #[test]
    fn follower_copy_group_formation_keeps_same_map_teleport_in_the_admitted_copy() {
        let (mut app, owner, run) = fixture();
        let admitted = *app.world().get::<InMap>(owner).unwrap();
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        assert_eq!(
            app.world()
                .resource::<GroupRegistry>()
                .dungeon_difficulty_of(30),
            Some(205)
        );
        assert_eq!(
            app.world()
                .resource::<GroupRegistry>()
                .raid_difficulty_of(30),
            Some(crate::maps::RaidDifficulty {
                primary: 15,
                legacy: 4,
            })
        );
        assert!(teleport_to(
            app.world_mut(),
            owner,
            run.dungeon.entrance,
            TeleportOptions::default()
        ));
        assert!(
            app.world().get::<MapTransfer>(owner).is_none(),
            "forming the follower party must not orphan its admitted solo copy"
        );
        assert_eq!(app.world().get::<InMap>(owner), Some(&admitted));
        for unit in run.members {
            assert_eq!(app.world().get::<InMap>(unit), Some(&admitted));
        }
    }

    #[test]
    fn follower_copy_run_ends_when_owner_enters_another_copy_of_the_same_map() {
        let (mut app, owner, run) = fixture();
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        let members = run.members.clone();
        app.world_mut().entity_mut(owner).insert((
            run,
            InMap {
                map_id: 2521,
                instance_id: 99,
            },
        ));
        advance_runs(app.world_mut());
        assert!(
            app.world().get::<FollowerRun>(owner).is_none(),
            "same map ID is not the same follower instance"
        );
        assert!(
            app.world()
                .resource::<GroupRegistry>()
                .group_id_of(30)
                .is_none()
        );
        assert!(
            members
                .iter()
                .all(|&unit| app.world().get_entity(unit).is_err())
        );
    }

    /// Live Ruby rerun: Rubynew killed Melidrussa, released outside, requeued, and was
    /// admitted to its loaded orphaned copy without Melidrussa. TC forms a new LFG group
    /// before the teleport, so the requeue must resolve a fresh copy.
    #[test]
    fn follower_requeue_after_a_progressed_copy_admits_a_fresh_copy() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let store = crate::maps::store::load_map_store(&db).unwrap();
        let catalog = super::super::catalog::load_catalog(&db).unwrap();
        let now = crate::maps::instances::Now {
            elapsed: 0.0,
            unix: 1_790_510_400,
        };
        let solo = crate::maps::instances::InstanceEntrant {
            character_id: 30,
            group: None,
            leader: 30,
            difficulty_id: 205,
            raid_difficulty: Default::default(),
            raid_group: false,
        };
        let mut instances = crate::maps::InstanceManager::default();
        let old = instances.create_map(&store, 2521, solo, now).unwrap().map;
        let melidrussa = (0..32)
            .find(|&id| {
                crate::scripted_ai::instance_script::done_boss_entries(&store, 2521, 1 << id)
                    == [188252]
            })
            .expect("Ruby script binds Melidrussa 188252");
        instances.update_instance_lock(old, &[], 1 << melidrussa, None, now.unix);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(store)
            .insert_resource(instances)
            .insert_resource(catalog)
            .init_resource::<GroupRegistry>();
        // Ruby's reciprocal exit safe-loc 100031, where the release revived Rubynew.
        let outside = WorldLocation {
            map_id: 2444,
            position: Vec3::new(1341.3, -132.202, 137.45),
            orientation: 0.0,
        };
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: "Rubynew".into(),
                    race: 1,
                    class: 8,
                    appearance: Default::default(),
                },
                ClientId(30),
                shared::components::Level(70),
                UnitLevel(70),
                outside.bevy_position(),
                shared::components::Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                InMap {
                    map_id: 2444,
                    instance_id: 0,
                },
                DungeonDifficulty(1),
            ))
            .id();
        queue_player(app.world_mut(), player, GroupRole::Dps, 2436);
        assert!(app.world().get::<MapTransfer>(player).is_some());
        let entrant = crate::maps::transfer::entrant_of(app.world(), player);
        let arrival = app.world_mut().resource_scope(
            |world, mut instances: Mut<crate::maps::InstanceManager>| {
                instances
                    .create_map(
                        world.resource::<crate::maps::MapStore>(),
                        2521,
                        entrant,
                        now,
                    )
                    .unwrap()
            },
        );
        let instances = app.world().resource::<crate::maps::InstanceManager>();
        assert_ne!(arrival.map, old, "requeue must not reuse the orphaned copy");
        assert!(arrival.created);
        assert_eq!(instances.initial_done_bosses(arrival.map), 0);
        assert_eq!(instances.initial_done_bosses(old), 1 << melidrussa);
    }
}
