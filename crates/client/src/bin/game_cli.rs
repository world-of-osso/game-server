use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use client::{CliAction, CliCommand, EconomyAction, EconomyOperation};
use shared::components::CharacterAppearance;
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, CreateAuction, PlaceBid,
};

#[derive(Parser)]
#[command(name = "game-cli", about = "Headless game server CLI client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Auction house operations (requires a nearby auctioneer's server entity bits)
    #[command(subcommand)]
    Ah(AhCommands),
    /// Merchant operations (requires a nearby vendor's server entity bits)
    #[command(subcommand)]
    Merchant(MerchantCommands),
    /// Standard inbox claims at a nearby mailbox GameObject
    #[command(subcommand)]
    Mail(MailCommands),
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

#[derive(Parser, Clone)]
struct EconomyArgs {
    #[command(flatten)]
    auth: TokenArgs,
    /// NPC server entity bits, NOT creature entry/spawn GUID
    #[arg(long)]
    npc: u64,
    /// Gossip option ID when the NPC presents a menu
    #[arg(long)]
    gossip_option: Option<u32>,
}

#[derive(Subcommand)]
enum AhCommands {
    Open(EconomyArgs),
    Search(SearchArgs),
    /// Group auctions by item, with global quantity and lowest unit price
    Browse(SearchArgs),
    Inventory(EconomyArgs),
    Owned(EconomyArgs),
    Bids(EconomyArgs),
    Create(AuctionCreateArgs),
    Bid(BidArgs),
    Buyout(AuctionIdArgs),
    Cancel(AuctionIdArgs),
}

#[derive(Subcommand)]
enum MerchantCommands {
    List(EconomyArgs),
    Buy(MerchantBuyArgs),
}

#[derive(Parser, Clone)]
struct MailArgs {
    #[command(flatten)]
    auth: TokenArgs,
    /// Mailbox GameObject server entity bits, NOT template entry/spawn GUID
    #[arg(long)]
    object: u64,
}

#[derive(Parser)]
struct MailIdArgs {
    #[command(flatten)]
    context: MailArgs,
    #[arg(long)]
    mail_id: u64,
}

#[derive(Parser)]
struct MailAttachmentArgs {
    #[command(flatten)]
    mail: MailIdArgs,
    /// Fixed attachment slot from mail list, NOT its position in the list
    #[arg(long)]
    slot: u8,
}

#[derive(Subcommand)]
enum MailCommands {
    List(MailArgs),
    TakeMoney(MailIdArgs),
    TakeAttachment(MailAttachmentArgs),
}

fn mail_action(command: MailCommands) -> CliAction {
    let (args, operation) = match command {
        MailCommands::List(a) => (a, EconomyOperation::MailList),
        MailCommands::TakeMoney(a) => (
            a.context,
            EconomyOperation::MailTakeMoney { mail_id: a.mail_id },
        ),
        MailCommands::TakeAttachment(a) => (
            a.mail.context,
            EconomyOperation::MailTakeAttachment {
                mail_id: a.mail.mail_id,
                slot: a.slot,
            },
        ),
    };
    action_from_token(
        args.auth,
        CliCommand::Economy(EconomyAction {
            npc: args.object,
            gossip_option: None,
            operation,
        }),
    )
}

#[derive(Parser)]
struct SearchArgs {
    #[command(flatten)]
    context: EconomyArgs,
    #[arg(long, default_value = "")]
    text: String,
    /// Zero-based page index
    #[arg(long, default_value_t = 0)]
    page: u32,
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(1..=100))]
    page_size: u32,
    #[arg(long, value_parser = ["name", "min-bid", "buyout", "time-left", "quality", "required-level"], default_value = "name")]
    sort: String,
    #[arg(long, value_parser = ["asc", "desc"], default_value = "asc")]
    direction: String,
    #[arg(long)]
    item_id: Option<u32>,
    /// Retail item class ID
    #[arg(long)]
    class_id: Option<u8>,
    #[arg(long)]
    quality: Option<u8>,
    #[arg(long)]
    min_level: Option<u16>,
    #[arg(long)]
    max_level: Option<u16>,
    #[arg(long)]
    usable_only: bool,
}

#[derive(Parser)]
struct AuctionCreateArgs {
    #[command(flatten)]
    context: EconomyArgs,
    #[arg(long)]
    item_guid: u64,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    count: u32,
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    min_bid: u64,
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    buyout: Option<u64>,
    #[arg(long, value_parser = ["short", "medium", "long"], default_value = "long")]
    duration: String,
}

#[derive(Parser)]
struct BidArgs {
    #[command(flatten)]
    context: EconomyArgs,
    #[arg(long)]
    auction_id: u64,
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    amount: u64,
}

#[derive(Parser)]
struct AuctionIdArgs {
    #[command(flatten)]
    context: EconomyArgs,
    #[arg(long)]
    auction_id: u64,
}

#[derive(Parser)]
struct MerchantBuyArgs {
    #[command(flatten)]
    context: EconomyArgs,
    #[arg(long)]
    slot: u32,
    #[arg(long)]
    item_id: u32,
    /// Number of purchases (each yields vendor stack_count items)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    count: u32,
}

fn economy_action(args: EconomyArgs, operation: EconomyOperation) -> CliAction {
    action_from_token(
        args.auth,
        CliCommand::Economy(EconomyAction {
            npc: args.npc,
            gossip_option: args.gossip_option,
            operation,
        }),
    )
}

fn ah_action(command: AhCommands) -> CliAction {
    let grouped = matches!(&command, AhCommands::Browse(_));
    match command {
        AhCommands::Open(a) => economy_action(a, EconomyOperation::Open),
        AhCommands::Inventory(a) => economy_action(a, EconomyOperation::Inventory),
        AhCommands::Owned(a) => economy_action(a, EconomyOperation::Owned),
        AhCommands::Bids(a) => economy_action(a, EconomyOperation::Bids),
        AhCommands::Search(a) | AhCommands::Browse(a) => {
            let sort_field = match a.sort.as_str() {
                "min-bid" => AuctionSortField::MinBid,
                "buyout" => AuctionSortField::Buyout,
                "time-left" => AuctionSortField::TimeLeft,
                "quality" => AuctionSortField::Quality,
                "required-level" => AuctionSortField::RequiredLevel,
                "name" => AuctionSortField::Name,
                _ => unreachable!("clap validates sort"),
            };
            let query = AuctionSearchQuery {
                text: a.text,
                page: a.page,
                page_size: a.page_size,
                sort_field,
                sort_dir: if a.direction == "desc" {
                    AuctionSortDir::Desc
                } else {
                    AuctionSortDir::Asc
                },
                item_id: a.item_id,
                class_id: a.class_id,
                quality: a.quality,
                min_level: a.min_level,
                max_level: a.max_level,
                usable_only: a.usable_only,
                ..Default::default()
            };
            economy_action(
                a.context,
                if grouped {
                    EconomyOperation::Browse(query)
                } else {
                    EconomyOperation::Search(query)
                },
            )
        }
        AhCommands::Create(a) => economy_action(
            a.context,
            EconomyOperation::Create(CreateAuction {
                item_guid: a.item_guid,
                stack_count: a.count,
                min_bid: a.min_bid,
                buyout_price: a.buyout,
                duration: match a.duration.as_str() {
                    "short" => AuctionDuration::Short,
                    "medium" => AuctionDuration::Medium,
                    "long" => AuctionDuration::Long,
                    _ => unreachable!("clap validates duration"),
                },
            }),
        ),
        AhCommands::Bid(a) => economy_action(
            a.context,
            EconomyOperation::Bid(PlaceBid {
                auction_id: a.auction_id,
                amount: a.amount,
            }),
        ),
        AhCommands::Buyout(a) => economy_action(
            a.context,
            EconomyOperation::Buyout(BuyoutAuction {
                auction_id: a.auction_id,
            }),
        ),
        AhCommands::Cancel(a) => economy_action(
            a.context,
            EconomyOperation::Cancel(CancelAuction {
                auction_id: a.auction_id,
            }),
        ),
    }
}

fn merchant_action(command: MerchantCommands) -> CliAction {
    match command {
        MerchantCommands::List(a) => economy_action(a, EconomyOperation::MerchantList),
        MerchantCommands::Buy(a) => economy_action(
            a.context,
            EconomyOperation::MerchantBuy {
                slot: a.slot,
                item_id: a.item_id,
                count: a.count,
            },
        ),
    }
}

fn main() {
    let cli = Cli::parse();
    let action = build_action(cli);
    std::process::exit(match client::run(action) {
        bevy::app::AppExit::Success => 0,
        bevy::app::AppExit::Error(code) => i32::from(code.get()),
    });
}

fn build_action(cli: Cli) -> CliAction {
    match cli.command {
        Commands::Ah(command) => ah_action(command),
        Commands::Merchant(command) => merchant_action(command),
        Commands::Mail(command) => mail_action(command),
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
    fn economy_cli_mail_requires_object_and_claim_identifiers() {
        for args in [
            vec!["mail", "list", "--object", "77"],
            vec![
                "mail",
                "take-money",
                "--object",
                "77",
                "--mail-id",
                "9000000000",
            ],
            vec![
                "mail",
                "take-attachment",
                "--object",
                "77",
                "--mail-id",
                "9",
                "--slot",
                "11",
            ],
        ] {
            let command = args[1];
            let cli = Cli::try_parse_from(std::iter::once("game-cli").chain(args)).unwrap();
            let CliCommand::Economy(action) = build_action(cli).command else {
                panic!("expected economy action");
            };
            assert_eq!(action.npc, 77);
            assert_eq!(action.gossip_option, None);
            match (command, action.operation) {
                ("list", EconomyOperation::MailList)
                | (
                    "take-money",
                    EconomyOperation::MailTakeMoney {
                        mail_id: 9_000_000_000,
                    },
                )
                | (
                    "take-attachment",
                    EconomyOperation::MailTakeAttachment {
                        mail_id: 9,
                        slot: 11,
                    },
                ) => {}
                _ => panic!("incorrect parsed mail operation"),
            }
        }
        for args in [
            vec!["mail", "list", "--npc", "77"],
            vec!["mail", "take-money", "--object", "77"],
            vec![
                "mail",
                "take-attachment",
                "--object",
                "77",
                "--mail-id",
                "9",
            ],
            vec![
                "mail",
                "take-attachment",
                "--object",
                "77",
                "--mail-id",
                "9",
                "--slot",
                "256",
            ],
        ] {
            assert!(Cli::try_parse_from(std::iter::once("game-cli").chain(args)).is_err());
        }
    }

    #[test]
    fn economy_cli_browse_and_search_preserve_all_filters() {
        for command in ["browse", "search"] {
            let cli = Cli::try_parse_from([
                "game-cli",
                "ah",
                command,
                "--npc",
                "77",
                "--token",
                "fixture",
                "--gossip-option",
                "3",
                "--text",
                "Linen",
                "--page",
                "1",
                "--page-size",
                "2",
                "--sort",
                "buyout",
                "--direction",
                "desc",
                "--item-id",
                "2589",
                "--class-id",
                "7",
                "--quality",
                "1",
                "--min-level",
                "10",
                "--max-level",
                "80",
                "--usable-only",
            ])
            .unwrap();
            let CliCommand::Economy(action) = build_action(cli).command else {
                panic!("expected economy action");
            };
            assert_eq!(action.npc, 77);
            assert_eq!(action.gossip_option, Some(3));
            let query = match (command, action.operation) {
                ("browse", EconomyOperation::Browse(query)) => query,
                ("search", EconomyOperation::Search(query)) => query,
                _ => panic!("wrong operation"),
            };
            assert_eq!(
                query,
                AuctionSearchQuery {
                    text: "Linen".into(),
                    page: 1,
                    page_size: 2,
                    sort_field: AuctionSortField::Buyout,
                    sort_dir: AuctionSortDir::Desc,
                    item_id: Some(2589),
                    class_id: Some(7),
                    quality: Some(1),
                    min_level: Some(10),
                    max_level: Some(80),
                    usable_only: true,
                    ..Default::default()
                }
            );
        }
    }

    #[test]
    fn economy_cli_browse_defaults_and_invalid_controls() {
        let cli = Cli::try_parse_from(["game-cli", "ah", "browse", "--npc", "77"]).unwrap();
        let CliCommand::Economy(action) = build_action(cli).command else {
            panic!("expected economy action");
        };
        let EconomyOperation::Browse(query) = action.operation else {
            panic!("expected grouped browse");
        };
        assert_eq!(query, AuctionSearchQuery::default());
        for (flag, value) in [
            ("--page-size", "0"),
            ("--page-size", "101"),
            ("--quality", "256"),
            ("--min-level", "65536"),
            ("--max-level", "65536"),
            ("--sort", "unknown"),
            ("--direction", "unknown"),
        ] {
            assert!(
                Cli::try_parse_from(["game-cli", "ah", "browse", "--npc", "77", flag, value])
                    .is_err()
            );
        }
    }

    #[test]
    fn economy_cli_accepts_actions_and_search_controls() {
        for args in [
            vec!["ah", "open"],
            vec!["ah", "inventory"],
            vec!["ah", "owned"],
            vec!["ah", "bids"],
            vec![
                "ah",
                "search",
                "--text",
                "Linen",
                "--page",
                "1",
                "--page-size",
                "2",
                "--sort",
                "buyout",
                "--direction",
                "desc",
            ],
            vec![
                "ah",
                "create",
                "--item-guid",
                "42",
                "--count",
                "2",
                "--min-bid",
                "100",
                "--buyout",
                "200",
                "--duration",
                "long",
            ],
            vec!["ah", "bid", "--auction-id", "9", "--amount", "150"],
            vec!["ah", "buyout", "--auction-id", "9"],
            vec!["ah", "cancel", "--auction-id", "9"],
            vec!["merchant", "list"],
            vec![
                "merchant",
                "buy",
                "--slot",
                "0",
                "--item-id",
                "123",
                "--count",
                "2",
            ],
        ] {
            let mut argv = vec!["game-cli"];
            argv.extend(args);
            argv.extend(["--npc", "77", "--token", "fixture"]);
            assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
        }
    }

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
