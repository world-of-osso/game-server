//! Shared types for the admin IPC protocol.

use crate::equipment_admin_types::{EquipmentGrantItem, EquipmentOwnership};
use serde::{Deserialize, Serialize};
use shared::components::CharacterAppearance;

/// IPC request from CLI to server.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    GrantEquipment {
        character_name: String,
        items: Vec<EquipmentGrantItem>,
    },
    ListEquipment {
        character_name: String,
    },
    ListAccounts,
    ListCharacters {
        username: Option<String>,
    },
    FindCharacter {
        name: String,
    },
    MoveCharacters {
        from_username: String,
        to_username: String,
    },
    CheckAccount {
        username: String,
    },
    CheckPassword {
        username: String,
        password: String,
    },
    CreateAccount {
        username: String,
        password: String,
    },
    CreateCharacter {
        username: String,
        name: String,
        race: u8,
        class: u8,
    },
    KickAccount {
        username: String,
        reason: Option<String>,
    },
    BanAccount {
        username: String,
        reason: Option<String>,
    },
    UnbanAccount {
        username: String,
    },
    ApproveRegistration {
        username: String,
    },
    CreateGuild {
        character_name: String,
        guild_id: u32,
        guild_name: String,
    },
    /// Guild rank (0 Guild Master .. 4 Initiate) of a guild member.
    SetGuildRank {
        character_name: String,
        rank: u8,
    },
    /// Guild bank tab rights of a rank.
    SetGuildBankTabRights {
        guild_id: u32,
        rank: u8,
        tab: u8,
        view: bool,
        deposit: bool,
        withdrawals_per_day: u32,
    },
    /// Guild bank gold withdrawal right and copper per day of a rank.
    SetGuildBankGoldRights {
        guild_id: u32,
        rank: u8,
        withdraw_gold: bool,
        gold_per_day: u64,
    },
    /// Guild bank repair right of a rank (TrinityCore `GR_RIGHT_WITHDRAW_REPAIR`).
    SetGuildBankRepairRight {
        guild_id: u32,
        rank: u8,
        withdraw_repair: bool,
    },
    SetLevel {
        character_name: String,
        level: u16,
    },
    SetRested {
        character_name: String,
        amount: u32,
    },
    SetCredentials {
        token: String,
        username: String,
        password: String,
    },
    SetPosition {
        character_name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    /// TrinityCore `.go xyz <x> <y> <z> <map>`: `Player::TeleportTo` for an online
    /// character (a map transfer when the map changes), the saved location otherwise.
    Teleport {
        character_name: String,
        map_id: u32,
        x: f32,
        y: f32,
        z: f32,
    },
    /// Admin-only harness staging of the owner's existing genuine follower party.
    StageFollowerParty {
        character_name: String,
        map_id: u32,
        x: f32,
        y: f32,
        z: f32,
    },
    /// TrinityCore `.die`: the online character dies (`Unit::Kill`).
    Kill {
        character_name: String,
    },
    /// Money in copper.
    SetGold {
        character_name: String,
        copper: u64,
    },
    /// Issue an explicitly selected locally qualified keystone to bags.
    GrantKeystone {
        character_name: String,
        challenge_id: u32,
        level: u32,
    },
    /// `count` of `item_id` into the character's bags.
    GrantItem {
        character_name: String,
        item_id: u32,
        count: u32,
    },
    /// TrinityCore `.quest complete`: the quest's kill and object objectives done.
    QuestComplete {
        character_name: String,
        quest_id: u32,
    },
    /// Deliver the character's mail still in transit now.
    MailDeliverNow {
        character_name: String,
    },
    /// Every equipped item to `percent` of its max durability.
    SetDurability {
        character_name: String,
        percent: u32,
    },
    SetAppearance {
        character_name: String,
        appearance: CharacterAppearance,
    },
    SetEquipmentAppearance {
        character_name: String,
        slot: String,
        display_info_id: u32,
    },
    ClearEquipmentAppearance {
        character_name: String,
        slot: String,
    },
    CombatLog {
        /// How many seconds of history to return (from now).
        last_seconds: f64,
    },
    // --- Ticket commands ---
    TicketList,
    TicketClaim {
        ticket_id: u64,
        gm_name: String,
    },
    TicketClaimNext {
        gm_name: String,
    },
    TicketRespond {
        ticket_id: u64,
        gm_name: String,
        message: String,
    },
    TicketResolve {
        ticket_id: u64,
    },
    TicketEscalate {
        ticket_id: u64,
    },
    TicketHistory {
        player_id: u64,
    },
    // --- MOTD commands ---
    GetMotd,
    SetMotd {
        message: String,
    },
    ClearMotd,
    // --- Autobroadcast commands ---
    BroadcastList,
    BroadcastAdd {
        message: String,
    },
    BroadcastRemove {
        index: usize,
    },
    BroadcastSetInterval {
        seconds: u64,
    },
    BroadcastEnable,
    BroadcastDisable,
    /// Teach a character a flight point (`TaxiNodes.ID`).
    TaxiLearn {
        character_name: String,
        node: u32,
    },
    LearnSpell {
        character_name: String,
        spell: u32,
    },
    /// Put the online character on Flight Style: Steady or Skyriding (skyriding also
    /// teaches Skyriding Basics 376777); its next mount picks the matching capability.
    FlightStyle {
        character_name: String,
        skyriding: bool,
    },
    /// TrinityCore `.damage <amount>`: the online character's target (or the unit
    /// `target`, entity bits) takes `amount` damage from it (`Unit::DealDamage`, school
    /// Normal).
    Damage {
        character_name: String,
        amount: f32,
        target: Option<u64>,
    },
    /// TrinityCore `.revive`: a dead online character comes back to life with full
    /// health (`Player::ResurrectPlayer(1.0f)`).
    Revive {
        character_name: String,
    },
    /// Shift the instance-lock and auction-expiry clock (`GameTime::GetSystemTime`)
    /// by `seconds`: resets, lock expiry and auction settlement without waiting.
    ClockAdvance {
        seconds: i64,
    },
    /// TrinityCore `.instance listbinds`: the character's instance locks.
    InstanceLocks {
        character_name: String,
    },
    /// TrinityCore `.instance unbind <mapid|all>`: expire the character's unused locks.
    InstanceUnbind {
        character_name: String,
        map_id: Option<u32>,
    },
    /// TrinityCore `.npc info` for every creature of `entry` in map copy
    /// (`map_id`, `instance_id`): level, health and the copy's difficulty.
    NpcInfo {
        map_id: u32,
        instance_id: u32,
        entry: u32,
    },
    /// Read-only lookup of live objects by entry in a map copy.
    GameObjectInfo {
        map_id: u32,
        instance_id: u32,
        entry: u32,
    },
    EconomyStatus,
    Ping,
    /// Private-server test setup: credit Trader's Tender to an account.
    TradingPostGrantTender {
        username: String,
        amount: u32,
    },
    /// Private-server test setup: refund window (seconds) for new purchases; 0 disables.
    TradingPostRefundWindow {
        seconds: u32,
    },
    /// Private-server test setup: add renown reputation for a sourced major faction.
    RenownGrant {
        character_name: String,
        major_faction: u32,
        amount: u32,
    },
}

/// IPC response from server to CLI.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AdminResponse {
    EconomyStatus {
        status: serde_json::Value,
    },
    EquipmentListed {
        character_id: u64,
        character_name: String,
        items: Vec<EquipmentOwnership>,
    },
    EquipmentGranted {
        character_id: u64,
        character_name: String,
        items: Vec<EquipmentOwnership>,
    },
    AccountsListed {
        accounts: Vec<AccountInfo>,
    },
    CharactersListed {
        characters: Vec<CharacterInfo>,
    },
    CharacterFound {
        character: Option<CharacterInfo>,
    },
    CharactersMoved {
        from_username: String,
        to_username: String,
        moved_count: usize,
    },
    AccountChecked {
        username: String,
        exists: bool,
        account_id: Option<u64>,
        token_count: usize,
        pending_registration: bool,
        banned: bool,
        ban_reason: Option<String>,
    },
    PasswordChecked {
        username: String,
        valid: bool,
    },
    AccountCreated {
        account_id: u64,
        token: String,
    },
    CharacterCreated {
        username: String,
        character_id: u64,
        name: String,
    },
    AccountKicked {
        username: String,
        disconnected: bool,
    },
    AccountBanned {
        username: String,
        disconnected: bool,
        reason: String,
    },
    AccountUnbanned {
        username: String,
    },
    RegistrationApproved {
        account_id: u64,
        token: String,
    },
    GuildCreated {
        character_id: u64,
        guild_id: u32,
        guild_name: String,
    },
    GuildRankSet {
        character_name: String,
        rank: u8,
        rank_name: String,
    },
    GuildBankRightsSet {
        guild_id: u32,
        rank: u8,
        rank_name: String,
    },
    LevelSet {
        character_name: String,
        level: u16,
    },
    RestedSet {
        character_name: String,
        amount: u32,
    },
    CredentialsSet {
        account_id: u64,
        username: String,
    },
    FollowerPartyStaged {
        character_name: String,
        followers: usize,
    },
    PositionSet {
        character_name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    Teleported {
        character_name: String,
        map_id: u32,
        online: bool,
    },
    Killed {
        character_name: String,
    },
    GoldSet {
        character_name: String,
        copper: u64,
    },
    ItemGranted {
        character_name: String,
        item_id: u32,
        name: String,
        count: u32,
    },
    MailDelivered {
        character_name: String,
        count: usize,
    },
    QuestCompleted {
        character_name: String,
        quest_id: u32,
    },
    DurabilitySet {
        character_name: String,
        percent: u32,
    },
    AppearanceSet {
        character_name: String,
        appearance: CharacterAppearance,
    },
    EquipmentAppearanceSet {
        character_name: String,
        slot: String,
        display_info_id: u32,
    },
    EquipmentAppearanceCleared {
        character_name: String,
        slot: String,
    },
    CombatLogEntries {
        count: usize,
        entries: Vec<CombatLogSummary>,
    },
    // --- Ticket responses ---
    TicketListed {
        tickets: Vec<TicketInfo>,
    },
    TicketClaimed {
        ticket_id: u64,
        gm_name: String,
    },
    TicketResponded {
        ticket_id: u64,
    },
    TicketResolved {
        ticket_id: u64,
    },
    TicketEscalated {
        ticket_id: u64,
    },
    TicketHistoryListed {
        player_id: u64,
        tickets: Vec<TicketInfo>,
    },
    // --- MOTD responses ---
    MotdRetrieved {
        message: String,
        updated_at: u64,
    },
    MotdSet,
    MotdCleared,
    // --- Autobroadcast responses ---
    BroadcastListed {
        messages: Vec<String>,
        interval: u64,
        enabled: bool,
    },
    BroadcastAdded {
        index: usize,
    },
    BroadcastRemoved {
        message: String,
    },
    BroadcastIntervalSet {
        seconds: u64,
    },
    BroadcastEnabled,
    BroadcastDisabled,
    TaxiNodeLearned {
        character_name: String,
        node: u32,
        node_name: String,
    },
    SpellLearned {
        character_name: String,
        spell: u32,
    },
    FlightStyleSet {
        character_name: String,
        skyriding: bool,
    },
    Damaged {
        character_name: String,
        amount: f32,
    },
    Revived {
        character_name: String,
    },
    ClockAdvanced {
        offset_secs: i64,
        unix: i64,
    },
    InstanceLockList {
        character_name: String,
        locks: Vec<String>,
    },
    InstanceUnbound {
        character_name: String,
        count: usize,
    },
    NpcInfoList {
        lines: Vec<String>,
    },
    GameObjectInfoList {
        lines: Vec<String>,
    },
    Pong,
    Error(String),
    TradingPostTenderGranted {
        username: String,
        tender: u32,
    },
    TradingPostRefundWindowSet {
        seconds: u32,
    },
    RenownGranted {
        character_name: String,
        major_faction: u32,
        level: u32,
        standing: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountInfo {
    pub username: String,
    pub account_id: u64,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterInfo {
    pub username: String,
    pub account_id: u64,
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub race: u8,
    pub class: u8,
}

/// Summary of a ticket for IPC transport.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TicketInfo {
    pub id: u64,
    pub player: u64,
    pub category: String,
    pub description: String,
    pub state: String,
    pub priority: String,
    pub assigned_to: Option<String>,
    pub response_count: usize,
    pub created_at: u64,
}

/// Summary of a combat log entry for IPC transport.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatLogSummary {
    pub timestamp: f64,
    pub event_type: String,
    pub attacker: u64,
    pub target: u64,
    pub amount: f32,
    pub spell_id: u32,
}
