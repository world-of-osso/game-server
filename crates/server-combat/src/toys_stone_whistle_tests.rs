//! Source Enchanted Stone Whistle: one Petrified Sand Piper near the selected unit.
use super::*;

#[test]
fn toyfx59_stone_whistle_real_learning_use_preserves_source_actor_and_expiry() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("stone-whistle.redb"));
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
    let target_position = Position {
        x: origin.x + 12.0,
        ..origin
    };
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((target_position, map));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    learn_effect_toy(&mut h, character, 129279);
    assert_eq!(
        activate(&mut h, 129279)[0].error,
        None,
        "real Stone Whistle journal admission"
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 193687));
    let caster = h.caster;
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 98300 && summon.spell_id == 193687)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1, "one authentic Petrified Sand Piper");
    let (actor, expires_at) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 71056);
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        caster.to_bits()
    );
    assert_eq!(world.get::<crate::maps::InMap>(actor), Some(&map));
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let placed = *world.get::<Position>(actor).unwrap();
    let target_offset = Vec2::new(placed.x - target_position.x, placed.z - target_position.z);
    assert!(
        target_offset.length() <= 3.001,
        "source63 selects target, then86 samples its3yd disk: {placed:?}"
    );
    assert!(
        (placed.x - origin.x).abs() >= 8.999,
        "not the caster's disk"
    );
    let remaining = expires_at - world.resource::<Time>().elapsed_secs_f64();
    assert!(
        (59.0..=60.0).contains(&remaining),
        "authored60s expiry, got {remaining}"
    );
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x -= 5.0;
    h.advance(20);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    let remaining = expires_at - h.app.world().resource::<Time>().elapsed_secs_f64();
    // Preserve the ordinary50ms harness, source clocks and cooldown history.
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}
