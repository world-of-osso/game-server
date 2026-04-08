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

/// Reliable ordered channel for trade operations, bidirectional.
pub struct TradeChannel;

/// Reliable ordered channel for talent operations, bidirectional.
pub struct TalentChannel;

/// Reliable ordered channel for profession operations, bidirectional.
pub struct ProfessionChannel;

/// Reliable ordered channel for reputation updates, bidirectional.
pub struct ReputationChannel;

/// Reliable ordered channel for collection updates, bidirectional.
pub struct CollectionChannel;

/// Reliable ordered channel for currency updates, bidirectional.
pub struct CurrencyChannel;

/// Reliable ordered channel for inspect operations, bidirectional.
pub struct InspectChannel;

/// Reliable ordered channel for duel operations, bidirectional.
pub struct DuelChannel;

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradePhase {
    PendingOutgoing,
    PendingIncoming,
    Open,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TradeItemSnapshot {
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub stack_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TradePartySnapshot {
    pub name: String,
    pub accepted: bool,
    pub gold: u32,
    pub slots: Vec<Option<TradeItemSnapshot>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TradeSnapshot {
    pub phase: TradePhase,
    pub player: TradePartySnapshot,
    pub other: TradePartySnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InitiateTrade {
    pub target_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AcceptTrade;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeclineTrade;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CancelTrade;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SetTradeItem {
    pub slot: u8,
    pub item_guid: u64,
    pub stack_count: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClearTradeItem {
    pub slot: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SetTradeMoney {
    pub copper: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConfirmTrade;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TradeStateUpdate {
    pub trade: Option<TradeSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryTalents;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApplyTalentChoice {
    pub talent_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ResetTalents;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TalentSpecTabSnapshot {
    pub name: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TalentNodeSnapshot {
    pub talent_id: u32,
    pub name: String,
    pub points_spent: u8,
    pub max_points: u8,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TalentSnapshot {
    pub spec_tabs: Vec<TalentSpecTabSnapshot>,
    pub talents: Vec<TalentNodeSnapshot>,
    pub points_remaining: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TalentStateUpdate {
    pub snapshot: Option<TalentSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryInspectTarget {
    pub target_entity: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InspectSnapshot {
    pub target_name: String,
    pub equipment_appearance: EquipmentAppearance,
    pub talents: TalentSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InspectStateUpdate {
    pub snapshot: Option<InspectSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InitiateDuel {
    pub target_entity: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AcceptDuel;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeclineDuel;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DuelPhaseSnapshot {
    PendingOutgoing,
    PendingIncoming,
    Active,
    Completed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DuelBoundarySnapshot {
    pub center_x: f32,
    pub center_z: f32,
    pub radius: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DuelResultSnapshot {
    Won,
    Lost,
    Declined,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DuelSnapshot {
    pub phase: DuelPhaseSnapshot,
    pub opponent_name: String,
    pub boundary: Option<DuelBoundarySnapshot>,
    pub result: Option<DuelResultSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DuelStateUpdate {
    pub snapshot: Option<DuelSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryProfessions;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CraftProfessionRecipe {
    pub recipe_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GatherProfessionNode {
    pub node_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProfessionStateUpdate {
    pub snapshot: Option<ProfessionSnapshot>,
    pub message: Option<String>,
    pub skill_up: Option<ProfessionSkillSnapshot>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ReputationStateUpdate {
    pub snapshot: Option<ReputationSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SummonMount {
    pub mount_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DismissMount;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SummonPet {
    pub pet_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DismissPet;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionStateUpdate {
    pub snapshot: Option<CollectionSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EarnCurrency {
    pub currency_id: u32,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SpendCurrency {
    pub currency_id: u32,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CurrencyStateUpdate {
    pub snapshot: Option<CurrencySnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
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
    register_trade_messages(app);
    register_talent_messages(app);
    register_inspect_messages(app);
    register_duel_messages(app);
    register_profession_messages(app);
    register_reputation_messages(app);
    register_collection_messages(app);
    register_currency_messages(app);
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

fn register_trade_messages(app: &mut App) {
    app.register_message::<InitiateTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AcceptTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeclineTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CancelTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetTradeItem>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ClearTradeItem>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetTradeMoney>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ConfirmTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<TradeStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_talent_messages(app: &mut App) {
    app.register_message::<QueryTalents>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ApplyTalentChoice>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ResetTalents>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<TalentStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_inspect_messages(app: &mut App) {
    app.register_message::<QueryInspectTarget>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<InspectStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_duel_messages(app: &mut App) {
    app.register_message::<InitiateDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AcceptDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeclineDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DuelStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_profession_messages(app: &mut App) {
    app.register_message::<QueryProfessions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CraftProfessionRecipe>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GatherProfessionNode>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ProfessionStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_reputation_messages(app: &mut App) {
    app.register_message::<ReputationStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_collection_messages(app: &mut App) {
    app.register_message::<SummonMount>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DismissMount>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SummonPet>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DismissPet>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CollectionStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_currency_messages(app: &mut App) {
    app.register_message::<EarnCurrency>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SpendCurrency>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CurrencyStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_channels(app: &mut App) {
    register_unreliable_channels(app);
    register_server_channels(app);
    register_bidirectional_channels(app);
}

fn register_unreliable_channels(app: &mut App) {
    add_channel::<MovementChannel>(
        app,
        ChannelMode::UnorderedUnreliable,
        NetworkDirection::ServerToClient,
    );
    add_channel::<InputChannel>(
        app,
        ChannelMode::UnorderedUnreliable,
        NetworkDirection::ClientToServer,
    );
}

fn register_server_channels(app: &mut App) {
    add_reliable_channel::<TerrainChannel>(app, NetworkDirection::ServerToClient);
}

fn register_bidirectional_channels(app: &mut App) {
    add_reliable_channel::<CombatChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ChatChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<AuthChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<AuctionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<TradeChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<TalentChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<InspectChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<DuelChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ProfessionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ReputationChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<CollectionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<CurrencyChannel>(app, NetworkDirection::Bidirectional);
}

fn add_reliable_channel<C: Channel>(app: &mut App, direction: NetworkDirection) {
    add_channel::<C>(app, ChannelMode::OrderedReliable(default()), direction);
}

fn add_channel<C: Channel>(app: &mut App, mode: ChannelMode, direction: NetworkDirection) {
    app.add_channel::<C>(ChannelSettings { mode, ..default() })
        .add_direction(direction);
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
