//! Concrete Retail matrix spells; all IDs come from world.db.
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitSummonedBy};

use super::test_harness::*;

#[test]
fn summon_at_rune_weapon_is_owned_and_expires() {
    let mut h = harness(Vec::new(), &[49028], 6).unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let conn = rusqlite::Connection::open(&world_db().unwrap().powers_db).unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.cast_and_deliver(49028, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, _)| npc.template_id == 27893)
        .map(|(entity, _, owner)| (entity, owner.0))
        .collect();
    assert_eq!(
        summons.len(),
        1,
        "Dancing Rune Weapon must spawn its DB2 creature"
    );
    let (summon, owner) = summons[0];
    assert_eq!(owner, h.caster.to_bits());
    assert_eq!(world.get::<Position>(summon), world.get::<Position>(h.npc));
    h.advance(150);
    assert!(h.app.world().get_entity(summon).is_ok());
    h.advance(12);
    assert!(h.app.world().get_entity(summon).is_err());
}
