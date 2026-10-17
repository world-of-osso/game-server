use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{GuildMembership, Player, Position, Zone};
use shared::protocol::{
    AuthChannel, ChatChannel, ChatMessage, ChatType, EmoteEvent, EmoteIntent, EmoteKind,
    GuildAcceptInviteRequest, GuildAcceptInviteResponse, GuildInviteRequest, GuildInviteResponse,
    SetTarget,
};

use crate::group::GroupRegistry;
use crate::ignore_list::PlayerIgnoreListState;
use crate::networking::{ChatHistoryBuffer, ChatHistoryEntry, CombatTarget, PlayerLink};
use crate::persistence::{self, ClientId, GameDatabase};

/// Retail /say and /emote hearing range in yards.
const SAY_RANGE: f32 = 25.0;
/// Retail /yell hearing range in yards (same zone only).
const YELL_RANGE: f32 = 300.0;
const CHAT_HISTORY_LIMIT: usize = 100;

type ChatProfileQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        Option<&'static Position>,
        Option<&'static Zone>,
        Option<&'static GuildMembership>,
        Option<&'static ClientId>,
        Option<&'static PlayerIgnoreListState>,
        Option<&'static crate::maps::InMap>,
    ),
>;

type MountedEmoteQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static shared::components::Mounted,
        &'static crate::interest::NetworkVisibility,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct ChatSystemQueries<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    receivers: Query<
        'w,
        's,
        (
            &'static mut MessageReceiver<ChatMessage>,
            &'static PlayerLink,
        ),
    >,
    emote_receivers: Query<
        'w,
        's,
        (
            &'static mut MessageReceiver<EmoteIntent>,
            &'static PlayerLink,
        ),
    >,
    profiles: ChatProfileQuery<'w, 's>,
    presences: Query<
        'w,
        's,
        (
            &'static mut shared::components::PresenceStatus,
            &'static mut crate::friends::PresenceTracker,
        ),
        With<Player>,
    >,
    ignore_lists: Query<'w, 's, Option<&'static PlayerIgnoreListState>>,
    chat_senders: Query<'w, 's, (&'static PlayerLink, &'static mut MessageSender<ChatMessage>)>,
    emote_senders: Query<
        'w,
        's,
        (
            Entity,
            &'static PlayerLink,
            &'static mut MessageSender<EmoteEvent>,
        ),
    >,
    mounted: MountedEmoteQuery<'w, 's>,
    history: ResMut<'w, ChatHistoryBuffer>,
    groups: Res<'w, GroupRegistry>,
    time: Res<'w, Time>,
    auto_afk: ResMut<'w, crate::friends::AutoAfkScheduler>,
    criteria: MessageWriter<'w, crate::achievement::CriteriaCredit>,
    entered: Query<
        'w,
        's,
        &'static PlayerLink,
        (With<crate::auth::Authenticated>, With<crate::auth::InWorld>),
    >,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
}

pub fn process_chat_messages(mut q: ChatSystemQueries) {
    let activity_at = q.time.elapsed();
    let collected = collect_chat_messages(&mut q.receivers, &q.profiles);
    let (mut collected, mut emotes) = split_chat_emotes(collected);
    emotes.extend(collect_emote_messages(&mut q.emote_receivers, &q.profiles));
    emotes.retain(|emote| accepts_mount_special(&q, emote));
    for chat in &collected {
        mark_activity(&mut q, chat.sender_entity, activity_at);
    }
    for emote in &emotes {
        mark_activity(&mut q, emote.player_entity, activity_at);
        if let Some(text_emote) = retail_text_emote(emote.event.emote)
            && q.entered.iter().any(|link| link.0 == emote.player_entity)
        {
            q.criteria.write(crate::achievement::CriteriaCredit {
                player: emote.player_entity,
                event: crate::achievement::CriteriaEvent::DoEmote(text_emote, emote.target_entity),
            });
        }
    }
    collected.extend(emotes.iter().filter_map(|emote| {
        emote.chat.clone().map(|msg| CollectedChat {
            msg,
            sender_entity: emote.player_entity,
        })
    }));
    let deliveries = {
        let online = online_participants(&q.chat_senders, &q.profiles);
        collected
            .iter()
            .flat_map(|chat| {
                route_guild_chat(
                    chat,
                    &online,
                    &q.groups,
                    &mut q.history,
                    q.db.as_deref().map(|db| db.db.as_ref()),
                )
            })
            .collect::<Vec<_>>()
    };
    send_deliveries(deliveries, &mut q.chat_senders);
    route_emote_events(&emotes, &mut q.emote_senders, &q.ignore_lists, &q.mounted);
}

/// The client selects stationary jump; TC's relay does not reclassify movement.
fn accepts_mount_special(q: &ChatSystemQueries, emote: &CollectedEmote) -> bool {
    if emote.event.emote != EmoteKind::MountSpecial {
        return true;
    }
    if !q.entered.iter().any(|link| link.0 == emote.player_entity) {
        return false;
    }
    q.mounted.contains(emote.player_entity)
}

fn mark_activity(q: &mut ChatSystemQueries, player: Entity, activity_at: std::time::Duration) {
    crate::friends::mark_player_activity(
        player,
        &mut q.presences,
        &mut q.auto_afk,
        activity_at,
        q.replication_dirty.as_deref(),
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
    players: Query<(&Player, Option<&PlayerIgnoreListState>)>,
    history: Res<ChatHistoryBuffer>,
    mut senders: Query<&mut MessageSender<shared::protocol::ChatHistoryResponse>>,
) {
    for (mut receiver, conn_entity) in receivers.iter_mut() {
        for req in receiver.receive() {
            let limit = usize::from(req.limit.clamp(1, 100));
            let messages = player_links
                .get(conn_entity)
                .ok()
                .and_then(|link| players.get(link.0).ok())
                .map(|(player, ignore_list)| {
                    let ignored = ignore_list.map(|list| list.names.as_slice());
                    history_for_player(&history, limit, &player.name, ignored)
                })
                .unwrap_or_default();
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
}

struct CollectedEmote {
    chat: Option<ChatMessage>,
    event: EmoteEvent,
    player_entity: Entity,
    target_entity: Option<Entity>,
}

/// An online player as seen by chat routing.
struct ChatParticipant<'a> {
    entity: Entity,
    name: &'a str,
    position: Option<&'a Position>,
    /// Say, yell and emote are heard on the sender's map copy only.
    map: Option<&'a crate::maps::InMap>,
    zone: Option<&'a Zone>,
    guild_id: Option<u32>,
    /// Character id (`ClientId`), the key for group membership.
    character_id: Option<u64>,
    ignored: &'a [String],
}

#[derive(Debug, PartialEq)]
enum ChatRoute {
    /// Player entities that receive the message.
    Deliver(Vec<Entity>),
    /// The message is not delivered; the sender gets this system message instead.
    Reject(String),
}

fn sender_name(profiles: &ChatProfileQuery, player: Entity) -> String {
    profiles
        .get(player)
        .map(|(p, ..)| p.name.clone())
        .unwrap_or_else(|_| "Unknown".into())
}

fn collect_chat_messages(
    receivers: &mut Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    profiles: &ChatProfileQuery,
) -> Vec<CollectedChat> {
    let mut outgoing = Vec::new();
    for (mut receiver, link) in receivers.iter_mut() {
        let sender_name = sender_name(profiles, link.0);
        for mut msg in receiver.receive() {
            msg.sender = sender_name.clone();
            info!("[Chat] {}: {}", msg.sender, msg.content);
            outgoing.push(CollectedChat {
                msg,
                sender_entity: link.0,
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
            player_entity: chat.sender_entity,
            target_entity: None,
        });
    }
    (chat_messages, emotes)
}

fn collect_emote_messages(
    receivers: &mut Query<(&mut MessageReceiver<EmoteIntent>, &PlayerLink)>,
    profiles: &ChatProfileQuery,
) -> Vec<CollectedEmote> {
    let mut outgoing = Vec::new();
    for (mut receiver, link) in receivers.iter_mut() {
        let sender_name = sender_name(profiles, link.0);
        for intent in receiver.receive() {
            info!("[Emote] {} {:?}", sender_name, intent.emote);
            outgoing.push(CollectedEmote {
                chat: emote_chat_message(&sender_name, intent.emote),
                event: EmoteEvent {
                    player_entity: link.0.to_bits(),
                    sender: sender_name.clone(),
                    emote: intent.emote,
                },
                player_entity: link.0,
                target_entity: intent.target_entity.and_then(Entity::try_from_bits),
            });
        }
    }
    outgoing
}

fn online_participants<'a>(
    conns: &Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
    profiles: &'a ChatProfileQuery,
) -> Vec<ChatParticipant<'a>> {
    conns
        .iter()
        .filter_map(|(link, _)| {
            let (player, position, zone, guild, client_id, ignore_list, map) =
                profiles.get(link.0).ok()?;
            Some(ChatParticipant {
                entity: link.0,
                name: &player.name,
                position,
                map,
                zone,
                guild_id: guild.map(|g| g.guild_id),
                character_id: client_id.map(|id| id.0),
                ignored: ignore_list.map_or(&[], |list| list.names.as_slice()),
            })
        })
        .collect()
}

/// Gate both speaking and hearing before history records recipients.
fn route_guild_chat(
    chat: &CollectedChat,
    online: &[ChatParticipant],
    groups: &GroupRegistry,
    history: &mut ChatHistoryBuffer,
    db: Option<&redb::Database>,
) -> Vec<(Entity, ChatMessage)> {
    use shared::protocol::*;
    let (speak, listen) = match chat.msg.channel {
        ChatType::Guild => (GUILD_RIGHT_CHAT_SPEAK, GUILD_RIGHT_CHAT_LISTEN),
        ChatType::Officer => (GUILD_RIGHT_OFFICER_SPEAK, GUILD_RIGHT_OFFICER_LISTEN),
        _ => return route_and_record(chat, online, groups, history),
    };
    let Some(sender) = online.iter().find(|p| p.entity == chat.sender_entity) else {
        return Vec::new();
    };
    let guild_id = db
        .and_then(|db| {
            sender
                .character_id
                .and_then(|id| persistence::load_character_guild(db, id))
        })
        .map(|guild| guild.guild_id);
    let allowed = |participant: &ChatParticipant, right| {
        db.is_some_and(|db| {
            participant.character_id.is_some_and(|id| {
                guild_id.is_some_and(|guild_id| {
                    crate::guild::character_has_right(db, id, guild_id, right)
                })
            })
        })
    };
    if !allowed(sender, speak) {
        return vec![(
            sender.entity,
            system_message(GuildRankError::Permissions.message().into()),
        )];
    }
    let recipients: Vec<_> = online
        .iter()
        .filter(|p| allowed(p, listen) && accepts_from(p, &chat.msg.sender))
        .map(|p| p.entity)
        .collect();
    record_history(history, &chat.msg, &recipients, online);
    recipients
        .into_iter()
        .map(|entity| (entity, chat.msg.clone()))
        .collect()
}

/// Route one message, record delivered copies in history, and return the
/// `(player, message)` pairs to send.
fn route_and_record(
    chat: &CollectedChat,
    online: &[ChatParticipant],
    groups: &GroupRegistry,
    history: &mut ChatHistoryBuffer,
) -> Vec<(Entity, ChatMessage)> {
    let Some(sender) = online.iter().find(|p| p.entity == chat.sender_entity) else {
        return Vec::new();
    };
    match route_chat(&chat.msg, sender, online, groups) {
        ChatRoute::Reject(text) => vec![(sender.entity, system_message(text))],
        ChatRoute::Deliver(recipients) => {
            record_history(history, &chat.msg, &recipients, online);
            recipients
                .into_iter()
                .map(|entity| (entity, chat.msg.clone()))
                .collect()
        }
    }
}

fn route_chat(
    msg: &ChatMessage,
    sender: &ChatParticipant,
    online: &[ChatParticipant],
    groups: &GroupRegistry,
) -> ChatRoute {
    let hears = |recipient: &ChatParticipant| match &msg.channel {
        ChatType::Say | ChatType::Emote => within_range(sender, recipient, SAY_RANGE),
        ChatType::Yell => {
            sender.zone == recipient.zone && within_range(sender, recipient, YELL_RANGE)
        }
        ChatType::Guild | ChatType::Officer => {
            sender.guild_id.is_some() && recipient.guild_id == sender.guild_id
        }
        ChatType::Party => match (sender.character_id, recipient.character_id) {
            (Some(a), Some(b)) => groups.is_in_same_group(a, b),
            _ => false,
        },
        // Whisper echo only; the target is added below.
        ChatType::Whisper(_) => recipient.entity == sender.entity,
        // Clients may not author system, broadcast or creature messages.
        ChatType::System
        | ChatType::ServerBroadcast
        | ChatType::MonsterSay(_)
        | ChatType::MonsterYell(_)
        | ChatType::MonsterEmote(_)
        | ChatType::RaidBossEmote(_) => false,
    };
    let mut recipients: Vec<Entity> = online
        .iter()
        .filter(|recipient| hears(recipient) && accepts_from(recipient, &msg.sender))
        .map(|recipient| recipient.entity)
        .collect();
    match &msg.channel {
        ChatType::Party
            if sender
                .character_id
                .is_none_or(|id| groups.group_members(id).is_empty()) =>
        {
            ChatRoute::Reject("You aren't in a party.".into())
        }
        ChatType::Whisper(target_name) => {
            let Some(target) = online
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(target_name))
            else {
                return ChatRoute::Reject(format!(
                    "No player named '{target_name}' is currently playing."
                ));
            };
            // Retail: a whisper to someone ignoring the sender is dropped silently;
            // the sender still sees their own echo and gets no error.
            if target.entity != sender.entity && accepts_from(target, &msg.sender) {
                recipients.push(target.entity);
            }
            ChatRoute::Deliver(recipients)
        }
        _ => ChatRoute::Deliver(recipients),
    }
}

fn within_range(sender: &ChatParticipant, recipient: &ChatParticipant, range: f32) -> bool {
    if sender.entity == recipient.entity {
        return true;
    }
    let (Some(a), Some(b)) = (sender.position, recipient.position) else {
        return false;
    };
    if sender.map != recipient.map {
        return false;
    }
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    dx * dx + dy * dy + dz * dz <= range * range
}

fn accepts_from(recipient: &ChatParticipant, sender_name: &str) -> bool {
    !sender_is_ignored(sender_name, Some(recipient.ignored))
}

/// Retail login system line: the message of the day, once per world entry.
pub fn send_motd_on_enter_world(
    motd: Res<shared::motd::Motd>,
    mut entered: Query<&mut MessageSender<ChatMessage>, Added<crate::auth::InWorld>>,
) {
    let Some(message) = motd.as_chat_message() else {
        return;
    };
    for mut sender in &mut entered {
        sender.send::<ChatChannel>(message.clone());
    }
}

fn system_message(content: String) -> ChatMessage {
    ChatMessage {
        sender: String::new(),
        content,
        channel: ChatType::System,
    }
}

fn record_history(
    history: &mut ChatHistoryBuffer,
    msg: &ChatMessage,
    recipients: &[Entity],
    online: &[ChatParticipant],
) {
    let recipients = online
        .iter()
        .filter(|p| recipients.contains(&p.entity))
        .map(|p| p.name.to_string())
        .collect();
    history.entries.push_back(ChatHistoryEntry {
        message: msg.clone(),
        recipients,
    });
    if history.entries.len() > CHAT_HISTORY_LIMIT {
        history.entries.pop_front();
    }
}

fn send_deliveries(
    deliveries: Vec<(Entity, ChatMessage)>,
    conns: &mut Query<(&PlayerLink, &mut MessageSender<ChatMessage>)>,
) {
    for (link, mut sender) in conns.iter_mut() {
        for (_, msg) in deliveries.iter().filter(|(player, _)| *player == link.0) {
            sender.send::<ChatChannel>(msg.clone());
        }
    }
}

fn route_emote_events(
    outgoing: &[CollectedEmote],
    emote_senders: &mut Query<(Entity, &PlayerLink, &mut MessageSender<EmoteEvent>)>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
    mounted: &MountedEmoteQuery<'_, '_>,
) {
    for emote in outgoing {
        for (connection, link, mut emote_sender) in emote_senders.iter_mut() {
            let accepts = if emote.event.emote == EmoteKind::MountSpecial {
                mounted
                    .get(emote.player_entity)
                    .is_ok_and(|(_, visibility)| {
                        link.0 == emote.player_entity
                            || visibility.visible_senders().any(|peer| peer == connection)
                    })
            } else {
                recipient_accepts_message(link.0, &emote.event.sender, ignore_lists)
            };
            if !accepts {
                continue;
            }
            emote_sender.send::<ChatChannel>(emote.event.clone());
        }
    }
}

fn history_for_player(
    history: &ChatHistoryBuffer,
    limit: usize,
    player_name: &str,
    ignored_names: Option<&[String]>,
) -> Vec<ChatMessage> {
    history
        .entries
        .iter()
        .filter(|entry| {
            entry
                .recipients
                .iter()
                .any(|name| name.eq_ignore_ascii_case(player_name))
        })
        .filter(|entry| !sender_is_ignored(&entry.message.sender, ignored_names))
        .rev()
        .take(limit)
        .map(|entry| entry.message.clone())
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

fn emote_chat_message(sender: &str, emote: EmoteKind) -> Option<ChatMessage> {
    Some(ChatMessage {
        sender: sender.into(),
        content: emote_chat_text(emote)?.into(),
        channel: ChatType::Emote,
    })
}

/// Pinned TC a352b1fa SharedDefines.h TextEmotes; not animation Emotes IDs.
fn retail_text_emote(emote: EmoteKind) -> Option<u32> {
    Some(match emote {
        EmoteKind::Dance => 34,
        EmoteKind::Wave => 101,
        EmoteKind::Sit => 86,
        EmoteKind::Sleep => 87,
        EmoteKind::Kneel => 59,
        EmoteKind::Love => 225,
        EmoteKind::Hug => 56,
        EmoteKind::MountSpecial => return None,
    })
}

fn emote_chat_text(emote: EmoteKind) -> Option<&'static str> {
    Some(match emote {
        EmoteKind::Dance => "dances.",
        EmoteKind::Wave => "waves.",
        EmoteKind::Sit => "sits down.",
        EmoteKind::Sleep => "falls asleep.",
        EmoteKind::Kneel => "kneels.",
        EmoteKind::Love => "loves everyone.",
        EmoteKind::Hug => "needs a hug!",
        EmoteKind::MountSpecial => return None,
    })
}

fn chat_message_emote(msg: &ChatMessage) -> Option<EmoteKind> {
    if !matches!(msg.channel, ChatType::Say) {
        return None;
    }
    match msg.content.trim().to_ascii_lowercase().as_str() {
        "/dance" => Some(EmoteKind::Dance),
        "/wave" => Some(EmoteKind::Wave),
        "/sit" => Some(EmoteKind::Sit),
        "/sleep" => Some(EmoteKind::Sleep),
        "/kneel" => Some(EmoteKind::Kneel),
        "/love" => Some(EmoteKind::Love),
        "/hug" => Some(EmoteKind::Hug),
        "/mountspecial" => Some(EmoteKind::MountSpecial),
        _ => None,
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
    mut invitees: Query<
        (
            &PlayerLink,
            &mut MessageSender<shared::protocol::GuildInvitation>,
        ),
        With<crate::auth::InWorld>,
    >,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            let response = handle_guild_invite(&db, &players, &guilds, &char_ids, link.0, &req);
            if response.success {
                let target =
                    persistence::find_character_id_by_name(&db.db, &req.target_character_name)
                        .expect("successful invite has target");
                let pending = crate::guild::load_pending_invitation(&db.db, target)
                    .expect("successful invite persisted");
                for (target_link, mut sender) in &mut invitees {
                    if char_ids.get(target_link.0).is_ok_and(|id| id.0 == target) {
                        sender.send::<AuthChannel>(pending.clone());
                    }
                }
            }
            if let Ok(mut sender) = senders.get_mut(conn_entity) {
                sender.send::<AuthChannel>(response);
            }
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
            error: Some(
                shared::protocol::GuildRankError::NotInGuild
                    .message()
                    .into(),
            ),
        };
    };

    if !char_ids.get(inviter_entity).is_ok_and(|id| {
        crate::guild::character_has_right(
            &db.db,
            id.0,
            inviter_guild.guild_id,
            shared::protocol::GUILD_RIGHT_INVITE,
        )
    }) {
        return GuildInviteResponse {
            success: false,
            error: Some(
                shared::protocol::GuildRankError::Permissions
                    .message()
                    .into(),
            ),
        };
    }

    let Some(target_character_id) =
        persistence::find_character_id_by_name(&db.db, &req.target_character_name)
    else {
        return GuildInviteResponse {
            success: false,
            error: Some(format!("\"{}\" not found.", req.target_character_name)),
        };
    };

    if let Ok(inviter_char_id) = char_ids.get(inviter_entity)
        && inviter_char_id.0 == target_character_id
    {
        return GuildInviteResponse {
            success: false,
            error: Some("You can't invite yourself to a guild.".into()),
        };
    }

    if persistence::load_character_guild(&db.db, target_character_id).is_some() {
        return GuildInviteResponse {
            success: false,
            error: Some(format!(
                "{} is already in a guild.",
                req.target_character_name
            )),
        };
    }
    if persistence::load_pending_guild_invite(&db.db, target_character_id).is_some() {
        return GuildInviteResponse {
            success: false,
            error: Some(format!(
                "{} has already been invited to a guild.",
                req.target_character_name
            )),
        };
    }
    if !char_ids.iter().any(|id| id.0 == target_character_id) {
        return GuildInviteResponse {
            success: false,
            error: Some(format!("\"{}\" not found.", req.target_character_name)),
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
    guilds: Query<&GuildMembership>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
    mut replies: GuildAcceptReplies,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        for _ in receiver.receive() {
            let response = handle_guild_accept(&db, &char_ids, &guilds, &mut commands, link.0);
            if let Some(guild_id) = response.guild_id {
                replies.changed.0.push(guild_id);
                for (connection, world) in &replies.connections {
                    if persistence::load_character_guild(&db.db, world.character_id)
                        .is_some_and(|g| g.guild_id == guild_id)
                    {
                        if let Ok(mut sender) = replies.ranks.get_mut(connection) {
                            sender.send::<shared::protocol::GuildChannel>(
                                crate::guild::guild_rank_snapshot(&db.db, world.character_id, None),
                            );
                        }
                    }
                }
            }
            if let Ok(mut sender) = replies.accepts.get_mut(conn_entity) {
                sender.send::<AuthChannel>(response);
            }
        }
    }
}

#[derive(SystemParam)]
pub struct GuildAcceptReplies<'w, 's> {
    accepts: Query<'w, 's, &'static mut MessageSender<GuildAcceptInviteResponse>>,
    ranks: Query<'w, 's, &'static mut MessageSender<shared::protocol::GuildRanksState>>,
    connections:
        Query<'w, 's, (Entity, &'static crate::auth::InWorld), With<crate::auth::Authenticated>>,
    changed: ResMut<'w, crate::guild_bank::ChangedGuilds>,
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
    guilds: &Query<&GuildMembership>,
    commands: &mut Commands,
    player_entity: Entity,
) -> GuildAcceptInviteResponse {
    let Ok(character_id) = char_ids.get(player_entity).map(|c| c.0) else {
        return guild_accept_error("Character id not found");
    };
    if persistence::load_character_guild(&db.db, character_id).is_some() {
        return guild_accept_error("You are already in a guild.");
    }
    let invite = match persistence::accept_pending_guild_invite(&db.db, character_id) {
        Ok(invite) => invite,
        Err(error) => return guild_accept_error(error),
    };
    let next_membership = GuildMembership {
        guild_id: invite.guild_id,
        guild_name: invite.guild_name.clone(),
    };
    let membership_changed = match guilds.get(player_entity) {
        Ok(current) => current != &next_membership,
        Err(_) => true,
    };
    if membership_changed {
        commands.entity(player_entity).insert(next_membership);
    }

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

#[cfg(test)]
#[path = "messaging_achievement_tests.rs"]
mod achievement_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn apply_guild_accept(app: &mut App, player_entity: Entity) -> GuildAcceptInviteResponse {
        app.world_mut()
            .run_system_once(
                move |db: Res<GameDatabase>,
                      char_ids: Query<&ClientId>,
                      guilds: Query<&GuildMembership>,
                      mut commands: Commands| {
                    handle_guild_accept(&db, &char_ids, &guilds, &mut commands, player_entity)
                },
            )
            .expect("guild accept handler should run")
    }

    #[test]
    fn guild_invites_reject_guilded_and_duplicate_targets_then_accept_at_lowest_rank() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invites.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (master, _) = persistence::create_character(&db, 1, "Master", 1, 1).unwrap();
        let (target, _) = persistence::create_character(&db, 2, "Invitee", 1, 1).unwrap();
        persistence::set_character_guild_by_id(&db, master, 7, "Raid Team").unwrap();
        persistence::banks::save_guild_member_rank(&db, master, 0);
        let mut app = App::new();
        app.insert_resource(GameDatabase::new(db));
        let actor = app
            .world_mut()
            .spawn((
                ClientId(master),
                GuildMembership {
                    guild_id: 7,
                    guild_name: "Raid Team".into(),
                },
                Player {
                    name: "Master".into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
            ))
            .id();
        let invitee = app.world_mut().spawn(ClientId(target)).id();
        let invite = |app: &mut App| {
            app.world_mut()
                .run_system_once(
                    move |db: Res<GameDatabase>,
                          players: Query<&Player>,
                          guilds: Query<&GuildMembership>,
                          ids: Query<&ClientId>| {
                        handle_guild_invite(
                            &db,
                            &players,
                            &guilds,
                            &ids,
                            actor,
                            &GuildInviteRequest {
                                target_character_name: "Invitee".into(),
                            },
                        )
                    },
                )
                .unwrap()
        };
        assert!(invite(&mut app).success);
        let duplicate = invite(&mut app);
        assert_eq!(
            duplicate.error.as_deref(),
            Some("Invitee has already been invited to a guild.")
        );
        let db = &app.world().resource::<GameDatabase>().db;
        assert_eq!(
            persistence::load_pending_guild_invite(db, target)
                .unwrap()
                .inviter_name,
            "Master"
        );
        persistence::take_pending_guild_invite(db, target); // decline
        assert!(persistence::load_pending_guild_invite(db, target).is_none());
        assert!(invite(&mut app).success);
        assert!(apply_guild_accept(&mut app, invitee).success);
        assert_eq!(
            invite(&mut app).error.as_deref(),
            Some("Invitee is already in a guild.")
        );
        let db = &app.world().resource::<GameDatabase>().db;
        assert!(persistence::load_pending_guild_invite(db, target).is_none());
        assert_eq!(
            persistence::banks::load_guild_member_rank(
                db,
                &persistence::banks::load_guild(db, 7),
                target
            ),
            4
        );
        drop(app);
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        assert_eq!(
            persistence::load_character_guild(&db, target)
                .unwrap()
                .guild_id,
            7
        );
        assert!(persistence::load_pending_guild_invite(&db, target).is_none());
    }

    #[test]
    fn guild_invite_accept_cannot_replace_existing_persisted_membership() {
        let db = crate::persistence::temp_database();
        let (id, _) =
            crate::persistence_character::create_character(&db, 1, "Invitee", 1, 1).unwrap();
        persistence::set_character_guild_by_id(&db, id, 9, "Existing").unwrap();
        persistence::set_pending_guild_invite(&db, id, 7, "Raid Team", "Inviter");
        let mut app = App::new();
        app.insert_resource(GameDatabase::new(db));
        let player = app.world_mut().spawn(ClientId(id)).id();
        let response = apply_guild_accept(&mut app, player);
        assert!(!response.success);
        assert_eq!(
            response.error.as_deref(),
            Some("You are already in a guild.")
        );
        let db = &app.world().resource::<GameDatabase>().db;
        assert_eq!(
            persistence::load_character_guild(db, id).unwrap().guild_id,
            9
        );
    }

    #[test]
    fn guild_accept_preserves_tick_for_repeated_membership_value() {
        let db = crate::persistence::temp_database();
        let (character_id, _) =
            crate::persistence_character::create_character(&db, 1, "Invitee", 1, 1)
                .expect("character should be created");
        crate::persistence::set_pending_guild_invite(&db, character_id, 7, "Raid Team", "Inviter");

        let mut app = App::new();
        app.insert_resource(GameDatabase::new(db));
        let player = app
            .world_mut()
            .spawn((
                ClientId(character_id),
                GuildMembership {
                    guild_id: 1,
                    guild_name: "Old Team".into(),
                },
            ))
            .id();
        let initial_tick = app
            .world()
            .entity(player)
            .get_ref::<GuildMembership>()
            .expect("initial guild membership")
            .last_changed();

        let first_response = apply_guild_accept(&mut app, player);
        assert!(first_response.success);
        assert_eq!(
            app.world().entity(player).get::<GuildMembership>(),
            Some(&GuildMembership {
                guild_id: 7,
                guild_name: "Raid Team".into(),
            })
        );
        let first_tick = app
            .world()
            .entity(player)
            .get_ref::<GuildMembership>()
            .expect("updated guild membership")
            .last_changed();
        assert_ne!(
            first_tick, initial_tick,
            "real membership changes must dirty the component"
        );

        crate::persistence::set_pending_guild_invite(
            &app.world().resource::<GameDatabase>().db,
            character_id,
            7,
            "Raid Team",
            "Inviter",
        );
        let second_response = apply_guild_accept(&mut app, player);
        assert!(!second_response.success);
        assert_eq!(
            second_response.error.as_deref(),
            Some("You are already in a guild.")
        );
        assert_eq!(
            app.world().entity(player).get::<GuildMembership>(),
            Some(&GuildMembership {
                guild_id: 7,
                guild_name: "Raid Team".into(),
            })
        );
        assert_eq!(
            app.world()
                .entity(player)
                .get_ref::<GuildMembership>()
                .expect("repeated guild membership")
                .last_changed(),
            first_tick,
            "repeating the same membership must not dirty the component"
        );
    }

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

    fn participant<'a>(
        world: &mut World,
        name: &'a str,
        position: &'a Position,
        ignored: &'a [String],
    ) -> ChatParticipant<'a> {
        ChatParticipant {
            entity: world.spawn_empty().id(),
            name,
            position: Some(position),
            map: None,
            zone: None,
            guild_id: None,
            character_id: None,
            ignored,
        }
    }

    fn at(x: f32) -> Position {
        Position { x, y: 0.0, z: 0.0 }
    }

    fn chat(sender: &str, channel: ChatType) -> ChatMessage {
        ChatMessage {
            sender: sender.into(),
            content: "hello".into(),
            channel,
        }
    }

    fn delivered(route: ChatRoute) -> Vec<Entity> {
        match route {
            ChatRoute::Deliver(recipients) => recipients,
            ChatRoute::Reject(text) => panic!("expected delivery, got rejection {text:?}"),
        }
    }

    #[test]
    fn guild_ranks_gate_guild_and_officer_chat_before_history() {
        use crate::persistence::banks::*;
        use shared::protocol::*;
        let db = persistence::temp_database();
        let mut world = World::new();
        let origin = at(0.0);
        let mut online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
        ];
        for (participant, rank) in online.iter_mut().zip([0, 4]) {
            let (id, _) = persistence::create_character(&db, 1, participant.name, 1, 1).unwrap();
            persistence::set_character_guild_by_id(&db, id, 7, "Bank Testers").unwrap();
            save_guild_member_rank(&db, id, rank);
            participant.character_id = Some(id);
            participant.guild_id = Some(7);
        }
        let mut history = ChatHistoryBuffer::default();
        let groups = GroupRegistry::default();
        let officer = CollectedChat {
            msg: chat("Alice", ChatType::Officer),
            sender_entity: online[0].entity,
        };
        let deliveries = route_guild_chat(&officer, &online, &groups, &mut history, Some(&db));
        assert_eq!(
            deliveries.iter().map(|d| d.0).collect::<Vec<_>>(),
            vec![online[0].entity]
        );
        assert!(history_for_player(&history, 100, "Bob", None).is_empty());
        let blocked = CollectedChat {
            msg: chat("Bob", ChatType::Officer),
            sender_entity: online[1].entity,
        };
        let deliveries = route_guild_chat(&blocked, &online, &groups, &mut history, Some(&db));
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].1.channel, ChatType::System);
        assert_eq!(
            deliveries[0].1.content,
            GuildRankError::Permissions.message()
        );
        let mut guild = load_guild(&db, 7);
        guild.ranks[4].rights = GUILD_RIGHT_CHAT_SPEAK;
        save_guild(&db, 7, &guild);
        let speaking = CollectedChat {
            msg: chat("Bob", ChatType::Guild),
            sender_entity: online[1].entity,
        };
        let deliveries = route_guild_chat(&speaking, &online, &groups, &mut history, Some(&db));
        assert_eq!(
            deliveries.iter().map(|d| d.0).collect::<Vec<_>>(),
            vec![online[0].entity]
        );
        assert!(history_for_player(&history, 100, "Bob", None).is_empty());
    }

    #[test]
    fn guild_ranks_chat_does_not_cross_guilds_with_stale_components() {
        use crate::persistence::banks::save_guild_member_rank;
        let db = persistence::temp_database();
        let mut world = World::new();
        let origin = at(0.0);
        let mut online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Carl", &origin, &[]),
        ];
        for (participant, guild) in online.iter_mut().zip([7, 8]) {
            let (id, _) = persistence::create_character(&db, 1, participant.name, 1, 1).unwrap();
            persistence::set_character_guild_by_id(&db, id, guild, "Test Guild").unwrap();
            save_guild_member_rank(&db, id, 0);
            participant.character_id = Some(id);
            // Membership components can lag persistent membership updates.
            participant.guild_id = Some(7);
        }
        let outgoing = CollectedChat {
            msg: chat("Alice", ChatType::Officer),
            sender_entity: online[0].entity,
        };
        let mut history = ChatHistoryBuffer::default();
        let deliveries = route_guild_chat(
            &outgoing,
            &online,
            &GroupRegistry::default(),
            &mut history,
            Some(&db),
        );
        assert_eq!(
            deliveries.iter().map(|d| d.0).collect::<Vec<_>>(),
            vec![online[0].entity]
        );
        assert!(history_for_player(&history, 100, "Carl", None).is_empty());
    }

    #[test]
    fn say_reaches_players_within_25_yards_only() {
        let mut world = World::new();
        let (origin, near, far) = (at(0.0), at(20.0), at(40.0));
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &near, &[]),
            participant(&mut world, "Carl", &far, &[]),
        ];

        let route = route_chat(
            &chat("Alice", ChatType::Say),
            &online[0],
            &online,
            &GroupRegistry::default(),
        );

        assert_eq!(delivered(route), vec![online[0].entity, online[1].entity]);
    }

    #[test]
    fn yell_reaches_300_yards_in_the_same_zone_only() {
        let mut world = World::new();
        let (origin, near, far) = (at(0.0), at(250.0), at(350.0));
        let (elwynn, westfall) = (Zone { id: 12 }, Zone { id: 40 });
        let mut online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &near, &[]),
            participant(&mut world, "Carl", &near, &[]),
            participant(&mut world, "Dana", &far, &[]),
        ];
        online[0].zone = Some(&elwynn);
        online[1].zone = Some(&elwynn);
        online[2].zone = Some(&westfall);
        online[3].zone = Some(&elwynn);

        let route = route_chat(
            &chat("Alice", ChatType::Yell),
            &online[0],
            &online,
            &GroupRegistry::default(),
        );

        assert_eq!(delivered(route), vec![online[0].entity, online[1].entity]);
    }

    #[test]
    fn whisper_reaches_only_the_target_plus_sender_echo() {
        let mut world = World::new();
        let origin = at(0.0);
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
            participant(&mut world, "Carl", &origin, &[]),
        ];

        let route = route_chat(
            &chat("Alice", ChatType::Whisper("bob".into())),
            &online[0],
            &online,
            &GroupRegistry::default(),
        );

        assert_eq!(delivered(route), vec![online[0].entity, online[1].entity]);
    }

    #[test]
    fn whisper_to_offline_name_returns_player_not_found_error() {
        let mut world = World::new();
        let origin = at(0.0);
        let online = [participant(&mut world, "Alice", &origin, &[])];
        let whisper = CollectedChat {
            msg: chat("Alice", ChatType::Whisper("Zed".into())),
            sender_entity: online[0].entity,
        };
        let mut history = ChatHistoryBuffer::default();

        let deliveries =
            route_and_record(&whisper, &online, &GroupRegistry::default(), &mut history);

        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, online[0].entity);
        assert_eq!(deliveries[0].1.channel, ChatType::System);
        assert_eq!(
            deliveries[0].1.content,
            "No player named 'Zed' is currently playing."
        );
        assert!(history.entries.is_empty());
    }

    #[test]
    fn whisper_from_ignored_sender_is_dropped_without_error() {
        let mut world = World::new();
        let origin = at(0.0);
        let bob_ignores = ["alice".to_string()];
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &bob_ignores),
        ];

        let route = route_chat(
            &chat("Alice", ChatType::Whisper("Bob".into())),
            &online[0],
            &online,
            &GroupRegistry::default(),
        );

        assert_eq!(delivered(route), vec![online[0].entity]);
    }

    fn online_view(online: &[ChatParticipant]) -> crate::group::OnlineView {
        online
            .iter()
            .map(|p| {
                let member = crate::group::OnlineMember {
                    entity: p.entity.to_bits(),
                    name: p.name.into(),
                    class: 1,
                    level: 10,
                    portrait: Default::default(),
                    dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                    raid_difficulty: crate::maps::RaidDifficulty::default(),
                };
                (
                    p.character_id.expect("participant has a character id"),
                    member,
                )
            })
            .collect()
    }

    /// Forms a group through the real invite flow: `leader` invites each of `members`,
    /// who accept.
    fn form_group(
        groups: &mut GroupRegistry,
        online: &[ChatParticipant],
        leader: usize,
        members: &[usize],
    ) {
        let view = online_view(online);
        groups.sync_presence(&view);
        for &member in members {
            groups.invite(
                online[leader].character_id.unwrap(),
                online[member].name,
                &view,
            );
            groups.respond_invite(online[member].character_id.unwrap(), true);
        }
    }

    fn with_character_ids<'a, const N: usize>(
        mut online: [ChatParticipant<'a>; N],
    ) -> [ChatParticipant<'a>; N] {
        for (id, participant) in online.iter_mut().enumerate() {
            participant.character_id = Some(id as u64 + 1);
        }
        online
    }

    #[test]
    fn party_chat_without_a_group_returns_not_in_group_error() {
        let mut world = World::new();
        let origin = at(0.0);
        let online = with_character_ids([
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
        ]);
        let mut groups = GroupRegistry::default();
        form_group(&mut groups, &online, 1, &[]);

        let route = route_chat(
            &chat("Alice", ChatType::Party),
            &online[0],
            &online,
            &groups,
        );

        assert_eq!(route, ChatRoute::Reject("You aren't in a party.".into()));
    }

    #[test]
    fn party_chat_reaches_group_members_and_not_other_groups_or_ungrouped() {
        let mut world = World::new();
        let origin = at(0.0);
        let online = with_character_ids([
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
            participant(&mut world, "Carl", &origin, &[]),
            participant(&mut world, "Dana", &origin, &[]),
            participant(&mut world, "Eve", &origin, &[]),
        ]);
        let mut groups = GroupRegistry::default();
        form_group(&mut groups, &online, 0, &[1]);
        form_group(&mut groups, &online, 2, &[3]);

        let route = route_chat(
            &chat("Alice", ChatType::Party),
            &online[0],
            &online,
            &groups,
        );

        assert_eq!(delivered(route), vec![online[0].entity, online[1].entity]);
    }

    #[test]
    fn history_returns_only_messages_the_requester_received() {
        let mut world = World::new();
        let (origin, far) = (at(0.0), at(40.0));
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
            participant(&mut world, "Carl", &far, &[]),
        ];
        let mut history = ChatHistoryBuffer::default();
        for (sender, channel) in [
            (0, ChatType::Say),
            (2, ChatType::Say),
            (0, ChatType::Whisper("Bob".into())),
        ] {
            let collected = CollectedChat {
                msg: chat(online[sender].name, channel),
                sender_entity: online[sender].entity,
            };
            route_and_record(&collected, &online, &GroupRegistry::default(), &mut history);
        }

        let senders_seen_by = |name: &str| {
            history_for_player(&history, 10, name, None)
                .into_iter()
                .map(|msg| (msg.sender, msg.channel))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            senders_seen_by("bob"),
            vec![
                ("Alice".into(), ChatType::Say),
                ("Alice".into(), ChatType::Whisper("Bob".into())),
            ]
        );
        assert_eq!(
            senders_seen_by("Carl"),
            vec![("Carl".into(), ChatType::Say)]
        );
    }

    #[test]
    fn history_filters_senders_ignored_after_delivery() {
        let mut history = ChatHistoryBuffer::default();
        for (sender, content) in [("Alice", "first"), ("Bob", "second")] {
            history.entries.push_back(ChatHistoryEntry {
                message: say_message(sender, content),
                recipients: vec!["Dana".into()],
            });
        }
        let ignored = vec!["alice".to_string()];

        let filtered = history_for_player(&history, 10, "Dana", Some(&ignored));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "second");
    }

    #[test]
    fn emote_chat_message_uses_social_text() {
        let dance = emote_chat_message("Alice", EmoteKind::Dance).unwrap();
        assert_eq!(dance.sender, "Alice");
        assert_eq!(dance.channel, ChatType::Emote);
        assert_eq!(dance.content, "dances.");

        let wave = emote_chat_message("Bob", EmoteKind::Wave).unwrap();
        assert_eq!(wave.content, "waves.");

        let sit = emote_chat_message("Cara", EmoteKind::Sit).unwrap();
        assert_eq!(sit.content, "sits down.");

        let sleep = emote_chat_message("Dane", EmoteKind::Sleep).unwrap();
        assert_eq!(sleep.content, "falls asleep.");

        let kneel = emote_chat_message("Eli", EmoteKind::Kneel).unwrap();
        assert_eq!(kneel.content, "kneels.");
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
        assert_eq!(
            chat_message_emote(&say_message("Alice", "/sit")),
            Some(EmoteKind::Sit)
        );
        assert_eq!(
            chat_message_emote(&say_message("Alice", "/sleep")),
            Some(EmoteKind::Sleep)
        );
        assert_eq!(
            chat_message_emote(&say_message("Alice", "/kneel")),
            Some(EmoteKind::Kneel)
        );
        assert_eq!(chat_message_emote(&say_message("Alice", "hello")), None);
    }
}
