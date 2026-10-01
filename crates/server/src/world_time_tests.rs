use std::time::Duration;

use chrono::{FixedOffset, TimeZone};
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, MessageReceiver, PeerId, RemoteId};
use shared::protocol::{GAME_TIME_SPEED, LoginSetTimeSpeed};

use super::*;

#[test]
fn wow_time_is_the_local_wall_clock_counted_as_utc() {
    // 20:30:20 UTC on a UTC-5 realm is 15:30:20 local: 2026-10-01T15:30:20 as UTC.
    let zone = FixedOffset::west_opt(5 * 3600).unwrap();
    let now = chrono::Utc
        .with_ymd_and_hms(2026, 10, 1, 20, 30, 20)
        .unwrap()
        .with_timezone(&zone);
    assert_eq!(wow_time_seconds(&now), 1_790_868_620);
    let message = LoginSetTimeSpeed {
        game_time_seconds: wow_time_seconds(&now),
        new_speed: GAME_TIME_SPEED,
    };
    assert_eq!(message.second_of_day(), 15 * 3600 + 30 * 60 + 20);
}

#[test]
fn entering_the_world_sends_the_local_game_time_once() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<LoginSetTimeSpeed>();
    app.add_plugins(WorldTimePlugin);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(7)),
            Connected,
            HostClient { buffer: Vec::new() },
            MessageSender::<LoginSetTimeSpeed>::default(),
            MessageReceiver::<LoginSetTimeSpeed>::default(),
        ))
        .id();
    // A receiver holds the messages delivered in the frame's `Last`.
    let received = |app: &mut App| -> Vec<LoginSetTimeSpeed> {
        app.world_mut()
            .get_mut::<MessageReceiver<LoginSetTimeSpeed>>(connection)
            .unwrap()
            .receive()
            .collect()
    };
    app.update();
    assert_eq!(received(&mut app), []);

    let before = wow_time_seconds(&chrono::Local::now());
    app.world_mut()
        .entity_mut(connection)
        .insert(InWorld { character_id: 42 });
    app.update();
    let after = wow_time_seconds(&chrono::Local::now());
    let messages = received(&mut app);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        (before..=after).contains(&messages[0].game_time_seconds),
        "{messages:?}"
    );
    assert_eq!(messages[0].new_speed, GAME_TIME_SPEED);

    app.update();
    assert_eq!(received(&mut app), []);
}
