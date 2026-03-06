mod admin_ipc;
mod auction_house;
mod auth;
mod combat;
mod creature_ai;
mod interest;
mod networking;
mod persistence;
mod terrain_height;
mod world_data;
mod zones;

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use std::time::Duration;

/// Server tick rate: 20Hz (50ms per tick).
const TICK_RATE_HZ: f64 = 20.0;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        .add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
        })
        .add_plugins(shared::ProtocolPlugin)
        .add_plugins(persistence::PersistencePlugin)
        .add_plugins(auction_house::AuctionHousePlugin)
        .add_plugins(networking::ServerNetworkPlugin)
        .add_plugins(world_data::WorldDataPlugin)
        .add_plugins(terrain_height::TerrainPlugin)
        .add_plugins(zones::ZonePlugin)
        .add_plugins(creature_ai::CreatureAiPlugin)
        .add_plugins(combat::CombatPlugin)
        .add_plugins(interest::InterestPlugin)
        .add_plugins(admin_ipc::AdminIpcPlugin)
        .add_systems(Startup, on_startup)
        .add_systems(FixedUpdate, tick_log)
        .run();
}

fn on_startup() {
    info!("Game server starting at {TICK_RATE_HZ}Hz tick rate");
}

fn tick_log(time: Res<Time>, mut tick_count: Local<u64>) {
    *tick_count += 1;
    if (*tick_count).is_multiple_of(TICK_RATE_HZ as u64 * 10) {
        info!("Tick {} (uptime: {:.0}s)", *tick_count, time.elapsed_secs());
    }
}
