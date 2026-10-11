//! Native DB2 12.1.0.69933 portal10059 and TDB176296 -> spell17334.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::Position;
use shared::protocol::{GameObjectInfo, InteractionChannel, UseGameObject};

fn portal_fixture() -> Harness {
    let mut h = harness_with_plugins(Vec::new(), &[10059, 17334], 8, false, |app| {
        app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
    })
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

#[test]
fn class_completeness_native_portal_owner_uses_authored_city_destination() {
    let mut h = portal_fixture();
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
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseGameObject>>(h.connection)
        .unwrap()
        .send::<InteractionChannel>(UseGameObject {
            object: objects[0].to_bits(),
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
        destinations.0[&(17334, 0)].bevy_position()
    );
}
