use super::*;
use shared::components::Health;
use shared::components::ThreatTable;

#[test]
fn chase_reaches_target_over_multiple_ticks() {
    let mut app = ai_test_app();

    let target = app
        .world_mut()
        .spawn(Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        })
        .id();

    let chaser = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureState::Pursuing {
                target: target.to_bits(),
            },
        ))
        .id();

    for _ in 0..200 {
        app.update();
        let pos = *app.world().entity(chaser).get::<Position>().unwrap();
        if (pos.x - 8.0).abs() < 0.01 {
            break;
        }
    }

    let pos = *app.world().entity(chaser).get::<Position>().unwrap();
    assert!(
        (pos.x - 8.0).abs() < 0.01,
        "chaser stops CHASE_REACH (2 yd) short of the target: {pos:?}"
    );
}

#[test]
fn chase_then_leash_evade() {
    let mut app = ai_test_app();

    let target = app
        .world_mut()
        .spawn(Position {
            x: 60.0,
            y: 0.0,
            z: 0.0,
        })
        .id();

    let mut threat = ThreatTable::default();
    threat.add_damage_threat(target.to_bits(), 100.0, 1.0);

    let chaser = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureState::Pursuing {
                target: target.to_bits(),
            },
            threat,
            Health {
                current: 50.0,
                max: 100.0,
            },
        ))
        .id();

    for _ in 0..1000 {
        app.update();
        let state = app.world().entity(chaser).get::<CreatureState>().unwrap();
        if *state == CreatureState::Returning {
            break;
        }
    }

    let state = app.world().entity(chaser).get::<CreatureState>().unwrap();
    assert_eq!(*state, CreatureState::Returning);

    let threat = app.world().entity(chaser).get::<ThreatTable>().unwrap();
    assert!(threat.is_empty(), "threat should be cleared on evade");

    let health = app.world().entity(chaser).get::<Health>().unwrap();
    assert_eq!(health.current, 50.0, "evade leaves health to regeneration");
}

#[test]
fn full_pipeline_pursue_evade_return_idle() {
    let mut app = ai_test_app();

    let target = app
        .world_mut()
        .spawn(Position {
            x: 80.0,
            y: 0.0,
            z: 0.0,
        })
        .id();

    let chaser = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureState::Pursuing {
                target: target.to_bits(),
            },
            Health {
                current: 60.0,
                max: 100.0,
            },
        ))
        .id();

    let mut evaded = false;
    for _ in 0..1000 {
        app.update();
        if *app.world().entity(chaser).get::<CreatureState>().unwrap() == CreatureState::Returning {
            evaded = true;
            break;
        }
    }
    assert!(evaded, "creature should have evaded after exceeding leash");

    let evade_x = app.world().entity(chaser).get::<Position>().unwrap().x;
    assert!(
        evade_x > 39.0,
        "evade pos should be near leash boundary: x={evade_x}"
    );

    let mut returned = false;
    for _ in 0..2000 {
        app.update();
        if *app.world().entity(chaser).get::<CreatureState>().unwrap() == CreatureState::Idle {
            returned = true;
            break;
        }
    }
    assert!(returned, "creature should have returned to idle at spawn");

    let final_pos = *app.world().entity(chaser).get::<Position>().unwrap();
    assert!(
        final_pos.x.abs() < 0.1,
        "creature should be at spawn x=0, got x={}",
        final_pos.x
    );
}
