use peercred_ipc::Client;
use serde::{Deserialize, Serialize};

const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

#[derive(Debug, Serialize, Deserialize)]
enum AdminRequest {
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
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
enum AdminResponse {
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
    Pong,
    Error(String),
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let request = parse_request(&args);
    let response: AdminResponse = match Client::call(SOCKET_PATH, &request) {
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
    eprintln!("  game-server-admin create-account <username> <password>");
    eprintln!("  game-server-admin approve-registration <username>");
    eprintln!("  game-server-admin create-guild <character_name> <guild_id> <guild_name>");
    eprintln!("  game-server-admin set-level <character_name> <level>");
    eprintln!("  game-server-admin set-credentials <token> <username> <password>");
}

fn parse_request(args: &[String]) -> AdminRequest {
    match args[1].as_str() {
        "ping" => AdminRequest::Ping,
        "create-account" => parse_create_account(args),
        "approve-registration" => parse_approve_registration(args),
        "create-guild" => parse_create_guild(args),
        "set-level" => parse_set_level(args),
        "set-credentials" => parse_set_credentials(args),
        other => {
            eprintln!("Unknown command: {other}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn require_args(args: &[String], min: usize, usage: &str) {
    if args.len() < min {
        eprintln!("Usage: game-server-admin {usage}");
        std::process::exit(1);
    }
}

fn parse_create_account(args: &[String]) -> AdminRequest {
    require_args(args, 4, "create-account <username> <password>");
    AdminRequest::CreateAccount { username: args[2].clone(), password: args[3].clone() }
}

fn parse_approve_registration(args: &[String]) -> AdminRequest {
    require_args(args, 3, "approve-registration <username>");
    AdminRequest::ApproveRegistration { username: args[2].clone() }
}

fn parse_create_guild(args: &[String]) -> AdminRequest {
    require_args(args, 5, "create-guild <character_name> <guild_id> <guild_name>");
    let guild_id: u32 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid guild_id: {}", args[3]);
        std::process::exit(1);
    });
    AdminRequest::CreateGuild { character_name: args[2].clone(), guild_id, guild_name: args[4..].join(" ") }
}

fn parse_set_level(args: &[String]) -> AdminRequest {
    require_args(args, 4, "set-level <character_name> <level>");
    let level: u16 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid level: {}", args[3]);
        std::process::exit(1);
    });
    AdminRequest::SetLevel { character_name: args[2].clone(), level }
}

fn parse_set_credentials(args: &[String]) -> AdminRequest {
    require_args(args, 5, "set-credentials <token> <username> <password>");
    AdminRequest::SetCredentials {
        token: args[2].clone(),
        username: args[3].clone(),
        password: args[4].clone(),
    }
}

fn print_response(response: &AdminResponse) {
    match response {
        AdminResponse::Pong => println!("pong"),
        AdminResponse::AccountCreated { account_id, token } => {
            println!("Account created: id={account_id} token={token}");
        }
        AdminResponse::RegistrationApproved { account_id, token } => {
            println!("Registration approved: id={account_id} token={token}");
        }
        AdminResponse::GuildCreated { character_id, guild_id, guild_name } => {
            println!(
                "Guild assigned: character_id={character_id} guild_id={guild_id} guild_name={guild_name}"
            );
        }
        AdminResponse::LevelSet { character_name, level } => {
            println!("Level set: {character_name} -> {level}");
        }
        AdminResponse::CredentialsSet { account_id, username } => {
            println!("Credentials set: account_id={account_id} username={username}");
        }
        AdminResponse::Error(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
