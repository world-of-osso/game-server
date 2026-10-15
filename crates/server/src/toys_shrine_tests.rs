//! Exact source Sea-Blessed Shrine cursor placement and generic summon lifetime.
use super::*;

#[test]
fn toyfx29_shrine_learn_use_cursor_stationary_and_source_expiry() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitIsPet, UnitLevel, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let native_level: u8 = conn
        .query_row(
            "SELECT minlevel FROM content_creature_template WHERE entry=246037",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(native_level, 1);
    let (mut h, character) = setup(&directory.path().join("shrine.redb"));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
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
    let destination = [origin.x + 3.0, origin.y, origin.z];
    learn_effect_toy(&mut h, character, 245942);
    assert_eq!(
        h.app.world().get::<Level>(h.caster).unwrap().0,
        80,
        "fixture caster level changed during learning"
    );
    assert_eq!(
        activate_at(&mut h, 245942, Some(destination))[0].error,
        None
    );
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 1240082));
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 246037 && summon.spell_id == 1240082)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1);
    let (actor, expires_at) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 129958);
    // Source2910 UseCreatureLevel: preserve the native creature level, not owner80.
    assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, native_level);
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        h.caster.to_bits()
    );
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let placed = *world.get::<Position>(actor).unwrap();
    assert_eq!([placed.x, placed.y, placed.z], destination);
    assert_eq!(
        world.get::<crate::maps::InMap>(actor),
        world.get::<crate::maps::InMap>(h.caster)
    );
    let remaining = expires_at - world.resource::<Time>().elapsed_secs_f64();
    assert!((177.0..=180.0).contains(&remaining), "{remaining}");
    world.get_mut::<Position>(h.caster).unwrap().x += 5.0;
    h.advance(100);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    let remaining = expires_at - h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs_f64(remaining + 0.2));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(remaining + 0.2),
        ));
    h.advance(1);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        display
    );
}
