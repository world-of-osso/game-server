use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::GuildMembership;
use shared::protocol::{
    AuthChannel, ChatChannel, ChatMessage, ChatType, EmoteEvent, EmoteIntent, EmoteKind,
    GuildAcceptInviteRequest, GuildAcceptInviteResponse, GuildInviteRequest, GuildInviteResponse,
    SetTarget,
};

use crate::ignore_list::PlayerIgnoreListState;
use crate::networking::{ChatHistoryBuffer, CombatTarget, PlayerLink};
use crate::persistence::{self, ClientId, GameDatabase};

pub fn process_chat_messages(
    mut receivers: Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    mut emote_receivers: Query<(&mut MessageReceiver<EmoteIntent>, &PlayerLink)>,
    players: Query<&shared::components::Player>,
    guilds: Query<Option<&GuildMembership>>,
    ignore_lists: Query<Option<&PlayerIgnoreListState>>,
    mut chat_senders: Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    mut emote_senders: Query<(&PlayerLink, &mut MessageSender<EmoteEvent>)>,
    mut history: ResMut<ChatHistoryBuffer>,
) {
    let collected = collect_chat_messages(&mut receivers, &players, &guilds);
    let (collected, mut collected_emotes) = split_chat_emotes(collected);
    collected_emotes.extend(collect_emote_messages(&mut emote_receivers, &players));
    for chat in &collected {
        history.entries.push_back(chat.msg.clone());
        if history.entries.len() > 100 {
            history.entries.pop_front();
        }
    }
    for emote in &collected_emotes {
        history.entries.push_back(emote.chat.clone());
        if history.entries.len() > 100 {
            history.entries.pop_front();
        }
    }
    route_chat_messages(&collected, &mut chat_senders, &guilds, &ignore_lists);
    route_emote_messages(
        &collected_emotes,
        &mut chat_senders,
        &mut emote_senders,
        &ignore_lists,
    );
}

pub fn process_chat_history_requests(
    mut receivers: Query<
        (
            &mut MessageReceiver<shared::protocol::ChatHistoryRequest>,
            Entity,
        ),
        With<crate::auth::Authenticated>,
    >,
    player_links: Query<&PlayerLink>,
    ignore_lists: Query<Option<&PlayerIgnoreListState>>,
    history: Res<ChatHistoryBuffer>,
    mut senders: Query<&mut MessageSender<shared::protocol::ChatHistoryResponse>>,
) {
    for (mut receiver, conn_entity) in receivers.iter_mut() {
        for req in receiver.receive() {
            let limit = usize::from(req.limit.clamp(1, 100));
            let ignored_names =
                ignored_names_for_connection(conn_entity, &player_links, &ignore_lists);
            let messages = history_for_connection(&history, limit, ignored_names.as_deref());
            let Ok(mut sender) = senders.get_mut(conn_entity) else {
                continue;
            };
            sender.send::<AuthChannel>(shared::protocol::ChatHistoryResponse {
                messages,
                error: None,
            });
        }
    }
}

struct CollectedChat {
    msg: ChatMessage,
    sender_entity: Entity,
    sender_guild_id: Option<u32>,
}

struct CollectedEmote {
    chat: ChatMessage,
    event: EmoteEvent,
}

fn collect_chat_messages(
    receivers: &mut Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    players: &Query<&shared::components::Player>,
    guilds: &Query<Option<&GuildMembership>>,
) -> Vec<CollectedChat> {
    let mut outgoing = Vec::new();
    for (mut receiver, link) in receivers.iter_mut() {
        let sender_name = players
            .get(link.0)
            .map(|p| p.name.clone())
            .unwrap_or_else(|_| "Unknown".into());
        let sender_guild_id = guilds.get(link.0).ok().flatten().map(|g| g.guild_id);
        for mut msg in receiver.receive() {
            msg.sender = sender_name.clone();
            if matches!(msg.channel, ChatType::Guild) && sender_guild_id.is_none() {
                warn!("[Chat] {} sent guild message but has no guild", msg.sender);
                continue;
            }
            info!("[Chat] {}: {}", msg.sender, msg.content);
            outgoing.push(CollectedChat {
                msg,
                sender_entity: link.0,
                sender_guild_id,
            });
        }
    }
    outgoing
}

fn split_chat_emotes(collected: Vec<CollectedChat>) -> (Vec<CollectedChat>, Vec<CollectedEmote>) {
    let mut chat_messages = Vec::new();
    let mut emotes = Vec::new();
    for chat in collected {
        let Some(emote) = chat_message_emote(&chat.msg) else {
            chat_messages.push(chat);
            continue;
        };
        emotes.push(CollectedEmote {
            chat: emote_chat_message(&chat.msg.sender, emote),
            event: EmoteEvent {
                player_entity: chat.sender_entity.to_bits(),
                sender: chat.msg.sender,
                emote,
            },
        });
    }
    (chat_messages, emotes)
}

fn collect_emote_messages(
    receivers: &mut Query<(&mut MessageReceiver<EmoteIntent>, &PlayerLink)>,
    players: &Query<&shared::components::Player>,
) -> Vec<CollectedEmote> {
    let mut outgoing = Vec::new();
    for (mut receiver, link) in receivers.iter_mut() {
        let sender_name = players
            .get(link.0)
            .map(|p| p.name.clone())
            .unwrap_or_else(|_| "Unknown".into());
        for intent in receiver.receive() {
            info!("[Emote] {} {:?}", sender_name, intent.emote);
            outgoing.push(CollectedEmote {
                chat: emote_chat_message(&sender_name, intent.emote),
                event: EmoteEvent {
                    player_entity: link.0.to_bits(),
                    sender: sender_name.clone(),
                    emote: intent.emote,
                },
            });
        }
    }
    outgoing
}

fn route_chat_messages(
    outgoing: &[CollectedChat],
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    guilds: &Query<Option<&GuildMembership>>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) {
    for chat in outgoing {
        match chat.msg.channel {
            ChatType::Guild => {
                send_to_guild(&chat.msg, chat.sender_guild_id, conns, guilds, ignore_lists)
            }
            _ => broadcast_to_all(&chat.msg, &mut *conns, ignore_lists),
        }
    }
}

fn route_emote_messages(
    outgoing: &[CollectedEmote],
    chat_senders: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    emote_senders: &mut Query<(&PlayerLink, &mut MessageSender<EmoteEvent>)>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) {
    for emote in outgoing {
        for (link, mut chat_sender) in chat_senders.iter_mut() {
            if !recipient_accepts_message(link.0, &emote.event.sender, ignore_lists) {
                continue;
            }
            chat_sender.send::<ChatChannel>(emote.chat.clone());
        }
        for (link, mut emote_sender) in emote_senders.iter_mut() {
            if !recipient_accepts_message(link.0, &emote.event.sender, ignore_lists) {
                continue;
            }
            emote_sender.send::<ChatChannel>(emote.event.clone());
        }
    }
}

fn send_to_guild(
    msg: &ChatMessage,
    sender_guild_id: Option<u32>,
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    guilds: &Query<Option<&GuildMembership>>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) {
    let Some(gid) = sender_guild_id else { return };
    for (link, mut sender) in conns.iter_mut() {
        let is_same_guild = guilds
            .get(link.0)
            .ok()
            .flatten()
            .is_some_and(|g| g.guild_id == gid);
        if is_same_guild && recipient_accepts_message(link.0, &msg.sender, ignore_lists) {
            sender.send::<ChatChannel>(msg.clone());
        }
    }
}

fn broadcast_to_all(
    msg: &ChatMessage,
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) {
    for (link, mut sender) in conns.iter_mut() {
        if !recipient_accepts_message(link.0, &msg.sender, ignore_lists) {
            continue;
        }
        sender.send::<ChatChannel>(msg.clone());
    }
}

fn ignored_names_for_connection(
    conn_entity: Entity,
    player_links: &Query<&PlayerLink>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) -> Option<Vec<String>> {
    let link = player_links.get(conn_entity).ok()?;
    let ignore_list = ignore_lists.get(link.0).ok().flatten()?;
    Some(ignore_list.names.clone())
}

fn history_for_connection(
    history: &ChatHistoryBuffer,
    limit: usize,
    ignored_names: Option<&[String]>,
) -> Vec<ChatMessage> {
    history
        .entries
        .iter()
        .filter(|message| !sender_is_ignored(&message.sender, ignored_names))
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn recipient_accepts_message(
    recipient_player: Entity,
    sender_name: &str,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) -> bool {
    let ignored_names = ignore_lists
        .get(recipient_player)
        .ok()
        .flatten()
        .map(|ignore_list| ignore_list.names.as_slice());
    !sender_is_ignored(sender_name, ignored_names)
}

fn sender_is_ignored(sender_name: &str, ignored_names: Option<&[String]>) -> bool {
    ignored_names
        .unwrap_or_default()
        .iter()
        .any(|ignored| ignored.eq_ignore_ascii_case(sender_name))
}

fn emote_chat_message(sender: &str, emote: EmoteKind) -> ChatMessage {
    ChatMessage {
        sender: sender.into(),
        content: emote_chat_text(emote).into(),
        channel: ChatType::Emote,
    }
}

fn emote_chat_text(emote: EmoteKind) -> &'static str {
    match emote {
        EmoteKind::Dance => "dances.",
        EmoteKind::Wave => "waves.",
    }
}

fn chat_message_emote(msg: &ChatMessage) -> Option<EmoteKind> {
    if !matches!(msg.channel, ChatType::Say) {
        return None;
    }
    match msg.content.trim().to_ascii_lowercase().as_str() {
        "/dance" => Some(EmoteKind::Dance),
        "/wave" => Some(EmoteKind::Wave),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn say_message(sender: &str, content: &str) -> ChatMessage {
        ChatMessage {
            sender: sender.into(),
            content: content.into(),
            channel: ChatType::Say,
        }
    }

    #[test]
    fn sender_is_ignored_matches_case_insensitively() {
        let ignored = vec!["Alice".to_string()];

        assert!(sender_is_ignored("alice", Some(&ignored)));
        assert!(!sender_is_ignored("Bob", Some(&ignored)));
    }

    #[test]
    fn history_for_connection_filters_ignored_senders() {
        let history = ChatHistoryBuffer {
            entries: VecDeque::from([
                say_message("Alice", "first"),
                say_message("Bob", "second"),
                say_message("Alice", "third"),
            ]),
        };
        let ignored = vec!["alice".to_string()];

        let filtered = history_for_connection(&history, 10, Some(&ignored));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].sender, "Bob");
        assert_eq!(filtered[0].content, "second");
    }

    #[test]
    fn emote_chat_message_uses_social_text() {
        let dance = emote_chat_message("Alice", EmoteKind::Dance);
        assert_eq!(dance.sender, "Alice");
        assert_eq!(dance.channel, ChatType::Emote);
        assert_eq!(dance.content, "dances.");

        let wave = emote_chat_message("Bob", EmoteKind::Wave);
        assert_eq!(wave.content, "waves.");
    }

    #[test]
    fn chat_message_emote_parses_supported_slash_commands() {
        assert_eq!(
            chat_message_emote(&say_message("Alice", "/dance")),
            Some(EmoteKind::Dance)
        );
        assert_eq!(
            chat_message_emote(&say_message("Alice", " /wave ")),
            Some(EmoteKind::Wave)
        );
        assert_eq!(chat_message_emote(&say_message("Alice", "hello")), None);
    }
}

pub fn process_guild_invites(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<GuildInviteRequest>,
        ),
        With<crate::auth::Authenticated>,
    >,
    players: Query<&shared::components::Player>,
    guilds: Query<&GuildMembership>,
    char_ids: Query<&ClientId>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<GuildInviteResponse>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            let response = handle_guild_invite(&db, &players, &guilds, &char_ids, link.0, &req);
            let Ok(mut sender) = senders.get_mut(conn_entity) else {
                continue;
            };
            sender.send::<AuthChannel>(response);
        }
    }
}

fn handle_guild_invite(
    db: &GameDatabase,
    players: &Query<&shared::components::Player>,
    guilds: &Query<&GuildMembership>,
    char_ids: &Query<&ClientId>,
    inviter_entity: Entity,
    req: &GuildInviteRequest,
) -> GuildInviteResponse {
    let Ok(inviter_player) = players.get(inviter_entity) else {
        return GuildInviteResponse {
            success: false,
            error: Some("Inviter player entity missing".into()),
        };
    };
    let Ok(inviter_guild) = guilds.get(inviter_entity) else {
        return GuildInviteResponse {
            success: false,
            error: Some("You are not in a guild".into()),
        };
    };

    let Some(target_character_id) =
        persistence::find_character_id_by_name(&db.db, &req.target_character_name)
    else {
        return GuildInviteResponse {
            success: false,
            error: Some("Target character not found".into()),
        };
    };

    if let Ok(inviter_char_id) = char_ids.get(inviter_entity)
        && inviter_char_id.0 == target_character_id
    {
        return GuildInviteResponse {
            success: false,
            error: Some("Cannot invite yourself".into()),
        };
    }

    persistence::set_pending_guild_invite(
        &db.db,
        target_character_id,
        inviter_guild.guild_id,
        &inviter_guild.guild_name,
        &inviter_player.name,
    );

    GuildInviteResponse {
        success: true,
        error: None,
    }
}

pub fn process_guild_accepts(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<GuildAcceptInviteRequest>,
        ),
        With<crate::auth::Authenticated>,
    >,
    char_ids: Query<&ClientId>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<GuildAcceptInviteResponse>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        for _ in receiver.receive() {
            let response = handle_guild_accept(&db, &char_ids, &mut commands, link.0);
            let Ok(mut sender) = senders.get_mut(conn_entity) else {
                continue;
            };
            sender.send::<AuthChannel>(response);
        }
    }
}

fn guild_accept_error(msg: impl Into<String>) -> GuildAcceptInviteResponse {
    GuildAcceptInviteResponse {
        success: false,
        guild_id: None,
        guild_name: None,
        error: Some(msg.into()),
    }
}

fn handle_guild_accept(
    db: &GameDatabase,
    char_ids: &Query<&ClientId>,
    commands: &mut Commands,
    player_entity: Entity,
) -> GuildAcceptInviteResponse {
    let Ok(character_id) = char_ids.get(player_entity).map(|c| c.0) else {
        return guild_accept_error("Character id not found");
    };
    let Some(invite) = persistence::take_pending_guild_invite(&db.db, character_id) else {
        return guild_accept_error("No pending guild invite");
    };
    if let Err(err) = persistence::set_character_guild_by_id(
        &db.db,
        character_id,
        invite.guild_id,
        &invite.guild_name,
    ) {
        return guild_accept_error(err);
    }

    commands.entity(player_entity).insert(GuildMembership {
        guild_id: invite.guild_id,
        guild_name: invite.guild_name.clone(),
    });

    GuildAcceptInviteResponse {
        success: true,
        guild_id: Some(invite.guild_id),
        guild_name: Some(invite.guild_name),
        error: None,
    }
}

pub fn process_target_messages(
    mut receivers: Query<(&mut MessageReceiver<SetTarget>, &PlayerLink)>,
    mut targets: Query<&mut CombatTarget>,
) {
    for (mut receiver, link) in receivers.iter_mut() {
        for msg in receiver.receive() {
            let target = msg.target_entity.map(Entity::from_bits);
            if let Ok(mut combat_target) = targets.get_mut(link.0) {
                combat_target.0 = target;
                debug!("Player {:?} target set to {:?}", link.0, target);
            }
        }
    }
}
