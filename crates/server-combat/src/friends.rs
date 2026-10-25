use std::collections::{BTreeMap, HashMap, HashSet};

use bevy::ecs::system::SystemParam;
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

mod auto_afk;
pub(crate) use auto_afk::AutoAfkScheduler;
#[cfg(test)]
use auto_afk::{
    AUTO_AFK_PLAYERS_VISITED, AUTO_AFK_TEST_LOCK, AUTO_AFK_TRANSITIONS, ManualPresence,
};
pub use auto_afk::{PresenceTracker, mark_player_activity};
use auto_afk::{process_due_auto_afk, schedule_auto_afk, set_presence};

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FriendPresence {
    level: u16,
    class_name: String,
    area: String,
    presence: PresenceStatus,
}

#[derive(Resource, Default)]
struct FriendSnapshotIndex {
    online: HashMap<String, FriendPresence>,
    online_name_by_entity: HashMap<Entity, String>,
    owners_by_friend: HashMap<String, HashSet<Entity>>,
    friend_names_by_owner: HashMap<Entity, HashSet<String>>,
    connection_by_owner: HashMap<Entity, Entity>,
    dirty_owners: HashSet<Entity>,
}

impl FriendSnapshotIndex {
    fn register_owner(&mut self, owner: Entity, connection: Entity, friends: &PlayerFriendsState) {
        self.remove_owner_friend_links(owner);
        self.connection_by_owner.insert(owner, connection);
        let names = friends
            .entries
            .iter()
            .map(|record| record.name.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        for name in &names {
            self.owners_by_friend
                .entry(name.clone())
                .or_default()
                .insert(owner);
        }
        self.friend_names_by_owner.insert(owner, names);
    }

    fn set_online(
        &mut self,
        entity: Entity,
        player: &Player,
        level: &Level,
        zone: Option<&Zone>,
        presence: Option<&PresenceStatus>,
    ) {
        let name = player.name.to_ascii_lowercase();
        let next = FriendPresence {
            level: level.0 as u16,
            class_name: class_name_from_id(player.class),
            area: zone
                .map(|zone| format!("Zone {}", zone.id))
                .unwrap_or_else(|| "Unknown".into()),
            presence: presence.copied().unwrap_or(PresenceStatus::Online),
        };
        let previous_name = self.online_name_by_entity.insert(entity, name.clone());
        if let Some(previous) = previous_name.filter(|previous| previous != &name) {
            self.online.remove(&previous);
            self.mark_owners_for_friend(&previous);
        }
        if self.online.get(&name) == Some(&next) {
            return;
        }
        self.online.insert(name.clone(), next);
        self.mark_owners_for_friend(&name);
    }

    fn remove_online(&mut self, entity: Entity) {
        let Some(name) = self.online_name_by_entity.remove(&entity) else {
            self.unregister_owner(entity);
            return;
        };
        self.online.remove(&name);
        self.mark_owners_for_friend(&name);
        self.unregister_owner(entity);
    }

    fn take_dirty_owners(&mut self) -> Vec<Entity> {
        self.dirty_owners.drain().collect()
    }

    fn mark_owners_for_friend(&mut self, name: &str) {
        let Some(owners) = self.owners_by_friend.get(name) else {
            return;
        };
        self.dirty_owners.extend(owners.iter().copied());
    }

    fn unregister_owner(&mut self, owner: Entity) {
        self.connection_by_owner.remove(&owner);
        self.dirty_owners.remove(&owner);
        self.remove_owner_friend_links(owner);
    }

    fn remove_owner_friend_links(&mut self, owner: Entity) {
        let Some(names) = self.friend_names_by_owner.remove(&owner) else {
            return;
        };
        for name in names {
            let remove_name = self.owners_by_friend.get_mut(&name).is_some_and(|owners| {
                owners.remove(&owner);
                owners.is_empty()
            });
            if remove_name {
                self.owners_by_friend.remove(&name);
            }
        }
    }
}

pub struct FriendsPlugin;

impl Plugin for FriendsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendSnapshotIndex>();
        app.init_resource::<AutoAfkScheduler>();
        app.add_systems(
            Update,
            (
                send_initial_friends_snapshot,
                process_query_requests,
                process_add_requests,
                process_remove_requests,
                process_presence_requests,
                process_due_auto_afk.after(crate::messaging::process_chat_messages),
                sync_changed_friend_presence,
                sync_removed_friend_presence,
                send_dirty_friends_snapshots,
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

type OnlineFriendPlayer = (
    Entity,
    &'static Player,
    &'static Level,
    Option<&'static Zone>,
    Option<&'static PresenceStatus>,
);
type OnlineFriendPlayers<'w, 's> = Query<'w, 's, OnlineFriendPlayer>;

type ChangedFriendPresenceFilter = Or<(
    Changed<Player>,
    Changed<Level>,
    Changed<Zone>,
    Changed<PresenceStatus>,
)>;
type ChangedFriendPlayers<'w, 's> = Query<'w, 's, OnlineFriendPlayer, ChangedFriendPresenceFilter>;

#[derive(SystemParam)]
struct SendInitialFriendsSnapshotParams<'w, 's> {
    commands: Commands<'w, 's>,
    connections: NewInWorldConnection<'w, 's>,
    friends_q: Query<'w, 's, &'static mut PlayerFriendsState>,
    online_players: OnlineFriendPlayers<'w, 's>,
    db: Option<Res<'w, GameDatabase>>,
    index: ResMut<'w, FriendSnapshotIndex>,
    time: Res<'w, Time>,
    auto_afk: ResMut<'w, AutoAfkScheduler>,
    senders: Query<'w, 's, &'static mut MessageSender<FriendsStateUpdate>>,
}

fn send_initial_friends_snapshot(params: SendInitialFriendsSnapshotParams) {
    let SendInitialFriendsSnapshotParams {
        mut commands,
        connections,
        mut friends_q,
        online_players,
        db,
        mut index,
        time,
        mut auto_afk,
        mut senders,
    } = params;
    let Some(db) = db else { return };
    if connections.is_empty() {
        return;
    }
    for (entity, player, level, zone, presence) in &online_players {
        index.set_online(entity, player, level, zone, presence);
    }
    for (conn_entity, link, _in_world) in &connections {
        let Ok(mut friends) = friends_q.get_mut(link.0) else {
            send_friends_error(conn_entity, &mut senders, "friends unavailable for player");
            continue;
        };
        sanitize_duplicates(&mut friends);
        index.register_owner(link.0, conn_entity, &friends);
        let mut tracker = PresenceTracker::default();
        schedule_auto_afk(link.0, &mut tracker, time.elapsed(), &mut auto_afk);
        commands
            .entity(link.0)
            .insert((PresenceStatus::Online, tracker));
        let snapshot = build_friends_snapshot(&db.db, &friends, &index.online);
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
    db: Option<Res<GameDatabase>>,
    index: Res<FriendSnapshotIndex>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
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
            build_friends_snapshot(&db.db, friends, &index.online),
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
    db: Option<Res<GameDatabase>>,
    mut index: ResMut<FriendSnapshotIndex>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &mut friends_q,
        &db,
        &mut index,
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
    db: Option<Res<GameDatabase>>,
    mut index: ResMut<FriendSnapshotIndex>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &mut friends_q,
        &db,
        &mut index,
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
    db: &GameDatabase,
    index: &mut FriendSnapshotIndex,
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
            index.register_owner(link.0, conn_entity, &friends);
            let snapshot = build_friends_snapshot(&db.db, &friends, &index.online);
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

#[derive(SystemParam)]
struct ProcessPresenceRequestsParams<'w, 's> {
    receivers: Query<
        'w,
        's,
        (
            Entity,
            &'static PlayerLink,
            &'static mut MessageReceiver<SetPresenceStatus>,
        ),
        With<Authenticated>,
    >,
    friends_q: Query<'w, 's, &'static PlayerFriendsState>,
    players:
        Query<'w, 's, (&'static mut PresenceStatus, &'static mut PresenceTracker), With<Player>>,
    db: Option<Res<'w, GameDatabase>>,
    index: Res<'w, FriendSnapshotIndex>,
    time: Res<'w, Time>,
    auto_afk: ResMut<'w, AutoAfkScheduler>,
    dirty_queue: Option<Res<'w, ReplicationDirtyQueue>>,
    senders: Query<'w, 's, &'static mut MessageSender<FriendsStateUpdate>>,
}

fn process_presence_requests(params: ProcessPresenceRequestsParams) {
    let ProcessPresenceRequestsParams {
        mut receivers,
        friends_q,
        mut players,
        db,
        index,
        time,
        mut auto_afk,
        dirty_queue,
        mut senders,
    } = params;
    let Some(db) = db else { return };
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
            let result = set_presence(
                link.0,
                request.status,
                &mut players,
                &mut auto_afk,
                time.elapsed(),
                dirty_queue.as_deref(),
            );
            let snapshot = build_friends_snapshot(&db.db, friends, &index.online);
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

fn sync_changed_friend_presence(
    players: ChangedFriendPlayers,
    mut index: ResMut<FriendSnapshotIndex>,
) {
    for (entity, player, level, zone, presence) in &players {
        index.set_online(entity, player, level, zone, presence);
    }
}

fn sync_removed_friend_presence(
    mut removed_players: RemovedComponents<Player>,
    mut index: ResMut<FriendSnapshotIndex>,
) {
    for entity in removed_players.read() {
        index.remove_online(entity);
    }
}

fn send_dirty_friends_snapshots(
    mut index: ResMut<FriendSnapshotIndex>,
    mut friends_q: Query<(&PlayerFriendsState, &mut FriendsSnapshotTracker)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    for owner in index.take_dirty_owners() {
        let Some(&connection) = index.connection_by_owner.get(&owner) else {
            continue;
        };
        let Ok((friends, mut tracker)) = friends_q.get_mut(owner) else {
            continue;
        };
        let snapshot = build_friends_snapshot(&db.db, friends, &index.online);
        if tracker.last_sent.as_ref() == Some(&snapshot) {
            continue;
        }
        tracker.last_sent = Some(snapshot.clone());
        send_friends_update(
            connection,
            snapshot,
            Some("friends updated".into()),
            None,
            &mut senders,
        );
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
