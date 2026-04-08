mod admin_handlers;
mod admin_ipc;
mod admin_types;
mod auction_house;
mod auth;
mod auth_character;
mod collection;
mod combat;
mod creature_ai;
mod currency;
mod duel;
mod inspect;
mod interest;
mod messaging;
mod networking;
mod persistence;
mod persistence_accounts;
mod persistence_character;
mod persistence_types;
mod profession;
mod reputation;
mod spell_db;
mod stats;
mod talent;
mod terrain_height;
mod trade;
mod world_data;
mod zones;

use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;
use std::time::Duration;

/// Server tick rate: 20Hz (50ms per tick).
const TICK_RATE_HZ: f64 = 20.0;

fn main() {
    let mut app = App::new();
    add_runtime_plugins(&mut app);
    add_game_plugins(&mut app);
    register_server_resources(&mut app);
    register_server_systems(&mut app);
    app.run();
}

fn add_runtime_plugins(app: &mut App) {
    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
        )),
    )
    .add_plugins(bevy::log::LogPlugin::default())
    .add_plugins(ServerPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
    });
}

fn add_game_plugins(app: &mut App) {
    app.add_plugins(shared::ProtocolPlugin)
        .add_plugins(persistence::PersistencePlugin)
        .add_plugins(auction_house::AuctionHousePlugin)
        .add_plugins(collection::CollectionPlugin)
        .add_plugins(currency::CurrencyPlugin)
        .add_plugins(profession::ProfessionPlugin)
        .add_plugins(reputation::ReputationPlugin)
        .add_plugins(duel::DuelPlugin)
        .add_plugins(inspect::InspectPlugin)
        .add_plugins(talent::TalentPlugin)
        .add_plugins(trade::TradePlugin)
        .add_plugins(networking::ServerNetworkPlugin)
        .add_plugins(world_data::WorldDataPlugin)
        .add_plugins(terrain_height::TerrainPlugin)
        .add_plugins(zones::ZonePlugin)
        .add_plugins(creature_ai::CreatureAiPlugin)
        .add_plugins(stats::StatsPlugin)
        .add_plugins(combat::CombatPlugin)
        .add_plugins(spell_db::SpellDatabasePlugin)
        .add_plugins(interest::InterestPlugin)
        .add_plugins(admin_ipc::AdminIpcPlugin);
}

fn register_server_resources(app: &mut App) {
    app.init_resource::<shared::ticket::TicketManager>()
        .init_resource::<shared::motd::Autobroadcast>();
}

fn register_server_systems(app: &mut App) {
    app.add_systems(Startup, on_startup)
        // Motd loaded from redb in PersistencePlugin startup
        .add_systems(FixedUpdate, tick_log);
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
