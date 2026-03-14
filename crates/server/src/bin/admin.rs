use peercred_ipc::Client;
use serde::{Deserialize, Serialize};
use shared::components::CharacterAppearance;

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
    eprintln!("  game-server-admin set-position <character_name> <x> <y> <z>");
    eprintln!(
        "  game-server-admin set-appearance <character_name> <sex> <skin_color> <face> <hair_style> <hair_color> <facial_style>"
    );
}

fn parse_request(args: &[String]) -> AdminRequest {
    match args[1].as_str() {
        "ping" => AdminRequest::Ping,
        "create-account" => parse_create_account(args),
        "approve-registration" => parse_approve_registration(args),
        "create-guild" => parse_create_guild(args),
        "set-level" => parse_set_level(args),
        "set-credentials" => parse_set_credentials(args),
        "set-position" => parse_set_position(args),
        "set-appearance" => parse_set_appearance(args),
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
    AdminRequest::CreateAccount {
        username: args[2].clone(),
        password: args[3].clone(),
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

fn parse_set_credentials(args: &[String]) -> AdminRequest {
    require_args(args, 5, "set-credentials <token> <username> <password>");
    AdminRequest::SetCredentials {
        token: args[2].clone(),
        username: args[3].clone(),
        password: args[4].clone(),
    }
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

fn parse_set_appearance(args: &[String]) -> AdminRequest {
    require_args(
        args,
        9,
        "set-appearance <character_name> <sex> <skin_color> <face> <hair_style> <hair_color> <facial_style>",
    );
    AdminRequest::SetAppearance {
        character_name: args[2].clone(),
        appearance: CharacterAppearance {
            sex: parse_u8_arg("sex", &args[3]),
            skin_color: parse_u8_arg("skin_color", &args[4]),
            face: parse_u8_arg("face", &args[5]),
            hair_style: parse_u8_arg("hair_style", &args[6]),
            hair_color: parse_u8_arg("hair_color", &args[7]),
            facial_style: parse_u8_arg("facial_style", &args[8]),
        },
    }
}

fn parse_u8_arg(name: &str, value: &str) -> u8 {
    value.parse().unwrap_or_else(|_| {
        eprintln!("Invalid {name}: {value}");
        std::process::exit(1);
    })
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
        AdminResponse::GuildCreated {
            character_id,
            guild_id,
            guild_name,
        } => {
            println!(
                "Guild assigned: character_id={character_id} guild_id={guild_id} guild_name={guild_name}"
            );
        }
        AdminResponse::LevelSet {
            character_name,
            level,
        } => {
            println!("Level set: {character_name} -> {level}");
        }
        AdminResponse::CredentialsSet {
            account_id,
            username,
        } => {
            println!("Credentials set: account_id={account_id} username={username}");
        }
        AdminResponse::PositionSet {
            character_name,
            x,
            y,
            z,
        } => {
            println!("Position set: {character_name} -> ({x}, {y}, {z})");
        }
        AdminResponse::AppearanceSet {
            character_name,
            appearance,
        } => {
            println!(
                "Appearance set: {character_name} -> sex={} skin_color={} face={} hair_style={} hair_color={} facial_style={}",
                appearance.sex,
                appearance.skin_color,
                appearance.face,
                appearance.hair_style,
                appearance.hair_color,
                appearance.facial_style
            );
        }
        AdminResponse::Error(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                && appearance.hair_style == 4
                && appearance.hair_color == 5
                && appearance.facial_style == 1
        ));
    }
}
