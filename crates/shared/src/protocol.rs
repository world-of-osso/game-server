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
    /// Server system message (MOTD, announcements).
    System,
    /// Periodic server broadcast (autobroadcast).
    ServerBroadcast,
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

/// Combat event type for damage/death/respawn/avoidance notifications.
///
/// Ref: WoW combat log COMBAT_LOG_EVENT types.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CombatEventType {
    /// Melee auto-attack damage (white hit).
    MeleeDamage,
    /// Direct spell damage (e.g. Fireball, Mortal Strike).
    SpellDamage,
    /// Direct spell healing (e.g. Flash Heal).
    SpellHeal,
    /// Periodic damage tick (DoT).
    PeriodicDamage,
    /// Periodic healing tick (HoT).
    PeriodicHeal,
    /// Damage absorbed by a shield.
    Absorb,
    /// Melee or spell miss.
    Miss,
    /// Target dodged the attack.
    Dodge,
    /// Target parried the attack.
    Parry,
    /// Target blocked the attack (damage still dealt, reduced by block value).
    Block,
    /// Critical strike (melee or spell).
    CriticalHit,
    /// Spell cast was interrupted (school lockout).
    Interrupt,
    /// Unit died.
    Death,
    /// Unit respawned.
    Respawn,
}

/// Combat event sent from server to clients.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CombatEvent {
    pub attacker: u64,
    pub target: u64,
    /// Damage dealt, healing done, or amount absorbed. Context depends on `event_type`.
    pub amount: f32,
    /// Spell ID for spell-related events (0 for melee/death/respawn).
    pub spell_id: u32,
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
    /// Whether the player is swimming.
    pub swimming: bool,
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
    /// Filter by faction: 0 = neutral (show all), 1 = Alliance, 2 = Horde.
    pub faction: u8,
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
#[path = "protocol_tests.rs"]
mod tests;
