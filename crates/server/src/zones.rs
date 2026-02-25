use bevy::prelude::*;

/// Zone management plugin.
pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_zones);
    }
}

fn update_zones() {
    // TODO: per-zone system sets
    // TODO: zone transition handling
    // TODO: interest management integration with lightyear
}
