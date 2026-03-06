use peercred_ipc::Client;
use serde::{Deserialize, Serialize};

const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

#[derive(Debug, Serialize, Deserialize)]
enum AdminRequest {
    CreateAccount { username: String, password: String },
    ApproveRegistration { username: String },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
enum AdminResponse {
    AccountCreated { account_id: u64, token: String },
    RegistrationApproved { account_id: u64, token: String },
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
}

fn parse_request(args: &[String]) -> AdminRequest {
    match args[1].as_str() {
        "ping" => AdminRequest::Ping,
        "create-account" => {
            if args.len() < 4 {
                eprintln!("Usage: game-server-admin create-account <username> <password>");
                std::process::exit(1);
            }
            AdminRequest::CreateAccount {
                username: args[2].clone(),
                password: args[3].clone(),
            }
        }
        "approve-registration" => {
            if args.len() < 3 {
                eprintln!("Usage: game-server-admin approve-registration <username>");
                std::process::exit(1);
            }
            AdminRequest::ApproveRegistration {
                username: args[2].clone(),
            }
        }
        other => {
            eprintln!("Unknown command: {other}");
            print_usage();
            std::process::exit(1);
        }
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
        AdminResponse::Error(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
