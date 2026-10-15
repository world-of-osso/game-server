//! Source property2301 joins the summoner spawn group and routes through SummonGuardian.
//! Count/placement/designation proof only; no invented guardian spell or native visual claim.
use super::*;

#[test]
fn toyfx39_scarab_storm_preserves_source_guardian_count_scatter_and_lifetime() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Npc, UnitIsPet, UnitLevel, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = import_toyfx8_world(directory.path());
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    let spell = store.get(92146).unwrap();
    assert_eq!(spell.effects[0].base_points, 25.0);
    let (mut h, character) = setup(&directory.path().join("scarab.redb"));
    let destination = crate::spell_cast::targets::effect_dest(
        h.app.world(),
        h.caster,
        None,
        &spell.effects[0],
        None,
    )
    .unwrap()
    .position;
    h.app.insert_resource(store);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::world_data::load_world_data(&path));
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, 64881);
    let result = activate(&mut h, 64881);
    assert_eq!(result[0].error, None, "source Scarab Storm admission");
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let caster = h.caster;
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 51113 && summon.spell_id == 92146)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    // Native25 varies by +/-20% before int32 conversion, never fixed25 or one actor.
    assert!(
        (20..=30).contains(&actors.len()),
        "source count {}",
        actors.len()
    );
    let points: Vec<_> = actors
        .iter()
        .map(|(actor, expires)| {
            assert_eq!(
                world.get::<UnitSummonedBy>(*actor).unwrap().0,
                caster.to_bits()
            );
            assert!(!world.get::<UnitIsPet>(*actor).is_some_and(|pet| pet.0));
            assert_eq!(world.get::<UnitLevel>(*actor).unwrap().0, 80);
            assert_eq!(
                world
                    .get::<crate::creature_ai::FollowOwner>(*actor)
                    .unwrap()
                    .owner,
                caster
            );
            assert_eq!(
                world.get::<crate::maps::InMap>(*actor),
                world.get::<crate::maps::InMap>(caster)
            );
            let remaining = expires - world.resource::<Time>().elapsed_secs_f64();
            assert!(
                (23.0..=24.0).contains(&remaining),
                "native24000ms lifetime {remaining}"
            );
            let position = world.get::<Position>(*actor).unwrap();
            Vec3::new(position.x, position.y, position.z)
        })
        .collect();
    assert!(
        points
            .iter()
            .any(|point| point.distance(destination) < 0.001)
    );
    assert!(
        points
            .iter()
            .all(|point| point.distance(destination) <= 5.001)
    );
    assert!(points.iter().any(|point| point.distance(destination) > 0.1));
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(24));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(24),
        ));
    h.advance(1);
    for (actor, _) in actors {
        assert!(h.app.world().get_entity(actor).is_err());
    }
}
