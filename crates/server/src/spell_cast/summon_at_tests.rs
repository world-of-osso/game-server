//! Concrete Retail matrix spells; all IDs come from world.db.
//! Purging Breath supplies an authentic non-matrix trigger binding: all nine
//! matrix roots lack such bindings; their lifecycle is tested without guessing one.
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitSummonedBy};

use super::test_harness::*;

#[test]
fn summon_at_rune_weapon_is_owned_and_expires() {
    let mut h = harness(Vec::new(), &[49028], 6).unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    install_summon_properties(&mut h);
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

fn install_summon_properties(h: &mut Harness) {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!(
        "../../tests/fixtures/summon-properties-12.1.0.69933.sql"
    ))
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
}

#[test]
fn summon_at_barrier_spawns_once_and_expires_without_invented_trigger() {
    let mut h = harness(Vec::new(), &[62618], 5).unwrap();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    h.cast_and_deliver(62618, Some(ally));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let world = h.app.world_mut();
    let triggers: Vec<_> = world
        .query::<(Entity, &super::area_trigger::SpellAreaTrigger)>()
        .iter(world)
        .filter(|(_, trigger)| trigger.spell_id == 62618)
        .map(|(entity, trigger)| (entity, trigger.caster))
        .collect();
    assert_eq!(
        triggers.len(),
        1,
        "one area entity, not one per selected ally"
    );
    assert_eq!(triggers[0].1, h.caster);
    assert_eq!(
        world.get::<Position>(triggers[0].0),
        world.get::<Position>(ally)
    );
    h.advance(190);
    assert!(h.app.world().get_entity(triggers[0].0).is_ok());
    h.advance(12);
    assert!(h.app.world().get_entity(triggers[0].0).is_err());
}

#[test]
fn summon_at_data_bound_purging_breath_hits_inside_not_outside() {
    // Real E179 MiscValue 42145 -> EffectTriggerSpell 1287711, radius 2,
    // duration 6000 ms. This is engine proof, NOT a missing matrix binding.
    let mut h = harness(Vec::new(), &[1287715, 1287711], 8).unwrap();
    let outside = spawn_npc(&mut h.app, 20.0);
    h.cast_and_deliver(1287715, None);
    h.advance(31); // Its real SpellCastTimes record is 1500 ms.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current
            < 5000.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(outside)
            .unwrap()
            .current,
        5000.0
    );
    let world = h.app.world_mut();
    let triggers: Vec<_> = world
        .query_filtered::<Entity, With<super::area_trigger::SpellAreaTrigger>>()
        .iter(world)
        .collect();
    assert_eq!(triggers.len(), 1);
    h.advance(122);
    assert!(h.app.world().get_entity(triggers[0]).is_err());
}
