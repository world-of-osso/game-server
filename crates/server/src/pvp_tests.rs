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

    let message = queue_for_battleground(&mut player_state, &mut state, 10, 90, 1014, 5)
        .expect("queue succeeds");

    assert_eq!(message, "queued for Warsong Gulch");
    assert!(matches!(
        player_state.queue,
        Some(PlayerPvpQueue::Battleground {
            battleground_id: 1014,
            ..
        })
    ));
}

/// Map.csv (12.1.0.69933), map 529: "Earn 1500 resources".
/// Exercises multiple players through the server queue and shared end condition;
/// it does not claim transport, invitations, or an instantiated map are wired.
#[test]
fn arathi_multiple_players_finish_at_authored_resource_limit() {
    use shared::battleground::{BgInstance, BgOutcome, BgPhase, BgTeam, arathi_basin};

    let template = arathi_basin();
    let mut state = PvpServerState::default();
    let player_count = u64::from(template.min_players_per_team) * 2;
    let mut players = vec![PlayerPvpState::default(); player_count as usize];
    for (index, player) in players.iter_mut().enumerate() {
        let character_id = index as u64 + 1;
        queue_for_battleground(player, &mut state, character_id, 30, template.id, 0)
            .expect("eligible player queues");
    }

    let matched = state.battleground_queue.try_match(template.id).unwrap();
    let mut participants: Vec<_> = matched
        .team_a
        .iter()
        .chain(&matched.team_b)
        .copied()
        .collect();
    participants.sort_unstable();
    assert_eq!(participants, (1..=player_count).collect::<Vec<_>>());
    assert!(state.battleground_queue.is_empty());

    let mut instance = BgInstance::from_match(&matched, &template);
    assert!(instance.start());
    instance.add_score(BgTeam::A, 1490, &template);
    assert_eq!(instance.phase, BgPhase::InProgress);
    instance.add_score(BgTeam::A, 10, &template);
    assert_eq!(instance.phase, BgPhase::Ended);
    assert_eq!(instance.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    instance.add_score(BgTeam::B, 1500, &template);
    assert_eq!(instance.score_b.score, 0);
    assert_eq!(instance.outcome, Some(BgOutcome::Winner(BgTeam::A)));
}

#[test]
fn objective_tick_without_matches_needs_no_world_object_catalog() {
    let mut world = World::new();
    world.init_resource::<PvpServerState>();
    for _ in 0..200 {
        super::pvp_objective::update_battleground_objectives(&mut world);
    }
    assert!(
        world
            .resource::<PvpServerState>()
            .battleground_matches
            .is_empty()
    );
}

#[test]
fn matched_warsong_players_keep_an_invitation_until_their_response() {
    let mut state = PvpServerState::default();
    let template = shared::battleground::warsong_gulch();
    let mut players = vec![PlayerPvpState::default(); 10];
    for (index, player) in players.iter_mut().enumerate() {
        queue_for_battleground(player, &mut state, index as u64 + 1, 90, template.id, 0).unwrap();
    }
    let match_id = form_battleground_invitation(&mut state, template.id).unwrap();
    assert!(state.battleground_queue.is_empty());
    let invitation = state
        .battleground_matches
        .get(&match_id)
        .expect("matched roster retained");
    assert_eq!(invitation.instance.team_a_players, vec![1, 3, 5, 7, 9]);
    assert_eq!(invitation.instance.team_b_players, vec![2, 4, 6, 8, 10]);
    assert!(invitation.accepted.is_empty());
    assert_eq!(
        invitation.instance.phase,
        shared::battleground::BgPhase::Waiting
    );
}

#[test]
fn battleground_response_is_owned_idempotent_and_scoped_to_the_invitation() {
    use shared::battleground::{BgInstance, BgMatch};
    let template = shared::battleground::warsong_gulch();
    let matched = BgMatch {
        bg_id: template.id,
        team_a: vec![1, 3, 5, 7, 9],
        team_b: vec![2, 4, 6, 8, 10],
    };
    let mut state = PvpServerState::default();
    state.battleground_matches.insert(
        17,
        InvitedBattleground {
            instance: BgInstance::from_match(&matched, &template),
            accepted: BTreeSet::new(),
            map: None,
            origins: BTreeMap::new(),
            flags: Default::default(),
            nodes: None,
            eye: None,
            kotmogu: None,
            silvershard: None,
            seething: None,
            deephaul: None,
            alterac: None,
            scoreboard: Vec::new(),
            flag_positions: [None, None],
        },
    );
    respond_to_battleground_invitation(&mut state, 17, 1, true).unwrap();
    assert_eq!(
        state.battleground_matches[&17].accepted,
        BTreeSet::from([1])
    );
    assert!(respond_to_battleground_invitation(&mut state, 17, 1, true).is_err());
    assert!(respond_to_battleground_invitation(&mut state, 18, 2, true).is_err());
    assert!(respond_to_battleground_invitation(&mut state, 17, 11, true).is_err());
    respond_to_battleground_invitation(&mut state, 17, 2, false).unwrap();
    assert_eq!(
        state.battleground_matches[&17].instance.team_b_players,
        vec![4, 6, 8, 10]
    );
    assert_eq!(
        state.battleground_matches[&17].instance.team_a_players,
        matched.team_a
    );
    assert_eq!(
        state.battleground_matches[&17].accepted,
        BTreeSet::from([1])
    );
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
        battleground: None,
        battleground_original_faction: None,
        war_mode: None,
        talents: None,
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
    wallet
        .add(1792, 275, currency_by_id(1792).unwrap())
        .unwrap();
    wallet
        .add(1602, 120, currency_by_id(1602).unwrap())
        .unwrap();

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

#[test]
fn new_pvp_progress_does_not_invent_rated_or_season_cap_numbers() {
    let state = PlayerPvpState::default();
    let snapshot = build_pvp_snapshot(&state);
    assert!(
        snapshot
            .brackets
            .iter()
            .all(|bracket| bracket.rating.is_none())
    );
    assert!(snapshot.conquest_max.is_none());
    assert_eq!(snapshot.honor_max, 15000);
}
