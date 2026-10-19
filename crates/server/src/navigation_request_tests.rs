use super::*;
use std::time::{Duration, Instant};

fn fixture() -> (Navigation, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("0")).unwrap();
    std::fs::write(
        dir.path().join("0/32_31.nav"),
        super::blocking_io_tests::baked_floor(),
    )
    .unwrap();
    (Navigation::new(dir.path()), dir)
}

fn movement(from: Position, x: f32, z: f32) -> MoveRequest {
    MoveRequest {
        map_id: 0,
        goal: Goal::Point,
        from,
        to: Position { x, y: 0.2, z },
        distance: 1.0,
        reach: 0.0,
    }
}

fn start() -> Position {
    Position {
        x: 100.0,
        y: 0.2,
        z: 100.0,
    }
}

#[test]
fn navigation_pending_is_not_unbaked_or_blocked_and_holds_initial_position() {
    let (mut nav, _dir) = fixture();
    let entity = Entity::from_bits(41);
    let request = movement(start(), 140.0, 100.0);
    assert!(matches!(nav.advance(entity, request), Step::Pending));
    assert!(nav.route_pending(entity));
    nav.drain_until_idle();
    assert!(!nav.route_pending(entity));
    let Step::Moved { position, .. } = nav.advance(entity, request) else {
        panic!("ready path must move")
    };
    assert!(position.x > request.from.x);
    assert!((position.z - request.from.z).abs() < 0.01);
}

#[test]
fn navigation_latest_request_never_applies_the_old_opposite_route() {
    let (mut nav, _dir) = fixture();
    let entity = Entity::from_bits(41);
    assert!(matches!(
        nav.advance(entity, movement(start(), 80.0, 100.0)),
        Step::Pending
    ));
    assert!(matches!(
        nav.advance(entity, movement(start(), 140.0, 100.0)),
        Step::Pending
    ));
    nav.drain_until_idle();
    let Step::Moved { position, .. } = nav.advance(entity, movement(start(), 140.0, 100.0)) else {
        panic!("latest route must publish")
    };
    assert!(
        position.x > 100.0,
        "stale opposite route moved to {position:?}"
    );
    assert_eq!(nav.applied.len(), 1);
}

#[test]
fn navigation_old_route_continues_pending_and_new_path_starts_at_latest_feet() {
    let (mut nav, _dir) = fixture();
    let entity = Entity::from_bits(41);
    let original = movement(start(), 140.0, 100.0);
    let _ = nav.advance(entity, original);
    nav.drain_until_idle();
    let mut at = start();
    for _ in 0..4 {
        let Step::Moved { position, reached } = nav.advance(entity, movement(at, 100.0, 140.0))
        else {
            panic!("existing route must continue while new one is pending")
        };
        assert!(position.x > at.x);
        assert!((position.z - 100.0).abs() < 0.01);
        assert!(!reached, "old route cannot announce arrival for new intent");
        at = position;
    }
    nav.drain_until_idle();
    let Step::Moved { position, .. } = nav.advance(entity, movement(at, 100.0, 140.0)) else {
        panic!("new path must publish from current feet")
    };
    assert!(
        position.z > at.z,
        "path backtracked to its obsolete start: {at:?}->{position:?}"
    );
    assert!(point(position).distance(point(at)) <= 1.01);
}

#[test]
fn navigation_completion_sets_apply_in_entity_request_order() {
    let (mut nav, _dir) = fixture();
    for actor in [43, 41, 42] {
        let entity = Entity::from_bits(actor);
        if actor == 42 {
            let _ = nav.query_deferred(
                entity,
                0,
                Goal::Approach(99),
                ::navigation::worker::Query::Path {
                    from: point(start()),
                    to: Vec3::new(140.0, 0.2, 100.0),
                },
            );
        } else {
            let _ = nav.advance(entity, movement(start(), 140.0, 100.0));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut count = 0;
    while !nav.worker.is_idle() {
        nav.publish_ready_routes(None);
        assert!(
            nav.applied.windows(2).all(|pair| {
                (pair[0].0.to_bits(), pair[0].1) < (pair[1].0.to_bits(), pair[1].1)
            })
        );
        count += nav.applied.len();
        assert!(Instant::now() < deadline, "paths never published");
        std::thread::yield_now();
    }
    assert_eq!(count, 3);
}

#[test]
fn navigation_probe_and_route_share_entity_without_starving_or_replacing_progress() {
    let (mut nav, _dir) = fixture();
    let entity = Entity::from_bits(41);
    let _ = nav.advance(entity, movement(start(), 140.0, 100.0));
    nav.drain_until_idle();
    let probe = ::navigation::worker::Query::Path {
        from: point(start()),
        to: Vec3::new(120.0, 0.2, 100.0),
    };
    assert!(
        nav.query_deferred(entity, 0, Goal::Approach(99), probe.clone())
            .is_none()
    );
    let Step::Moved {
        position: at,
        reached,
    } = nav.advance(entity, movement(start(), 100.0, 140.0))
    else {
        panic!("raw probe must retain published route progress");
    };
    assert!(at.x > 100.0);
    assert!(!reached);
    nav.drain_until_idle();
    assert!(
        matches!(
            nav.query_deferred(entity, 0, Goal::Approach(99), probe),
            Some(Ok(Some(::navigation::worker::Output::Path(_))))
        ),
        "route resubmission must not replace the probe"
    );
    let Step::Moved { position: at, .. } = nav.advance(entity, movement(at, 100.0, 140.0)) else {
        panic!("old route continues during the new route request");
    };
    nav.drain_until_idle();
    let Step::Moved { position, .. } = nav.advance(entity, movement(at, 100.0, 140.0)) else {
        panic!("route must resume after probe");
    };
    assert!(position.z > at.z);
}

#[test]
fn navigation_chase_cancel_does_not_strand_an_unrelated_probe() {
    let (mut nav, _dir) = fixture();
    let entity = Entity::from_bits(41);
    let query = ::navigation::worker::Query::Collision {
        from: point(start()),
        to: Vec3::new(120.0, 0.2, 100.0),
    };
    assert!(
        nav.query_deferred(entity, 0, Goal::FearCollision, query.clone())
            .is_none()
    );
    nav.cancel_route(entity);
    nav.drain_until_idle();
    assert!(matches!(
        nav.query_deferred(entity, 0, Goal::FearCollision, query),
        Some(Ok(Some(::navigation::worker::Output::Collision(_))))
    ));
}

#[test]
fn navigation_player_probe_old_map_is_invalidated_before_publication() {
    let (nav, _dir) = fixture();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NavigationPlugin));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.update();
    app.insert_resource(nav);
    let actor = app
        .world_mut()
        .spawn((start(), crate::maps::InMap::continent(0)))
        .id();
    let _ = app.world_mut().resource_mut::<Navigation>().query_deferred(
        actor,
        0,
        Goal::FearCollision,
        ::navigation::worker::Query::Collision {
            from: point(start()),
            to: Vec3::new(120.0, 0.2, 100.0),
        },
    );
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::maps::InMap::continent(1));
    app.update();
    let nav = app.world().resource::<Navigation>();
    assert!(!nav.route_pending(actor));
    assert!(
        nav.applied.is_empty(),
        "non-creature old-map result must not publish"
    );
}

#[test]
fn navigation_teleport_map_change_cancels_an_inflight_old_map_result() {
    let (nav, _dir) = fixture();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NavigationPlugin));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.update();
    app.insert_resource(nav);
    let actor = app
        .world_mut()
        .spawn((
            start(),
            crate::creature_ai::CreatureState::Idle,
            crate::maps::InMap::continent(0),
        ))
        .id();
    let _ = app
        .world_mut()
        .resource_mut::<Navigation>()
        .advance(actor, movement(start(), 140.0, 100.0));
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::maps::InMap::continent(1));
    app.update();
    let nav = app.world().resource::<Navigation>();
    assert!(!nav.route_pending(actor));
    assert!(nav.routes.get(&actor).is_none());
    assert!(
        nav.applied.is_empty(),
        "old map result was applied before invalidation"
    );
}
