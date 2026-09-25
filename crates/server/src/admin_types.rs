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
    SetLevel {
        character_name: String,
        level: u16,
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
    /// Money in copper.
    SetGold {
        character_name: String,
        copper: u64,
    },
    /// `count` of `item_id` into the character's bags.
    GrantItem {
        character_name: String,
        item_id: u32,
        count: u32,
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
    Ping,
}

/// IPC response from server to CLI.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AdminResponse {
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
    CredentialsSet {
        account_id: u64,
        username: String,
    },
    PositionSet {
        character_name: String,
        x: f32,
        y: f32,
        z: f32,
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
    Pong,
    Error(String),
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
