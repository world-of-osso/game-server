//! Keystone headless setup must publish native navigation worker results.
use bevy::prelude::*;
use server::navigation::{Goal, MoveRequest, Navigation, Step};
use shared::components::Position;
use std::{path::Path, time::Duration};

#[test]
#[ignore = "requires the canonical Altar navmesh"]
fn headless_keystone_navigation_publishes_worker_routes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_hz(20.0));
    server::test_support::install_navigation(
        &mut app,
        Path::new("/home/osso/Projects/world-of-osso/game-engine/data/navmesh"),
    );
    app.finish();
    app.cleanup();
    app.update();
    let from = Position {
        x: 1614.2029,
        y: 15.618515,
        z: -2126.5059,
    };
    let to = Position {
        x: 1605.9896,
        y: 15.828735,
        z: -2127.083,
    };
    let actor = app
        .world_mut()
        .spawn((from, server::maps::InMap::continent(2993)))
        .id();
    let request = MoveRequest {
        map_id: 2993,
        goal: Goal::Point,
        from,
        to,
        distance: 100.0,
        reach: 0.0,
    };
    let mut reached = false;
    for _ in 0..1000 {
        match app
            .world_mut()
            .resource_mut::<Navigation>()
            .advance(actor, request)
        {
            Step::Moved { reached: true, .. } => {
                reached = true;
                break;
            }
            Step::Pending => {}
            Step::Moved { .. } => panic!("short route did not complete"),
            Step::Blocked => panic!("native route was blocked"),
            Step::Unbaked => panic!("native map was not baked"),
        }
        app.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        reached,
        "the headless setup must publish the real worker result"
    );
}
