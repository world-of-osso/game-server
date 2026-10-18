use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use rusqlite::OpenFlags;
use shared::components::MovementControl;

const COPY: InMap = InMap {
    map_id: 617,
    instance_id: 9,
};

fn source_harness(corner: [f32; 2], height: f32) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[32727, 34709, 57405, 96538, 96539, 58873, 254013, 48018],
        1,
    )
    .unwrap();
    let db = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let data = crate::world_data::load_world_data(&crate::test_data::world_db_path());
    h.app.insert_resource(Catalog::load(&db).unwrap());
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_arena_ground(
            617, corner, height,
        ));
    let los_dir = crate::ground::ground_data_dir().join("los");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&los_dir).unwrap());
    let objects = crate::game_objects::load_game_object_spawns(&db).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        &data,
        crate::zones::MapContent {
            map: COPY,
            difficulty_id: 0,
            dead_bosses: Vec::new(),
        },
        false,
        Some(&mut crate::test_data::ground_map()),
    );
    crate::game_objects::spawn_map_game_objects(&mut commands, &objects, COPY, 0, None);
    queue.apply(h.app.world_mut());
    h.app.insert_resource(data);
    h.app.world_mut().entity_mut(h.caster).insert(COPY);
    assert_eq!(
        spouts_on(h.app.world_mut(), COPY, WaterSpout::Pipe).len(),
        2
    );
    assert_eq!(
        spouts_on(h.app.world_mut(), COPY, WaterSpout::Center).len(),
        1
    );
    h
}

fn place_player(h: &mut Harness, point: Vec3) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::game_objects::bevy_position(point));
}

fn object_collision(h: &mut Harness, entry: u32) -> bool {
    h.app
        .world_mut()
        .query::<(&InMap, &GameObjectInfo, &GameObjectCollision)>()
        .iter(h.app.world())
        .find(|(map, info, _)| **map == COPY && info.entry == entry)
        .map(|(_, _, collision)| collision.0)
        .unwrap()
}

#[test]
fn arena_dalaran_static_water_collision_has_no_unsupported_animation_contract() {
    let mut h = source_harness([1333.3334, 833.3334], 7.19796);
    let mut clock = Clock::default();
    for (elapsed, active) in [(0.0, false), (30.0, false), (35.0, true), (65.0, false)] {
        update_running(h.app.world_mut(), COPY, elapsed, &mut clock);
        let mut query = h.app.world_mut().query::<(
            &InMap,
            &GameObjectInfo,
            &GameObjectCollision,
            Option<&GameObjectAnimation>,
        )>();
        let (_, _, collision, animation) = query
            .iter(h.app.world())
            .find(|(map, info, ..)| **map == COPY && info.entry == WATER_COLLISION)
            .unwrap();
        assert_eq!(collision.0, active);
        assert_eq!(
            animation, None,
            "display8592/FDID252201 is a static collision mesh, not the animated waterfall"
        );
    }
}

#[test]
fn arena_dalaran_source_spouts_water_cycle_center_push_and_end_cleanup_are_real() {
    let mut h = source_harness([1333.3334, 833.3334], 7.19796);
    let center = spouts_on(h.app.world_mut(), COPY, WaterSpout::Center)[0];
    let position =
        crate::game_objects::wow_position(h.app.world().get::<Position>(center).unwrap());
    place_player(&mut h, position + Vec3::X);
    let mut clock = Clock::default();
    update_running(h.app.world_mut(), COPY, 0.0, &mut clock);
    update_running(h.app.world_mut(), COPY, 5.999, &mut clock);
    for pipe in spouts_on(h.app.world_mut(), COPY, WaterSpout::Pipe) {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            pipe,
            PIPE_SEARCH_AURA
        ));
    }
    update_running(h.app.world_mut(), COPY, 6.0, &mut clock);
    for pipe in spouts_on(h.app.world_mut(), COPY, WaterSpout::Pipe) {
        assert!(crate::auras::has_aura(
            h.app.world(),
            pipe,
            PIPE_SEARCH_AURA
        ));
    }
    update_running(h.app.world_mut(), COPY, 29.999, &mut clock);
    assert!(!object_collision(&mut h, WATERFALL) && !object_collision(&mut h, WATER_COLLISION));
    update_running(h.app.world_mut(), COPY, 30.0, &mut clock);
    assert!(object_collision(&mut h, WATERFALL));
    assert!(!object_collision(&mut h, WATER_COLLISION));
    update_running(h.app.world_mut(), COPY, 35.0, &mut clock);
    assert!(object_collision(&mut h, WATER_COLLISION));
    let initial = *h.app.world().get::<Position>(h.caster).unwrap();
    update_running(h.app.world_mut(), COPY, 36.499, &mut clock);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    update_running(h.app.world_mut(), COPY, 36.5, &mut clock);
    h.advance(1);
    let moved = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    assert!(
        (moved.x - initial.x - 1.0).abs() < 0.001,
        "source center200×0.1 horizontal speed"
    );
    assert!(moved.y > initial.y);
    h.advance(10);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    update_running(h.app.world_mut(), COPY, 65.0, &mut clock);
    assert!(!object_collision(&mut h, WATERFALL) && !object_collision(&mut h, WATER_COLLISION));
    update_running(h.app.world_mut(), COPY, 94.999, &mut clock);
    assert!(!object_collision(&mut h, WATERFALL));
    update_running(h.app.world_mut(), COPY, 95.0, &mut clock);
    assert!(object_collision(&mut h, WATERFALL));
    stop_hazards(h.app.world_mut(), COPY);
    for pipe in spouts_on(h.app.world_mut(), COPY, WaterSpout::Pipe) {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            pipe,
            PIPE_SEARCH_AURA
        ));
    }
    assert!(!object_collision(&mut h, WATERFALL) && !object_collision(&mut h, WATER_COLLISION));
}

#[test]
fn arena_dalaran_source_pipe_force_cast_uses_thirty_nineteen_impulse_and_periodic_search() {
    let mut h = source_harness([1400.0, 866.6667], 16.0872);
    let pipes = spouts_on(h.app.world_mut(), COPY, WaterSpout::Pipe);
    let pipe = *pipes
        .iter()
        .max_by(|a, b| {
            h.app
                .world()
                .get::<Position>(**a)
                .unwrap()
                .x
                .total_cmp(&h.app.world().get::<Position>(**b).unwrap().x)
        })
        .unwrap();
    let pipe_origin = *h.app.world().get::<Position>(pipe).unwrap();
    let point = crate::game_objects::wow_position(&pipe_origin);
    place_player(&mut h, point - Vec3::X);
    let initial = *h.app.world().get::<Position>(h.caster).unwrap();
    let mut clock = Clock::default();
    update_running(h.app.world_mut(), COPY, 5.999, &mut clock);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    update_running(h.app.world_mut(), COPY, 6.0, &mut clock);
    h.advance(1);
    let pushed = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(pipe).unwrap(),
        pipe_origin,
        "source ExcludeCaster attribute prevents the spout from knocking itself away"
    );
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    assert!(
        (pushed.x - initial.x + 1.5).abs() < 0.001,
        "pinned pipe script30yd/s"
    );
    let expected_y = initial.y + 19.0 * 0.05 - 0.5 * shared::skyriding::GRAVITY * 0.05 * 0.05;
    assert!(
        (pushed.y - expected_y).abs() < 0.001,
        "pinned pipe script19yd/s"
    );
    h.advance(45);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    place_player(&mut h, point - Vec3::X);
    h.advance(15); // actual source96539 3000ms aura tick ->96538 ->57405 ->script61698.
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled,
        "real periodic search re-flushes returned player"
    );
    stop_hazards(h.app.world_mut(), COPY);
}

#[test]
fn arena_dalaran_authored_pipe_entry_exit_clear_circle_without_claiming_pet_relocation() {
    let mut h = source_harness([1300.0, 833.3334], 21.4662);
    let trigger = h
        .app
        .world()
        .resource::<Catalog>()
        .triggers
        .iter()
        .find(|trigger| trigger.id == 5347)
        .unwrap()
        .clone();
    place_player(&mut h, trigger.center);
    let apply_circle = |h: &mut Harness| {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: DEMONIC_CIRCLE,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        )
    };
    let mut clock = Clock::default();
    apply_circle(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        DEMONIC_CIRCLE
    ));
    update_pipe_presence(h.app.world_mut(), COPY, &mut clock);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        DEMONIC_CIRCLE
    ));
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, PET_TELEPORT),
        "source254013 marker, not pet relocation proof"
    );
    apply_circle(&mut h);
    place_player(&mut h, trigger.center + Vec3::X * 20.0);
    update_pipe_presence(h.app.world_mut(), COPY, &mut clock);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        DEMONIC_CIRCLE
    ));
    assert!(clock.pipe_presence.is_empty());
}
