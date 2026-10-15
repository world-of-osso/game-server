//! TC a352b1fa destination18/78: source Bashiok is five yards ahead, not at caster feet.
use super::*;

#[test]
fn toyfx44_bashiok_source_destination_uses_secondary_radius_and_caster_facing() {
    use shared::components::Rotation;
    let mut h = harness(Vec::new(), &[173895], 8).unwrap();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(173895)
        .unwrap()
        .clone();
    let origin = Vec3::new(13.0, 7.0, -11.0);
    for facing in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        h.app.world_mut().entity_mut(h.caster).insert((
            Position {
                x: origin.x,
                y: origin.y,
                z: origin.z,
            },
            Rotation {
                x: 0.0,
                y: facing + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ));
        let destination = crate::spell_cast::targets::effect_dest(
            h.app.world(),
            h.caster,
            None,
            &spell.effects[0],
            None,
        )
        .expect("authored18/78 must select a destination");
        let expected = origin + Vec3::new(5.0 * facing.cos(), 0.0, -5.0 * facing.sin());
        assert!(
            destination.position.distance(expected) < 0.001,
            "source five-yard front position: {:?} != {expected:?}",
            destination.position
        );
        assert!(
            (destination.orientation - facing).abs() < 0.001,
            "effect-facing pi is inactive without USE_FACING_FROM_SPELL"
        );
    }
}

#[test]
fn toyfx44_bashiok_real_learn_use_preserves_source_actor_and_natural_lifetime() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, Rotation, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = import_toyfx8_world(directory.path());
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("bashiok.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::world_data::load_world_data(&path));
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
    learn_effect_toy(&mut h, character, 118222);
    assert_eq!(
        activate(&mut h, 118222)[0].error,
        None,
        "actual Bashiok journal admission"
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 173895));
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 86916 && summon.spell_id == 173895)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1, "one source Spirit of Bashiok");
    let (actor, expiry) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 60051);
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        h.caster.to_bits()
    );
    assert_eq!(world.get::<crate::maps::InMap>(actor), Some(&map));
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let position = *world.get::<Position>(actor).unwrap();
    assert!((position.x - origin.x - 5.0).abs() < 0.001);
    assert_eq!(position.y, origin.y);
    assert!((position.z - origin.z).abs() < 0.001);
    assert!((world.get::<Rotation>(actor).unwrap().y - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    assert!((59.0..=60.0).contains(&(expiry - world.resource::<Time>().elapsed_secs_f64())));
    assert_eq!(world.get::<Health>(h.caster), Some(&health));
    assert_eq!(world.get::<ModelDisplay>(h.caster).copied(), display);
    h.advance(10);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&position));
    // Fixture-only virtual advance; this is not a native wall-time certificate.
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(60));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(60),
        ));
    h.advance(1);
    assert!(h.app.world().get_entity(actor).is_err());
}
