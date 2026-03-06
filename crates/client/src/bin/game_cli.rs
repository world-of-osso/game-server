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
    Login(LoginArgs),
    /// Register a new account
    Register(RegisterArgs),
    /// Chat operations
    #[command(subcommand)]
    Chat(ChatCommands),
    /// Send a guild chat message and exit
    GuildChat(ChatSendArgs),
    /// Show account status (uses token auth)
    Status(TokenArgs),
}

#[derive(Subcommand)]
enum ChatCommands {
    /// Send a chat message and exit
    Send(ChatSendArgs),
    /// Listen for chat messages (stays connected)
    Listen(TokenArgs),
}

#[derive(Parser, Clone)]
struct ServerArgs {
    /// Server address
    #[arg(short, long, default_value = "127.0.0.1:5000")]
    server: SocketAddr,
}

#[derive(Parser, Clone)]
struct LoginArgs {
    #[command(flatten)]
    server: ServerArgs,
    /// Username
    #[arg(short, long)]
    username: String,
    /// Password
    #[arg(short, long)]
    password: String,
}

#[derive(Parser, Clone)]
struct RegisterArgs {
    #[command(flatten)]
    server: ServerArgs,
    /// Username
    #[arg(short, long)]
    username: String,
    /// Password
    #[arg(short, long)]
    password: String,
}

#[derive(Parser, Clone)]
struct TokenArgs {
    #[command(flatten)]
    server: ServerArgs,
    /// Auth token (defaults to data/auth_token)
    #[arg(short, long)]
    token: Option<String>,
}

#[derive(Parser, Clone)]
struct ChatSendArgs {
    #[command(flatten)]
    auth: TokenArgs,
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
        Commands::Login(args) => action_from_login(args, CliCommand::Login),
        Commands::Status(args) => action_from_token(args, CliCommand::Status),
        Commands::Register(args) => action_from_register(args, CliCommand::Register),
        Commands::Chat(ChatCommands::Send(args)) => CliAction {
            command: CliCommand::ChatSend(args.message),
            server: args.auth.server.server,
            username: None,
            password: None,
            token: args.auth.token,
        },
        Commands::GuildChat(args) => CliAction {
            command: CliCommand::GuildChat(args.message),
            server: args.auth.server.server,
            username: None,
            password: None,
            token: args.auth.token,
        },
        Commands::Chat(ChatCommands::Listen(args)) => {
            action_from_token(args, CliCommand::ChatListen)
        }
    }
}

fn action_from_login(args: LoginArgs, command: CliCommand) -> CliAction {
    CliAction {
        command,
        server: args.server.server,
        username: Some(args.username),
        password: Some(args.password),
        token: None,
    }
}

fn action_from_register(args: RegisterArgs, command: CliCommand) -> CliAction {
    CliAction {
        command,
        server: args.server.server,
        username: Some(args.username),
        password: Some(args.password),
        token: None,
    }
}

fn action_from_token(args: TokenArgs, command: CliCommand) -> CliAction {
    CliAction {
        command,
        server: args.server.server,
        username: None,
        password: None,
        token: args.token,
    }
}
