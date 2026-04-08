use super::*;
use crate::auth_flow::save_login_token;

pub(super) fn receive_login_response(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut char_list: ResMut<CharacterList>,
    mut next_state: ResMut<NextState<CliState>>,
    action: Res<CliAction>,
    mut login_senders: LoginSenders,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            handle_login_response(
                response,
                &mut auth_token,
                &mut char_list,
                &mut next_state,
                &action,
                &mut login_senders,
            );
        }
    }
}

fn handle_login_response(
    response: LoginResponse,
    auth_token: &mut AuthToken,
    char_list: &mut CharacterList,
    next_state: &mut NextState<CliState>,
    action: &CliAction,
    login_senders: &mut LoginSenders,
) {
    if !response.success {
        eprintln!("[login] Failed: {}", response.error.unwrap_or_default());
        login_senders
            .exit
            .write(AppExit::Error(1.try_into().unwrap()));
        return;
    }

    save_login_token(auth_token, &response.token);
    char_list.0 = response.characters;
    print_characters(&char_list.0);
    after_login(
        action,
        char_list,
        next_state,
        &mut login_senders.senders,
        &mut login_senders.create_senders,
        &mut login_senders.history_senders,
        &mut login_senders.exit,
    );
}

fn print_characters(chars: &[shared::protocol::CharacterListEntry]) {
    println!("[login] Characters:");
    if chars.is_empty() {
        println!("  (none)");
        return;
    }
    for (index, character) in chars.iter().enumerate() {
        println!(
            "  {}. {} (Level {} Race:{} Class:{})",
            index + 1,
            character.name,
            character.level,
            character.race,
            character.class
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
        } => send_create_character_request(name, *race, *class, *appearance, create_senders, exit),
        CliCommand::ChatHistory { limit } => {
            send_chat_history_request(*limit, history_senders, exit);
        }
        CliCommand::ChatSend(_)
        | CliCommand::ChatListen
        | CliCommand::GuildChat(_)
        | CliCommand::GuildInvite { .. }
        | CliCommand::GuildAccept => {
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
    let request = ChatHistoryRequest { limit };
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
        sent = true;
    }
    if sent {
        return;
    }
    eprintln!("[history] Unable to send history request");
    exit.write(AppExit::Error(1.try_into().unwrap()));
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
    if !sent {
        eprintln!("[cli] Unable to send create-character request");
        exit.write(AppExit::Error(1.try_into().unwrap()));
        return;
    }
    println!("[cli] Requested character creation: '{name}'");
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
    let message = SelectCharacter {
        character_id: first.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(message.clone());
    }
    println!("[cli] Selected character '{}'", first.name);
    next_state.set(CliState::SelectingChar);
}

pub(super) fn receive_register_response(
    mut receivers: Query<&mut MessageReceiver<RegisterResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if response.success {
                save_login_token(&mut auth_token, &response.token);
                println!("[register] Success! Account created.");
            } else if response.pending_approval {
                println!("[register] Registration submitted and pending admin approval.");
            } else {
                eprintln!("[register] Failed: {}", response.error.unwrap_or_default());
                exit.write(AppExit::Error(1.try_into().unwrap()));
                continue;
            }
            exit.write(AppExit::Success);
        }
    }
}

pub(super) fn receive_create_character_response(
    mut receivers: Query<&mut MessageReceiver<CreateCharacterResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if !response.success {
                eprintln!("[create] Failed: {}", response.error.unwrap_or_default());
                exit.write(AppExit::Error(1.try_into().unwrap()));
                continue;
            }
            match response.character {
                Some(character) => println!(
                    "[create] Created character '{}' (id={}, race={}, class={})",
                    character.name, character.character_id, character.race, character.class
                ),
                None => println!("[create] Character created"),
            }
            exit.write(AppExit::Success);
        }
    }
}

pub(super) fn receive_enter_world_response(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut selected: ResMut<SelectedCharacterId>,
    mut next_state: ResMut<NextState<CliState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if response.success {
                selected.0 = response.player_entity;
                println!("[cli] Entered world (entity: {:?})", response.player_entity);
                next_state.set(CliState::InWorld);
                continue;
            }
            eprintln!(
                "[cli] Enter world failed: {}",
                response.error.unwrap_or_default()
            );
            exit.write(AppExit::Error(1.try_into().unwrap()));
        }
    }
}

pub(super) fn send_action_on_enter_world(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut senders: EnterWorldSenders,
    mut sent: Local<bool>,
) {
    if *state.get() != CliState::InWorld || *sent {
        return;
    }
    if !dispatch_enter_world_action(&action, &mut senders) {
        return;
    }
    *sent = true;
}

pub(super) fn receive_guild_invite_response(
    mut receivers: Query<&mut MessageReceiver<GuildInviteResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if response.success {
                println!("[guild] Invite queued");
                exit.write(AppExit::Success);
            } else {
                eprintln!(
                    "[guild] Invite failed: {}",
                    response.error.unwrap_or_default()
                );
                exit.write(AppExit::Error(1.try_into().unwrap()));
            }
        }
    }
}

pub(super) fn receive_guild_accept_response(
    mut receivers: Query<&mut MessageReceiver<GuildAcceptInviteResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if response.success {
                println!(
                    "[guild] Joined guild '{}' ({})",
                    response.guild_name.unwrap_or_default(),
                    response.guild_id.unwrap_or_default()
                );
                exit.write(AppExit::Success);
            } else {
                eprintln!(
                    "[guild] Accept failed: {}",
                    response.error.unwrap_or_default()
                );
                exit.write(AppExit::Error(1.try_into().unwrap()));
            }
        }
    }
}

pub(super) fn receive_chat_history_response(
    mut receivers: Query<&mut MessageReceiver<ChatHistoryResponse>>,
    mut exit: MessageWriter<AppExit>,
) {
    for mut receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            if let Some(error) = response.error {
                eprintln!("[history] Failed: {error}");
                exit.write(AppExit::Error(1.try_into().unwrap()));
                continue;
            }
            for message in response.messages {
                println!(
                    "[{}] <{}> {}",
                    channel_prefix(message.channel),
                    message.sender,
                    message.content
                );
            }
            exit.write(AppExit::Success);
        }
    }
}

pub(super) fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    action: Res<CliAction>,
    mut exit: MessageWriter<AppExit>,
    mut received_count: Local<u32>,
) {
    for mut receiver in receivers.iter_mut() {
        for message in receiver.receive() {
            println!(
                "[{}] <{}> {}",
                channel_prefix(message.channel),
                message.sender,
                message.content
            );
            *received_count += 1;
            if matches!(
                action.command,
                CliCommand::ChatSend(_) | CliCommand::GuildChat(_)
            ) {
                exit.write(AppExit::Success);
            }
        }
    }
}

pub(super) fn timeout_system(
    start: Res<StartTime>,
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = start.0.elapsed().as_secs();
    if matches!(action.command, CliCommand::ChatListen) && *state.get() == CliState::InWorld {
        return;
    }
    if elapsed > 10 {
        eprintln!("[cli] Timeout after 10s (state: {:?})", state.get());
        exit.write(AppExit::Error(1.try_into().unwrap()));
    }
}

fn channel_prefix(channel: ChatType) -> &'static str {
    if channel == ChatType::Guild {
        "Guild"
    } else {
        "chat"
    }
}
