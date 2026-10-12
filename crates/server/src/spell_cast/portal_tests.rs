//! Native DB2 12.1.0.69933 portal10059 and TDB176296 ->17334 ->121857.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Health, Position};
use shared::protocol::{GameObjectInfo, InteractionChannel, UseGameObject};

fn portal_fixture() -> Harness {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[10059, 17334, 121857, 11416, 44089, 121851],
        8,
        false,
        |app| {
            app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
            app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
        },
    )
    .unwrap();
    let data = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    h.app
        .insert_resource(super::teleport_positions::load_positions(&conn).unwrap());
    h.app.init_resource::<crate::group::GroupRegistry>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<UseGameObject>::default(),
        MessageReceiver::<UseGameObject>::default(),
        MessageSender::<shared::protocol::InteractionFailed>::default(),
        MessageReceiver::<shared::protocol::InteractionFailed>::default(),
    ));
    h
}

fn spawn_portal(h: &mut Harness) -> Entity {
    let spell = world_db().unwrap().store.get(10059).unwrap().clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None)
        .expect("native Portal: Stormwind");
    let world = h.app.world_mut();
    let objects: Vec<_> = world
        .query::<(Entity, &GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| info.entry == 176296)
        .map(|(unit, _)| unit)
        .collect();
    assert_eq!(objects.len(), 1);
    objects[0]
}

#[test]
fn class_completeness_native_portal_identity_condition_keeps_signed_alliance_race_mask() {
    let mut h = portal_fixture();
    let spell = world_db().unwrap().store.get(11416).unwrap().clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None)
        .expect("native Ironforge portal");
    let world = h.app.world_mut();
    let portal = world
        .query::<(Entity, &GameObjectInfo)>()
        .iter(world)
        .find(|(_, info)| info.entry == 176497)
        .unwrap()
        .0;
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = 2;
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(
        crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, portal),
        Err(shared::protocol::InteractionError::InvalidTarget)
    );
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), before);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = 1;
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, portal).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let destinations = super::teleport_positions::load_positions(&conn).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        destinations.0[&(121851, 0)].bevy_position()
    );
}

#[test]
fn class_completeness_native_portal_owner_uses_authored_city_destination() {
    let mut h = portal_fixture();
    let portal = spawn_portal(&mut h);
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseGameObject>>(h.connection)
        .unwrap()
        .send::<InteractionChannel>(UseGameObject {
            object: portal.to_bits(),
        });
    h.advance(4);
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let destinations = super::teleport_positions::load_positions(&conn).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        destinations.0[&(121857, 0)].bevy_position()
    );
}

#[test]
fn class_completeness_native_portal_rejects_strangers_dead_users_and_other_maps() {
    use shared::protocol::InteractionError;
    let mut h = portal_fixture();
    let portal = spawn_portal(&mut h);
    let stranger = spawn_player(&mut h.app, 8, 1.0);
    h.app
        .world_mut()
        .entity_mut(stranger)
        .insert(crate::persistence::ClientId(2));
    let before = *h.app.world().get::<Position>(stranger).unwrap();
    assert_eq!(
        crate::game_objects::spellcaster::use_object(h.app.world_mut(), stranger, portal),
        Err(InteractionError::InvalidTarget)
    );
    assert_eq!(*h.app.world().get::<Position>(stranger).unwrap(), before);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    assert_eq!(
        crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, portal),
        Err(InteractionError::PlayerDead)
    );
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(1));
    assert_eq!(
        crate::game_objects::spellcaster::use_object(h.app.world_mut(), h.caster, portal),
        Err(InteractionError::TooFarAway)
    );
}

#[test]
fn class_completeness_native_portal_party_member_requires_reach_then_transports() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    let mut h = portal_fixture();
    let portal = spawn_portal(&mut h);
    let member = spawn_player(&mut h.app, 8, 100.0);
    h.app
        .world_mut()
        .entity_mut(member)
        .insert(crate::persistence::ClientId(2));
    let online: OnlineView = [(1, h.caster), (2, member)]
        .into_iter()
        .map(|(id, unit)| {
            (
                id,
                OnlineMember {
                    entity: unit.to_bits(),
                    name: format!("Portal{id}"),
                    class: 8,
                    level: 80,
                    portrait: Default::default(),
                    dungeon_difficulty: 0,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.invite(1, "Portal2", &online);
    groups.respond_invite(2, true);
    assert!(groups.is_in_same_group(1, 2));
    h.app.insert_resource(groups);
    assert_eq!(
        crate::game_objects::spellcaster::use_object(h.app.world_mut(), member, portal),
        Err(shared::protocol::InteractionError::TooFarAway)
    );
    h.app.world_mut().get_mut::<Position>(member).unwrap().x = 1.0;
    crate::game_objects::spellcaster::use_object(h.app.world_mut(), member, portal).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let destinations = super::teleport_positions::load_positions(&conn).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(member).unwrap(),
        destinations.0[&(121857, 0)].bevy_position()
    );
}
