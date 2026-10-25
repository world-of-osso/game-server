use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::Player;
use shared::protocol::{
    AddIgnore, IgnoreChannel, IgnoreListSnapshot, IgnoreListStateUpdate, QueryIgnoreList,
    RemoveIgnore,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerIgnoreListState {
    pub names: Vec<String>,
}

pub struct IgnoreListPlugin;

impl Plugin for IgnoreListPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_initial_ignore_list,
                process_query_requests,
                process_add_requests,
                process_remove_requests,
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

fn send_initial_ignore_list(
    connections: NewInWorldConnection,
    mut ignore_q: Query<&mut PlayerIgnoreListState>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    for (conn_entity, link, _in_world) in &connections {
        let Ok(mut ignore_list) = ignore_q.get_mut(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "ignore list unavailable for player",
            );
            continue;
        };
        sanitize_names(&mut ignore_list);
        send_update(
            conn_entity,
            snapshot_from_state(&ignore_list),
            Some("ignore list loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_query_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryIgnoreList>),
        With<Authenticated>,
    >,
    ignore_q: Query<&PlayerIgnoreListState>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let mut saw_request = false;
        for _ in receiver.receive() {
            saw_request = true;
        }
        if !saw_request {
            continue;
        }
        let Ok(ignore_list) = ignore_q.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "ignore list unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            snapshot_from_state(ignore_list),
            Some("ignore list refreshed".into()),
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
            &mut MessageReceiver<AddIgnore>,
        ),
        With<Authenticated>,
    >,
    mut ignore_q: Query<&mut PlayerIgnoreListState>,
    players: Query<&Player>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &mut ignore_q,
        &db,
        &mut senders,
        |link, request, ignore_list| {
            let player = players
                .get(link.0)
                .map_err(|_| "ignore list unavailable for player".to_string())?;
            add_ignore(&db.db, ignore_list, &player.name, request.name.trim())
        },
    );
}

fn process_remove_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<RemoveIgnore>,
        ),
        With<Authenticated>,
    >,
    mut ignore_q: Query<&mut PlayerIgnoreListState>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &mut ignore_q,
        &db,
        &mut senders,
        |_, request, ignore_list| remove_ignore(ignore_list, request.name.trim()),
    );
}

fn process_mutation_requests<R, F>(
    receivers: &mut Query<
        (Entity, &PlayerLink, &InWorld, &mut MessageReceiver<R>),
        With<Authenticated>,
    >,
    ignore_q: &mut Query<&mut PlayerIgnoreListState>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
    mut mutate: F,
) where
    R: lightyear::prelude::Message,
    F: FnMut(&PlayerLink, &R, &mut PlayerIgnoreListState) -> Result<String, String>,
{
    for (conn_entity, link, in_world, mut receiver) in receivers.iter_mut() {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(mut ignore_list) = ignore_q.get_mut(link.0) else {
            send_error(conn_entity, senders, "ignore list unavailable for player");
            continue;
        };
        for request in requests {
            let result = mutate(link, &request, &mut ignore_list);
            handle_mutation_result(
                conn_entity,
                in_world.character_id,
                db,
                &ignore_list,
                result,
                senders,
            );
        }
    }
}

fn handle_mutation_result(
    conn_entity: Entity,
    character_id: u64,
    db: &GameDatabase,
    ignore_list: &PlayerIgnoreListState,
    result: Result<String, String>,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let snapshot = snapshot_from_state(ignore_list);
    match result {
        Ok(message) => {
            persistence::save_ignore_list(&db.db, character_id, ignore_list);
            send_update(conn_entity, snapshot, Some(message), None, senders);
        }
        Err(error) => {
            send_update(conn_entity, snapshot, None, Some(error), senders);
        }
    }
}

fn add_ignore(
    db: &redb::Database,
    ignore_list: &mut PlayerIgnoreListState,
    self_name: &str,
    target_name: &str,
) -> Result<String, String> {
    if target_name.is_empty() {
        return Err("ignore name cannot be empty".into());
    }
    if self_name.eq_ignore_ascii_case(target_name) {
        return Err("cannot ignore yourself".into());
    }
    if ignore_list
        .names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(target_name))
    {
        return Err(format!("'{target_name}' is already ignored"));
    }
    if persistence::find_character_id_by_name(db, target_name).is_none() {
        return Err(format!("character '{target_name}' not found"));
    }
    ignore_list.names.push(target_name.to_string());
    sort_names(ignore_list);
    Ok(format!("ignored: {target_name}"))
}

fn remove_ignore(
    ignore_list: &mut PlayerIgnoreListState,
    target_name: &str,
) -> Result<String, String> {
    let original_len = ignore_list.names.len();
    ignore_list
        .names
        .retain(|name| !name.eq_ignore_ascii_case(target_name));
    if ignore_list.names.len() == original_len {
        return Err(format!("'{target_name}' is not ignored"));
    }
    Ok(format!("ignore removed: {target_name}"))
}

fn sanitize_names(ignore_list: &mut PlayerIgnoreListState) {
    let mut deduped = Vec::with_capacity(ignore_list.names.len());
    for name in ignore_list.names.drain(..) {
        if deduped
            .iter()
            .any(|entry: &String| entry.eq_ignore_ascii_case(&name))
        {
            continue;
        }
        deduped.push(name);
    }
    ignore_list.names = deduped;
    sort_names(ignore_list);
}

fn sort_names(ignore_list: &mut PlayerIgnoreListState) {
    ignore_list.names.sort_by(|a, b| {
        a.to_ascii_lowercase()
            .cmp(&b.to_ascii_lowercase())
            .then(a.cmp(b))
    });
}

fn snapshot_from_state(ignore_list: &PlayerIgnoreListState) -> IgnoreListSnapshot {
    IgnoreListSnapshot {
        names: ignore_list.names.clone(),
    }
}

fn send_update(
    conn_entity: Entity,
    snapshot: IgnoreListSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<IgnoreChannel>(IgnoreListStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

fn send_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<IgnoreChannel>(IgnoreListStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_ignore_rejects_self_and_duplicates() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        crate::persistence::create_character(&db, 1, "Theron", 1, 2).unwrap();
        crate::persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();

        let mut state = PlayerIgnoreListState::default();
        assert_eq!(
            add_ignore(&db, &mut state, "Theron", "Theron").unwrap_err(),
            "cannot ignore yourself"
        );
        assert_eq!(
            add_ignore(&db, &mut state, "Theron", "Alice").unwrap(),
            "ignored: Alice"
        );
        assert_eq!(
            add_ignore(&db, &mut state, "Theron", "alice").unwrap_err(),
            "'alice' is already ignored"
        );
    }

    #[test]
    fn remove_ignore_requires_existing_entry() {
        let mut state = PlayerIgnoreListState {
            names: vec!["Alice".into()],
        };

        assert_eq!(
            remove_ignore(&mut state, "Bob").unwrap_err(),
            "'Bob' is not ignored"
        );
        assert_eq!(
            remove_ignore(&mut state, "alice").unwrap(),
            "ignore removed: alice"
        );
        assert!(state.names.is_empty());
    }

    #[test]
    fn ignore_list_round_trips_through_persistence() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);

        let state = PlayerIgnoreListState {
            names: vec!["Alice".into(), "Bob".into()],
        };
        crate::persistence::save_ignore_list(&db, 42, &state);

        let loaded = crate::persistence::load_ignore_list(&db, 42);
        assert_eq!(loaded, state);
    }
}
