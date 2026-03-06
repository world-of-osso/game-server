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
use shared::protocol::{
    AuthChannel, ChatChannel, ChatMessage, ChatType, EnterWorldResponse, LoginRequest,
    LoginResponse, RegisterRequest, RegisterResponse, SelectCharacter,
};

const TICK_RATE_HZ: f64 = 20.0;
const AUTH_TOKEN_PATH: &str = "data/auth_token";

// -- CLI action types --

/// What the CLI binary wants to do after connecting.
#[derive(Clone, Debug)]
pub enum CliCommand {
    Login,
    Register,
    ChatSend(String),
    ChatListen,
    GuildChat(String),
}

/// Resource describing the CLI action to perform.
#[derive(Resource, Clone)]
pub struct CliAction {
    pub command: CliCommand,
    pub server: SocketAddr,
    pub username: String,
    pub password: String,
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
    app.add_plugins(MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
        Duration::from_millis(10),
    )));
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
    app.init_resource::<CharacterList>();
    app.init_resource::<SelectedCharacterId>();
    app.insert_resource(StartTime(std::time::Instant::now()));
}

fn register_systems(app: &mut App) {
    app.add_systems(Startup, connect_to_server);
    app.add_systems(
        Update,
        (
            receive_login_response,
            receive_register_response,
            receive_enter_world_response,
            receive_chat_messages,
            send_chat_on_enter_world,
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

fn on_link_established(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationReceiver::default());
}

// -- On connected: send auth request --

fn on_connected(
    _trigger: On<Add, Connected>,
    action: Res<CliAction>,
    auth_token: Res<AuthToken>,
    mut login_senders: Query<&mut MessageSender<LoginRequest>>,
    mut register_senders: Query<&mut MessageSender<RegisterRequest>>,
    mut next_state: ResMut<NextState<CliState>>,
) {
    println!("[cli] Connected to {}", action.server);
    match &action.command {
        CliCommand::Register => {
            send_register_request(&action, &mut register_senders);
        }
        _ => {
            send_login_request(&action, &auth_token, &mut login_senders);
        }
    }
    next_state.set(CliState::Authenticating);
}

fn send_login_request(
    action: &CliAction,
    auth_token: &AuthToken,
    senders: &mut Query<&mut MessageSender<LoginRequest>>,
) {
    let request = LoginRequest {
        token: auth_token.0.clone(),
        username: action.username.clone(),
        password: action.password.clone(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    println!("[cli] Sent login request for '{}'", action.username);
}

fn send_register_request(
    action: &CliAction,
    senders: &mut Query<&mut MessageSender<RegisterRequest>>,
) {
    let request = RegisterRequest {
        username: action.username.clone(),
        password: action.password.clone(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    println!("[cli] Sent register request for '{}'", action.username);
}

// -- Response handlers --

fn receive_login_response(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut char_list: ResMut<CharacterList>,
    mut next_state: ResMut<NextState<CliState>>,
    action: Res<CliAction>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
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
    after_login(action, char_list, next_state, senders, exit);
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
    exit: &mut MessageWriter<AppExit>,
) {
    match &action.command {
        CliCommand::Login => {
            next_state.set(CliState::Done);
            exit.write(AppExit::Success);
        }
        CliCommand::ChatSend(_) | CliCommand::ChatListen | CliCommand::GuildChat(_) => {
            select_first_character(char_list, next_state, senders, exit);
        }
        CliCommand::Register => unreachable!(),
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
            } else {
                eprintln!(
                    "[register] Failed: {}",
                    resp.error.unwrap_or_default()
                );
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

/// When entering InWorld with a ChatSend command, send the message and schedule exit.
fn send_chat_on_enter_world(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
    mut sent: Local<bool>,
) {
    if *state.get() != CliState::InWorld || *sent {
        return;
    }
    let (text, channel) = match &action.command {
        CliCommand::ChatSend(t) => (t, ChatType::Say),
        CliCommand::GuildChat(t) => (t, ChatType::Guild),
        _ => return,
    };
    let msg = ChatMessage {
        sender: action.username.clone(),
        content: text.clone(),
        channel,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
    }
    println!("[chat] Sent: {text}");
    *sent = true;
    // Give the server a moment to echo back, then exit on next timeout tick
    // We don't exit immediately so we can see the echo
}

fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    action: Res<CliAction>,
    mut exit: MessageWriter<AppExit>,
    mut received_count: Local<u32>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let prefix = if msg.channel == ChatType::Guild { "Guild" } else { "chat" };
            println!("[{prefix}] <{}> {}", msg.sender, msg.content);
            *received_count += 1;
            // For ChatSend/GuildChat, exit after receiving the echo
            if matches!(action.command, CliCommand::ChatSend(_) | CliCommand::GuildChat(_)) {
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
    if matches!(action.command, CliCommand::ChatListen) && *state.get() == CliState::InWorld
    {
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
