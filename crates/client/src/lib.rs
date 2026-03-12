//! Headless lightyear client library for the game server.
//!
//! Provides connection setup, auth flow, and message handling
//! as a minimal Bevy app (no rendering).

use std::time::Duration;

use bevy::app::AppExit;
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

const TICK_RATE_HZ: f64 = 20.0;
const AUTH_TOKEN_PATH: &str = "data/auth_token";

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

// -- Auth token persistence --

fn load_auth_token() -> Option<String> {
    std::fs::read_to_string(AUTH_TOKEN_PATH)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn save_auth_token(token: &str) {
    let _ = std::fs::create_dir_all("data");
    if let Err(e) = std::fs::write(AUTH_TOKEN_PATH, token) {
        warn!("Failed to save auth token: {e}");
    }
}

// -- Connection (same pattern as game-engine) --

fn connect_to_server(mut commands: Commands, action: Res<CliAction>) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let client_id = rand_client_id();
    let auth = Authentication::Manual {
        server_addr: action.server,
        client_id,
        private_key: [0; 32],
        protocol_id: 0,
    };
    let netcode = match NetcodeClient::new(auth, NetcodeConfig::default()) {
        Ok(nc) => nc,
        Err(e) => {
            error!("Failed to create netcode client: {e}");
            return;
        }
    };
    let entity = commands
        .spawn((
            LocalAddr(bind_addr),
            PeerAddr(action.server),
            UdpIo::default(),
            netcode,
        ))
        .id();
    commands.trigger(Connect { entity });
    println!("[cli] Connecting to {}...", action.server);
}

fn on_link_established(trigger: On<Add, Connected>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationReceiver::default());
}

// -- On connected: send auth request --

fn on_connected(
    _trigger: On<Add, Connected>,
    action: Res<CliAction>,
    auth_token: Res<AuthToken>,
    mut next_state: ResMut<NextState<CliState>>,
) {
    println!("[cli] Connected to {}", action.server);
    if needs_token(&action.command) && effective_token(&action, &auth_token).is_none() {
        eprintln!("[cli] No auth token available. Pass --token or login first.");
        std::process::exit(1);
    }
    next_state.set(CliState::Authenticating);
}

fn send_auth_request_when_ready(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    auth_token: Res<AuthToken>,
    mut sent: ResMut<AuthRequestSent>,
    mut login_senders: Query<&mut MessageSender<LoginRequest>>,
    mut register_senders: Query<&mut MessageSender<RegisterRequest>>,
) {
    if *state.get() != CliState::Authenticating {
        return;
    }
    if sent.0 {
        return;
    }

    let sent_now = match &action.command {
        CliCommand::Register => send_register_request(&action, &mut register_senders),
        _ => send_login_request(&action, &auth_token, &mut login_senders),
    };

    if sent_now {
        sent.0 = true;
    }
}

fn send_login_request(
    action: &CliAction,
    auth_token: &AuthToken,
    senders: &mut Query<&mut MessageSender<LoginRequest>>,
) -> bool {
    let token = effective_token(action, auth_token);
    let (username, password) = if matches!(action.command, CliCommand::Login) {
        (
            action.username.clone().unwrap_or_default(),
            action.password.clone().unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    };

    let request = LoginRequest {
        token,
        username,
        password,
    };
    let mut sent_count = 0;
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
        sent_count += 1;
    }

    if sent_count == 0 {
        return false;
    }

    if matches!(action.command, CliCommand::Login) {
        let username = action.username.as_deref().unwrap_or("<missing>");
        println!("[cli] Sent login request for '{username}'");
    } else {
        println!("[cli] Sent token login request");
    }

    true
}

fn send_register_request(
    action: &CliAction,
    senders: &mut Query<&mut MessageSender<RegisterRequest>>,
) -> bool {
    let Some(username) = action.username.as_ref() else {
        eprintln!("[register] Missing username");
        std::process::exit(1);
    };
    let Some(password) = action.password.as_ref() else {
        eprintln!("[register] Missing password");
        std::process::exit(1);
    };

    let request = RegisterRequest {
        username: username.clone(),
        password: password.clone(),
    };
    let mut sent_count = 0;
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
        sent_count += 1;
    }

    if sent_count == 0 {
        return false;
    }

    println!("[cli] Sent register request for '{}'", username);

    true
}

// -- Response handlers --

fn receive_login_response(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut char_list: ResMut<CharacterList>,
    mut next_state: ResMut<NextState<CliState>>,
    action: Res<CliAction>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut history_senders: Query<&mut MessageSender<ChatHistoryRequest>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_login_resp(
                resp,
                &mut auth_token,
                &mut char_list,
                &mut next_state,
                &action,
                &mut senders,
                &mut create_senders,
                &mut history_senders,
                &mut exit,
            );
        }
    }
}

fn handle_login_resp(
    resp: LoginResponse,
    auth_token: &mut AuthToken,
    char_list: &mut CharacterList,
    next_state: &mut NextState<CliState>,
    action: &CliAction,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    history_senders: &mut Query<&mut MessageSender<ChatHistoryRequest>>,
    exit: &mut MessageWriter<AppExit>,
) {
    if !resp.success {
        eprintln!("[login] Failed: {}", resp.error.unwrap_or_default());
        exit.write(AppExit::Error(1.try_into().unwrap()));
        return;
    }
    save_auth_token(&resp.token);
    auth_token.0 = Some(resp.token);
    char_list.0 = resp.characters;
    print_characters(&char_list.0);
    after_login(
        action,
        char_list,
        next_state,
        senders,
        create_senders,
        history_senders,
        exit,
    );
}

fn print_characters(chars: &[shared::protocol::CharacterListEntry]) {
    println!("[login] Characters:");
    if chars.is_empty() {
        println!("  (none)");
        return;
    }
    for (i, ch) in chars.iter().enumerate() {
        println!(
            "  {}. {} (Level {} Race:{} Class:{})",
            i + 1,
            ch.name,
            ch.level,
            ch.race,
            ch.class
        );
    }
}

fn after_login(
    action: &CliAction,
    char_list: &CharacterList,
    next_state: &mut NextState<CliState>,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    history_senders: &mut Query<&mut MessageSender<ChatHistoryRequest>>,
    exit: &mut MessageWriter<AppExit>,
) {
    match &action.command {
        CliCommand::Login | CliCommand::Status => {
            next_state.set(CliState::Done);
            exit.write(AppExit::Success);
        }
        CliCommand::CreateCharacter {
            name,
            race,
            class,
            appearance,
        } => {
            send_create_character_request(name, *race, *class, *appearance, create_senders, exit);
        }
        CliCommand::ChatHistory { limit } => {
            send_chat_history_request(*limit, history_senders, exit);
        }
        CliCommand::ChatSend(_) | CliCommand::ChatListen | CliCommand::GuildChat(_) => {
            select_first_character(char_list, next_state, senders, exit);
        }
        CliCommand::GuildInvite { .. } | CliCommand::GuildAccept => {
            select_first_character(char_list, next_state, senders, exit);
        }
        CliCommand::Register => unreachable!(),
    }
}

fn send_chat_history_request(
    limit: u16,
    senders: &mut Query<&mut MessageSender<ChatHistoryRequest>>,
    exit: &mut MessageWriter<AppExit>,
) {
    let req = ChatHistoryRequest { limit };
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(req.clone());
        sent = true;
    }
    if !sent {
        eprintln!("[history] Unable to send history request");
        exit.write(AppExit::Error(1.try_into().unwrap()));
    }
}

fn send_create_character_request(
    name: &str,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    exit: &mut MessageWriter<AppExit>,
) {
    let request = CreateCharacter {
        name: name.to_string(),
        race,
        class,
        appearance,
    };
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
        sent = true;
    }

    if sent {
        println!("[cli] Requested character creation: '{name}'");
    } else {
        eprintln!("[cli] Unable to send create-character request");
        exit.write(AppExit::Error(1.try_into().unwrap()));
    }
}

fn select_first_character(
    char_list: &CharacterList,
    next_state: &mut NextState<CliState>,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    exit: &mut MessageWriter<AppExit>,
) {
    let Some(first) = char_list.0.first() else {
        eprintln!("[cli] No characters to select");
        exit.write(AppExit::Error(1.try_into().unwrap()));
        return;
    };
    let msg = SelectCharacter {
        character_id: first.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    println!("[cli] Selected character '{}'", first.name);
    next_state.set(CliState::SelectingChar);
}

fn receive_register_response(
    mut receivers: Query<&mut MessageReceiver<RegisterResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                save_auth_token(&resp.token);
                auth_token.0 = Some(resp.token);
                println!("[register] Success! Account created.");
            } else if resp.pending_approval {
                println!("[register] Registration submitted and pending admin approval.");
            } else {
                eprintln!("[register] Failed: {}", resp.error.unwrap_or_default());
            }
            exit.write(AppExit::Success);
        }
    }
}

fn receive_create_character_response(
    mut receivers: Query<&mut MessageReceiver<CreateCharacterResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                if let Some(ch) = resp.character {
                    println!(
                        "[create] Created character '{}' (id={}, race={}, class={})",
                        ch.name, ch.character_id, ch.race, ch.class
                    );
                } else {
                    println!("[create] Character created");
                }
            } else {
                eprintln!("[create] Failed: {}", resp.error.unwrap_or_default());
            }
            exit.write(AppExit::Success);
        }
    }
}

fn receive_enter_world_response(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut selected: ResMut<SelectedCharacterId>,
    mut next_state: ResMut<NextState<CliState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                selected.0 = resp.player_entity;
                println!("[cli] Entered world (entity: {:?})", resp.player_entity);
                next_state.set(CliState::InWorld);
            } else {
                eprintln!(
                    "[cli] Enter world failed: {}",
                    resp.error.unwrap_or_default()
                );
                exit.write(AppExit::Error(1.try_into().unwrap()));
            }
        }
    }
}

// -- Chat systems --

/// When entering InWorld with an action command, send it once.
fn send_action_on_enter_world(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut chat_senders: Query<&mut MessageSender<ChatMessage>>,
    mut invite_senders: Query<&mut MessageSender<GuildInviteRequest>>,
    mut accept_senders: Query<&mut MessageSender<GuildAcceptInviteRequest>>,
    mut sent: Local<bool>,
) {
    if *state.get() != CliState::InWorld || *sent {
        return;
    }
    match &action.command {
        CliCommand::ChatSend(text) => {
            let msg = ChatMessage {
                sender: action.username.clone().unwrap_or_else(|| "cli".to_string()),
                content: text.clone(),
                channel: ChatType::Say,
            };
            for mut sender in chat_senders.iter_mut() {
                sender.send::<ChatChannel>(msg.clone());
            }
            println!("[chat] Sent: {text}");
        }
        CliCommand::GuildChat(text) => {
            let msg = ChatMessage {
                sender: action.username.clone().unwrap_or_else(|| "cli".to_string()),
                content: text.clone(),
                channel: ChatType::Guild,
            };
            for mut sender in chat_senders.iter_mut() {
                sender.send::<ChatChannel>(msg.clone());
            }
            println!("[chat] Sent: {text}");
        }
        CliCommand::GuildInvite { target } => {
            let req = GuildInviteRequest {
                target_character_name: target.clone(),
            };
            for mut sender in invite_senders.iter_mut() {
                sender.send::<AuthChannel>(req.clone());
            }
            println!("[guild] Invite sent to '{target}'");
        }
        CliCommand::GuildAccept => {
            for mut sender in accept_senders.iter_mut() {
                sender.send::<AuthChannel>(GuildAcceptInviteRequest);
            }
            println!("[guild] Accept request sent");
        }
        _ => return,
    }
    *sent = true;
}

fn receive_guild_invite_response(
    mut receivers: Query<&mut MessageReceiver<GuildInviteResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                println!("[guild] Invite queued");
            } else {
                eprintln!("[guild] Invite failed: {}", resp.error.unwrap_or_default());
            }
            exit.write(AppExit::Success);
        }
    }
}

fn receive_guild_accept_response(
    mut receivers: Query<&mut MessageReceiver<GuildAcceptInviteResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                println!(
                    "[guild] Joined guild '{}' ({})",
                    resp.guild_name.unwrap_or_default(),
                    resp.guild_id.unwrap_or_default()
                );
            } else {
                eprintln!("[guild] Accept failed: {}", resp.error.unwrap_or_default());
            }
            exit.write(AppExit::Success);
        }
    }
}

fn receive_chat_history_response(
    mut receivers: Query<&mut MessageReceiver<ChatHistoryResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if let Some(err) = resp.error {
                eprintln!("[history] Failed: {err}");
                exit.write(AppExit::Error(1.try_into().unwrap()));
                continue;
            }
            for msg in resp.messages {
                let prefix = if msg.channel == ChatType::Guild {
                    "Guild"
                } else {
                    "chat"
                };
                println!("[{prefix}] <{}> {}", msg.sender, msg.content);
            }
            exit.write(AppExit::Success);
        }
    }
}

fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    action: Res<CliAction>,
    mut exit: MessageWriter<AppExit>,
    mut received_count: Local<u32>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let prefix = if msg.channel == ChatType::Guild {
                "Guild"
            } else {
                "chat"
            };
            println!("[{prefix}] <{}> {}", msg.sender, msg.content);
            *received_count += 1;
            // For ChatSend/GuildChat, exit after receiving the echo
            if matches!(
                action.command,
                CliCommand::ChatSend(_) | CliCommand::GuildChat(_)
            ) {
                exit.write(AppExit::Success);
            }
        }
    }
}

// -- Timeout --

fn timeout_system(
    start: Res<StartTime>,
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = start.0.elapsed().as_secs();
    // ChatListen stays connected indefinitely
    if matches!(action.command, CliCommand::ChatListen) && *state.get() == CliState::InWorld {
        return;
    }
    if elapsed > 10 {
        eprintln!("[cli] Timeout after 10s (state: {:?})", state.get());
        exit.write(AppExit::Error(1.try_into().unwrap()));
    }
}

// -- Helpers --

fn rand_client_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn needs_token(command: &CliCommand) -> bool {
    !matches!(command, CliCommand::Login | CliCommand::Register)
}

fn effective_token(action: &CliAction, auth_token: &AuthToken) -> Option<String> {
    action.token.clone().or_else(|| auth_token.0.clone())
}
