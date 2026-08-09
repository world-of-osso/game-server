//! Headless lightyear client library for the game server.
//!
//! Provides connection setup, auth flow, and message handling
//! as a minimal Bevy app (no rendering).

use std::path::PathBuf;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::components::CharacterAppearance;
use shared::protocol::{
    AuthChannel, ChatChannel, ChatHistoryRequest, ChatHistoryResponse, ChatMessage, ChatType,
    CreateCharacter, CreateCharacterResponse, EnterWorldResponse, GuildAcceptInviteRequest,
    GuildAcceptInviteResponse, GuildInviteRequest, GuildInviteResponse, LoginRequest,
    LoginResponse, RegisterRequest, RegisterResponse, SelectCharacter,
};

mod auth_flow;
mod chat_actions;
mod response_handlers;

use self::auth_flow::{
    connect_to_server, load_auth_token, on_connected, on_link_established,
    send_auth_request_when_ready,
};
use self::chat_actions::dispatch_enter_world_action;
use self::response_handlers::{
    receive_chat_history_response, receive_chat_messages, receive_create_character_response,
    receive_enter_world_response, receive_guild_accept_response, receive_guild_invite_response,
    receive_login_response, receive_register_response, send_action_on_enter_world, timeout_system,
};

const TICK_RATE_HZ: f64 = 20.0;
const AUTH_TOKEN_FILE_NAME: &str = "auth_token";

// -- CLI action types --

/// What the CLI binary wants to do after connecting.
#[derive(Clone, Debug)]
pub enum CliCommand {
    Login,
    Status,
    Register,
    CreateCharacter {
        name: String,
        race: u8,
        class: u8,
        appearance: CharacterAppearance,
    },
    GuildInvite {
        target: String,
    },
    GuildAccept,
    ChatHistory {
        limit: u16,
    },
    ChatSend(String),
    ChatListen,
    GuildChat(String),
}

/// Resource describing the CLI action to perform.
#[derive(Resource, Clone)]
pub struct CliAction {
    pub command: CliCommand,
    pub server: SocketAddr,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

// -- State machine --

/// Tracks the CLI connection/auth flow.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliState {
    #[default]
    Connecting,
    Authenticating,
    SelectingChar,
    InWorld,
    Done,
}

// -- Internal resources --

#[derive(Resource, Default)]
struct AuthToken(Option<String>);

fn world_of_osso_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("world-of-osso")
}

fn auth_token_path() -> PathBuf {
    world_of_osso_config_dir().join(AUTH_TOKEN_FILE_NAME)
}

#[derive(Resource, Default)]
struct AuthRequestSent(bool);

#[derive(Resource, Default)]
struct CharacterList(Vec<shared::protocol::CharacterListEntry>);

#[derive(Resource, Default)]
struct SelectedCharacterId(Option<u64>);

/// Tracks elapsed time for timeout.
#[derive(Resource)]
struct StartTime(std::time::Instant);

// -- Public API --

/// Build and run the headless CLI app. Blocks until exit.
pub fn run(action: CliAction) {
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(10),
        )),
    );
    app.add_plugins(bevy::log::LogPlugin::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ClientPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.init_state::<CliState>();
    register_resources(&mut app, &action);
    register_systems(&mut app);
    register_observers(&mut app);
    app.run();
}

fn register_resources(app: &mut App, action: &CliAction) {
    app.insert_resource(action.clone());
    app.insert_resource(AuthToken(load_auth_token()));
    app.init_resource::<AuthRequestSent>();
    app.init_resource::<CharacterList>();
    app.init_resource::<SelectedCharacterId>();
    app.insert_resource(StartTime(std::time::Instant::now()));
}

fn register_systems(app: &mut App) {
    app.add_systems(Startup, connect_to_server);
    app.add_systems(FixedUpdate, send_auth_request_when_ready);
    app.add_systems(
        Update,
        (
            receive_login_response,
            receive_register_response,
            receive_create_character_response,
            receive_enter_world_response,
            receive_guild_invite_response,
            receive_guild_accept_response,
            receive_chat_history_response,
            receive_chat_messages,
            send_action_on_enter_world,
            timeout_system,
        ),
    );
}

fn register_observers(app: &mut App) {
    app.add_observer(on_connected);
    app.add_observer(on_link_established);
}

// -- Response handlers --

#[derive(SystemParam)]
struct LoginSenders<'w, 's> {
    senders: Query<'w, 's, &'static mut MessageSender<SelectCharacter>>,
    create_senders: Query<'w, 's, &'static mut MessageSender<CreateCharacter>>,
    history_senders: Query<'w, 's, &'static mut MessageSender<ChatHistoryRequest>>,
    exit: MessageWriter<'w, AppExit>,
}

#[derive(SystemParam)]
struct EnterWorldSenders<'w, 's> {
    chat_senders: Query<'w, 's, &'static mut MessageSender<ChatMessage>>,
    invite_senders: Query<'w, 's, &'static mut MessageSender<GuildInviteRequest>>,
    accept_senders: Query<'w, 's, &'static mut MessageSender<GuildAcceptInviteRequest>>,
}
