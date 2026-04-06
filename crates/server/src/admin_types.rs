//! Shared types for the admin IPC protocol.

use serde::{Deserialize, Serialize};
use shared::components::CharacterAppearance;

/// IPC request from CLI to server.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminRequest {
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
    ApproveRegistration {
        username: String,
    },
    CreateGuild {
        character_name: String,
        guild_id: u32,
        guild_name: String,
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
    Ping,
}

/// IPC response from server to CLI.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AdminResponse {
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
    },
    PasswordChecked {
        username: String,
        valid: bool,
    },
    AccountCreated {
        account_id: u64,
        token: String,
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
