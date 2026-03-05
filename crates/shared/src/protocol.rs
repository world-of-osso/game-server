use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection};
use serde::{Deserialize, Serialize};

use crate::components::{Health, Mana, Player, Position};

/// Unreliable movement position updates, server-to-client only.
pub struct MovementChannel;

/// Reliable ordered channel for combat events, bidirectional.
pub struct CombatChannel;

/// Reliable ordered channel for chat messages, bidirectional.
pub struct ChatChannel;

/// Unreliable client-to-server channel for movement inputs.
pub struct InputChannel;

/// Client movement input sent each tick.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInput {
    /// Normalized movement direction in world space.
    pub direction: [f32; 3],
    /// Character facing yaw in radians.
    pub facing_yaw: f32,
    /// Whether the player is jumping.
    pub jumping: bool,
}

/// Registers shared protocol: components for replication and channels.
/// Must be added AFTER `ServerPlugins`/`ClientPlugins` but BEFORE any entity is spawned.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Register components for replication
        app.register_component::<Position>();
        app.register_component::<Health>();
        app.register_component::<Mana>();
        app.register_component::<Player>();

        // Register messages
        app.register_message::<PlayerInput>()
            .add_direction(NetworkDirection::ClientToServer);

        // MovementChannel: unreliable, server-to-client only
        app.add_channel::<MovementChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        // InputChannel: unreliable, client-to-server only
        app.add_channel::<InputChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        // CombatChannel: reliable ordered, bidirectional
        app.add_channel::<CombatChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(default()),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);

        // ChatChannel: reliable ordered, bidirectional
        app.add_channel::<ChatChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(default()),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);
    }
}
