//! Publication-boundary regression: actual source catalogs, real message send/receive,
//! and Lightyear's default message codec. Live UDP reproduction is recorded separately.
use super::*;
use lightyear::connection::host::HostClient;
use lightyear_serde::{registry::SerializeFns, writer::Writer};
use shared::protocol::{AchievementDelta, AdventureGuideClock, AdventureGuideSnapshot};

const PEERS: usize = 11;
const INCREMENTAL_BYTES_PER_PEER: usize = 1024;

struct Peer {
    player: Entity,
    connection: Entity,
}

struct Fixture {
    app: App,
    peers: Vec<Peer>,
}

#[derive(Default)]
struct Received {
    achievements: Vec<AchievementStateUpdate>,
    deltas: Vec<AchievementDelta>,
    guides: Vec<AdventureGuideSnapshot>,
    clocks: Vec<AdventureGuideClock>,
}

impl Received {
    fn bytes(&self) -> usize {
        self.achievements.iter().map(encoded_bytes).sum::<usize>()
            + self.deltas.iter().map(encoded_bytes).sum::<usize>()
            + self.guides.iter().map(encoded_bytes).sum::<usize>()
            + self.clocks.iter().map(encoded_bytes).sum::<usize>()
    }
}

fn encoded_bytes<M: Serialize + serde::de::DeserializeOwned>(message: &M) -> usize {
    let mut writer = Writer::with_capacity(0);
    (SerializeFns::<M>::default().serialize)(message, &mut writer).unwrap();
    writer.len()
}

fn fixture() -> Fixture {
    let path = std::env::var_os("ADVENTURE_GUIDE_WORLD_COPY")
        .expect("explicit private prepared world copy");
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let criteria = load_achievement_catalog(&conn).unwrap();
    let journal = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("db2/active/AdventureJournal.csv");
    let guide = crate::adventure_guide::load_catalog(&conn, &criteria, &journal);
    assert!(matches!(
        guide,
        crate::adventure_guide::AdventureCatalog::Ready(_)
    ));
    let db = persistence::temp_database();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.add_plugins((
        AchievementPlugin,
        crate::adventure_guide::AdventureGuidePlugin,
    ));
    app.insert_resource(GameDatabase { db });
    app.insert_resource(criteria);
    app.insert_resource(guide);
    app.insert_resource(crate::maps::ServerClock {
        fixed_unix: Some(1_792_180_800),
        offset_secs: 0,
    });
    app.finish();
    app.cleanup();
    let peers = (0..PEERS)
        .map(|index| spawn_peer(&mut app, index))
        .collect();
    let mut fixture = Fixture { app, peers };
    let initial = fixture.advance();
    for received in initial {
        assert_eq!(received.guides.len(), 1, "initial full Guide");
        assert_eq!(
            received.achievements.len(),
            1,
            "initial achievement baseline"
        );
        assert!(received.deltas.is_empty(), "no delta before baseline");
        assert!(
            received.guides[0].activities.len() > 100,
            "real full catalog"
        );
    }
    fixture
}

fn spawn_peer(app: &mut App, index: usize) -> Peer {
    let name = format!("Sendproof{}", char::from(b'a' + index as u8));
    let account = index as u64 + 17;
    let (character, _) = persistence::create_character(
        &app.world().resource::<GameDatabase>().db,
        account,
        &name,
        1,
        1,
    )
    .unwrap();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name,
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(1),
            InMap::continent(0),
            PlayerAchievementState::default(),
            crate::persistence::ClientId(character),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(index as u64)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: account,
            },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
            MessageSender::<AchievementStateUpdate>::default(),
            MessageReceiver::<AchievementStateUpdate>::default(),
            MessageSender::<AchievementDelta>::default(),
            MessageReceiver::<AchievementDelta>::default(),
            MessageSender::<AdventureGuideSnapshot>::default(),
            MessageReceiver::<AdventureGuideSnapshot>::default(),
            MessageSender::<AdventureGuideClock>::default(),
            MessageReceiver::<AdventureGuideClock>::default(),
        ))
        .id();
    Peer { player, connection }
}

fn take<M: lightyear::prelude::Message>(world: &mut World, connection: Entity) -> Vec<M> {
    world
        .get_mut::<MessageReceiver<M>>(connection)
        .unwrap()
        .receive()
        .collect()
}

impl Fixture {
    fn advance(&mut self) -> Vec<Received> {
        let mut received: Vec<Received> = (0..PEERS).map(|_| Received::default()).collect();
        for _ in 0..3 {
            self.app.update();
            for (peer, received) in self.peers.iter().zip(&mut received) {
                let world = self.app.world_mut();
                received.achievements.extend(take(world, peer.connection));
                received.deltas.extend(take(world, peer.connection));
                received.guides.extend(take(world, peer.connection));
                received.clocks.extend(take(world, peer.connection));
            }
        }
        received
    }
}

#[test]
fn combat_progress_send_budget_for_eleven_players_does_not_resend_full_catalogs() {
    let mut fixture = fixture();
    for peer in &fixture.peers {
        for hit in [101, 202, 303, 404, 505] {
            record_criteria_event(
                fixture.app.world_mut(),
                peer.player,
                CriteriaEvent::HighestDamageDone(hit),
            );
        }
    }
    let received = fixture.advance();
    let total_bytes: usize = received.iter().map(Received::bytes).sum();
    let guides: usize = received.iter().map(|peer| peer.guides.len()).sum();
    let full_states: usize = received
        .iter()
        .flat_map(|peer| &peer.achievements)
        .filter(|update| update.snapshot.is_some())
        .count();
    let deltas: usize = received.iter().map(|peer| peer.deltas.len()).sum();
    println!(
        "COMBAT_SEND_BUDGET peers={PEERS} events=55 guides={guides} full_states={full_states} deltas={deltas} encoded_bytes={total_bytes}"
    );
    assert_eq!(
        guides, 0,
        "achievement-only progress must not resend Guide content"
    );
    assert_eq!(
        full_states, 0,
        "incremental progress must not resend full achievement state"
    );
    assert_eq!(deltas, PEERS, "one final delta per player for the frame");
    assert!(
        total_bytes <= PEERS * INCREMENTAL_BYTES_PER_PEER,
        "incremental publication bytes: {total_bytes}"
    );
    for peer in received {
        assert!(
            peer.deltas[0]
                .progress_upserts
                .iter()
                .any(|row| row.current == 505),
            "final highest hit retained"
        );
    }
}

#[test]
fn combat_progress_send_budget_preserves_every_toast_without_full_snapshot_clones() {
    let mut fixture = fixture();
    for peer in &fixture.peers {
        for level in [10, 20] {
            fixture
                .app
                .world_mut()
                .get_mut::<Level>(peer.player)
                .unwrap()
                .0 = level;
            record_criteria_event(
                fixture.app.world_mut(),
                peer.player,
                CriteriaEvent::ReachLevel(level),
            );
        }
    }
    let received = fixture.advance();
    for peer in received {
        let toasts: Vec<u32> = peer
            .achievements
            .iter()
            .filter_map(|message| message.completed.as_ref().map(|toast| toast.achievement_id))
            .collect();
        assert_eq!(
            toasts,
            vec![6, 7],
            "source Level10/20 toasts keep their order"
        );
        let full = peer
            .achievements
            .iter()
            .filter(|message| message.snapshot.is_some())
            .count();
        println!(
            "COMBAT_TOAST_BUDGET toasts={toasts:?} full_states={full} deltas={} achievement_bytes={}",
            peer.deltas.len(),
            peer.achievements.iter().map(encoded_bytes).sum::<usize>()
        );
        assert_eq!(
            full, 0,
            "earned alerts must not clone the full achievement snapshot"
        );
        assert_eq!(peer.deltas.len(), 1, "latest earned state coalesced once");
        assert!(
            peer.achievements.iter().map(encoded_bytes).sum::<usize>()
                <= INCREMENTAL_BYTES_PER_PEER,
            "toast payload bounded independently of the full catalog"
        );
    }
}

#[test]
fn combat_progress_send_budget_uses_tiny_clock_for_clock_only_guide_changes() {
    let mut fixture = fixture();
    fixture
        .app
        .world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .fixed_unix = Some(1_792_180_801);
    for peer in &fixture.peers {
        crate::adventure_guide::refresh_for_connection(fixture.app.world_mut(), peer.connection);
    }
    let received = fixture.advance();
    for peer in received {
        println!(
            "COMBAT_CLOCK_BUDGET full_guides={} clocks={} bytes={}",
            peer.guides.len(),
            peer.clocks.len(),
            peer.bytes()
        );
        assert!(
            peer.guides.is_empty(),
            "clock-only refresh must not resend Guide catalog"
        );
        assert_eq!(
            peer.clocks,
            vec![AdventureGuideClock {
                now_unix: 1_792_180_801
            }]
        );
        assert!(
            peer.bytes() <= 64,
            "clock byte bound independent of catalog size"
        );
    }
}
