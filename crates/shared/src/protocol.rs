use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection};
use serde::{Deserialize, Serialize};

pub use crate::protocol_snapshots::*;

use crate::components::{
    CharacterAppearance, EquipmentAppearance, GuildMembership, Health, Mana, ModelDisplay,
    MovementSpeed, Npc, Player, Position, Rotation, Zone,
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

/// Reliable ordered channel for auction house operations, bidirectional.
pub struct AuctionChannel;

/// Chat message type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ChatType {
    Say,
    Yell,
    Party,
    Guild,
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

/// Client requests to start casting a spell.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpellCastIntent {
    pub spell_id: Option<u32>,
    pub spell: String,
    pub target_entity: Option<u64>,
}

/// Client requests to stop the current spell cast.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StopSpellCast;

/// Client requests to invite a player to the group.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupInviteIntent {
    pub name: String,
}

/// Client requests to uninvite a player from the group.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupUninviteIntent {
    pub name: String,
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
    pub appearance: CharacterAppearance,
    pub equipment_appearance: EquipmentAppearance,
}

/// Client sends after connecting to authenticate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub token: Option<String>,
    pub username: String,
    pub password: String,
}

/// Server responds with auth result and character list.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginResponse {
    pub success: bool,
    pub token: String,
    pub characters: Vec<CharacterListEntry>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterListUpdate {
    pub character: CharacterListEntry,
}

/// Client sends to register a new account.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// Server responds to account registration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterResponse {
    pub success: bool,
    pub token: String,
    pub pending_approval: bool,
    pub error: Option<String>,
}

/// Client requests creating a new character.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateCharacter {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub appearance: CharacterAppearance,
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

/// Client requests inviting another character into their guild.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildInviteRequest {
    pub target_character_name: String,
}

/// Server responds to guild invite request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildInviteResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Client accepts their pending guild invitation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildAcceptInviteRequest;

/// Server responds to guild invitation acceptance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildAcceptInviteResponse {
    pub success: bool,
    pub guild_id: Option<u32>,
    pub guild_name: Option<String>,
    pub error: Option<String>,
}

/// Client requests recent chat history.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatHistoryRequest {
    pub limit: u16,
}

/// Server responds with recent chat history.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatHistoryResponse {
    pub messages: Vec<ChatMessage>,
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuctionDuration {
    Short,
    Medium,
    Long,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuctionTimeLeft {
    Short,
    Medium,
    Long,
    VeryLong,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuctionSortField {
    Name,
    MinBid,
    Buyout,
    TimeLeft,
    Quality,
    RequiredLevel,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuctionSortDir {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionInventoryItem {
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub stack_count: u32,
    pub vendor_sell_price: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionListingSummary {
    pub auction_id: u64,
    pub item: AuctionInventoryItem,
    pub owner_name: String,
    pub stack_count: u32,
    pub min_bid: u32,
    pub current_bid: Option<u32>,
    pub min_next_bid: u32,
    pub buyout_price: Option<u32>,
    pub time_left: AuctionTimeLeft,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionSearchQuery {
    pub text: String,
    pub page: u32,
    pub page_size: u32,
    pub min_level: Option<u16>,
    pub max_level: Option<u16>,
    pub quality: Option<u8>,
    pub usable_only: bool,
    pub sort_field: AuctionSortField,
    pub sort_dir: AuctionSortDir,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionHouseOpened {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OpenAuctionHouse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryAuctions {
    pub query: AuctionSearchQuery,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionSearchResults {
    pub query: AuctionSearchQuery,
    pub total_results: u32,
    pub results: Vec<AuctionListingSummary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlaceBid {
    pub auction_id: u64,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BuyoutAuction {
    pub auction_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CreateAuction {
    pub item_guid: u64,
    pub stack_count: u32,
    pub min_bid: u32,
    pub buyout_price: Option<u32>,
    pub duration: AuctionDuration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CancelAuction {
    pub auction_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryOwnedAuctions;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryBidAuctions;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryAuctionInventory;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionInventorySnapshot {
    pub gold: u32,
    pub items: Vec<AuctionInventoryItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OwnedAuctionListResponse {
    pub listings: Vec<AuctionListingSummary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BidAuctionListResponse {
    pub listings: Vec<AuctionListingSummary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionOperationResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionMailEntry {
    pub mail_id: u64,
    pub subject: String,
    pub body: String,
    pub attached_money: u32,
    pub attached_item: Option<AuctionInventoryItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryAuctionMailbox;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuctionMailboxSnapshot {
    pub entries: Vec<AuctionMailEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClaimAuctionMail {
    pub mail_id: u64,
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
    app.register_component::<GuildMembership>();
    app.register_component::<EquipmentAppearance>();
}

fn register_messages(app: &mut App) {
    register_core_messages(app);
    register_account_messages(app);
    register_character_messages(app);
    register_guild_and_chat_messages(app);
    register_auction_messages(app);
    crate::protocol_snapshots::register_snapshot_messages(app);
}

fn register_core_messages(app: &mut App) {
    app.register_message::<PlayerInput>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ChatMessage>()
        .add_direction(NetworkDirection::Bidirectional);
    app.register_message::<SetTarget>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CombatEvent>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<SpellCastIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<StopSpellCast>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GroupInviteIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GroupUninviteIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LoadTerrain>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_account_messages(app: &mut App) {
    app.register_message::<LoginRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LoginResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<RegisterRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RegisterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_character_messages(app: &mut App) {
    app.register_message::<CreateCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CreateCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<DeleteCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeleteCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<CharacterListUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<SelectCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<EnterWorldResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_guild_and_chat_messages(app: &mut App) {
    app.register_message::<GuildInviteRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GuildInviteResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<GuildAcceptInviteRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GuildAcceptInviteResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ChatHistoryRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ChatHistoryResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_messages(app: &mut App) {
    register_auction_query_messages(app);
    register_auction_action_messages(app);
    register_auction_mail_messages(app);
}

fn register_auction_query_messages(app: &mut App) {
    app.register_message::<OpenAuctionHouse>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionHouseOpened>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionSearchResults>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryOwnedAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<OwnedAuctionListResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryBidAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<BidAuctionListResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryAuctionInventory>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionInventorySnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_action_messages(app: &mut App) {
    app.register_message::<PlaceBid>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<BuyoutAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CreateAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CancelAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionOperationResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_mail_messages(app: &mut App) {
    app.register_message::<QueryAuctionMailbox>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionMailboxSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ClaimAuctionMail>()
        .add_direction(NetworkDirection::ClientToServer);
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

    app.add_channel::<AuctionChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{EquipmentVisualSlot, EquippedAppearanceEntry};

    #[test]
    fn chat_type_serialization_round_trip() {
        let types = vec![
            ChatType::Say,
            ChatType::Yell,
            ChatType::Party,
            ChatType::Guild,
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

    #[test]
    fn guild_invite_messages_round_trip() {
        let invite = GuildInviteRequest {
            target_character_name: "TargetOne".into(),
        };
        let encoded = serde_json::to_string(&invite).unwrap();
        let decoded: GuildInviteRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(invite.target_character_name, decoded.target_character_name);

        let invite_resp = GuildInviteResponse {
            success: true,
            error: None,
        };
        let encoded = serde_json::to_string(&invite_resp).unwrap();
        let decoded: GuildInviteResponse = serde_json::from_str(&encoded).unwrap();
        assert_eq!(invite_resp.success, decoded.success);
        assert_eq!(invite_resp.error, decoded.error);

        let accept_resp = GuildAcceptInviteResponse {
            success: true,
            guild_id: Some(3),
            guild_name: Some("Raid Team".into()),
            error: None,
        };
        let encoded = serde_json::to_string(&accept_resp).unwrap();
        let decoded: GuildAcceptInviteResponse = serde_json::from_str(&encoded).unwrap();
        assert_eq!(accept_resp.success, decoded.success);
        assert_eq!(accept_resp.guild_id, decoded.guild_id);
        assert_eq!(accept_resp.guild_name, decoded.guild_name);
        assert_eq!(accept_resp.error, decoded.error);
    }

    #[test]
    fn chat_history_response_round_trip() {
        let resp = ChatHistoryResponse {
            messages: vec![
                ChatMessage {
                    sender: "Alice".into(),
                    content: "hello".into(),
                    channel: ChatType::Say,
                },
                ChatMessage {
                    sender: "Bob".into(),
                    content: "guild hi".into(),
                    channel: ChatType::Guild,
                },
            ],
            error: None,
        };
        let encoded = serde_json::to_string(&resp).unwrap();
        let decoded: ChatHistoryResponse = serde_json::from_str(&encoded).unwrap();
        assert_eq!(resp.messages.len(), decoded.messages.len());
        assert_eq!(resp.messages[0].sender, decoded.messages[0].sender);
        assert_eq!(resp.messages[1].channel, decoded.messages[1].channel);
        assert_eq!(resp.error, decoded.error);
    }

    fn sample_auction_search_results() -> AuctionSearchResults {
        AuctionSearchResults {
            query: AuctionSearchQuery {
                text: "linen".into(),
                page: 0,
                page_size: 20,
                min_level: Some(1),
                max_level: Some(10),
                quality: Some(1),
                usable_only: false,
                sort_field: AuctionSortField::Name,
                sort_dir: AuctionSortDir::Asc,
            },
            total_results: 1,
            results: vec![AuctionListingSummary {
                auction_id: 7,
                item: AuctionInventoryItem {
                    item_guid: 12,
                    item_id: 2589,
                    name: "Linen Cloth".into(),
                    quality: 1,
                    required_level: 1,
                    stack_count: 20,
                    vendor_sell_price: 13,
                },
                owner_name: "Seller".into(),
                stack_count: 20,
                min_bid: 100,
                current_bid: Some(125),
                min_next_bid: 131,
                buyout_price: Some(200),
                time_left: AuctionTimeLeft::Long,
            }],
        }
    }

    #[test]
    fn auction_search_results_round_trip() {
        let msg = sample_auction_search_results();
        let encoded = serde_json::to_string(&msg).unwrap();
        let decoded: AuctionSearchResults = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.query.text, "linen");
        assert_eq!(decoded.total_results, 1);
        assert_eq!(decoded.results[0].auction_id, 7);
        assert_eq!(decoded.results[0].item.name, "Linen Cloth");
        assert_eq!(decoded.results[0].time_left, AuctionTimeLeft::Long);
    }

    #[test]
    fn equipment_appearance_round_trips() {
        let snapshot = EquipmentAppearance {
            entries: vec![
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Head,
                    item_id: Some(19019),
                    display_info_id: Some(12345),
                    inventory_type: 1,
                    hidden: false,
                },
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::MainHand,
                    item_id: Some(17182),
                    display_info_id: Some(54321),
                    inventory_type: 21,
                    hidden: false,
                },
            ],
        };

        let bitcode_encoded = bitcode::encode(&snapshot);
        let bitcode_decoded: EquipmentAppearance = bitcode::decode(&bitcode_encoded).unwrap();
        assert_eq!(bitcode_decoded, snapshot);

        let json_encoded = serde_json::to_string(&snapshot).unwrap();
        let json_decoded: EquipmentAppearance = serde_json::from_str(&json_encoded).unwrap();
        assert_eq!(json_decoded, snapshot);
    }
}
