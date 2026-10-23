use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Player, Zone};
use shared::protocol::{QueryWho, WhoChannel, WhoCharacterSnapshot, WhoSnapshot, WhoStateUpdate};

use crate::auth::{Authenticated, InWorld};

#[cfg(test)]
use std::sync::Mutex;
#[cfg(any(test, feature = "test-support"))]
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct WhoPlugin;

#[cfg(any(test, feature = "test-support"))]
static ONLINE_ROSTER_MATERIALIZATIONS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static ONLINE_ROSTER_TEST_LOCK: Mutex<()> = Mutex::new(());

impl Plugin for WhoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_query_requests);
    }
}

type WhoQueryReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static mut MessageReceiver<QueryWho>,
    ),
    With<Authenticated>,
>;

fn process_query_requests(
    mut receivers: WhoQueryReceivers,
    players: Query<(&Player, &Level, Option<&Zone>)>,
    mut senders: Query<&mut MessageSender<WhoStateUpdate>>,
) {
    let pending_queries = drain_latest_queries(&mut receivers);
    if pending_queries.is_empty() {
        return;
    }

    let online_entries = collect_online_entries(&players);
    for (conn_entity, request) in pending_queries {
        let snapshot = build_who_snapshot(&request.query, online_entries.clone());
        let result_count = snapshot.entries.len();
        send_who_update(
            conn_entity,
            snapshot,
            Some(format!("who: {result_count} result(s)")),
            None,
            &mut senders,
        );
    }
}

fn drain_latest_queries(receivers: &mut WhoQueryReceivers) -> Vec<(Entity, QueryWho)> {
    receivers
        .iter_mut()
        .filter_map(|(conn_entity, _in_world, mut receiver)| {
            receiver
                .receive()
                .last()
                .map(|request| (conn_entity, request))
        })
        .collect()
}

fn collect_online_entries(
    players: &Query<(&Player, &Level, Option<&Zone>)>,
) -> Vec<WhoCharacterSnapshot> {
    #[cfg(any(test, feature = "test-support"))]
    ONLINE_ROSTER_MATERIALIZATIONS.fetch_add(1, Ordering::SeqCst);

    players
        .iter()
        .map(|(player, level, zone)| WhoCharacterSnapshot {
            name: player.name.clone(),
            level: level.0 as u16,
            class_name: class_name_from_id(player.class),
            area: zone
                .map(|zone| format!("Zone {}", zone.id))
                .unwrap_or_else(|| "-".into()),
        })
        .collect()
}

fn build_who_snapshot(query: &str, mut entries: Vec<WhoCharacterSnapshot>) -> WhoSnapshot {
    let query = query.trim().to_string();
    entries.retain(|entry| matches_query(&entry.name, &query));
    entries.sort_by_cached_key(|entry| entry.name.to_ascii_lowercase());
    WhoSnapshot { query, entries }
}

fn matches_query(name: &str, query: &str) -> bool {
    query.is_empty()
        || name
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
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

fn send_who_update(
    conn_entity: Entity,
    snapshot: WhoSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<WhoStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<WhoChannel>(WhoStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lightyear::connection::host::HostClient;
    use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
    use shared::components::CharacterAppearance;

    use super::*;

    fn setup_message_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.register_message::<QueryWho>();
        app.register_message::<WhoStateUpdate>();
        app.add_plugins(WhoPlugin);
        app.finish();
        app.cleanup();
        app
    }

    fn spawn_connection(app: &mut App, peer_id: u64) -> Entity {
        app.world_mut()
            .spawn((
                RemoteId(PeerId::Local(peer_id)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated {
                    account_id: peer_id,
                },
                InWorld {
                    character_id: peer_id,
                },
                MessageSender::<QueryWho>::default(),
                MessageReceiver::<QueryWho>::default(),
                MessageSender::<WhoStateUpdate>::default(),
                MessageReceiver::<WhoStateUpdate>::default(),
            ))
            .id()
    }

    fn spawn_player(app: &mut App, name: &str, level: u8, class: u8, zone: Option<u32>) {
        let player = (
            Player {
                name: name.into(),
                race: 1,
                class,
                appearance: CharacterAppearance::default(),
            },
            Level(level),
        );
        let entity = app.world_mut().spawn(player).id();
        if let Some(zone_id) = zone {
            app.world_mut()
                .entity_mut(entity)
                .insert(Zone { id: zone_id });
        }
    }

    fn send_query(app: &mut App, connection: Entity, query: &str) {
        app.world_mut()
            .entity_mut(connection)
            .get_mut::<MessageSender<QueryWho>>()
            .expect("connection should have a Who query sender")
            .send::<WhoChannel>(QueryWho {
                query: query.into(),
            });
    }

    fn receive_updates(app: &mut App, connection: Entity) -> Vec<WhoStateUpdate> {
        app.world_mut()
            .entity_mut(connection)
            .get_mut::<MessageReceiver<WhoStateUpdate>>()
            .expect("connection should have a Who update receiver")
            .receive()
            .collect()
    }

    fn sample_entries() -> Vec<WhoCharacterSnapshot> {
        vec![
            WhoCharacterSnapshot {
                name: "Zulu".into(),
                level: 42,
                class_name: "Mage".into(),
                area: "Zone 2".into(),
            },
            WhoCharacterSnapshot {
                name: "Alice".into(),
                level: 12,
                class_name: "Paladin".into(),
                area: "Zone 1".into(),
            },
            WhoCharacterSnapshot {
                name: "Theron".into(),
                level: 18,
                class_name: "Hunter".into(),
                area: "Zone 3".into(),
            },
        ]
    }

    #[test]
    fn idle_updates_do_not_materialize_online_roster() {
        let _test_lock = ONLINE_ROSTER_TEST_LOCK.lock().unwrap();
        ONLINE_ROSTER_MATERIALIZATIONS.store(0, Ordering::SeqCst);

        let mut app = App::new();
        app.add_plugins(WhoPlugin);
        app.world_mut().spawn((
            Player {
                name: "Elara".into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            },
            Level(30),
        ));
        app.world_mut().spawn((
            Authenticated { account_id: 7 },
            InWorld { character_id: 11 },
            MessageReceiver::<QueryWho>::default(),
            MessageSender::<WhoStateUpdate>::default(),
        ));

        app.update();
        app.update();

        assert_eq!(
            ONLINE_ROSTER_MATERIALIZATIONS.load(Ordering::SeqCst),
            0,
            "idle updates must not materialize the online roster"
        );
    }

    #[test]
    fn requested_queries_share_one_roster_and_preserve_results() {
        let _test_lock = ONLINE_ROSTER_TEST_LOCK.lock().unwrap();
        let mut app = setup_message_app();
        spawn_player(&mut app, "Zulu", 42, 8, Some(2));
        spawn_player(&mut app, "Alice", 12, 2, Some(1));
        spawn_player(&mut app, "Theron", 18, 3, None);
        let all_results_connection = spawn_connection(&mut app, 21);
        let latest_query_connection = spawn_connection(&mut app, 22);

        app.update();
        ONLINE_ROSTER_MATERIALIZATIONS.store(0, Ordering::SeqCst);
        send_query(&mut app, all_results_connection, "");
        send_query(&mut app, latest_query_connection, "z");
        send_query(&mut app, latest_query_connection, "ali");
        app.update();
        app.update();

        let all_updates = receive_updates(&mut app, all_results_connection);
        assert_eq!(all_updates.len(), 1);
        assert_eq!(all_updates[0].message.as_deref(), Some("who: 3 result(s)"));
        assert_eq!(all_updates[0].error, None);
        let all_snapshot = all_updates[0]
            .snapshot
            .as_ref()
            .expect("Who response should contain a snapshot");
        assert_eq!(all_snapshot.query, "");
        assert_eq!(
            all_snapshot
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alice", "Theron", "Zulu"]
        );
        assert_eq!(all_snapshot.entries[0].class_name, "Paladin");
        assert_eq!(all_snapshot.entries[0].area, "Zone 1");
        assert_eq!(all_snapshot.entries[1].area, "-");

        let latest_updates = receive_updates(&mut app, latest_query_connection);
        assert_eq!(latest_updates.len(), 1);
        assert_eq!(
            latest_updates[0].message.as_deref(),
            Some("who: 1 result(s)")
        );
        assert_eq!(latest_updates[0].error, None);
        let latest_snapshot = latest_updates[0]
            .snapshot
            .as_ref()
            .expect("Who response should contain a snapshot");
        assert_eq!(latest_snapshot.query, "ali");
        assert_eq!(latest_snapshot.entries.len(), 1);
        assert_eq!(latest_snapshot.entries[0].name, "Alice");

        assert_eq!(
            ONLINE_ROSTER_MATERIALIZATIONS.load(Ordering::SeqCst),
            1,
            "one update with requests should materialize one shared online roster"
        );
    }

    #[test]
    fn ineligible_connections_do_not_receive_responses_or_build_roster() {
        let _test_lock = ONLINE_ROSTER_TEST_LOCK.lock().unwrap();
        let mut app = setup_message_app();
        spawn_player(&mut app, "Elara", 30, 2, Some(1));
        let unauthenticated = spawn_connection(&mut app, 31);
        let outside_world = spawn_connection(&mut app, 32);
        app.world_mut()
            .entity_mut(unauthenticated)
            .remove::<Authenticated>();
        app.world_mut()
            .entity_mut(outside_world)
            .remove::<InWorld>();

        app.update();
        ONLINE_ROSTER_MATERIALIZATIONS.store(0, Ordering::SeqCst);
        send_query(&mut app, unauthenticated, "Elara");
        send_query(&mut app, outside_world, "Elara");
        app.update();
        app.update();

        assert_eq!(
            ONLINE_ROSTER_MATERIALIZATIONS.load(Ordering::SeqCst),
            0,
            "ineligible connections must not trigger online roster materialization"
        );
        assert!(receive_updates(&mut app, unauthenticated).is_empty());
        assert!(receive_updates(&mut app, outside_world).is_empty());
    }

    #[test]
    fn build_who_snapshot_filters_case_insensitively() {
        let snapshot = build_who_snapshot("ali", sample_entries());

        assert_eq!(snapshot.query, "ali");
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].name, "Alice");
    }

    #[test]
    fn build_who_snapshot_sorts_results_by_name() {
        let snapshot = build_who_snapshot("", sample_entries());

        assert_eq!(snapshot.entries.len(), 3);
        assert_eq!(snapshot.entries[0].name, "Alice");
        assert_eq!(snapshot.entries[1].name, "Theron");
        assert_eq!(snapshot.entries[2].name, "Zulu");
    }
}
