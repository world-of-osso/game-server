mod actor;
use actor::{Mutation, PendingMutations, publish_mutations};
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
        app.init_resource::<PendingMutations>().add_systems(
            Update,
            (
                send_initial_ignore_list,
                process_query_requests,
                process_add_requests,
                process_remove_requests,
                publish_mutations,
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
    ignore_q: Query<&PlayerIgnoreListState>,
    players: Query<&Player>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingMutations>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &ignore_q,
        &db,
        &mut pending,
        &mut senders,
        |link, request| {
            let player = players
                .get(link.0)
                .map_err(|_| "ignore list unavailable for player".to_string())?;
            Ok(Mutation::Add {
                self_name: player.name.clone(),
                target: request.name.trim().to_string(),
            })
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
    ignore_q: Query<&PlayerIgnoreListState>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingMutations>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_mutation_requests(
        &mut receivers,
        &ignore_q,
        &db,
        &mut pending,
        &mut senders,
        |_, request| {
            Ok(Mutation::Remove {
                target: request.name.trim().to_string(),
            })
        },
    );
}

fn process_mutation_requests<R, F>(
    receivers: &mut Query<
        (Entity, &PlayerLink, &InWorld, &mut MessageReceiver<R>),
        With<Authenticated>,
    >,
    ignore_q: &Query<&PlayerIgnoreListState>,
    db: &GameDatabase,
    pending: &mut PendingMutations,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
    mut mutate: F,
) where
    R: lightyear::prelude::Message,
    F: FnMut(&PlayerLink, &R) -> Result<Mutation, String>,
{
    for (conn_entity, link, in_world, mut receiver) in receivers.iter_mut() {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(_) = ignore_q.get(link.0) else {
            send_error(conn_entity, senders, "ignore list unavailable for player");
            continue;
        };
        for request in requests {
            match mutate(link, &request) {
                Ok(mutation) => {
                    pending.enqueue(db, conn_entity, link.0, in_world.character_id, mutation)
                }
                Err(error) => send_error(conn_entity, senders, &error),
            }
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
    fn ignore_mutation_waits_for_fifo_without_blocking_updates() {
        use lightyear::connection::host::HostClient;
        use std::time::{Duration, Instant};
        let db = persistence::temp_database();
        persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.register_message::<QueryIgnoreList>();
        app.register_message::<AddIgnore>();
        app.register_message::<RemoveIgnore>();
        app.register_message::<IgnoreListStateUpdate>();
        app.insert_resource(GameDatabase::new(db))
            .add_plugins(IgnoreListPlugin);
        app.finish();
        app.cleanup();
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: "Fifoignore".into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                PlayerIgnoreListState::default(),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                Connected,
                RemoteId(PeerId::Local(1)),
                HostClient { buffer: Vec::new() },
                Authenticated { account_id: 1 },
                InWorld { character_id: 42 },
                PlayerLink(player),
                MessageSender::<AddIgnore>::default(),
                MessageReceiver::<AddIgnore>::default(),
                MessageSender::<RemoveIgnore>::default(),
                MessageReceiver::<RemoveIgnore>::default(),
                MessageSender::<IgnoreListStateUpdate>::default(),
                MessageReceiver::<IgnoreListStateUpdate>::default(),
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<MessageReceiver<IgnoreListStateUpdate>>(connection)
            .unwrap()
            .receive()
            .for_each(drop);
        let held = app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            held.abort().unwrap();
        });
        app.world_mut()
            .get_mut::<MessageSender<AddIgnore>>(connection)
            .unwrap()
            .send::<IgnoreChannel>(AddIgnore {
                name: "Alice".into(),
            });
        let started = Instant::now();
        app.update();
        app.update();
        let elapsed = started.elapsed();
        let premature = app
            .world_mut()
            .get_mut::<MessageReceiver<IgnoreListStateUpdate>>(connection)
            .unwrap()
            .receive()
            .next();
        release.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(50),
            "ignore tick waited: {elapsed:?}"
        );
        assert!(premature.is_none(), "ignore reply precedes commit");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            app.update();
            if let Some(response) = app
                .world_mut()
                .get_mut::<MessageReceiver<IgnoreListStateUpdate>>(connection)
                .unwrap()
                .receive()
                .next()
            {
                assert!(response.error.is_none());
                assert_eq!(response.snapshot.unwrap().names, vec!["Alice"]);
                break;
            }
            assert!(Instant::now() < deadline, "no durable ignore reply");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            persistence::load_ignore_list(&app.world().resource::<GameDatabase>().db, 42).names,
            vec!["Alice"]
        );
        assert_eq!(
            app.world()
                .get::<PlayerIgnoreListState>(player)
                .unwrap()
                .names,
            vec!["Alice"]
        );
    }

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
