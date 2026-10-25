//! Source Winner's Podium: actual bag learning/use, grounded placement and20min lifetime.
use super::*;

#[test]
fn toyfx54_podium_real_learning_use_preserves_static_source_actor_and_expiry() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, Rotation, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("podium.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    let (x, y, z): (f32, f32, f32) = conn.query_row(
        "SELECT position_x,position_y,position_z FROM tdb_playercreateinfo WHERE race=1 AND class=8",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    let origin = Vec3::new(x, y, z);
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        maps.maps
            .values()
            .map(|map| (map.id, map.wdt_file_data_id))
            .collect(),
    );
    let expected = origin + Vec3::X * 1.5;
    let crate::ground::GroundProbe::Supported(height) = ground.ground_at(0, expected) else {
        panic!("authored Northshire podium terrain required");
    };
    h.app.insert_resource(ground);
    h.app.world_mut().entity_mut(h.caster).insert((
        Position { x, y, z },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    learn_effect_toy(&mut h, character, 239018);
    assert_eq!(
        activate(&mut h, 239018)[0].error,
        None,
        "real Podium journal admission"
    );
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 1227063));
    let caster = h.caster;
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 240220 && summon.spell_id == 1227063)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1, "one authentic Winner's Podium");
    let (actor, expires_at) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 127527);
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
    let expected = Vec3::new(expected.x, height, expected.z);
    assert!(Vec3::new(placed.x, placed.y, placed.z).distance(expected) < 0.01);
    let remaining = expires_at - world.resource::<Time>().elapsed_secs_f64();
    assert!(
        (1197.0..=1200.0).contains(&remaining),
        "authored20min expiry, got {remaining}"
    );
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x += 5.0;
    h.advance(100);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    let remaining = expires_at - h.app.world().resource::<Time>().elapsed_secs_f64();
    // Advance the existing50ms harness unchanged; no timer/cooldown reset or max-delta override.
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}
