//! Explicit diagnostic companion attachment; never a Retail catalog/admission entry.
use crate::maps::{DungeonDifficulty, InMap};
use crate::persistence::ClientId;
use bevy::prelude::*;
use shared::group::GroupRole;

const STOCKADE_MAP: u32 = 34;
const REQUEST_ENV: &str = "GAME_SERVER_STOCKADE_FOLLOWER_DIAGNOSTIC";

#[derive(Resource)]
struct DiagnosticRequest(String);

#[derive(Component, Clone)]
pub(super) struct DiagnosticParty {
    copy: InMap,
    members: Vec<Entity>,
}

pub(super) fn load_request(mut commands: Commands) {
    match std::env::var(REQUEST_ENV) {
        Ok(name) if !name.is_empty() => {
            commands.insert_resource(DiagnosticRequest(name));
        }
        Ok(_) => panic!("{REQUEST_ENV} requires a character name"),
        Err(std::env::VarError::NotPresent) => {}
        Err(error) => panic!("read {REQUEST_ENV}: {error}"),
    }
}

pub(super) fn advance(world: &mut World) {
    retire_departed_parties(world);
    let Some(request) = world.get_resource::<DiagnosticRequest>() else {
        return;
    };
    let name = request.0.clone();
    let owner = world
        .query::<(Entity, &shared::components::Player, &InMap, &ClientId)>()
        .iter(world)
        .find(|(_, player, map, _)| player.name == name && map.map_id == STOCKADE_MAP)
        .map(|(entity, _, _, _)| entity);
    let Some(owner) = owner else { return };
    world.remove_resource::<DiagnosticRequest>();
    match spawn_party(world, owner, GroupRole::Dps) {
        Ok(members) => info!(
            "DIAGNOSTIC Stockade follower party: character {name}, {} companions, Normal instance; NOT Retail follower admission",
            members.len()
        ),
        Err(error) => error!("DIAGNOSTIC Stockade follower party for {name} refused: {error}"),
    }
}

fn retire_departed_parties(world: &mut World) {
    let departed: Vec<_> = world
        .query::<(Entity, &DiagnosticParty)>()
        .iter(world)
        .filter(|(owner, party)| world.get::<InMap>(*owner) != Some(&party.copy))
        .map(|(owner, party)| (owner, party.members.clone()))
        .collect();
    for (owner, members) in departed {
        super::party::leave_members(world, owner, &members);
        world.entity_mut(owner).remove::<DiagnosticParty>();
    }
}

fn stockade_copy(world: &World, owner: Entity) -> Result<InMap, String> {
    let copy = *world
        .get::<InMap>(owner)
        .ok_or("diagnostic owner has no map")?;
    if copy.map_id != STOCKADE_MAP || copy.instance_id == 0 {
        return Err("diagnostic requires an existing Stockade instance".into());
    }
    let difficulty = world
        .resource::<crate::maps::InstanceManager>()
        .copy_difficulty(copy)
        .map(|entry| entry.difficulty_id);
    if difficulty != Some(crate::maps::store::DIFFICULTY_NORMAL)
        || world.get::<DungeonDifficulty>(owner).map(|d| d.0) != difficulty
    {
        return Err("diagnostic requires the owner's admitted Normal Stockade copy".into());
    }
    let id = world
        .get::<ClientId>(owner)
        .ok_or("diagnostic owner is not authenticated")?
        .0;
    if world
        .resource::<crate::group::GroupRegistry>()
        .group_id_of(id)
        .is_some()
    {
        return Err("diagnostic requires a solo owner".into());
    }
    Ok(copy)
}

pub(super) fn spawn_party(
    world: &mut World,
    owner: Entity,
    role: GroupRole,
) -> Result<Vec<Entity>, String> {
    let copy = stockade_copy(world, owner)?;
    let profiles = super::select_party(&world.resource::<super::FollowerCatalog>().profiles, role)?;
    let members = super::spawning::spawn_members(world, owner, &profiles)?;
    if let Err(error) = super::party::join_members(world, owner, role, &members) {
        super::spawning::unsummon_members(world, &members);
        return Err(error);
    }
    world.entity_mut(owner).insert(DiagnosticParty {
        copy,
        members: members.clone(),
    });
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::followers::{Follower, FollowerCatalog};
    use crate::maps::{DungeonDifficulty, InMap};
    use shared::components::Position;

    fn fixture() -> (crate::spell_cast::test_harness::Harness, InMap) {
        let (mut h, initial, _) = super::super::navigation_tests::stockade_fixture(Position {
            x: 150.557,
            y: -34.85624,
            z: -45.8164,
        });
        h.app.world_mut().despawn(initial);
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let store = crate::maps::store::load_map_store(&db).unwrap();
        let mut instances = crate::maps::InstanceManager::default();
        let copy = instances
            .create_map(
                &store,
                34,
                crate::maps::instances::InstanceEntrant {
                    character_id: 30,
                    group: None,
                    leader: 30,
                    difficulty_id: 1,
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
        h.app
            .insert_resource(store)
            .insert_resource(instances)
            .insert_resource(super::super::catalog::load_catalog(&db).unwrap())
            .insert_resource(crate::world_data::load_world_data(
                &crate::test_data::world_db_path(),
            ))
            .init_resource::<crate::group::GroupRegistry>();
        h.app.world_mut().entity_mut(h.caster).insert((
            crate::persistence::ClientId(30),
            DungeonDifficulty(1),
            copy,
            crate::lfg::PlayerLfgState::default(),
        ));
        (h, copy)
    }

    #[test]
    fn stockade_diagnostic_attaches_four_native_companions_without_retail_queue_or_movement() {
        let (mut h, copy) = fixture();
        let before = *h.app.world().get::<Position>(h.caster).unwrap();
        let catalog: Vec<_> = h
            .app
            .world()
            .resource::<FollowerCatalog>()
            .dungeons
            .keys()
            .copied()
            .collect();
        let members = spawn_party(h.app.world_mut(), h.caster, GroupRole::Dps).unwrap();
        assert_eq!(members.len(), 4);
        assert_eq!(h.app.world().get::<Position>(h.caster), Some(&before));
        assert_eq!(h.app.world().get::<InMap>(h.caster), Some(&copy));
        assert!(
            h.app
                .world()
                .get::<crate::maps::MapTransfer>(h.caster)
                .is_none()
        );
        assert!(
            h.app
                .world()
                .get::<crate::lfg::PlayerLfgState>(h.caster)
                .unwrap()
                .match_found
                .is_none()
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::group::GroupRegistry>()
                .dungeon_difficulty_of(30),
            Some(1)
        );
        for member in members {
            assert_eq!(h.app.world().get::<InMap>(member), Some(&copy));
            let follower = h.app.world().get::<Follower>(member).unwrap();
            assert_eq!(follower.owner, h.caster);
            let (stats, _, _) =
                super::super::spawning::class_stats(h.app.world(), &follower.profile, 10).unwrap();
            let health = h
                .app
                .world()
                .get::<shared::components::Health>(member)
                .unwrap();
            assert_eq!(health.max, stats.max_health);
            assert_eq!(health.current, health.max);
        }
        let after: Vec<_> = h
            .app
            .world()
            .resource::<FollowerCatalog>()
            .dungeons
            .keys()
            .copied()
            .collect();
        assert_eq!(after, catalog);
    }

    #[test]
    fn stockade_diagnostic_preserves_companions_until_owner_leaves_the_exact_copy() {
        let (mut h, _) = fixture();
        let members = spawn_party(h.app.world_mut(), h.caster, GroupRole::Dps).unwrap();
        super::super::party::advance_runs(h.app.world_mut());
        assert!(
            members
                .iter()
                .all(|unit| h.app.world().get_entity(*unit).is_ok())
        );
        h.app.world_mut().entity_mut(h.caster).insert(InMap {
            map_id: 34,
            instance_id: 99,
        });
        advance(h.app.world_mut());
        assert!(
            members
                .iter()
                .all(|unit| h.app.world().get_entity(*unit).is_err())
        );
        assert!(h.app.world().get::<DiagnosticParty>(h.caster).is_none());
        assert!(
            h.app
                .world()
                .resource::<crate::group::GroupRegistry>()
                .group_id_of(30)
                .is_none()
        );
        assert_eq!(
            h.app.world().get::<DungeonDifficulty>(h.caster).unwrap().0,
            1
        );
    }

    #[test]
    fn stockade_diagnostic_refuses_another_map_without_spawning_companions() {
        let (mut h, _) = fixture();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(InMap::continent(0));
        assert!(spawn_party(h.app.world_mut(), h.caster, GroupRole::Dps).is_err());
        assert_eq!(
            h.app
                .world_mut()
                .query::<&Follower>()
                .iter(h.app.world())
                .count(),
            0
        );
        assert!(
            h.app
                .world()
                .resource::<crate::group::GroupRegistry>()
                .group_id_of(30)
                .is_none()
        );
    }
}
