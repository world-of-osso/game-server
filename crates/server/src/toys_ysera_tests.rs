//! Source Ysera130170: one private, non-pet vision with an ordinary120s lifetime.
//! Procedural visual8605/type22 is not covered by this server integration test.
use super::*;

#[test]
fn toyfx61_ysera_real_learning_use_preserves_private_source_actor_and_expiry() {
    use crate::interest::PrivateObjectOwner;
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, Rotation, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("ysera.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    learn_effect_toy(&mut h, character, 130170);
    assert_eq!(
        activate(&mut h, 130170)[0].error,
        None,
        "real Ysera journal admission"
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 195708));
    let caster = h.caster;
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 99518 && summon.spell_id == 195708)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1, "one authentic Vision of Ysera");
    let (actor, expires_at) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 66207);
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        caster.to_bits()
    );
    assert_eq!(world.get::<PrivateObjectOwner>(actor).unwrap().0, caster);
    assert_eq!(world.get::<crate::maps::InMap>(actor), Some(&map));
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let placed = *world.get::<Position>(actor).unwrap();
    assert!(
        (placed.x - origin.x - 8.0).abs() < 0.01 && (placed.z - origin.z).abs() < 0.01,
        "source47 uses its8yd radius in front of the caster: {placed:?}"
    );
    let remaining = expires_at - world.resource::<Time>().elapsed_secs_f64();
    assert!(
        (119.0..=120.0).contains(&remaining),
        "authored120s expiry, got {remaining}"
    );
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x -= 5.0;
    h.advance(20);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    let remaining = expires_at - h.app.world().resource::<Time>().elapsed_secs_f64();
    // Keep source duration and the ordinary50ms simulation harness unchanged.
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}
