//! Source property2301 joins the summoner spawn group and routes through SummonGuardian.
//! Count/placement/designation proof only; no invented guardian spell or native visual claim.
use super::*;

fn assert_source_guardian_scatter_point(
    point: Vec3,
    destination: Vec3,
    map: crate::maps::InMap,
    ground: &mut crate::ground::GroundMap,
) {
    // WoW horizontal x/y is Bevy x/z; terrain elevation is independent of disk radius.
    let horizontal_distance = (point.x - destination.x).hypot(point.z - destination.z);
    assert!(
        horizontal_distance <= 5.001,
        "source five-yard scatter: destination={destination:?}, point={point:?}"
    );
    let crate::ground::GroundProbe::Supported(height) = ground.ground_in(map, point) else {
        panic!("source scatter point has no supporting ground: {point:?}");
    };
    assert_eq!(
        point.y, height,
        "scatter height must match projected source ground: destination={destination:?}, point={point:?}"
    );
}

#[test]
fn toyfx39_scarab_scatter_accepts_actual_source_slope_outer_ring() {
    let data = std::fs::read(crate::ground::ground_data_dir().join("terrain/778027.adt")).unwrap();
    let terrain = crate::terrain_height::parse_adt_heights(&data).unwrap();
    let destination = Vec3::new(
        -8949.95,
        terrain.height_at(-8949.95, -132.493).unwrap(),
        132.493,
    );
    // Exact f32 representation of WoW (-8952.673828, -128.299637) on this ADT.
    let point = Vec3::new(
        -8952.674,
        terrain.height_at(-8952.674, -128.29964).unwrap(),
        128.29964,
    );
    assert!(
        point.distance(destination) > 5.001,
        "the actual source slope must expose the incorrect three-dimensional metric"
    );
    let tile = crate::ground::bevy_to_tile(destination.x, destination.z);
    let mut ground = crate::ground::GroundMap::with_terrain(tile, terrain);
    assert_source_guardian_scatter_point(
        point,
        destination,
        crate::maps::InMap::continent(0),
        &mut ground,
    );
}

#[test]
fn toyfx39_scarab_storm_preserves_source_count_scatter_follow_and_lifetime() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Npc, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    let spell = store.get(92146).unwrap();
    assert_eq!(spell.effects[0].base_points, 25.0);
    let (mut h, character) = setup(&directory.path().join("scarab.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
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
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
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
            assert!(
                world
                    .get::<shared::components::UnitFlags>(*actor)
                    .is_some_and(
                        |flags| flags.0 & crate::faction::UNIT_FLAG_PLAYER_CONTROLLED != 0
                    )
            );
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
    {
        let map = *world.get::<crate::maps::InMap>(caster).unwrap();
        let mut ground = world.resource_mut::<crate::ground::GroundMap>();
        for point in &points {
            assert_source_guardian_scatter_point(*point, destination, map, &mut ground);
        }
    }
    assert!(points.iter().any(|point| point.distance(destination) > 0.1));

    // Exercise the existing authoritative follow system, not only its marker.
    h.app
        .init_resource::<lightyear::prelude::ReplicationDirtyQueue>();
    h.app.init_resource::<crate::interest::InterestDirtyQueue>();
    crate::creature_ai::install_follow_fixture(&mut h.app);
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x += 20.0;
    let owner = h.app.world().get::<Position>(caster).unwrap();
    let owner_position = Vec3::new(owner.x, owner.y, owner.z);
    h.advance(10);
    for ((actor, _), before) in actors.iter().zip(&points) {
        let position = h.app.world().get::<Position>(*actor).unwrap();
        let after = Vec3::new(position.x, position.y, position.z);
        assert!(after.distance(owner_position) < before.distance(owner_position));
    }
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

#[test]
fn toyfx51_scarab_level_clamps_source_region_before_guardian_initialization() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Npc, UnitLevel};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    // Actual70391 NorthshireArea9 usesContentTuning73, whose authored range is1..30.
    let bounds = conn
        .query_row(
            "SELECT minlevel,maxlevel FROM content_tuning_level WHERE ID=73",
            [],
            |row| Ok((row.get::<_, u8>(0)?, row.get::<_, u8>(1)?)),
        )
        .unwrap();
    assert_eq!(bounds, (1, 30));
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (owner_level, expected_level, use_creature_level) in
        [(20, 20, false), (80, 30, false), (20, 30, true)]
    {
        let (mut h, character) = setup(&directory.path().join(format!(
            "scarab-level-{owner_level}-{use_creature_level}.redb"
        )));
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(Level(owner_level));
        h.app.insert_resource(store.clone());
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        let mut properties = crate::spell_cast::summon::load_summon_properties(&conn).unwrap();
        if use_creature_level {
            properties.0.get_mut(&2301).unwrap().flags |= 0x100; // Source UseCreatureLevel control.
        }
        h.app.insert_resource(properties);
        learn_effect_toy(&mut h, character, 64881);
        assert_eq!(activate(&mut h, 64881)[0].error, None);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let world = h.app.world_mut();
        let levels: Vec<_> = world
            .query::<(&Npc, &SummonedCreature, &UnitLevel)>()
            .iter(world)
            .filter(|(npc, summon, _)| npc.template_id == 51113 && summon.spell_id == 92146)
            .map(|(_, _, level)| level.0)
            .collect();
        assert!(!levels.is_empty(), "actual source Storm Scarabs must spawn");
        assert!(
            levels.iter().all(|&level| level == expected_level),
            "owner{owner_level}: source region expects{expected_level}, observed{levels:?}"
        );
    }
}
