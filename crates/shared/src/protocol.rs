use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;

pub use crate::protocol_snapshots::*;

use crate::components::{
    CombatStatus, EquipmentAppearance, Gold, GuildMembership, Health, Mana, ModelDisplay, Mounted,
    MovementSpeed, Npc, Player, Position, PresenceStatus, Rotation, Zone,
};

mod channels;
mod core_messages;
mod gameplay_messages;
mod registration;

pub use channels::*;
pub use core_messages::*;
pub use gameplay_messages::*;

use registration::{register_channels, register_messages};

/// Registers shared protocol: components for replication and channels.
/// Must be added AFTER `ServerPlugins`/`ClientPlugins` but BEFORE any entity is spawned.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        register_replicated_components(app);
        register_messages(app);
        register_channels(app);
    }
}

fn register_replicated_components(app: &mut App) {
    app.register_component::<Position>();
    app.register_component::<Health>();
    app.register_component::<Mana>();
    app.register_component::<Gold>();
    app.register_component::<Player>();
    app.register_component::<Npc>();
    app.register_component::<ModelDisplay>();
    app.register_component::<Rotation>();
    app.register_component::<MovementSpeed>();
    app.register_component::<CombatStatus>();
    app.register_component::<Mounted>();
    app.register_component::<Zone>();
    app.register_component::<GuildMembership>();
    app.register_component::<PresenceStatus>();
    app.register_component::<EquipmentAppearance>();
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
