use bevy::prelude::*;

/// Default server bind address.
pub const DEFAULT_BIND: &str = "0.0.0.0:5000";

/// Server networking plugin — will configure lightyear ServerPlugin.
pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_networking);
    }
}

fn setup_networking() {
    info!("Networking: bind address {DEFAULT_BIND}");
    // TODO: configure lightyear ServerPlugin with transport
    // TODO: register protocol components for replication
    // TODO: connection/disconnection event handlers
}
