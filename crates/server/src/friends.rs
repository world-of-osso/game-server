use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use lightyear::prelude::Message as NetMessage;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Level, Player, PresenceStatus, Zone};
use shared::protocol::{
    AddFriend, FriendCharacterSnapshot, FriendsChannel, FriendsSnapshot, FriendsStateUpdate,
    QueryFriends, RemoveFriend, SetPresenceStatus,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerFriendsState {
    pub entries: Vec<FriendRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRecord {
    pub name: String,
    pub note: String,
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
struct FriendsSnapshotTracker {
    last_sent: Option<FriendsSnapshot>,
}

#[derive(Debug, Clone)]
struct FriendPresence {
    level: u16,
    class_name: String,
    area: String,
    presence: PresenceStatus,
}

const AUTO_AFK_SECS: f32 = 300.0;

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PresenceTracker {
    idle_secs: f32,
    manual: Option<ManualPresence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualPresence {
    Afk,
    Dnd,
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self {
            idle_secs: 0.0,
            manual: None,
        }
    }
}

pub struct FriendsPlugin;

impl Plugin for FriendsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_initial_friends_snapshot,
                process_query_requests,
                process_add_requests,
                process_remove_requests,
                process_presence_requests,
                tick_auto_afk,
                sync_online_status_updates,
            )
                .chain(),
        );
    }
}

type NewInWorldConnection<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, Added<InWorld>),
>;

fn send_initial_friends_snapshot(
    mut commands: Commands,
    connections: NewInWorldConnection,
    mut friends_q: Query<&mut PlayerFriendsState>,
    online_players: Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&online_players);
    for (conn_entity, link, _in_world) in &connections {
        let Ok(mut friends) = friends_q.get_mut(link.0) else {
            send_friends_error(conn_entity, &mut senders, "friends unavailable for player");
            continue;
        };
        sanitize_duplicates(&mut friends);
        commands
            .entity(link.0)
            .insert((PresenceStatus::Online, PresenceTracker::default()));
        let snapshot = build_friends_snapshot(&db.db, &friends, &online);
        send_friends_update(
            conn_entity,
            snapshot.clone(),
            Some("friends loaded".into()),
            None,
            &mut senders,
        );
        commands.entity(link.0).insert(FriendsSnapshotTracker {
            last_sent: Some(snapshot),
        });
    }
}

fn process_query_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryFriends>),
        With<Authenticated>,
    >,
    friends_q: Query<&PlayerFriendsState>,
    online_players: Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&online_players);
    for (conn_entity, link, mut receiver) in &mut receivers {
        let mut saw_request = false;
        for _ in receiver.receive() {
            saw_request = true;
        }
        if !saw_request {
            continue;
        }
        let Ok(friends) = friends_q.get(link.0) else {
            send_friends_error(conn_entity, &mut senders, "friends unavailable for player");
            continue;
        };
        send_friends_update(
            conn_entity,
            build_friends_snapshot(&db.db, friends, &online),
            Some("friends refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_add_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<AddFriend>,
        ),
        With<Authenticated>,
    >,
    mut friends_q: Query<&mut PlayerFriendsState>,
    players: Query<&Player>,
    online_players: Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&online_players);
    process_mutation_requests(
        &mut receivers,
        &mut friends_q,
        &online,
        &db,
        &mut senders,
        |link, request, friends| {
            let player = players
                .get(link.0)
                .map_err(|_| "friends unavailable for player".to_string())?;
            add_friend(&db.db, friends, &player.name, request.name.trim())
        },
    );
}

fn process_remove_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<RemoveFriend>,
        ),
        With<Authenticated>,
    >,
    mut friends_q: Query<&mut PlayerFriendsState>,
    online_players: Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&online_players);
    process_mutation_requests(
        &mut receivers,
        &mut friends_q,
        &online,
        &db,
        &mut senders,
        |_, request, friends| remove_friend(friends, request.name.trim()),
    );
}

fn process_mutation_requests<R, F>(
    receivers: &mut Query<
        (Entity, &PlayerLink, &InWorld, &mut MessageReceiver<R>),
        With<Authenticated>,
    >,
    friends_q: &mut Query<&mut PlayerFriendsState>,
    online: &HashMap<String, FriendPresence>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<FriendsStateUpdate>>,
    mut mutate: F,
) where
    R: NetMessage,
    F: FnMut(&PlayerLink, &R, &mut PlayerFriendsState) -> Result<String, String>,
{
    for (conn_entity, link, in_world, mut receiver) in receivers.iter_mut() {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(mut friends) = friends_q.get_mut(link.0) else {
            send_friends_error(conn_entity, senders, "friends unavailable for player");
            continue;
        };
        for request in requests {
            let result = mutate(link, &request, &mut friends);
            let snapshot = build_friends_snapshot(&db.db, &friends, online);
            match result {
                Ok(message) => {
                    persistence::save_friends(&db.db, in_world.character_id, &friends);
                    send_friends_update(conn_entity, snapshot, Some(message), None, senders);
                }
                Err(error) => {
                    send_friends_update(conn_entity, snapshot, None, Some(error), senders);
                }
            }
        }
    }
}

type PresenceParamSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        Query<
            'static,
            'static,
            (&'static mut PresenceStatus, &'static mut PresenceTracker),
            With<Player>,
        >,
        Query<
            'static,
            'static,
            (
                &'static Player,
                &'static Level,
                Option<&'static Zone>,
                Option<&'static PresenceStatus>,
            ),
        >,
    ),
>;

fn process_presence_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<SetPresenceStatus>),
        With<Authenticated>,
    >,
    friends_q: Query<&PlayerFriendsState>,
    mut presence_set: PresenceParamSet,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&presence_set.p1());
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(friends) = friends_q.get(link.0) else {
            send_friends_error(conn_entity, &mut senders, "friends unavailable for player");
            continue;
        };
        for request in requests {
            let result = set_presence(link.0, request.status, &mut presence_set.p0());
            let snapshot = build_friends_snapshot(&db.db, friends, &online);
            match result {
                Ok(message) => {
                    send_friends_update(conn_entity, snapshot, Some(message), None, &mut senders);
                }
                Err(error) => {
                    send_friends_update(conn_entity, snapshot, None, Some(error), &mut senders);
                }
            }
        }
    }
}

fn sync_online_status_updates(
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut friends_q: Query<(&PlayerFriendsState, &mut FriendsSnapshotTracker)>,
    online_players: Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    let online = collect_online_players(&online_players);
    for (conn_entity, link) in &connections {
        let Ok((friends, mut tracker)) = friends_q.get_mut(link.0) else {
            continue;
        };
        let snapshot = build_friends_snapshot(&db.db, friends, &online);
        if tracker.last_sent.as_ref() == Some(&snapshot) {
            continue;
        }
        tracker.last_sent = Some(snapshot.clone());
        send_friends_update(
            conn_entity,
            snapshot,
            Some("friends updated".into()),
            None,
            &mut senders,
        );
    }
}

fn tick_auto_afk(
    time: Res<Time>,
    mut players: Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
) {
    for (mut status, mut tracker) in &mut players {
        if tracker.manual.is_some() {
            continue;
        }
        tracker.idle_secs += time.delta_secs();
        *status = if tracker.idle_secs >= AUTO_AFK_SECS {
            PresenceStatus::Afk
        } else {
            PresenceStatus::Online
        };
    }
}

fn add_friend(
    db: &redb::Database,
    friends: &mut PlayerFriendsState,
    self_name: &str,
    target_name: &str,
) -> Result<String, String> {
    if target_name.is_empty() {
        return Err("friend name cannot be empty".into());
    }
    if self_name.eq_ignore_ascii_case(target_name) {
        return Err("cannot add yourself as a friend".into());
    }
    if persistence::find_character_id_by_name(db, target_name).is_none() {
        return Err(format!("character '{target_name}' not found"));
    }
    if friends
        .entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(target_name))
    {
        return Err(format!("'{target_name}' is already on your friends list"));
    }
    friends.entries.push(FriendRecord {
        name: target_name.to_string(),
        note: String::new(),
    });
    sort_records(friends);
    Ok(format!("friend added: {target_name}"))
}

fn remove_friend(friends: &mut PlayerFriendsState, target_name: &str) -> Result<String, String> {
    let original_len = friends.entries.len();
    friends
        .entries
        .retain(|entry| !entry.name.eq_ignore_ascii_case(target_name));
    if friends.entries.len() == original_len {
        return Err(format!("'{target_name}' is not on your friends list"));
    }
    Ok(format!("friend removed: {target_name}"))
}

fn sanitize_duplicates(friends: &mut PlayerFriendsState) {
    let mut deduped = BTreeMap::new();
    for entry in friends.entries.drain(..) {
        deduped
            .entry(entry.name.to_ascii_lowercase())
            .or_insert(entry);
    }
    friends.entries = deduped.into_values().collect();
    sort_records(friends);
}

fn sort_records(friends: &mut PlayerFriendsState) {
    friends.entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then(a.name.cmp(&b.name))
    });
}

fn collect_online_players(
    players: &Query<(&Player, &Level, Option<&Zone>, Option<&PresenceStatus>)>,
) -> HashMap<String, FriendPresence> {
    players
        .iter()
        .map(|(player, level, zone, presence)| {
            (
                player.name.to_ascii_lowercase(),
                FriendPresence {
                    level: level.0 as u16,
                    class_name: class_name_from_id(player.class),
                    area: zone
                        .map(|zone| format!("Zone {}", zone.id))
                        .unwrap_or_else(|| "Unknown".into()),
                    presence: presence.copied().unwrap_or(PresenceStatus::Online),
                },
            )
        })
        .collect()
}

fn build_friends_snapshot(
    db: &redb::Database,
    friends: &PlayerFriendsState,
    online: &HashMap<String, FriendPresence>,
) -> FriendsSnapshot {
    let mut entries: Vec<_> = friends
        .entries
        .iter()
        .filter_map(|entry| build_friend_snapshot(db, entry, online))
        .collect();
    entries.sort_by(|a, b| {
        presence_sort_key(b.presence)
            .cmp(&presence_sort_key(a.presence))
            .then(a.name.cmp(&b.name))
    });
    FriendsSnapshot { entries }
}

fn build_friend_snapshot(
    db: &redb::Database,
    record: &FriendRecord,
    online: &HashMap<String, FriendPresence>,
) -> Option<FriendCharacterSnapshot> {
    if let Some(presence) = online.get(&record.name.to_ascii_lowercase()) {
        return Some(FriendCharacterSnapshot {
            name: record.name.clone(),
            level: presence.level,
            class_name: presence.class_name.clone(),
            area: presence.area.clone(),
            presence: presence.presence,
            note: record.note.clone(),
        });
    }
    let character_id = persistence::find_character_id_by_name(db, &record.name)?;
    let character = persistence::load_character(db, character_id)?;
    Some(FriendCharacterSnapshot {
        name: character.name,
        level: character.level,
        class_name: class_name_from_id(character.class),
        area: "Unknown".into(),
        presence: PresenceStatus::Offline,
        note: record.note.clone(),
    })
}

pub fn mark_player_activity(
    player_entity: Entity,
    players: &mut Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
) {
    let Ok((mut status, mut tracker)) = players.get_mut(player_entity) else {
        return;
    };
    tracker.idle_secs = 0.0;
    match tracker.manual {
        Some(ManualPresence::Dnd) => {
            *status = PresenceStatus::Dnd;
        }
        Some(ManualPresence::Afk) => {
            tracker.manual = None;
            *status = PresenceStatus::Online;
        }
        None => {
            *status = PresenceStatus::Online;
        }
    }
}

fn set_presence(
    player_entity: Entity,
    requested: PresenceStatus,
    players: &mut Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
) -> Result<String, String> {
    let Ok((mut status, mut tracker)) = players.get_mut(player_entity) else {
        return Err("presence unavailable for player".into());
    };
    tracker.idle_secs = 0.0;
    tracker.manual = match requested {
        PresenceStatus::Online | PresenceStatus::Offline => None,
        PresenceStatus::Afk => Some(ManualPresence::Afk),
        PresenceStatus::Dnd => Some(ManualPresence::Dnd),
    };
    *status = match requested {
        PresenceStatus::Offline => PresenceStatus::Online,
        other => other,
    };
    Ok(format!(
        "presence set to {}",
        match *status {
            PresenceStatus::Online => "online",
            PresenceStatus::Afk => "afk",
            PresenceStatus::Dnd => "dnd",
            PresenceStatus::Offline => "offline",
        }
    ))
}

fn class_name_from_id(class_id: u8) -> String {
    match class_id {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        6 => "Death Knight",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        10 => "Monk",
        11 => "Druid",
        12 => "Demon Hunter",
        13 => "Evoker",
        _ => "Unknown",
    }
    .into()
}

fn presence_sort_key(presence: PresenceStatus) -> u8 {
    match presence {
        PresenceStatus::Online => 3,
        PresenceStatus::Afk => 2,
        PresenceStatus::Dnd => 1,
        PresenceStatus::Offline => 0,
    }
}

fn send_friends_update(
    conn_entity: Entity,
    snapshot: FriendsSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<FriendsChannel>(FriendsStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

fn send_friends_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<FriendsStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<FriendsChannel>(FriendsStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
#[path = "friends_tests.rs"]
mod tests;
