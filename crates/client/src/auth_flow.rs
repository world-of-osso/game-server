use super::*;

pub(super) fn load_auth_token() -> Option<String> {
    std::fs::read_to_string(auth_token_path())
        .ok()
        .filter(|content| !content.trim().is_empty())
}

fn save_auth_token(token: &str) {
    let path = auth_token_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(&path, token) {
        warn!("Failed to save auth token: {error}");
    }
}

pub(super) fn connect_to_server(mut commands: Commands, action: Res<CliAction>) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let auth = Authentication::Manual {
        server_addr: action.server,
        client_id: rand_client_id(),
        private_key: [0; 32],
        protocol_id: 0,
    };
    let netcode = match NetcodeClient::new(auth, NetcodeConfig::default()) {
        Ok(client) => client,
        Err(error) => {
            error!("Failed to create netcode client: {error}");
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

pub(super) fn on_link_established(trigger: On<Add, Connected>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(ReplicationReceiver);
}

pub(super) fn on_connected(
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

pub(super) fn send_auth_request_when_ready(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    auth_token: Res<AuthToken>,
    mut sent: ResMut<AuthRequestSent>,
    mut login_senders: Query<&mut MessageSender<LoginRequest>>,
    mut register_senders: Query<&mut MessageSender<RegisterRequest>>,
) {
    if *state.get() != CliState::Authenticating || sent.0 {
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

pub(super) fn save_login_token(auth_token: &mut AuthToken, token: &str) {
    save_auth_token(token);
    auth_token.0 = Some(token.to_string());
}

fn send_login_request(
    action: &CliAction,
    auth_token: &AuthToken,
    senders: &mut Query<&mut MessageSender<LoginRequest>>,
) -> bool {
    let request = LoginRequest {
        token: effective_token(action, auth_token),
        username: login_username(action),
        password: login_password(action),
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

fn login_username(action: &CliAction) -> String {
    if matches!(action.command, CliCommand::Login) {
        return action.username.clone().unwrap_or_default();
    }
    String::new()
}

fn login_password(action: &CliAction) -> String {
    if matches!(action.command, CliCommand::Login) {
        return action.password.clone().unwrap_or_default();
    }
    String::new()
}

fn send_register_request(
    action: &CliAction,
    senders: &mut Query<&mut MessageSender<RegisterRequest>>,
) -> bool {
    let username = require_credential(action.username.as_ref(), "username");
    let password = require_credential(action.password.as_ref(), "password");
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

fn require_credential<'a>(value: Option<&'a String>, label: &str) -> &'a String {
    let Some(value) = value else {
        eprintln!("[register] Missing {label}");
        std::process::exit(1);
    };
    value
}

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
