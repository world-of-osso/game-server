use super::*;
use crate::persistence::GameDatabase;
use crate::pvp::pvp_support::build_pvp_snapshot;
use shared::currency::{CurrencyWallet, currency_by_id};
use shared::protocol::CurrencyStateUpdate;
use shared::protocol::{PvpQueueKindSnapshot, PvpQueueSnapshot};

#[test]
fn queue_for_battleground_sets_queue_state() {
    let mut player_state = PlayerPvpState::default();
    let mut state = PvpServerState::default();

    let message = queue_for_battleground(&mut player_state, &mut state, 10, 30, 1, 5)
        .expect("queue succeeds");

    assert_eq!(message, "queued for Warsong Gulch");
    assert!(matches!(
        player_state.queue,
        Some(PlayerPvpQueue::Battleground {
            battleground_id: 1,
            ..
        })
    ));
}

#[test]
fn queue_for_rated_bracket_blocks_double_queue() {
    let mut player_state = PlayerPvpState::default();
    let mut state = PvpServerState::default();

    queue_for_rated_bracket(
        &mut player_state,
        &mut state,
        44,
        PvpBracketSnapshot::Arena2v2,
    )
    .expect("first queue succeeds");

    let error = queue_for_rated_bracket(
        &mut player_state,
        &mut state,
        44,
        PvpBracketSnapshot::Arena3v3,
    )
    .expect_err("second queue must fail");

    assert!(error.contains("already queued"));
}

#[test]
fn build_snapshot_reflects_queue_kind() {
    let state = PlayerPvpState {
        progress: PlayerPvpProgress::default(),
        queue: Some(PlayerPvpQueue::RatedBracket {
            bracket: PvpBracketSnapshot::SoloShuffle,
        }),
    };

    let snapshot = build_pvp_snapshot(&state);

    assert_eq!(snapshot.brackets.len(), 4);
    assert!(matches!(
        snapshot.queue,
        Some(PvpQueueSnapshot {
            kind: PvpQueueKindSnapshot::RatedBracket {
                bracket: PvpBracketSnapshot::SoloShuffle
            }
        })
    ));
}

#[test]
fn sync_progress_currencies_from_wallet_uses_wallet_amounts() {
    let mut progress = PlayerPvpProgress::default();
    let mut wallet = CurrencyWallet::default();
    wallet.add(1, 275, currency_by_id(1).unwrap()).unwrap();
    wallet.add(2, 120, currency_by_id(2).unwrap()).unwrap();

    let changed = sync_progress_currencies_from_wallet(&mut progress, &wallet);

    assert!(changed);
    assert_eq!(progress.honor, 275);
    assert_eq!(progress.conquest, 120);
}

#[test]
fn rated_pairing_never_manufactures_a_result_or_currency() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.add_plugins(PvpPlugin);
    app.finish();
    app.cleanup();
    app.insert_resource(GameDatabase {
        db: crate::persistence::temp_database(),
    });
    let mut owners = Vec::new();
    for id in [41, 42] {
        let owner = app
            .world_mut()
            .spawn((
                shared::components::Player {
                    name: format!("fb_pvp_{id}"),
                    race: 1,
                    class: 1,
                    appearance: shared::components::CharacterAppearance::default(),
                },
                Level(90),
                PlayerPvpState::default(),
                crate::currency::PlayerCurrencies::default(),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                Authenticated { account_id: id },
                InWorld { character_id: id },
                PlayerLink(owner),
                RemoteId(PeerId::Local(id)),
                Connected,
                lightyear::connection::host::HostClient { buffer: Vec::new() },
                MessageSender::<QueueForRatedPvp>::default(),
                MessageReceiver::<QueueForRatedPvp>::default(),
                MessageSender::<PvpStateUpdate>::default(),
                MessageReceiver::<PvpStateUpdate>::default(),
                MessageSender::<CurrencyStateUpdate>::default(),
            ))
            .id();
        app.world_mut()
            .get_mut::<MessageSender<QueueForRatedPvp>>(connection)
            .unwrap()
            .send::<shared::protocol::PvpChannel>(QueueForRatedPvp {
                bracket: PvpBracketSnapshot::Arena2v2,
            });
        owners.push(owner);
    }
    for _ in 0..5 {
        app.update();
    }
    for owner in owners {
        let state = app.world().get::<PlayerPvpState>(owner).unwrap();
        assert_eq!(
            state.progress,
            PlayerPvpProgress::default(),
            "pairing is not a played arena"
        );
        assert!(
            state.queue.is_some(),
            "queue remains pending real match lifecycle"
        );
        assert!(
            app.world()
                .get::<crate::currency::PlayerCurrencies>(owner)
                .unwrap()
                .0
                .entries
                .is_empty()
        );
    }
}
