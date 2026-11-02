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
            .find(|run| run.run.members.contains(&member))
            .map(|run| run.run.entry_difficulty())
    }
}

#[derive(Component, Clone)]
pub(crate) struct FollowerRun {
    dungeon: Dungeon,
    profiles: Vec<Profile>,
    class_profiles: Vec<super::class_profiles::ClassProfile>,
    seed: u64,
    gear: super::class_spawn::GearSource,
    keystone: Option<shared::protocol::KeystoneInfo>,
    members: Vec<Entity>,
    copy: Option<InMap>,
    previous_difficulty: u32,
    origin: WorldLocation,
    player_role: GroupRole,
}

impl FollowerRun {
    fn entry_difficulty(&self) -> u32 {
        match self.gear {
            super::class_spawn::GearSource::Journal => FOLLOWER_DIFFICULTY,
            super::class_spawn::GearSource::KeystoneBand(_) => self.previous_difficulty,
        }
    }

    pub(crate) fn contains_member(&self, member: Entity) -> bool {
        self.copy.is_some() && self.members.contains(&member)
    }
}

pub(crate) fn validate_keystone(
    world: &World,
    owner: Entity,
    key: &shared::protocol::KeystoneInfo,
) -> Result<(), String> {
    if let Some(run) = world.get::<FollowerRun>(owner) {
        let matches = run.keystone.as_ref().is_some_and(|expected| {
            expected.item_guid == key.item_guid
                && expected.challenge_id == key.challenge_id
                && expected.level == key.level
        });
        if !matches {
            return Err("follower party was formed for a different keystone".into());
        }
    }
    Ok(())
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

#[cfg(test)]
pub(crate) fn update_test_releases(world: &mut World) {
    super::release::update(world);
}

#[cfg(test)]
pub(crate) fn spawn_keystone_test_party(
    world: &mut World,
    owner: Entity,
    dungeon_id: u32,
    key: shared::protocol::KeystoneInfo,
) -> Result<Vec<Entity>, String> {
    let dungeon = world
        .resource::<FollowerCatalog>()
        .dungeons
        .get(&dungeon_id)
        .cloned()
        .ok_or("test dungeon unavailable")?;
    let level = world
        .get::<shared::components::Level>(owner)
        .ok_or("test owner level unavailable")?
        .0;
    let spec = world
        .get::<shared::components::ActiveSpec>(owner)
        .ok_or("test owner spec unavailable")?
        .0;
    let role = GroupRole::Dps;
    let seed = 1;
    let class_profiles = super::class_profiles::select_keystone_party_profiles(
        world, role, spec, level, key.level, seed,
    )?;
    let profiles = class_profiles
        .iter()
        .enumerate()
        .map(|(slot, profile)| super::class_spawn::profile_for(profile, slot))
        .collect::<Result<Vec<_>, _>>()?;
    let run = FollowerRun {
        dungeon,
        profiles,
        class_profiles,
        seed,
        gear: super::class_spawn::GearSource::KeystoneBand(key.level),
        keystone: Some(key),
        members: Vec::new(),
        copy: None,
        previous_difficulty: 23,
        origin: WorldLocation {
            map_id: 0,
            position: Vec3::ZERO,
            orientation: 0.0,
        },
        player_role: role,
    };
    world.entity_mut(owner).insert(run.clone());
    spawn_party(world, owner, &run)?;
    Ok(world
        .get::<FollowerRun>(owner)
        .expect("spawned party")
        .members
        .clone())
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
    let player_spec = world
        .get::<shared::components::ActiveSpec>(player)
        .ok_or("follower queue requires an active specialization")?
        .0;
    if player_spec == 0 {
        return Err("follower queue requires an active specialization".into());
    }
    let level = world
        .get::<shared::components::Level>(player)
        .ok_or("follower queue requires a level")?
        .0;
    if let Some(reason) = dungeon.level_lock(u32::from(level)) {
        return Err(reason.into());
    }
    let previous_difficulty = world
        .get::<DungeonDifficulty>(player)
        .ok_or("player has no dungeon difficulty")?
        .0;
    let keystone = if previous_difficulty == 23 {
        Some(crate::mythic_plus::follower_key_for_map(
            world,
            id,
            dungeon.entrance.map_id,
        )?)
    } else {
        None
    };
    let gear = match &keystone {
        Some(key) => super::class_spawn::GearSource::KeystoneBand(key.level),
        None => super::class_spawn::GearSource::Journal,
    };
    let seed = crate::random::random::<u64>();
    let class_profiles = match gear {
        super::class_spawn::GearSource::Journal => {
            super::class_profiles::select_party_profiles(world, role, player_spec, level, seed)?
        }
        super::class_spawn::GearSource::KeystoneBand(key_level) => {
            super::class_profiles::select_keystone_party_profiles(
                world,
                role,
                player_spec,
                level,
                key_level,
                seed,
            )?
        }
    };
    let profiles = class_profiles
        .iter()
        .enumerate()
        .map(|(slot, profile)| super::class_spawn::profile_for(profile, slot))
        .collect::<Result<Vec<_>, _>>()?;
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
    let entry_difficulty = match gear {
        super::class_spawn::GearSource::Journal => FOLLOWER_DIFFICULTY,
        super::class_spawn::GearSource::KeystoneBand(_) => previous_difficulty,
    };
    world
        .resource_mut::<crate::maps::InstanceManager>()
        .forget_recent_instance(id, dungeon.entrance.map_id);
    world
        .entity_mut(player)
        .insert(DungeonDifficulty(entry_difficulty));
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
    info!(
        "class follower queue seed {seed}, player spec {player_spec}, follower specs {:?}",
        class_profiles
            .iter()
            .map(|profile| profile.spec_id)
            .collect::<Vec<_>>()
    );
    world.entity_mut(player).insert(FollowerRun {
        dungeon,
        profiles,
        class_profiles,
        seed,
        gear,
        keystone,
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
    let members =
        super::class_spawn::spawn_members(world, owner, &run.class_profiles, run.seed, run.gear)?;
    if let Err(error) = join_party(world, owner, run, &members) {
        super::class_spawn::unsummon_members(world, &members);
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
            let level = world.get::<UnitLevel>(unit).expect("follower level").0;
            let member =
                if let Some(visual) = world.get::<shared::components::CharacterVisual>(unit) {
                    follower.online_character(
                        unit,
                        level,
                        difficulty,
                        visual,
                        world.get::<shared::components::EquipmentAppearance>(unit),
                    )
                } else {
                    let display = world
                        .get::<shared::components::ModelDisplay>(unit)
                        .expect("spawned follower display");
                    follower.online(unit, level, difficulty, display.display_id)
                };
            online.insert(Follower::member_id(unit), member);
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

pub(crate) fn notify_reconnected(world: &mut World, player: Entity) {
    let Some(run) = world.get::<FollowerRun>(player).cloned() else {
        return;
    };
    let Some(character) = world.get::<ClientId>(player).map(|id| id.0) else {
        return;
    };
    let Some(group) = world.resource::<GroupRegistry>().group_id_of(character) else {
        return;
    };
    let Ok(owner) = online_player(world, player, run.entry_difficulty()) else {
        return;
    };
    let mut online = OnlineView::from([(character, owner)]);
    for &member in &run.members {
        let Some(follower) = world.get::<Follower>(member) else {
            continue;
        };
        let Some(level) = world.get::<UnitLevel>(member) else {
            continue;
        };
        let entry = if let Some(visual) = world.get::<shared::components::CharacterVisual>(member) {
            follower.online_character(
                member,
                level.0,
                run.entry_difficulty(),
                visual,
                world.get::<shared::components::EquipmentAppearance>(member),
            )
        } else if let Some(display) = world.get::<shared::components::ModelDisplay>(member) {
            follower.online(member, level.0, run.entry_difficulty(), display.display_id)
        } else {
            continue;
        };
        online.insert(Follower::member_id(member), entry);
    }
    let roster = world
        .resource::<GroupRegistry>()
        .build_roster(group, &online);
    if let (Some(roster), Some(connection)) =
        (roster, crate::spell_cast::owner_connection(world, player))
        && let Some(mut sender) =
            world.get_mut::<MessageSender<shared::protocol::GroupRosterSnapshot>>(connection)
    {
        sender.send::<GroupChannel>(roster);
    }
    send_update(world, player, None, None);
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
        if let Some(mut summoned_by) = world.get_mut::<shared::components::UnitSummonedBy>(member) {
            summoned_by.0 = player.to_bits();
        }
    }
    let difficulty = run.entry_difficulty();
    world
        .entity_mut(player)
        .insert((DungeonDifficulty(difficulty), run, lfg));
    notify_reconnected(world, player);
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
                shared::components::Health {
                    current: 1000.0,
                    max: 1000.0,
                },
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
            class_profiles: Vec::new(),
            seed: 0,
            gear: super::super::class_spawn::GearSource::Journal,
            keystone: None,
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
    fn follower_dungeon_real_selection_resumes_same_combat_copy() {
        exercise_real_reconnect(false);
    }

    #[test]
    fn follower_dungeon_real_selection_resumes_after_logout_grace() {
        exercise_real_reconnect(true);
    }

    fn exercise_real_reconnect(after_grace: bool) {
        let (mut source, _, mut run) = fixture();
        let store = source
            .world_mut()
            .remove_resource::<crate::maps::MapStore>()
            .unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        let (character, _) = crate::persistence::create_character(&db, 11, "Resume", 1, 1).unwrap();
        let (mut app, connection) = crate::auth_character::tests::queue_character_selection(
            crate::persistence::GameDatabase::new(db),
            11,
            character,
            crate::maps::MapStore::continents(),
        );
        let owner = crate::auth_character::tests::finish_character_selection(&mut app, connection);
        let mut instances = crate::maps::InstanceManager::default();
        let map = instances
            .create_map(
                &store,
                2521,
                crate::maps::instances::InstanceEntrant {
                    character_id: character,
                    group: None,
                    leader: character,
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
        app.insert_resource(store)
            .insert_resource(instances)
            .init_resource::<GroupRegistry>();
        let position = shared::components::Position {
            x: 1403.56,
            y: 128.0,
            z: 173.39,
        };
        app.world_mut()
            .entity_mut(owner)
            .insert((map, position, DungeonDifficulty(205)));
        run.copy = Some(map);
        run.members = run
            .profiles
            .iter()
            .map(|profile| {
                app.world_mut()
                    .spawn((
                        Follower {
                            owner,
                            profile: profile.clone(),
                        },
                        crate::zones::TempSummon { summoner: owner },
                        shared::components::Health {
                            current: 900.0,
                            max: 1000.0,
                        },
                        shared::components::UnitLevel(12),
                        shared::components::ModelDisplay { display_id: 1 },
                        shared::components::Npc {
                            template_id: profile.entry,
                            name: profile.name.clone(),
                        },
                        position,
                        map,
                    ))
                    .id()
            })
            .collect();
        join_party(app.world_mut(), owner, &run, &run.members).unwrap();
        app.world_mut()
            .entity_mut(owner)
            .insert((run.clone(), crate::lfg::PlayerLfgState::default()));
        let enemy = app
            .world_mut()
            .spawn((
                shared::components::Health {
                    current: 250.0,
                    max: 500.0,
                },
                map,
            ))
            .id();
        crate::combat_refs::set_in_combat_with(app.world_mut(), owner, enemy);
        app.world_mut()
            .entity_mut(run.members[0])
            .insert(crate::combat::AutoAttack::new(enemy));
        crate::networking::install_disconnect_fixture(&mut app);
        app.world_mut()
            .entity_mut(connection)
            .insert(lightyear::prelude::server::ClientOf)
            .remove::<Connected>();
        app.world_mut().flush();
        app.update();
        super::super::update_ai(app.world_mut());
        assert!(
            app.world()
                .get::<crate::networking::DisconnectedPlayer>(owner)
                .is_some()
        );
        assert!(
            app.world()
                .get::<crate::combat::AutoAttack>(run.members[0])
                .is_none()
        );
        assert!(crate::combat_refs::has_active_reference_to(
            app.world(),
            owner,
            enemy
        ));
        if after_grace {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(60));
            app.world_mut().run_schedule(PreUpdate);
            advance_runs(app.world_mut());
            assert!(app.world().get_entity(owner).is_err());
        }
        let restored = crate::auth_character::tests::reconnect_character(&mut app, 11, character);
        let new_connection = app
            .world_mut()
            .query::<(Entity, &crate::networking::PlayerLink)>()
            .iter(app.world())
            .find(|(_, link)| link.0 == restored)
            .unwrap()
            .0;
        app.update();
        let rosters: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<shared::protocol::GroupRosterSnapshot>>(new_connection)
            .unwrap()
            .receive()
            .collect();
        assert!(
            rosters.iter().any(|roster| roster.members.len() == 5),
            "reconnected owner receives the follower party roster"
        );
        assert_eq!(restored == owner, !after_grace);
        assert_eq!(*app.world().get::<InMap>(restored).unwrap(), map);
        assert_eq!(
            *app.world()
                .get::<shared::components::Position>(restored)
                .unwrap(),
            position
        );
        assert_eq!(
            app.world()
                .get::<shared::components::Health>(enemy)
                .unwrap()
                .current,
            250.0
        );
        if !after_grace {
            assert!(crate::combat_refs::has_active_reference_to(
                app.world(),
                restored,
                enemy
            ));
        }
        assert!(run.members.iter().all(|member| {
            app.world()
                .get::<Follower>(*member)
                .is_some_and(|f| f.owner == restored)
                && app
                    .world()
                    .get::<shared::components::Health>(*member)
                    .is_some_and(|health| health.current == 900.0)
                && super::super::acting_follower(app.world(), *member).is_some()
        }));
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
        assert!(
            app.world()
                .get::<crate::networking::DisconnectedPlayer>(owner)
                .is_some()
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(60));
        app.world_mut().run_schedule(PreUpdate);
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
    fn follower_reconnect_restores_owner_target_healing() {
        let (fixture_app, _, mut run) = fixture();
        let map = run.copy.unwrap();
        drop(fixture_app);
        let (mut h, healer) = super::super::kit_tests::level32(209072);
        run.members = vec![healer];
        let old_owner = h.caster;
        h.app.init_resource::<GroupRegistry>();
        h.app.world_mut().entity_mut(old_owner).insert((
            ClientId(30),
            map,
            run,
            crate::lfg::PlayerLfgState::default(),
        ));
        h.app.world_mut().entity_mut(healer).insert(map);
        crate::networking::install_disconnect_fixture(&mut h.app);
        h.app
            .world_mut()
            .entity_mut(h.connection)
            .insert(lightyear::prelude::server::ClientOf)
            .remove::<Connected>();
        h.app.world_mut().flush();
        h.app.update();
        assert!(
            h.app
                .world()
                .get::<crate::networking::DisconnectedPlayer>(old_owner)
                .is_some()
        );
        h.app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(60));
        h.app.world_mut().run_schedule(PreUpdate);
        assert!(h.app.world().get_entity(old_owner).is_err());

        let owner = crate::spell_cast::test_harness::spawn_player(&mut h.app, 8, 0.0);
        h.app.world_mut().entity_mut(owner).insert((
            ClientId(30),
            map,
            shared::components::Health {
                current: 300.0,
                max: 1000.0,
            },
        ));
        reconnect(h.app.world_mut(), owner, 30);
        assert!(!crate::auras::has_aura(h.app.world(), owner, 774));
        crate::spell_cast::creature_cast(h.app.world_mut(), healer, 18562, Some(owner), false)
            .expect("Swiftmend must recognize the reconnected owner without a heal-over-time aura");
        h.advance(3);
        assert!(h.health(owner) > 300.0);
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
        assert!(
            app.world()
                .get::<crate::networking::DisconnectedPlayer>(owner)
                .is_some()
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(60));
        app.world_mut().run_schedule(PreUpdate);
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

    #[derive(Clone, Default)]
    struct ReleaseLogBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for ReleaseLogBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> bevy::log::tracing_subscriber::fmt::MakeWriter<'a> for ReleaseLogBuffer {
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture_release_update(app: &mut App) -> String {
        let buffer = ReleaseLogBuffer::default();
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_max_level(bevy::log::tracing::Level::INFO)
            .with_writer(buffer.clone())
            .without_time()
            .with_ansi(false)
            .finish();
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            super::super::release::update(app.world_mut());
        });
        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        output
    }

    #[test]
    fn follower_release_logs_combat_pending_and_reachable_decisions() {
        use shared::components::{CombatStatus, Health, Position};
        let (mut app, owner, run) = fixture();
        let member = run.members[0];
        app.insert_resource(crate::navigation::Navigation::new(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
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
        ));
        let buffer = ReleaseLogBuffer::default();
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_max_level(bevy::log::tracing::Level::INFO)
            .with_writer(buffer.clone())
            .without_time()
            .with_ansi(false)
            .finish();
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            super::super::release::update(app.world_mut());
            app.world_mut().get_mut::<CombatStatus>(owner).unwrap().0 = false;
            super::super::release::update(app.world_mut());
            app.world_mut()
                .resource_mut::<crate::navigation::Navigation>()
                .drain_until_idle();
            super::super::release::update(app.world_mut());
        });
        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        let events: Vec<_> = output
            .lines()
            .filter(|line| line.contains("Follower release evaluation"))
            .collect();
        assert_eq!(events.len(), 3, "{output}");
        assert!(
            events.iter().all(|line| line.contains("decision=")),
            "{output}"
        );
        assert!(
            events[0].contains("party_in_combat") && events[0].contains("walk_seconds=None"),
            "{output}"
        );
        assert!(
            events[1].contains("route_pending") && events[1].contains("walk_seconds=None"),
            "{output}"
        );
        assert!(
            events[2].contains("route_reachable") && events[2].contains("walk_seconds=Some("),
            "{output}"
        );
        assert!(
            events
                .iter()
                .all(|line| line.contains(&format!("{member:?}"))),
            "{output}"
        );
    }

    #[test]
    fn follower_release_staggered_deaths_survive_navigation_pruning() {
        use shared::components::{CombatStatus, Health, Position};
        let (fixture_app, _, mut run) = fixture();
        let store = fixture_app
            .world()
            .resource::<crate::maps::MapStore>()
            .clone();
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        run.dungeon = super::super::catalog::load_catalog(&db).unwrap().dungeons[&2745].clone();
        let copy = InMap {
            map_id: 2805,
            instance_id: 1,
        };
        run.copy = Some(copy);
        let (mut h, healer) = super::super::kit_tests::at_level_unengaged(209072, 32, |_| {});
        let first = super::super::utility_policy_tests::add_follower(&mut h, 215011);
        let second = super::super::utility_policy_tests::add_follower(&mut h, 215011);
        run.members = vec![healer, first, second];
        h.app.insert_resource(store);
        crate::navigation::install_fixture(
            &mut h.app,
            crate::navigation::Navigation::new(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../data/ground/navmesh"),
            ),
        );
        h.app
            .add_systems(FixedUpdate, super::super::release::update);
        h.app.world_mut().entity_mut(h.caster).insert((
            run,
            copy,
            Position {
                x: 5316.5,
                y: 127.4,
                z: 3125.3,
            },
        ));
        h.app.world_mut().entity_mut(healer).insert(copy);
        // The two corpse positions and owner position are from the Windrunner live snapshot.
        for (member, x, z) in [(first, 5254.49, 3158.06), (second, 5257.81, 3158.04)] {
            h.app.world_mut().entity_mut(member).insert((
                copy,
                Position { x, y: 126.30, z },
                crate::combat::CorpseDelay(60.0),
            ));
        }
        h.app.world_mut().entity_mut(h.npc).insert(copy);
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), h.caster, h.npc);
        crate::combat::kill_unit(h.app.world_mut(), h.npc, first);
        h.advance(3);
        assert!(h.app.world().get::<crate::combat::Corpse>(first).is_some());
        crate::combat::kill_unit(h.app.world_mut(), h.npc, second);
        assert!(h.app.world().get::<CombatStatus>(h.caster).unwrap().0);
        crate::combat::combat_stop(h.app.world_mut(), h.npc);
        h.advance(30);
        for member in [first, second] {
            assert!(
                h.app.world().get::<crate::combat::Corpse>(member).is_none(),
                "member {member:?} did not release"
            );
            assert!(h.app.world().get::<Health>(member).unwrap().current > 0.0);
        }
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
        let log = capture_release_update(&mut app);
        assert!(log.contains("route_incomplete"), "{log}");
        assert!(log.contains("error=Some("), "{log}");
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

    fn brackenhide_release_fixture(
        owner_position: shared::components::Position,
    ) -> (
        crate::spell_cast::test_harness::Harness,
        Entity,
        Entity,
        shared::components::Position,
    ) {
        use shared::components::CombatStatus;
        let (fixture_app, _, mut run) = fixture();
        let store = fixture_app
            .world()
            .resource::<crate::maps::MapStore>()
            .clone();
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        run.dungeon = super::super::catalog::load_catalog(&db).unwrap().dungeons[&2439].clone();
        let copy = InMap {
            map_id: 2520,
            instance_id: 1,
        };
        run.copy = Some(copy);
        let entrance = run.dungeon.entrance.bevy_position();
        let (mut h, caster) = super::super::kit_tests::level32(209072);
        let target = super::super::utility_policy_tests::add_follower(&mut h, 215011);
        run.members = vec![caster, target];
        h.app.insert_resource(store);
        h.app.insert_resource(crate::navigation::Navigation::new(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
        ));
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert((run, copy, owner_position));
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
        crate::combat::kill_unit(h.app.world_mut(), h.npc, target);
        crate::combat::combat_stop(h.app.world_mut(), h.npc);
        crate::combat::combat_stop(h.app.world_mut(), h.caster);
        assert!(!h.app.world().get::<CombatStatus>(h.caster).unwrap().0);
        (h, caster, target, entrance)
    }

    #[test]
    fn follower_release_long_brackenhide_route_waits_for_resurrection() {
        use shared::components::{Health, Position};
        let owner_position = Position {
            x: -3344.54,
            y: 111.17662,
            z: -3663.96,
        };
        let (mut h, caster, target, entrance) = brackenhide_release_fixture(owner_position);
        let copy = *h.app.world().get::<InMap>(h.caster).unwrap();
        let point = |position: Position| Vec3::new(position.x, position.y, position.z);
        let query = ::navigation::worker::Query::CompleteRoute {
            from: point(entrance),
            to: point(owner_position),
        };
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
        drop(nav);
        let Some(Ok(Some(::navigation::worker::Output::CompleteRoute(route)))) = result else {
            panic!("Brackenhide needs a complete entrance-to-boss route: {result:?}");
        };
        assert!(route.complete, "Brackenhide route: {:?}", route.reason);
        let distance = route.distance;
        let speed = crate::creature_ai::BASE_RUN_SPEED
            * h.app
                .world()
                .get::<crate::auras::AuraStatMods>(target)
                .map_or(1.0, |mods| mods.movement_speed);
        assert!(distance > speed * 120.0, "{distance} yards at {speed} yd/s");
        let log = capture_release_update(&mut h.app);
        assert!(log.contains("route_too_long"), "{log}");
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_some());
        assert_eq!(h.app.world().get::<Health>(target).unwrap().current, 0.0);
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, 20484, Some(target), false)
            .unwrap();
        h.advance(80);
        assert!(h.app.world().get::<Health>(target).unwrap().current > 0.0);
        assert_eq!(*h.app.world().get::<Position>(target).unwrap(), entrance);
    }

    #[test]
    fn follower_release_brackenhide_route_under_two_minutes_walks_back() {
        use shared::components::{Health, Position};
        let owner_position = Position {
            x: -3217.9695,
            y: 34.78784,
            z: -4078.9055,
        };
        let (mut h, _, target, entrance) = brackenhide_release_fixture(owner_position);
        super::super::release::update(h.app.world_mut());
        h.app
            .world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
        let log = capture_release_update(&mut h.app);
        assert!(log.contains("route_reachable"), "{log}");
        assert!(h.app.world().get::<Health>(target).unwrap().current > 0.0);
        assert!(h.app.world().get::<crate::combat::Corpse>(target).is_none());
        assert_eq!(*h.app.world().get::<Position>(target).unwrap(), entrance);
        crate::creature_ai::install_follow_fixture(&mut h.app);
        h.advance(80);
        let moved = *h.app.world().get::<Position>(target).unwrap();
        let distance = |position: Position| {
            (position.x - owner_position.x).hypot(position.z - owner_position.z)
        };
        assert!(
            distance(moved) < distance(entrance) - 5.0,
            "follower must walk back: {moved:?}"
        );
    }

    #[test]
    fn follower_release_distant_walkable_corpse_waits_for_real_resurrection() {
        use shared::components::{CombatStatus, Health, Position};
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
        h.app.world_mut().entity_mut(h.npc).insert(copy);
        crate::combat::kill_unit(h.app.world_mut(), h.npc, target);
        crate::combat::combat_stop(h.app.world_mut(), h.npc);
        crate::combat::combat_stop(h.app.world_mut(), h.caster);
        assert!(!h.app.world().get::<CombatStatus>(h.caster).unwrap().0);
        let mods = crate::auras::AuraStatMods {
            movement_speed: 0.01,
            ..Default::default()
        };
        h.app.world_mut().entity_mut(target).insert(mods);
        let start = Vec3::new(entrance.x, entrance.y, entrance.z);
        let end = Vec3::new(1403.56, 128.0, 173.39);
        let query = ::navigation::worker::Query::CompleteRoute {
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
            let Some(Ok(Some(::navigation::worker::Output::CompleteRoute(route)))) = result else {
                panic!("expected a complete walkable path");
            };
            assert!(route.complete, "{:?}", route.reason);
            assert!(route.distance > 7.0 * 0.01 * 120.0);
        }
        let log = capture_release_update(&mut h.app);
        assert!(log.contains("route_too_long"), "{log}");
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

    #[test]
    fn follower_halls_queue_enforces_content_tuning_levels() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let store = crate::maps::store::load_map_store(&db).unwrap();
        let catalog = super::super::catalog::load_catalog(&db).unwrap();
        let halls = catalog.dungeons.get(&2444).unwrap();
        assert_eq!((halls.min_level, halls.max_level), (40, 70));
        let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(store)
            .init_resource::<crate::maps::InstanceManager>()
            .insert_resource(catalog)
            .insert_resource(classes)
            .insert_resource(spells)
            .init_resource::<GroupRegistry>();
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: "Test".into(),
                    race: 1,
                    class: 8,
                    appearance: Default::default(),
                },
                ClientId(30),
                shared::components::Level(80),
                shared::components::ActiveSpec(62),
                UnitLevel(70),
                WorldLocation {
                    map_id: 2444,
                    position: Vec3::new(1341.3, -132.202, 137.45),
                    orientation: 0.0,
                }
                .bevy_position(),
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
        assert_eq!(
            begin_run(app.world_mut(), player, GroupRole::Dps, 2444),
            Err("LFG_LOCKSTATUS_TOO_HIGH_LEVEL".into())
        );
        assert!(app.world().get::<MapTransfer>(player).is_none());
        app.world_mut()
            .entity_mut(player)
            .insert(shared::components::Level(39));
        assert_eq!(
            begin_run(app.world_mut(), player, GroupRole::Dps, 2444),
            Err("LFG_LOCKSTATUS_TOO_LOW_LEVEL".into())
        );
        assert!(app.world().get::<MapTransfer>(player).is_none());
        app.world_mut()
            .entity_mut(player)
            .insert(shared::components::Level(70));
        begin_run(app.world_mut(), player, GroupRole::Dps, 2444).unwrap();
        assert!(app.world().get::<MapTransfer>(player).is_some());
        assert!(app.world().get::<FollowerRun>(player).is_some());
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
        let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
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
            .insert_resource(classes)
            .insert_resource(spells)
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
                shared::components::ActiveSpec(62),
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
        let run = app
            .world()
            .get::<FollowerRun>(player)
            .expect("accepted follower queue");
        assert_eq!(run.class_profiles.len(), 4);
        assert!(
            !run.class_profiles
                .iter()
                .any(|profile| profile.spec_id == 62)
        );
        let available = super::super::class_profiles::read_profiles().unwrap();
        let qualified = super::super::class_profiles::validate_profiles(
            &available,
            app.world().resource::<crate::spell_info::ClassSpells>(),
            70,
        );
        let replay =
            super::super::class_profiles::select_profiles(&qualified, GroupRole::Dps, 62, run.seed)
                .unwrap();
        assert_eq!(
            run.class_profiles
                .iter()
                .map(|profile| profile.spec_id)
                .collect::<Vec<_>>(),
            replay
                .iter()
                .map(|profile| profile.spec_id)
                .collect::<Vec<_>>()
        );
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
