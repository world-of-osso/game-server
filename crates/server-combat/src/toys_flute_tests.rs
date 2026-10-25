//! TC a352b1fa EffectSummonType/CalcValue: source Flute uses variable wild count.
//! Bounded count/placement/expiry proof, NOT the Retail description's follow promise.
use super::*;

#[test]
fn toyfx32_wild_count_truncates_fractional_values_and_clamps_nonpositive_values() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::Npc;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let source_store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (value, expected) in [(4.5, 4), (1.9, 1), (0.0, 1), (-2.0, 1)] {
        let (mut h, character) = setup(&directory.path().join(format!("count-{value}.redb")));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        let mut spell = source_store.get(142917).unwrap().clone();
        // Isolate CalcValue's conversion boundary, not a claim these modified
        // values are the authentic Flute contract. The other test uses raw source.
        spell.effects[0].base_points = value;
        spell.effects[0].variance = 0.0;
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell]));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        learn_effect_toy(&mut h, character, 98552);
        assert_eq!(activate(&mut h, 98552)[0].error, None);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let world = h.app.world_mut();
        let count = world
            .query::<(&Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(npc, summon)| npc.template_id == 71500 && summon.spell_id == 142917)
            .count();
        assert_eq!(count, expected, "effect value{value}");
    }
}

#[test]
fn toyfx32_flute_learn_use_summons_source_snakes_at_one_destination_and_expires() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("flute.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    learn_effect_toy(&mut h, character, 98552);
    assert_eq!(activate(&mut h, 98552)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 142917));
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 71500 && summon.spell_id == 142917)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    // CalcValue varies 4.5 by +/-0.33333349 before int32 truncation. Float
    // endpoints can reach 2 or6; never reinterpret fractional4.5 as fixed4.
    assert!(
        (2..=6).contains(&actors.len()),
        "source count: {}",
        actors.len()
    );
    let first_position = *world.get::<Position>(actors[0].0).unwrap();
    let destination_distance = Vec3::new(
        first_position.x - origin.x,
        first_position.y - origin.y,
        first_position.z - origin.z,
    )
    .length();
    // TC CalcRadius randomizes target86 within its authored2yd disk, not a ring.
    assert!(destination_distance <= 2.0 + 0.001);
    for (actor, expires_at) in &actors {
        let actor_display = world.get::<ModelDisplay>(*actor).unwrap().display_id;
        assert!([3126, 4440, 35803, 36580].contains(&actor_display));
        assert_eq!(
            world.get::<UnitSummonedBy>(*actor).unwrap().0,
            h.caster.to_bits()
        );
        assert!(!world.get::<UnitIsPet>(*actor).is_some_and(|pet| pet.0));
        // CalcRadius(TargetA) has no entry: additional summons do not scatter
        // with TargetB's2yd destination radius.
        assert_eq!(world.get::<Position>(*actor), Some(&first_position));
        assert_eq!(
            world.get::<crate::maps::InMap>(*actor),
            world.get::<crate::maps::InMap>(h.caster)
        );
        let remaining = expires_at - world.resource::<Time>().elapsed_secs_f64();
        assert!((58.0..=60.0).contains(&remaining), "{remaining}");
    }
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(60));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(60),
        ));
    h.advance(1);
    for (actor, _) in actors {
        assert!(h.app.world().get_entity(actor).is_err());
    }
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        display
    );
}
