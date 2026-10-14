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

#[derive(Component, Clone)]
pub(crate) struct FollowerRun {
    dungeon: Dungeon,
    profiles: Vec<Profile>,
    members: Vec<Entity>,
    previous_difficulty: u32,
    origin: WorldLocation,
    player_role: GroupRole,
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
        if world
            .get::<InMap>(player)
            .is_none_or(|map| map.map_id != run.dungeon.entrance.map_id)
        {
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
        .filter(|(_, follower)| world.get::<FollowerRun>(follower.owner).is_none())
        .map(|(unit, _)| unit)
        .collect();
    for unit in orphans {
        crate::zones::unsummon(world, unit);
    }
}

fn spawn_party(world: &mut World, owner: Entity, run: &FollowerRun) -> Result<(), String> {
    let mut members = Vec::new();
    for profile in &run.profiles {
        match super::spawning::spawn(world, owner, profile.clone()) {
            Ok(unit) => members.push(unit),
            Err(error) => {
                for unit in members {
                    crate::zones::unsummon(world, unit);
                }
                return Err(error);
            }
        }
    }
    if let Err(error) = join_party(world, owner, run, &members) {
        for unit in members {
            crate::zones::unsummon(world, unit);
        }
        return Err(error);
    }
    world
        .get_mut::<FollowerRun>(owner)
        .expect("queued follower run")
        .members = members;
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
        dungeon_difficulty: FOLLOWER_DIFFICULTY,
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
                ),
            );
            (Follower::member_id(unit), follower.profile.role)
        })
        .collect();
    let mut registry = world.resource_mut::<GroupRegistry>();
    let group = registry.create_companion_party(id, run.player_role, &companion_roles, &online)?;
    let roster = registry
        .build_roster(group, &online)
        .expect("formed companion roster");
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
    for &unit in &run.members {
        if world.get_entity(unit).is_ok() {
            crate::zones::unsummon(world, unit);
        }
    }
    if let Some(id) = world.get::<ClientId>(player).map(|id| id.0) {
        world.resource_mut::<GroupRegistry>().leave(id);
    }
    world.entity_mut(player).remove::<FollowerRun>().insert((
        DungeonDifficulty(run.previous_difficulty),
        crate::lfg::PlayerLfgState::default(),
    ));
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
