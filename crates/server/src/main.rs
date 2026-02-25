#[allow(unused)]
mod networking;
#[allow(unused)]
mod persistence;
#[allow(unused)]
mod zones;

use bevy::prelude::*;

/// Server tick rate: 20Hz (50ms per tick).
const TICK_RATE_HZ: f64 = 20.0;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
        )))
        .add_plugins(bevy::log::LogPlugin::default())
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
