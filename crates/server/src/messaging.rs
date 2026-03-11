use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::GuildMembership;
use shared::protocol::{
    AuthChannel, ChatChannel, ChatMessage, ChatType, GuildAcceptInviteRequest,
    GuildAcceptInviteResponse, GuildInviteRequest, GuildInviteResponse, SetTarget,
};

use crate::networking::{ChatHistoryBuffer, CombatTarget, PlayerLink};
use crate::persistence::{self, ClientId, GameDatabase};

pub fn process_chat_messages(
    mut receivers: Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    players: Query<&shared::components::Player>,
    guilds: Query<Option<&GuildMembership>>,
    mut conns: Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    mut history: ResMut<ChatHistoryBuffer>,
) {
    let collected = collect_chat_messages(&mut receivers, &players, &guilds);
    for chat in &collected {
        history.entries.push_back(chat.msg.clone());
        if history.entries.len() > 100 {
            history.entries.pop_front();
        }
    }
    route_chat_messages(&collected, &mut conns, &guilds);
}

pub fn process_chat_history_requests(
    mut receivers: Query<
        (
            &mut MessageReceiver<shared::protocol::ChatHistoryRequest>,
            Entity,
        ),
        With<crate::auth::Authenticated>,
    >,
    history: Res<ChatHistoryBuffer>,
    mut senders: Query<&mut MessageSender<shared::protocol::ChatHistoryResponse>>,
) {
    for (mut receiver, conn_entity) in receivers.iter_mut() {
        for req in receiver.receive() {
            let limit = usize::from(req.limit.clamp(1, 100));
            let start = history.entries.len().saturating_sub(limit);
            let messages: Vec<ChatMessage> = history.entries.iter().skip(start).cloned().collect();
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
    sender_guild_id: Option<u32>,
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
                sender_guild_id,
            });
        }
    }
    outgoing
}

fn route_chat_messages(
    outgoing: &[CollectedChat],
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    guilds: &Query<Option<&GuildMembership>>,
) {
    for chat in outgoing {
        match chat.msg.channel {
            ChatType::Guild => send_to_guild(&chat.msg, chat.sender_guild_id, conns, guilds),
            _ => broadcast_to_all(&chat.msg, &mut *conns),
        }
    }
}

fn send_to_guild(
    msg: &ChatMessage,
    sender_guild_id: Option<u32>,
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    guilds: &Query<Option<&GuildMembership>>,
) {
    let Some(gid) = sender_guild_id else { return };
    for (link, mut sender) in conns.iter_mut() {
        let is_same_guild = guilds
            .get(link.0)
            .ok()
            .flatten()
            .is_some_and(|g| g.guild_id == gid);
        if is_same_guild {
            sender.send::<ChatChannel>(msg.clone());
        }
    }
}

fn broadcast_to_all(
    msg: &ChatMessage,
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
) {
    for (_link, mut sender) in conns.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
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
            let target = msg.target_entity.map(|bits| Entity::from_bits(bits));
            if let Ok(mut combat_target) = targets.get_mut(link.0) {
                combat_target.0 = target;
                debug!("Player {:?} target set to {:?}", link.0, target);
            }
        }
    }
}
