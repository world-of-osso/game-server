use peercred_ipc::Client;
use serde::{Deserialize, Serialize};
use shared::components::CharacterAppearance;

#[path = "../admin_output.rs"]
mod admin_output;
#[path = "../equipment_admin_types.rs"]
mod equipment_admin_types;
use equipment_admin_types::{EquipmentGrantItem, EquipmentOwnership};

use admin_output::print_response;

const DEFAULT_SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// The server's admin socket; `GAME_SERVER_ADMIN_SOCKET` as the server reads it.
fn socket_path() -> std::path::PathBuf {
    std::env::var_os("GAME_SERVER_ADMIN_SOCKET")
        .map_or_else(|| DEFAULT_SOCKET_PATH.into(), Into::into)
}

#[derive(Debug, Serialize, Deserialize)]
enum AdminRequest {
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
    /// TrinityCore `.go xyz <x> <y> <z> <map>`: `Player::TeleportTo` for an online
    /// character (a map transfer when the map changes), the saved location otherwise.
    Teleport {
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
    /// `count` of `item_id` into the character's bags.
    GrantItem {
        character_name: String,
        item_id: u32,
        count: u32,
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
    TaxiLearn {
        character_name: String,
        node: u32,
    },
    CombatLog {
        last_seconds: f64,
    },
    /// TrinityCore `.damage <amount>`: the online character's target takes `amount`
    /// damage from it (`Unit::DealDamage`, school Normal).
    Damage {
        character_name: String,
        amount: f32,
    },
    /// TrinityCore `.revive`: a dead online character comes back to life with full
    /// health (`Player::ResurrectPlayer(1.0f)`).
    Revive {
        character_name: String,
    },
    /// Shift the instance-lock clock by `seconds`.
    ClockAdvance {
        seconds: i64,
    },
    /// TrinityCore `.instance listbinds`.
    InstanceLocks {
        character_name: String,
    },
    /// TrinityCore `.instance unbind <mapid|all>`.
    InstanceUnbind {
        character_name: String,
        map_id: Option<u32>,
    },
    Ping,
}

/// Mirror of the server's `CombatLogSummary` (same field order: msgpack
/// encodes structs as arrays).
#[derive(Debug, Serialize, Deserialize)]
struct CombatLogSummary {
    timestamp: f64,
    event_type: String,
    attacker: u64,
    target: u64,
    amount: f32,
    spell_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
enum AdminResponse {
    CombatLogEntries {
        count: usize,
        entries: Vec<CombatLogSummary>,
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
    TaxiNodeLearned {
        character_name: String,
        node: u32,
        node_name: String,
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
    Pong,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountInfo {
    username: String,
    account_id: u64,
    token_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct CharacterInfo {
    username: String,
    account_id: u64,
    character_id: u64,
    name: String,
    level: u16,
    race: u8,
    class: u8,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let request = parse_request(&args);
    let response: AdminResponse = match Client::call(socket_path(), &request) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect: {e}");
            std::process::exit(1);
        }
    };

    print_response(&response);
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  game-server-admin ping");
    eprintln!("  game-server-admin list-accounts");
    eprintln!("  game-server-admin list-characters [username]");
    eprintln!("  game-server-admin find-character <name>");
    eprintln!("  game-server-admin combat-log <last_seconds>");
    eprintln!("  game-server-admin move-characters <from_username> <to_username>");
    eprintln!("  game-server-admin check-account <username>");
    eprintln!("  game-server-admin check-password <username> <password>");
    eprintln!("  game-server-admin create-account <username> <password>");
    eprintln!("  game-server-admin kick-account <username> [reason]");
    eprintln!("  game-server-admin ban-account <username> [reason]");
    eprintln!("  game-server-admin unban-account <username>");
    eprintln!("  game-server-admin approve-registration <username>");
    eprintln!("  game-server-admin create-guild <character_name> <guild_id> <guild_name>");
    eprintln!("  game-server-admin set-guild-rank <character_name> <rank 0-4>");
    eprintln!(
        "  game-server-admin set-guild-bank-rights <guild_id> <rank> tab <tab> <view 0|1> <deposit 0|1> <stacks_per_day>"
    );
    eprintln!(
        "  game-server-admin set-guild-bank-rights <guild_id> <rank> gold <withdraw 0|1> <copper_per_day>"
    );
    eprintln!("  game-server-admin set-level <character_name> <level>");
    eprintln!("  game-server-admin set-credentials <token> <username> <password>");
    eprintln!("  game-server-admin set-position <character_name> <x> <y> <z>");
    eprintln!("  game-server-admin set-gold <character_name> <copper>");
    eprintln!("  game-server-admin grant-item <character_name> <item_id> <count>");
    eprintln!("  game-server-admin mail-deliver-now <character_name>");
    eprintln!("  game-server-admin set-durability <character_name> <percent>");
    eprintln!(
        "  game-server-admin set-appearance <character_name> <sex> <skin_color> <face> <eye_color> <hair_style> <hair_color> <facial_style>"
    );
    eprintln!(
        "  game-server-admin set-equipment-appearance <character_name> <slot> <display_info_id>"
    );
    eprintln!("  game-server-admin clear-equipment-appearance <character_name> <slot>");
    eprintln!("  game-server-admin grant-equipment <character> <manifest.json>");
    eprintln!("  game-server-admin list-equipment <character>");
    eprintln!("  game-server-admin taxi-learn <character_name> <taxi_node_id>");
    eprintln!("  game-server-admin clock-advance <seconds>");
    eprintln!("  game-server-admin instance-locks <character_name>");
    eprintln!("  game-server-admin instance-unbind <character_name> <map_id|all>");
}

fn parse_request(args: &[String]) -> AdminRequest {
    match args[1].as_str() {
        "grant-equipment" => parse_grant_equipment(args).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        }),
        "list-equipment" => {
            require_args(args, 3, "list-equipment <character>");
            AdminRequest::ListEquipment {
                character_name: args[2].clone(),
            }
        }
        "ping" => AdminRequest::Ping,
        "list-accounts" => AdminRequest::ListAccounts,
        "list-characters" => parse_list_characters(args),
        "find-character" => parse_find_character(args),
        "move-characters" => parse_move_characters(args),
        "check-account" => parse_check_account(args),
        "check-password" => parse_check_password(args),
        "create-account" => parse_create_account(args),
        "kick-account" => parse_kick_account(args),
        "ban-account" => parse_ban_account(args),
        "unban-account" => parse_unban_account(args),
        "approve-registration" => parse_approve_registration(args),
        "create-guild" => parse_create_guild(args),
        "set-guild-rank" => parse_set_guild_rank(args),
        "set-guild-bank-rights" => parse_set_guild_bank_rights(args),
        "set-level" => parse_set_level(args),
        "set-credentials" => parse_set_credentials(args),
        "set-position" => parse_set_position(args),
        "teleport" => parse_teleport(args),
        "revive" => {
            require_args(args, 3, "revive <character_name>");
            AdminRequest::Revive {
                character_name: args[2].clone(),
            }
        }
        "damage" => {
            require_args(args, 4, "damage <character_name> <amount>");
            AdminRequest::Damage {
                character_name: args[2].clone(),
                amount: args[3].parse().expect("amount must be a number"),
            }
        }
        "clock-advance" => {
            require_args(args, 3, "clock-advance <seconds>");
            AdminRequest::ClockAdvance {
                seconds: args[2].parse().expect("seconds must be an integer"),
            }
        }
        "instance-locks" => {
            require_args(args, 3, "instance-locks <character_name>");
            AdminRequest::InstanceLocks {
                character_name: args[2].clone(),
            }
        }
        "instance-unbind" => {
            require_args(args, 4, "instance-unbind <character_name> <map_id|all>");
            AdminRequest::InstanceUnbind {
                character_name: args[2].clone(),
                map_id: (args[3] != "all")
                    .then(|| args[3].parse().expect("map_id must be a number or all")),
            }
        }
        "kill" => {
            require_args(args, 3, "kill <character_name>");
            AdminRequest::Kill {
                character_name: args[2].clone(),
            }
        }
        "combat-log" => parse_combat_log(args),
        "set-gold" => parse_set_gold(args),
        "grant-item" => parse_grant_item(args),
        "mail-deliver-now" => {
            require_args(args, 3, "mail-deliver-now <character_name>");
            AdminRequest::MailDeliverNow {
                character_name: args[2].clone(),
            }
        }
        "set-durability" => parse_set_durability(args),
        "set-appearance" => parse_set_appearance(args),
        "set-equipment-appearance" => parse_set_equipment_appearance(args),
        "clear-equipment-appearance" => parse_clear_equipment_appearance(args),
        "taxi-learn" => parse_taxi_learn(args),
        other => {
            eprintln!("Unknown command: {other}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn parse_grant_equipment(args: &[String]) -> Result<AdminRequest, String> {
    if args.len() != 4 {
        return Err("Usage: grant-equipment <character> <manifest.json>".into());
    }
    let bytes = std::fs::read(&args[3])
        .map_err(|error| format!("read equipment manifest {}: {error}", args[3]))?;
    let items = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse equipment manifest {}: {error}", args[3]))?;
    Ok(AdminRequest::GrantEquipment {
        character_name: args[2].clone(),
        items,
    })
}

fn parse_list_characters(args: &[String]) -> AdminRequest {
    AdminRequest::ListCharacters {
        username: args.get(2).cloned(),
    }
}

fn parse_find_character(args: &[String]) -> AdminRequest {
    require_args(args, 3, "find-character <name>");
    AdminRequest::FindCharacter {
        name: args[2].clone(),
    }
}

fn parse_move_characters(args: &[String]) -> AdminRequest {
    require_args(args, 4, "move-characters <from_username> <to_username>");
    AdminRequest::MoveCharacters {
        from_username: args[2].clone(),
        to_username: args[3].clone(),
    }
}

fn parse_check_account(args: &[String]) -> AdminRequest {
    require_args(args, 3, "check-account <username>");
    AdminRequest::CheckAccount {
        username: args[2].clone(),
    }
}

fn parse_check_password(args: &[String]) -> AdminRequest {
    require_args(args, 4, "check-password <username> <password>");
    AdminRequest::CheckPassword {
        username: args[2].clone(),
        password: args[3].clone(),
    }
}

fn parse_set_equipment_appearance(args: &[String]) -> AdminRequest {
    require_args(
        args,
        5,
        "set-equipment-appearance <character_name> <slot> <display_info_id>",
    );
    let display_info_id = args[4].parse().unwrap_or_else(|_| {
        eprintln!("display_info_id must be a number");
        std::process::exit(1);
    });
    AdminRequest::SetEquipmentAppearance {
        character_name: args[2].clone(),
        slot: args[3].clone(),
        display_info_id,
    }
}

fn parse_clear_equipment_appearance(args: &[String]) -> AdminRequest {
    require_args(
        args,
        4,
        "clear-equipment-appearance <character_name> <slot>",
    );
    AdminRequest::ClearEquipmentAppearance {
        character_name: args[2].clone(),
        slot: args[3].clone(),
    }
}

fn require_args(args: &[String], min: usize, usage: &str) {
    if args.len() < min {
        eprintln!("Usage: game-server-admin {usage}");
        std::process::exit(1);
    }
}

fn optional_reason_arg(args: &[String], start: usize) -> Option<String> {
    let reason = args.get(start..).unwrap_or(&[]).join(" ");
    (!reason.trim().is_empty()).then_some(reason)
}

fn parse_create_account(args: &[String]) -> AdminRequest {
    require_args(args, 4, "create-account <username> <password>");
    AdminRequest::CreateAccount {
        username: args[2].clone(),
        password: args[3].clone(),
    }
}

fn parse_kick_account(args: &[String]) -> AdminRequest {
    require_args(args, 3, "kick-account <username> [reason]");
    AdminRequest::KickAccount {
        username: args[2].clone(),
        reason: optional_reason_arg(args, 3),
    }
}

fn parse_ban_account(args: &[String]) -> AdminRequest {
    require_args(args, 3, "ban-account <username> [reason]");
    AdminRequest::BanAccount {
        username: args[2].clone(),
        reason: optional_reason_arg(args, 3),
    }
}

fn parse_unban_account(args: &[String]) -> AdminRequest {
    require_args(args, 3, "unban-account <username>");
    AdminRequest::UnbanAccount {
        username: args[2].clone(),
    }
}

fn parse_approve_registration(args: &[String]) -> AdminRequest {
    require_args(args, 3, "approve-registration <username>");
    AdminRequest::ApproveRegistration {
        username: args[2].clone(),
    }
}

fn parse_create_guild(args: &[String]) -> AdminRequest {
    require_args(
        args,
        5,
        "create-guild <character_name> <guild_id> <guild_name>",
    );
    let guild_id: u32 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid guild_id: {}", args[3]);
        std::process::exit(1);
    });
    AdminRequest::CreateGuild {
        character_name: args[2].clone(),
        guild_id,
        guild_name: args[4..].join(" "),
    }
}

fn parse_taxi_learn(args: &[String]) -> AdminRequest {
    require_args(args, 4, "taxi-learn <character_name> <taxi_node_id>");
    AdminRequest::TaxiLearn {
        character_name: args[2].clone(),
        node: parse_number(&args[3], "taxi node id"),
    }
}

fn parse_set_level(args: &[String]) -> AdminRequest {
    require_args(args, 4, "set-level <character_name> <level>");
    let level: u16 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid level: {}", args[3]);
        std::process::exit(1);
    });
    AdminRequest::SetLevel {
        character_name: args[2].clone(),
        level,
    }
}

fn parse_number(value: &str, what: &str) -> u32 {
    value.parse().unwrap_or_else(|_| {
        eprintln!("Invalid {what}: {value}");
        std::process::exit(1);
    })
}

fn parse_flag(value: &str, what: &str) -> bool {
    match value {
        "0" => false,
        "1" => true,
        _ => {
            eprintln!("Invalid {what}: {value} (expected 0 or 1)");
            std::process::exit(1);
        }
    }
}

fn parse_small(value: &str, what: &str) -> u8 {
    u8::try_from(parse_number(value, what)).unwrap_or_else(|_| {
        eprintln!("Invalid {what}: {value}");
        std::process::exit(1);
    })
}

fn parse_set_guild_rank(args: &[String]) -> AdminRequest {
    require_args(args, 4, "set-guild-rank <character_name> <rank 0-4>");
    AdminRequest::SetGuildRank {
        character_name: args[2].clone(),
        rank: parse_small(&args[3], "rank"),
    }
}

fn parse_set_guild_bank_rights(args: &[String]) -> AdminRequest {
    const USAGE: &str = "set-guild-bank-rights <guild_id> <rank> tab <tab> <view 0|1> <deposit 0|1> <stacks_per_day> | <guild_id> <rank> gold <withdraw 0|1> <copper_per_day>";
    require_args(args, 5, USAGE);
    let guild_id = parse_number(&args[2], "guild_id");
    let rank = parse_small(&args[3], "rank");
    match args[4].as_str() {
        "tab" => {
            require_args(args, 9, USAGE);
            AdminRequest::SetGuildBankTabRights {
                guild_id,
                rank,
                tab: parse_small(&args[5], "tab"),
                view: parse_flag(&args[6], "view"),
                deposit: parse_flag(&args[7], "deposit"),
                withdrawals_per_day: parse_number(&args[8], "stacks_per_day"),
            }
        }
        "gold" => {
            require_args(args, 7, USAGE);
            AdminRequest::SetGuildBankGoldRights {
                guild_id,
                rank,
                withdraw_gold: parse_flag(&args[5], "withdraw"),
                gold_per_day: args[6].parse().unwrap_or_else(|_| {
                    eprintln!("Invalid copper_per_day: {}", args[6]);
                    std::process::exit(1);
                }),
            }
        }
        _ => {
            eprintln!("Usage: game-server-admin {USAGE}");
            std::process::exit(1);
        }
    }
}

fn parse_set_gold(args: &[String]) -> AdminRequest {
    require_args(args, 4, "set-gold <character_name> <copper>");
    AdminRequest::SetGold {
        character_name: args[2].clone(),
        copper: args[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid copper: {}", args[3]);
            std::process::exit(1);
        }),
    }
}

fn parse_grant_item(args: &[String]) -> AdminRequest {
    require_args(args, 5, "grant-item <character_name> <item_id> <count>");
    AdminRequest::GrantItem {
        character_name: args[2].clone(),
        item_id: parse_number(&args[3], "item_id"),
        count: parse_number(&args[4], "count"),
    }
}

fn parse_set_durability(args: &[String]) -> AdminRequest {
    require_args(args, 4, "set-durability <character_name> <percent>");
    AdminRequest::SetDurability {
        character_name: args[2].clone(),
        percent: parse_number(&args[3], "percent"),
    }
}

fn parse_set_credentials(args: &[String]) -> AdminRequest {
    require_args(args, 5, "set-credentials <token> <username> <password>");
    AdminRequest::SetCredentials {
        token: args[2].clone(),
        username: args[3].clone(),
        password: args[4].clone(),
    }
}

fn parse_combat_log(args: &[String]) -> AdminRequest {
    require_args(args, 3, "combat-log <last_seconds>");
    let last_seconds: f64 = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid last_seconds: {}", args[2]);
        std::process::exit(1);
    });
    AdminRequest::CombatLog { last_seconds }
}

fn parse_set_position(args: &[String]) -> AdminRequest {
    require_args(args, 6, "set-position <character_name> <x> <y> <z>");
    let x: f32 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid x: {}", args[3]);
        std::process::exit(1);
    });
    let y: f32 = args[4].parse().unwrap_or_else(|_| {
        eprintln!("Invalid y: {}", args[4]);
        std::process::exit(1);
    });
    let z: f32 = args[5].parse().unwrap_or_else(|_| {
        eprintln!("Invalid z: {}", args[5]);
        std::process::exit(1);
    });
    AdminRequest::SetPosition {
        character_name: args[2].clone(),
        x,
        y,
        z,
    }
}

fn parse_teleport(args: &[String]) -> AdminRequest {
    require_args(args, 7, "teleport <character_name> <map_id> <x> <y> <z>");
    let number = |index: usize| -> f32 {
        args[index].parse().unwrap_or_else(|_| {
            eprintln!("Invalid number: {}", args[index]);
            std::process::exit(1);
        })
    };
    AdminRequest::Teleport {
        character_name: args[2].clone(),
        map_id: args[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid map id: {}", args[3]);
            std::process::exit(1);
        }),
        x: number(4),
        y: number(5),
        z: number(6),
    }
}

fn parse_set_appearance(args: &[String]) -> AdminRequest {
    require_args(
        args,
        10,
        "set-appearance <character_name> <sex> <skin_color> <face> <eye_color> <hair_style> <hair_color> <facial_style>",
    );
    AdminRequest::SetAppearance {
        character_name: args[2].clone(),
        appearance: CharacterAppearance {
            sex: parse_u8_arg("sex", &args[3]),
            skin_color: parse_u8_arg("skin_color", &args[4]),
            face: parse_u8_arg("face", &args[5]),
            eye_color: parse_u8_arg("eye_color", &args[6]),
            hair_style: parse_u8_arg("hair_style", &args[7]),
            hair_color: parse_u8_arg("hair_color", &args[8]),
            facial_style: parse_u8_arg("facial_style", &args[9]),
            customization_choices: Vec::new(),
        },
    }
}

fn parse_u8_arg(name: &str, value: &str) -> u8 {
    value.parse().unwrap_or_else(|_| {
        eprintln!("Invalid {name}: {value}");
        std::process::exit(1);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn parse_guild_rank_and_bank_rights_requests() {
        let rank = parse_request(&args(&[
            "game-server-admin",
            "set-guild-rank",
            "Bankalt",
            "3",
        ]));
        assert!(matches!(
            rank,
            AdminRequest::SetGuildRank { ref character_name, rank: 3 } if character_name == "Bankalt"
        ));
        let tab = parse_request(&args(&[
            "game-server-admin",
            "set-guild-bank-rights",
            "7",
            "3",
            "tab",
            "0",
            "1",
            "1",
            "2",
        ]));
        assert!(matches!(
            tab,
            AdminRequest::SetGuildBankTabRights {
                guild_id: 7,
                rank: 3,
                tab: 0,
                view: true,
                deposit: true,
                withdrawals_per_day: 2
            }
        ));
        let gold = parse_request(&args(&[
            "game-server-admin",
            "set-guild-bank-rights",
            "7",
            "3",
            "gold",
            "1",
            "50000",
        ]));
        assert!(matches!(
            gold,
            AdminRequest::SetGuildBankGoldRights {
                guild_id: 7,
                rank: 3,
                withdraw_gold: true,
                gold_per_day: 50_000
            }
        ));
    }

    #[test]
    fn parse_mail_deliver_now_request() {
        let request = parse_request(&args(&["game-server-admin", "mail-deliver-now", "Tradeb"]));
        assert!(matches!(
            request,
            AdminRequest::MailDeliverNow { ref character_name } if character_name == "Tradeb"
        ));
    }

    #[test]
    fn parse_set_gold_and_set_durability_requests() {
        let gold = parse_request(&args(&[
            "game-server-admin",
            "set-gold",
            "Shopper",
            "250000",
        ]));
        assert!(matches!(
            gold,
            AdminRequest::SetGold { ref character_name, copper: 250_000 } if character_name == "Shopper"
        ));
        let durability = parse_request(&args(&[
            "game-server-admin",
            "set-durability",
            "Shopper",
            "40",
        ]));
        assert!(matches!(
            durability,
            AdminRequest::SetDurability { ref character_name, percent: 40 } if character_name == "Shopper"
        ));
    }

    #[test]
    fn parse_grant_item_request() {
        let grant = parse_request(&args(&[
            "game-server-admin",
            "grant-item",
            "Stitchwell",
            "2589",
            "4",
        ]));
        assert!(matches!(
            grant,
            AdminRequest::GrantItem { ref character_name, item_id: 2589, count: 4 }
                if character_name == "Stitchwell"
        ));
    }

    #[test]
    fn parse_combat_log_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "combat-log".to_string(),
            "25".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::CombatLog { last_seconds } if last_seconds == 25.0
        ));
    }

    #[test]
    fn parse_set_position_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "set-position".to_string(),
            "Mover".to_string(),
            "-8920.5".to_string(),
            "-188.25".to_string(),
            "97.75".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::SetPosition {
                character_name,
                x,
                y,
                z,
            } if character_name == "Mover"
                && (x + 8920.5).abs() < f32::EPSILON
                && (y + 188.25).abs() < f32::EPSILON
                && (z - 97.75).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn parse_set_appearance_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "set-appearance".to_string(),
            "Theron".to_string(),
            "0".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
            "1".to_string(),
            "6".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::SetAppearance {
                character_name,
                appearance,
            } if character_name == "Theron"
                && appearance.sex == 0
                && appearance.skin_color == 2
                && appearance.face == 3
                && appearance.eye_color == 4
                && appearance.hair_style == 5
                && appearance.hair_color == 1
                && appearance.facial_style == 6
                && appearance.customization_choices.is_empty()
        ));
    }

    #[test]
    fn parse_check_account_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "check-account".to_string(),
            "admin".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::CheckAccount { username } if username == "admin"
        ));
    }

    #[test]
    fn parse_check_password_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "check-password".to_string(),
            "admin".to_string(),
            "admin".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::CheckPassword { username, password }
                if username == "admin" && password == "admin"
        ));
    }

    #[test]
    fn parse_list_characters_request_with_username() {
        let args = vec![
            "game-server-admin".to_string(),
            "list-characters".to_string(),
            "admin".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::ListCharacters { username } if username.as_deref() == Some("admin")
        ));
    }

    #[test]
    fn parse_find_character_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "find-character".to_string(),
            "Theron".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::FindCharacter { name } if name == "Theron"
        ));
    }

    #[test]
    fn parse_move_characters_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "move-characters".to_string(),
            "alice".to_string(),
            "admin".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::MoveCharacters {
                from_username,
                to_username,
            } if from_username == "alice" && to_username == "admin"
        ));
    }

    #[test]
    fn equipment_manifest_command_reads_typed_items_and_rejects_unknown_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("gear.json");
        let item = EquipmentGrantItem {
            item_id: 25,
            name: "Worn Shortsword".into(),
            quality: 1,
            required_level: 1,
            inventory_type: 13,
            vendor_sell_price: 1,
            allowable_class: 1,
            allowable_race: 1,
        };
        std::fs::write(&path, serde_json::to_vec(&vec![item.clone()]).unwrap()).unwrap();
        let args = vec![
            "admin".into(),
            "grant-equipment".into(),
            "Theron".into(),
            path.display().to_string(),
        ];
        let AdminRequest::GrantEquipment {
            character_name,
            items,
        } = parse_grant_equipment(&args).unwrap()
        else {
            panic!("grant request")
        };
        assert_eq!(character_name, "Theron");
        assert_eq!(items, vec![item]);
        std::fs::write(&path, b"[{\"item_id\":25,\"unknown\":true}]").unwrap();
        assert!(
            parse_grant_equipment(&args)
                .unwrap_err()
                .contains("manifest")
        );
        assert!(parse_grant_equipment(&args[..3]).is_err());
        let listed = parse_request(&["admin".into(), "list-equipment".into(), "Theron".into()]);
        assert!(
            matches!(listed, AdminRequest::ListEquipment { character_name } if character_name == "Theron")
        );
    }

    #[test]
    fn parse_set_equipment_appearance_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "set-equipment-appearance".to_string(),
            "Theron".to_string(),
            "chest".to_string(),
            "777".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::SetEquipmentAppearance {
                character_name,
                slot,
                display_info_id,
            } if character_name == "Theron" && slot == "chest" && display_info_id == 777
        ));
    }

    #[test]
    fn parse_clear_equipment_appearance_request() {
        let args = vec![
            "game-server-admin".to_string(),
            "clear-equipment-appearance".to_string(),
            "Theron".to_string(),
            "chest".to_string(),
        ];

        let request = parse_request(&args);

        assert!(matches!(
            request,
            AdminRequest::ClearEquipmentAppearance { character_name, slot }
                if character_name == "Theron" && slot == "chest"
        ));
    }
}
