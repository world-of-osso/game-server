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
    let orphans: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .filter(|(_, follower)| {
            let retail_run = world.get::<FollowerRun>(follower.owner).is_some();
            let diagnostic_run = world
                .get::<super::stockade_diagnostic::DiagnosticParty>(follower.owner)
                .is_some();
            !retail_run && !diagnostic_run
        })
        .map(|(unit, _)| unit)
        .collect();
    for unit in orphans {
        crate::zones::unsummon(world, unit);
    }
}

fn spawn_party(world: &mut World, owner: Entity, run: &FollowerRun) -> Result<(), String> {
    let members = super::spawning::spawn_members(world, owner, &run.profiles)?;
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
    let difficulty = world
        .get::<DungeonDifficulty>(owner)
        .ok_or("companion owner has no dungeon difficulty")?
        .0;
    let id = world.get::<ClientId>(owner).ok_or("owner logged out")?.0;
    let player = world.get::<Player>(owner).ok_or("owner is not a player")?;
    let online_player = OnlineMember {
        entity: owner.to_bits(),
        name: player.name.clone(),
        class: player.class,
        level: world.get::<UnitLevel>(owner).ok_or("owner level absent")?.0,
        portrait: crate::group::portrait_appearance(
            player,
            world.get::<shared::components::EquipmentAppearance>(owner),
        ),
        dungeon_difficulty: difficulty,
        raid_difficulty: world
            .get::<crate::maps::RaidDifficulty>(owner)
            .copied()
            .unwrap_or_default(),
    };
    let mut online = OnlineView::from([(id, online_player)]);
    let companion_roles: Vec<_> = members
        .iter()
        .map(|&unit| {
            let follower = world.get::<Follower>(unit).expect("spawned follower");
            online.insert(
                Follower::member_id(unit),
                follower.online(
                    unit,
                    world.get::<UnitLevel>(unit).expect("follower level").0,
                    difficulty,
                ),
            );
            (Follower::member_id(unit), follower.profile.role)
        })
        .collect();
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
    #[ignore = "party follower portrait RED; enable after native/headless GREEN"]
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
}
