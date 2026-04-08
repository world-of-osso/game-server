use super::*;

pub(super) fn dispatch_enter_world_action(
    action: &CliAction,
    senders: &mut EnterWorldSenders,
) -> bool {
    match &action.command {
        CliCommand::ChatSend(text) => {
            send_chat_command(&mut senders.chat_senders, action, text, ChatType::Say);
            true
        }
        CliCommand::GuildChat(text) => {
            send_chat_command(&mut senders.chat_senders, action, text, ChatType::Guild);
            true
        }
        CliCommand::GuildInvite { target } => {
            send_guild_invite(&mut senders.invite_senders, target);
            true
        }
        CliCommand::GuildAccept => {
            send_guild_accept(&mut senders.accept_senders);
            true
        }
        _ => false,
    }
}

pub(super) fn send_chat_command(
    senders: &mut Query<&mut MessageSender<ChatMessage>>,
    action: &CliAction,
    text: &str,
    channel: ChatType,
) {
    let msg = ChatMessage {
        sender: cli_sender_name(action),
        content: text.to_string(),
        channel,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
    }
    println!("[chat] Sent: {text}");
}

pub(super) fn send_guild_invite(
    senders: &mut Query<&mut MessageSender<GuildInviteRequest>>,
    target: &str,
) {
    let req = GuildInviteRequest {
        target_character_name: target.to_string(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(req.clone());
    }
    println!("[guild] Invite sent to '{target}'");
}

pub(super) fn send_guild_accept(senders: &mut Query<&mut MessageSender<GuildAcceptInviteRequest>>) {
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(GuildAcceptInviteRequest);
    }
    println!("[guild] Accept request sent");
}

pub(super) fn cli_sender_name(action: &CliAction) -> String {
    action.username.clone().unwrap_or_else(|| "cli".to_string())
}
