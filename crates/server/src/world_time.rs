//! The game time a player receives on entering the world (TrinityCore
//! `Player::SendInitialPacketsBeforeAddToMap`, Player.cpp:24988-24996:
//! `SMSG_LOGIN_SET_TIME_SPEED` with `GameTime::GetWowTime` and `TimeSpeed` 0.01666667). The
//! game time is the realm's local wall clock (`GameTime::UpdateGameTimers`: UTC plus the
//! system zone offset); the client's sky follows it.

use bevy::prelude::*;
use chrono::{DateTime, TimeZone};
use lightyear::prelude::*;
use shared::protocol::{GAME_TIME_SPEED, LoginSetTimeSpeed, WorldTimeChannel};

use crate::auth::InWorld;

/// `GameTime::GetWowTime` of `now`: its local date and time, counted in seconds as if
/// they were UTC.
pub fn wow_time_seconds<Tz: TimeZone>(now: &DateTime<Tz>) -> i64 {
    now.naive_local().and_utc().timestamp()
}

/// Every connection that entered the world this frame gets the game time.
pub fn send_login_time_speed(
    entered: Query<Entity, Added<InWorld>>,
    mut senders: Query<&mut MessageSender<LoginSetTimeSpeed>>,
) {
    let message = LoginSetTimeSpeed {
        game_time_seconds: wow_time_seconds(&chrono::Local::now()),
        new_speed: GAME_TIME_SPEED,
    };
    for connection in &entered {
        match senders.get_mut(connection) {
            Ok(mut sender) => sender.send::<WorldTimeChannel>(message),
            Err(_) => warn!("No MessageSender<LoginSetTimeSpeed> on connection {connection:?}"),
        }
    }
}

pub struct WorldTimePlugin;

impl Plugin for WorldTimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_login_time_speed);
    }
}

#[cfg(test)]
#[path = "world_time_tests.rs"]
mod tests;
