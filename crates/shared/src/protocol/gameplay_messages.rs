use serde::{Deserialize, Serialize};

use crate::components::{CharacterAppearance, EquipmentAppearance};
use crate::protocol_snapshots::*;

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
pub struct AchievementStateUpdate {
    pub snapshot: Option<AchievementSnapshot>,
    pub completed: Option<AchievementToastSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorldMapStateUpdate {
    pub snapshot: Option<WorldMapSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RestStateUpdate {
    pub snapshot: Option<RestSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryFriends;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AddFriend {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RemoveFriend {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetPresenceStatus {
    pub status: crate::components::PresenceStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FriendsStateUpdate {
    pub snapshot: Option<FriendsSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryWho {
    pub query: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WhoStateUpdate {
    pub snapshot: Option<WhoSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryCalendar;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ScheduleCalendarEvent {
    pub title: String,
    pub starts_at_unix_secs: u64,
    pub max_signups: u8,
    pub is_raid: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RespondCalendarSignup {
    pub event_id: u64,
    pub status: CalendarSignupStatusSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CalendarStateUpdate {
    pub snapshot: Option<CalendarSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryIgnoreList;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AddIgnore {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RemoveIgnore {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IgnoreListStateUpdate {
    pub snapshot: Option<IgnoreListSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryLfgStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueueForLfg {
    pub role: GroupRoleSnapshot,
    pub dungeon_ids: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DequeueFromLfg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RespondToLfgRoleCheck {
    pub accepted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LfgStateUpdate {
    pub snapshot: Option<LfgSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryPvpStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueueForBattleground {
    pub battleground_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueueForRatedPvp {
    pub bracket: PvpBracketSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DequeueFromPvp;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PvpStateUpdate {
    pub snapshot: Option<PvpSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryBarberShopStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApplyBarberShopChanges {
    pub appearance: CharacterAppearance,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BarberShopStateUpdate {
    pub snapshot: Option<BarberShopSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryDeathStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ReleaseSpirit;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ResurrectAtCorpse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AcceptSpiritHealerResurrection;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UseStuckEscape;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeathStateUpdate {
    pub snapshot: Option<DeathSnapshot>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryDurabilityStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DurabilityStateUpdate {
    pub snapshot: Option<DurabilitySnapshot>,
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
