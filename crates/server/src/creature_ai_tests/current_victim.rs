//! Current-victim stop controls over the same real Stockade corridor as the RED.
use super::*;
use crate::navigation::{Goal, MoveRequest, Step};
use bevy::ecs::system::RunSystemOnce;
use shared::components::ModelDisplay;

pub(super) fn fixture() -> (App, Entity, Entity, Position) {
    let mut app = stockade_app();
    let data =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    app.insert_resource(crate::navigation::Navigation::new(&data.join("navmesh")));
    app.insert_resource(crate::line_of_sight::LineOfSight::load(&data.join("los")).unwrap());
    let from = Position {
        x: 121.1,
        y: -25.6,
        z: -3.9,
    };
    let cached = Position {
        x: 123.6,
        y: -25.61,
        z: -4.14,
    };
    let map = crate::maps::InMap::continent(STOCKADE);
    let victim = app
        .world_mut()
        .spawn((
            cached,
            Health {
                current: 100.0,
                max: 100.0,
            },
            map,
            ModelDisplay { display_id: 35076 },
        ))
        .id();
    let chaser = app
        .world_mut()
        .spawn((
            from,
            SpawnPoint {
                x: from.x,
                y: from.y,
                z: from.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.14286,
            },
            CreatureState::Pursuing {
                target: victim.to_bits(),
            },
            crate::networking::CombatTarget(Some(victim)),
            Health {
                current: 100.0,
                max: 100.0,
            },
            map,
            ModelDisplay { display_id: 113753 },
        ))
        .id();
    let seeded = app
        .world_mut()
        .resource_mut::<crate::navigation::Navigation>()
        .advance(
            chaser,
            MoveRequest {
                map_id: STOCKADE,
                goal: Goal::Chase(victim.to_bits()),
                from,
                to: cached,
                distance: 0.0,
                reach: CHASE_REACH,
            },
        );
    assert!(
        matches!(seeded, Step::Moved { .. }),
        "real baked route required"
    );
    *app.world_mut().get_mut::<Position>(victim).unwrap() = Position {
        x: 122.66,
        y: -25.61,
        z: -3.91,
    };
    (app, victim, chaser, from)
}

#[test]
#[ignore = "requires real Stockade navigation, ground and server LOS bake"]
fn current_victim_run24_height_difference_moves_into_real_three_dimensional_reach() {
    let (mut app, victim, chaser, _) = fixture();
    let from = Position {
        x: 109.70202,
        y: -25.606234,
        z: 2.872613,
    };
    let target = Position {
        x: 107.715996,
        y: -26.124136,
        z: 3.108669,
    };
    *app.world_mut().get_mut::<Position>(chaser).unwrap() = from;
    *app.world_mut().get_mut::<Position>(victim).unwrap() = target;
    app.world_mut()
        .entity_mut(chaser)
        .insert(crate::followers::FollowerReach(CHASE_REACH));
    let point = |p: Position| Vec3::new(p.x, p.y, p.z);
    let gap = point(from) - point(target);
    assert!(gap.x.hypot(gap.z) <= CHASE_REACH + 0.0001);
    assert!(gap.length() > CHASE_REACH);
    let map = crate::maps::InMap::continent(STOCKADE);
    let mut bodies = app.world_mut().query::<crate::line_of_sight::BodyParts>();
    let world = app.world();
    assert!(crate::line_of_sight::unit_sees(
        Some(world.resource::<crate::line_of_sight::LineOfSight>()),
        map,
        bodies.get(world, chaser).unwrap(),
        bodies.get(world, victim).unwrap()
    ));
    let data =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    let mut los = los::LosMap::new(&data.join("los/34"));
    let mut ground = stockade_ground();
    let mut last = from;
    for _ in 0..40 {
        step(&mut app);
        let next = *app.world().get::<Position>(chaser).unwrap();
        clear_step(&mut los, last, next);
        let floor = floor_under(&mut ground, next).expect("real support under chase feet");
        assert!((next.y - floor).abs() < 0.05, "feet {next:?} floor{floor}");
        last = next;
        if point(next).distance_squared(point(target)) <= CHASE_REACH * CHASE_REACH {
            break;
        }
    }
    assert!(
        point(last).distance(point(from)) > 0.001,
        "live victim outside3Dreach must not leave Garrick motionless in Pursuing"
    );
    assert!(
        point(last).distance_squared(point(target)) <= CHASE_REACH * CHASE_REACH,
        "grounded chase must reach the unchanged3Dsphere: {last:?} target{target:?}"
    );
    step(&mut app);
    step(&mut app);
    assert_eq!(
        *app.world().get::<Position>(chaser).unwrap(),
        last,
        "after real reach and LOS, the existing stop must hold without jitter"
    );
}

#[test]
fn current_victim_unbaked_ground_step_uses_the_same_three_dimensional_sphere() {
    let from = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let victim = Position {
        x: 2.0,
        y: -0.5,
        z: 0.0,
    };
    let after = chase_step(from, &victim, 0.4, CHASE_REACH);
    let gap = Vec3::new(after.x - victim.x, after.y - victim.y, after.z - victim.z);
    assert!(after.x > from.x, "outside3Dsphere requires ground progress");
    assert!(gap.length_squared() <= CHASE_REACH * CHASE_REACH);
    assert_eq!(
        after.y, from.y,
        "unbaked ground step cannot fly toward target height"
    );
    let flat = Position { y: 0.0, ..victim };
    assert_eq!(chase_step(from, &flat, 0.4, CHASE_REACH), from);
}

fn step(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(50));
    app.world_mut().run_system_once(chase_system).unwrap();
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_restarts_toward_a_moving_living_target_after_stop() {
    let (mut app, victim, chaser, from) = fixture();
    step(&mut app);
    assert_eq!(*app.world().get::<Position>(chaser).unwrap(), from);
    *app.world_mut().get_mut::<Position>(victim).unwrap() = Position {
        x: 126.42,
        y: -25.61,
        z: -4.14,
    };
    step(&mut app);
    let after = *app.world().get::<Position>(chaser).unwrap();
    assert!(
        after.x > from.x,
        "an out-of-reach live victim must still be chased"
    );
    assert!(
        (after.x - from.x).hypot(after.z - from.z) <= 0.401,
        "ordinary authored speed retained"
    );
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_dead_health_cancels_motion_without_chasing_a_corpse() {
    let (mut app, victim, chaser, from) = fixture();
    *app.world_mut().get_mut::<Position>(victim).unwrap() = Position {
        x: 130.0,
        y: -25.61,
        z: -4.14,
    };
    app.world_mut().get_mut::<Health>(victim).unwrap().current = 0.0;
    app.world_mut()
        .entity_mut(chaser)
        .insert(CreatureMotion::Run);
    step(&mut app);
    assert_eq!(*app.world().get::<Position>(chaser).unwrap(), from);
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_despawn_cancels_motion() {
    let (mut app, victim, chaser, from) = fixture();
    app.world_mut()
        .entity_mut(chaser)
        .insert(CreatureMotion::Run);
    app.world_mut().despawn(victim);
    step(&mut app);
    assert_eq!(*app.world().get::<Position>(chaser).unwrap(), from);
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_mismatch_waits_for_real_reselection_then_chases_live_alternate() {
    let (mut app, victim, chaser, from) = fixture();
    let map = crate::maps::InMap::continent(STOCKADE);
    let alternate = app
        .world_mut()
        .spawn((
            Position {
                x: 126.42,
                y: -25.61,
                z: -4.14,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            map,
            ModelDisplay { display_id: 35076 },
        ))
        .id();
    app.world_mut()
        .entity_mut(chaser)
        .insert(crate::networking::CombatTarget(Some(alternate)));
    step(&mut app);
    assert_eq!(
        *app.world().get::<Position>(chaser).unwrap(),
        from,
        "old generator must stop after victim changes"
    );
    app.world_mut()
        .entity_mut(chaser)
        .insert(CreatureState::Pursuing {
            target: alternate.to_bits(),
        });
    app.world_mut().get_mut::<Health>(victim).unwrap().current = 0.0;
    step(&mut app);
    assert!(
        app.world().get::<Position>(chaser).unwrap().x > from.x,
        "valid alternate defence remains possible"
    );
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_within_two_yards_but_blocked_by_server_los_does_not_stop() {
    let (mut app, victim, chaser, from) = fixture();
    // A closed dynamic GO wall blocks all server body rays without changing navmesh.
    let x = 121.8;
    let vertices = vec![
        Vec3::new(x, -30.0, -10.0),
        Vec3::new(x, -20.0, -10.0),
        Vec3::new(x, -20.0, 0.0),
        Vec3::new(x, -30.0, 0.0),
    ];
    let wall = shared::los::LosTile::build(0, 0, vertices, vec![[0, 1, 2], [0, 2, 3]]);
    let map = crate::maps::InMap::continent(STOCKADE);
    let mut hulls = std::collections::HashMap::new();
    hulls.insert(map, vec![std::sync::Arc::new(wall)]);
    app.world_mut()
        .resource_mut::<crate::line_of_sight::LineOfSight>()
        .set_live_hulls(hulls);
    let mut bodies = app.world_mut().query::<crate::line_of_sight::BodyParts>();
    let world = app.world();
    assert!(!crate::line_of_sight::unit_sees(
        Some(world.resource::<crate::line_of_sight::LineOfSight>()),
        map,
        bodies.get(world, chaser).unwrap(),
        bodies.get(world, victim).unwrap()
    ));
    step(&mut app);
    let after = app.world().get::<Position>(chaser).unwrap();
    assert!(
        after.x > from.x,
        "blocked LOS must not be treated as a valid in-range stop"
    );
    assert!(
        after.x < x,
        "this bounded path step must not cross the wall"
    );
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_wrong_copy_does_not_advance_a_cached_route() {
    let (mut app, victim, chaser, from) = fixture();
    app.world_mut()
        .entity_mut(victim)
        .insert(crate::maps::InMap::continent(0));
    step(&mut app);
    assert_eq!(*app.world().get::<Position>(chaser).unwrap(), from);
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_ghost_with_positive_health_is_not_chased() {
    let (mut app, victim, chaser, from) = fixture();
    app.world_mut()
        .entity_mut(victim)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Ghost,
            ..Default::default()
        });
    step(&mut app);
    assert_eq!(*app.world().get::<Position>(chaser).unwrap(), from);
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_smaller_existing_reach_is_not_increased_to_two_yards() {
    let (mut app, _, chaser, from) = fixture();
    app.world_mut()
        .entity_mut(chaser)
        .insert(crate::followers::FollowerReach(0.5));
    step(&mut app);
    assert!(app.world().get::<Position>(chaser).unwrap().x > from.x);
}

#[test]
#[ignore = "requires real Stockade navigation bake"]
fn current_victim_missing_server_los_is_not_a_positive_in_range_stop() {
    let (mut app, _, chaser, from) = fixture();
    app.world_mut()
        .remove_resource::<crate::line_of_sight::LineOfSight>();
    step(&mut app);
    assert!(
        app.world().get::<Position>(chaser).unwrap().x > from.x,
        "no LOS resource cannot certify the new in-range pre-stop"
    );
}
