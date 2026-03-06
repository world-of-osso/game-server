use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use client::{CliAction, CliCommand};

#[derive(Parser)]
#[command(name = "game-cli", about = "Headless game server CLI client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Connect, authenticate, print character list, exit
    Login(AuthArgs),
    /// Register a new account
    Register(AuthArgs),
    /// Chat operations
    #[command(subcommand)]
    Chat(ChatCommands),
    /// Send a guild chat message and exit
    GuildChat(ChatSendArgs),
    /// Show account status (alias for login)
    Status(AuthArgs),
}

#[derive(Subcommand)]
enum ChatCommands {
    /// Send a chat message and exit
    Send(ChatSendArgs),
    /// Listen for chat messages (stays connected)
    Listen(AuthArgs),
}

#[derive(Parser, Clone)]
struct AuthArgs {
    /// Server address
    #[arg(short, long, default_value = "127.0.0.1:5000")]
    server: SocketAddr,
    /// Username
    #[arg(short, long)]
    username: String,
    /// Password
    #[arg(short, long)]
    password: String,
}

#[derive(Parser, Clone)]
struct ChatSendArgs {
    #[command(flatten)]
    auth: AuthArgs,
    /// Message to send
    #[arg(short, long)]
    message: String,
}

fn main() {
    let cli = Cli::parse();
    let action = build_action(cli);
    client::run(action);
}

fn build_action(cli: Cli) -> CliAction {
    match cli.command {
        Commands::Login(args) | Commands::Status(args) => {
            action_from_auth(args, CliCommand::Login)
        }
        Commands::Register(args) => action_from_auth(args, CliCommand::Register),
        Commands::Chat(ChatCommands::Send(args)) => CliAction {
            command: CliCommand::ChatSend(args.message),
            server: args.auth.server,
            username: args.auth.username,
            password: args.auth.password,
        },
        Commands::GuildChat(args) => CliAction {
            command: CliCommand::GuildChat(args.message),
            server: args.auth.server,
            username: args.auth.username,
            password: args.auth.password,
        },
        Commands::Chat(ChatCommands::Listen(args)) => {
            action_from_auth(args, CliCommand::ChatListen)
        }
    }
}

fn action_from_auth(args: AuthArgs, command: CliCommand) -> CliAction {
    CliAction {
        command,
        server: args.server,
        username: args.username,
        password: args.password,
    }
}
