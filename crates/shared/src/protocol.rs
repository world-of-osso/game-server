use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection};
use serde::{Deserialize, Serialize};

use crate::components::{
    Health, Mana, ModelDisplay, MovementSpeed, Npc, Player, Position, Rotation, Zone,
};

/// Unreliable movement position updates, server-to-client only.
pub struct MovementChannel;

/// Reliable ordered channel for combat events, bidirectional.
pub struct CombatChannel;

/// Reliable ordered channel for chat messages, bidirectional.
pub struct ChatChannel;

/// Unreliable client-to-server channel for movement inputs.
pub struct InputChannel;

/// Reliable ordered channel for server-to-client terrain loading commands.
pub struct TerrainChannel;

/// Reliable ordered channel for authentication and character management, bidirectional.
pub struct AuthChannel;

/// Chat message type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ChatType {
    Say,
    Yell,
    Party,
    Whisper(String),
}

/// A chat message sent between client and server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub channel: ChatType,
}

/// Client request to set (or clear) their combat target.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetTarget {
    /// Server entity bits of the target, or None to clear.
    pub target_entity: Option<u64>,
}

/// Combat event type for damage/death/respawn notifications.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CombatEventType {
    MeleeDamage,
    Death,
    Respawn,
}

/// Combat event sent from server to clients.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CombatEvent {
    pub attacker: u64,
    pub target: u64,
    pub damage: f32,
    pub event_type: CombatEventType,
}

/// Server tells clients which terrain map and initial tile to load.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadTerrain {
    pub map_name: String,
    pub initial_tile_y: u32,
    pub initial_tile_x: u32,
}

/// Entry in the character list sent to the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterListEntry {
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub race: u8,
    pub class: u8,
}

/// Client sends after connecting to authenticate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub token: Option<String>,
    pub username: String,
}

/// Server responds with auth result and character list.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginResponse {
    pub success: bool,
    pub token: String,
    pub characters: Vec<CharacterListEntry>,
    pub error: Option<String>,
}

/// Client requests creating a new character.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateCharacter {
    pub name: String,
    pub race: u8,
    pub class: u8,
}

/// Server responds to character creation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateCharacterResponse {
    pub success: bool,
    pub character: Option<CharacterListEntry>,
    pub error: Option<String>,
}

/// Client requests deleting a character.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteCharacter {
    pub character_id: u64,
}

/// Server responds to character deletion.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteCharacterResponse {
    pub success: bool,
    pub character_id: u64,
    pub error: Option<String>,
}

/// Client selects a character to enter the world with.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SelectCharacter {
    pub character_id: u64,
}

/// Server responds to character selection with world entry result.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnterWorldResponse {
    pub success: bool,
    pub player_entity: Option<u64>,
    pub error: Option<String>,
}

/// Client movement input sent each tick.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInput {
    /// Normalized movement direction in world space.
    pub direction: [f32; 3],
    /// Character facing yaw in radians.
    pub facing_yaw: f32,
    /// Whether the player is jumping.
    pub jumping: bool,
    /// Whether the player is running (vs walking).
    pub running: bool,
}

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
    app.register_component::<Player>();
    app.register_component::<Npc>();
    app.register_component::<ModelDisplay>();
    app.register_component::<Rotation>();
    app.register_component::<MovementSpeed>();
    app.register_component::<Zone>();
}

fn register_messages(app: &mut App) {
    app.register_message::<PlayerInput>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ChatMessage>()
        .add_direction(NetworkDirection::Bidirectional);
    app.register_message::<SetTarget>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CombatEvent>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<LoadTerrain>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<LoginRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LoginResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<CreateCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CreateCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<DeleteCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeleteCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<SelectCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<EnterWorldResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_channels(app: &mut App) {
    app.add_channel::<MovementChannel>(ChannelSettings {
        mode: ChannelMode::UnorderedUnreliable,
        ..default()
    })
    .add_direction(NetworkDirection::ServerToClient);

    app.add_channel::<InputChannel>(ChannelSettings {
        mode: ChannelMode::UnorderedUnreliable,
        ..default()
    })
    .add_direction(NetworkDirection::ClientToServer);

    app.add_channel::<CombatChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);

    app.add_channel::<ChatChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);

    app.add_channel::<TerrainChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(default()),
        ..default()
    })
    .add_direction(NetworkDirection::ServerToClient);

    app.add_channel::<AuthChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_type_serialization_round_trip() {
        let types = vec![
            ChatType::Say,
            ChatType::Yell,
            ChatType::Party,
            ChatType::Whisper("TargetPlayer".into()),
        ];
        for ct in types {
            let serialized = serde_json::to_string(&ct).unwrap();
            let deserialized: ChatType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(ct, deserialized);
        }
    }

    #[test]
    fn chat_message_serialization_round_trip() {
        let msg = ChatMessage {
            sender: "Alice".into(),
            content: "Hello world".into(),
            channel: ChatType::Whisper("Bob".into()),
        };
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(msg.sender, deserialized.sender);
        assert_eq!(msg.content, deserialized.content);
        assert_eq!(msg.channel, deserialized.channel);
    }
}
