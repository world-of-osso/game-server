use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use client::{CliAction, CliCommand};
use shared::components::CharacterAppearance;

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
    /// Invite another character to your guild
    GuildInvite(GuildInviteArgs),
    /// Accept your pending guild invitation
    GuildAccept(TokenArgs),
    /// Send a guild chat message and exit
    GuildChat(ChatSendArgs),
    /// Create a character for the authenticated account
    CreateCharacter(CreateCharacterArgs),
    /// Show account status (uses token auth)
    Status(TokenArgs),
}

#[derive(Subcommand)]
enum ChatCommands {
    /// Send a chat message and exit
    Send(ChatSendArgs),
    /// Listen for chat messages (stays connected)
    Listen(TokenArgs),
    /// Show recent chat history
    History(ChatHistoryArgs),
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
    /// Auth token (defaults to ~/.config/world-of-osso/auth_token)
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

#[derive(Parser, Clone)]
struct CreateCharacterArgs {
    #[command(flatten)]
    auth: TokenArgs,
    /// Character name
    #[arg(short, long)]
    name: String,
    /// Race ID
    #[arg(long, default_value_t = 1)]
    race: u8,
    /// Class ID
    #[arg(long = "class", default_value_t = 1)]
    class_id: u8,
    /// Sex (0=male, 1=female)
    #[arg(long, default_value_t = 0)]
    sex: u8,
    #[arg(long, default_value_t = 0)]
    skin_color: u8,
    #[arg(long, default_value_t = 0)]
    face: u8,
    #[arg(long, default_value_t = 0)]
    eye_color: u8,
    #[arg(long, default_value_t = 0)]
    hair_style: u8,
    #[arg(long, default_value_t = 0)]
    hair_color: u8,
    #[arg(long, default_value_t = 0)]
    facial_style: u8,
}

#[derive(Parser, Clone)]
struct GuildInviteArgs {
    #[command(flatten)]
    auth: TokenArgs,
    /// Target character name
    #[arg(long)]
    target: String,
}

#[derive(Parser, Clone)]
struct ChatHistoryArgs {
    #[command(flatten)]
    auth: TokenArgs,
    /// Number of messages to return (max 100)
    #[arg(long, default_value_t = 100)]
    limit: u16,
}

fn main() {
    let cli = Cli::parse();
    let action = build_action(cli);
    client::run(action);
}

fn build_action(cli: Cli) -> CliAction {
    match cli.command {
        Commands::Login(args) => login_action(args),
        Commands::Status(args) => action_from_token(args, CliCommand::Status),
        Commands::Register(args) => register_action(args),
        Commands::GuildInvite(args) => guild_invite_action(args),
        Commands::GuildAccept(args) => action_from_token(args, CliCommand::GuildAccept),
        Commands::CreateCharacter(args) => create_character_action(args),
        Commands::Chat(chat) => chat_action(chat),
        Commands::GuildChat(args) => {
            action_from_token(args.auth, CliCommand::GuildChat(args.message))
        }
    }
}

fn login_action(args: LoginArgs) -> CliAction {
    action_from_credentials(
        args.server.server,
        args.username,
        args.password,
        CliCommand::Login,
    )
}

fn register_action(args: RegisterArgs) -> CliAction {
    action_from_credentials(
        args.server.server,
        args.username,
        args.password,
        CliCommand::Register,
    )
}

fn guild_invite_action(args: GuildInviteArgs) -> CliAction {
    action_from_token(
        args.auth,
        CliCommand::GuildInvite {
            target: args.target,
        },
    )
}

fn create_character_action(args: CreateCharacterArgs) -> CliAction {
    let appearance = character_appearance(&args);
    action_from_token(
        args.auth,
        CliCommand::CreateCharacter {
            name: args.name,
            race: args.race,
            class: args.class_id,
            appearance,
        },
    )
}

fn character_appearance(args: &CreateCharacterArgs) -> CharacterAppearance {
    CharacterAppearance {
        sex: args.sex,
        skin_color: args.skin_color,
        face: args.face,
        eye_color: args.eye_color,
        hair_style: args.hair_style,
        hair_color: args.hair_color,
        facial_style: args.facial_style,
        customization_choices: Vec::new(),
        visage: None,
    }
}

fn chat_action(chat: ChatCommands) -> CliAction {
    match chat {
        ChatCommands::Send(args) => {
            action_from_token(args.auth, CliCommand::ChatSend(args.message))
        }
        ChatCommands::Listen(args) => action_from_token(args, CliCommand::ChatListen),
        ChatCommands::History(args) => action_from_token(
            args.auth,
            CliCommand::ChatHistory {
                limit: args.limit.min(100),
            },
        ),
    }
}

fn action_from_credentials(
    server: SocketAddr,
    username: String,
    password: String,
    command: CliCommand,
) -> CliAction {
    CliAction {
        command,
        server,
        username: Some(username),
        password: Some(password),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_character_cli_keeps_core_selectors_and_empty_additional_choices() {
        let cli = Cli::try_parse_from([
            "game-cli",
            "create-character",
            "--name",
            "Elara",
            "--sex",
            "1",
            "--skin-color",
            "2",
            "--face",
            "3",
            "--eye-color",
            "4",
            "--hair-style",
            "5",
            "--hair-color",
            "6",
            "--facial-style",
            "7",
        ])
        .unwrap();
        let Commands::CreateCharacter(args) = cli.command else {
            panic!("expected creation command")
        };
        let CliCommand::CreateCharacter {
            name, appearance, ..
        } = create_character_action(args).command
        else {
            panic!("expected creation action")
        };
        assert_eq!(name, "Elara");
        assert_eq!(
            appearance,
            CharacterAppearance {
                sex: 1,
                skin_color: 2,
                face: 3,
                eye_color: 4,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
                customization_choices: Vec::new(),
                visage: None,
            }
        );
    }
}
