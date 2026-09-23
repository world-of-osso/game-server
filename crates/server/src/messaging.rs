use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{GuildMembership, Player, Position, Zone};
use shared::protocol::{
    AuthChannel, ChatChannel, ChatMessage, ChatType, EmoteEvent, EmoteIntent, EmoteKind,
    GuildAcceptInviteRequest, GuildAcceptInviteResponse, GuildInviteRequest, GuildInviteResponse,
    SetTarget,
};

use crate::ignore_list::PlayerIgnoreListState;
use crate::networking::{ChatHistoryBuffer, ChatHistoryEntry, CombatTarget, PlayerLink};
use crate::persistence::{self, ClientId, GameDatabase};

/// Retail /say and /emote hearing range in yards.
const SAY_RANGE: f32 = 25.0;
/// Retail /yell hearing range in yards (same zone only).
const YELL_RANGE: f32 = 300.0;
const CHAT_HISTORY_LIMIT: usize = 100;

/// Party identity for party chat routing. The group system inserts this on every
/// member's player entity and removes it on leave; no component means no party.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyMembership {
    pub party_id: u64,
}

type ChatProfileQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        Option<&'static Position>,
        Option<&'static Zone>,
        Option<&'static GuildMembership>,
        Option<&'static PartyMembership>,
        Option<&'static PlayerIgnoreListState>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct ChatSystemQueries<'w, 's> {
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
    emote_senders: Query<'w, 's, (&'static PlayerLink, &'static mut MessageSender<EmoteEvent>)>,
    history: ResMut<'w, ChatHistoryBuffer>,
    time: Res<'w, Time>,
    auto_afk: ResMut<'w, crate::friends::AutoAfkScheduler>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
}

pub fn process_chat_messages(mut q: ChatSystemQueries) {
    let activity_at = q.time.elapsed();
    let collected = collect_chat_messages(&mut q.receivers, &q.profiles);
    let (mut collected, mut emotes) = split_chat_emotes(collected);
    emotes.extend(collect_emote_messages(&mut q.emote_receivers, &q.profiles));
    for chat in &collected {
        mark_activity(&mut q, chat.sender_entity, activity_at);
    }
    for emote in &emotes {
        mark_activity(&mut q, emote.player_entity, activity_at);
    }
    collected.extend(emotes.iter().map(|emote| CollectedChat {
        msg: emote.chat.clone(),
        sender_entity: emote.player_entity,
    }));
    let deliveries = {
        let online = online_participants(&q.chat_senders, &q.profiles);
        collected
            .iter()
            .flat_map(|chat| route_and_record(chat, &online, &mut q.history))
            .collect::<Vec<_>>()
    };
    send_deliveries(deliveries, &mut q.chat_senders);
    route_emote_events(&emotes, &mut q.emote_senders, &q.ignore_lists);
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
    chat: ChatMessage,
    event: EmoteEvent,
    player_entity: Entity,
}

/// An online player as seen by chat routing.
struct ChatParticipant<'a> {
    entity: Entity,
    name: &'a str,
    position: Option<&'a Position>,
    zone: Option<&'a Zone>,
    guild_id: Option<u32>,
    party_id: Option<u64>,
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
            let (player, position, zone, guild, party, ignore_list) = profiles.get(link.0).ok()?;
            Some(ChatParticipant {
                entity: link.0,
                name: &player.name,
                position,
                zone,
                guild_id: guild.map(|g| g.guild_id),
                party_id: party.map(|p| p.party_id),
                ignored: ignore_list.map_or(&[], |list| list.names.as_slice()),
            })
        })
        .collect()
}

/// Route one message, record delivered copies in history, and return the
/// `(player, message)` pairs to send.
fn route_and_record(
    chat: &CollectedChat,
    online: &[ChatParticipant],
    history: &mut ChatHistoryBuffer,
) -> Vec<(Entity, ChatMessage)> {
    let Some(sender) = online.iter().find(|p| p.entity == chat.sender_entity) else {
        return Vec::new();
    };
    match route_chat(&chat.msg, sender, online) {
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
) -> ChatRoute {
    let hears = |recipient: &ChatParticipant| match &msg.channel {
        ChatType::Say | ChatType::Emote => within_range(sender, recipient, SAY_RANGE),
        ChatType::Yell => {
            sender.zone == recipient.zone && within_range(sender, recipient, YELL_RANGE)
        }
        ChatType::Guild => sender.guild_id.is_some() && recipient.guild_id == sender.guild_id,
        ChatType::Party => recipient.party_id == sender.party_id,
        // Whisper echo only; the target is added below.
        ChatType::Whisper(_) => recipient.entity == sender.entity,
        // Clients may not author system or broadcast messages.
        ChatType::System | ChatType::ServerBroadcast => false,
    };
    let mut recipients: Vec<Entity> = online
        .iter()
        .filter(|recipient| hears(recipient) && accepts_from(recipient, &msg.sender))
        .map(|recipient| recipient.entity)
        .collect();
    match &msg.channel {
        ChatType::Party if sender.party_id.is_none() => {
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
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    dx * dx + dy * dy + dz * dz <= range * range
}

fn accepts_from(recipient: &ChatParticipant, sender_name: &str) -> bool {
    !sender_is_ignored(sender_name, Some(recipient.ignored))
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
    emote_senders: &mut Query<(&PlayerLink, &mut MessageSender<EmoteEvent>)>,
    ignore_lists: &Query<Option<&PlayerIgnoreListState>>,
) {
    for emote in outgoing {
        for (link, mut emote_sender) in emote_senders.iter_mut() {
            if !recipient_accepts_message(link.0, &emote.event.sender, ignore_lists) {
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
        EmoteKind::Sit => "sits down.",
        EmoteKind::Sleep => "falls asleep.",
        EmoteKind::Kneel => "kneels.",
    }
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
    guilds: Query<&GuildMembership>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<GuildAcceptInviteResponse>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        for _ in receiver.receive() {
            let response = handle_guild_accept(&db, &char_ids, &guilds, &mut commands, link.0);
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
    guilds: &Query<&GuildMembership>,
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
    fn guild_accept_preserves_tick_for_repeated_membership_value() {
        let db = crate::persistence::temp_database();
        let character_id = 42;
        let _ = crate::persistence_character::create_character(&db, 1, "Invitee", 1, 1)
            .expect("character should be created");
        crate::persistence::set_pending_guild_invite(&db, character_id, 7, "Raid Team", "Inviter");

        let mut app = App::new();
        app.insert_resource(GameDatabase { db });
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
        assert!(second_response.success);
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
            zone: None,
            guild_id: None,
            party_id: None,
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
    fn say_reaches_players_within_25_yards_only() {
        let mut world = World::new();
        let (origin, near, far) = (at(0.0), at(20.0), at(40.0));
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &near, &[]),
            participant(&mut world, "Carl", &far, &[]),
        ];

        let route = route_chat(&chat("Alice", ChatType::Say), &online[0], &online);

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

        let route = route_chat(&chat("Alice", ChatType::Yell), &online[0], &online);

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

        let deliveries = route_and_record(&whisper, &online, &mut history);

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
        );

        assert_eq!(delivered(route), vec![online[0].entity]);
    }

    #[test]
    fn party_chat_without_a_group_returns_not_in_group_error() {
        let mut world = World::new();
        let origin = at(0.0);
        let online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
        ];

        let route = route_chat(&chat("Alice", ChatType::Party), &online[0], &online);

        assert_eq!(route, ChatRoute::Reject("You aren't in a party.".into()));
    }

    #[test]
    fn party_chat_reaches_only_party_members() {
        let mut world = World::new();
        let origin = at(0.0);
        let mut online = [
            participant(&mut world, "Alice", &origin, &[]),
            participant(&mut world, "Bob", &origin, &[]),
            participant(&mut world, "Carl", &origin, &[]),
        ];
        online[0].party_id = Some(7);
        online[1].party_id = Some(7);
        online[2].party_id = Some(8);

        let route = route_chat(&chat("Alice", ChatType::Party), &online[0], &online);

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
            route_and_record(&collected, &online, &mut history);
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
        let dance = emote_chat_message("Alice", EmoteKind::Dance);
        assert_eq!(dance.sender, "Alice");
        assert_eq!(dance.channel, ChatType::Emote);
        assert_eq!(dance.content, "dances.");

        let wave = emote_chat_message("Bob", EmoteKind::Wave);
        assert_eq!(wave.content, "waves.");

        let sit = emote_chat_message("Cara", EmoteKind::Sit);
        assert_eq!(sit.content, "sits down.");

        let sleep = emote_chat_message("Dane", EmoteKind::Sleep);
        assert_eq!(sleep.content, "falls asleep.");

        let kneel = emote_chat_message("Eli", EmoteKind::Kneel);
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
