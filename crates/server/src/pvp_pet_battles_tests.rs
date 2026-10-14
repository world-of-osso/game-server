use super::*;

fn app() -> App {
    build_app(false)
}

fn build_app(track_achievements: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    if track_achievements {
        app.add_plugins(crate::achievement::AchievementPlugin);
    }
    app.finish();
    app.cleanup();
    app.insert_resource(super::super::tests::catalog());
    app.insert_resource(GameDatabase::new(
        redb::Database::create(tempfile::NamedTempFile::new().unwrap().path()).unwrap(),
    ));
    app.insert_resource(crate::battle_pets::load_test_catalog());
    app.add_systems(Update, crate::battle_pets::load_account_journals);
    app.init_resource::<Matchmaker>();
    app.init_resource::<BattleIds>();
    app
}
fn player(app: &mut App, account: u64) -> (Entity, Entity) {
    let mut journal = PetJournal::default();
    for guid in account * 10..account * 10 + 3 {
        journal
            .add_with_guid(39, 1, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    save_journal(
        &app.world().resource::<GameDatabase>().db,
        account,
        &journal,
    );
    let owner = app
        .world_mut()
        .spawn((
            crate::collection::default_collection_state(),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Authenticated {
                account_id: account,
            },
            InWorld {
                character_id: account,
            },
            PlayerLink(owner),
            lightyear::prelude::RemoteId(lightyear::prelude::PeerId::Local(account)),
            lightyear::prelude::Connected,
            lightyear::connection::host::HostClient { buffer: Vec::new() },
            MessageSender::<PetBattleQueueUpdate>::default(),
            MessageReceiver::<PetBattleQueueUpdate>::default(),
            MessageSender::<WildPetBattleUpdate>::default(),
            MessageReceiver::<WildPetBattleUpdate>::default(),
        ))
        .id();
    app.update();
    (owner, connection)
}
#[test]
fn petbattle_pvp_initial_choices_wait_for_both_without_spending_a_round() {
    let mut app = app();
    let [a, b] = proposed(&mut app);
    let id = app.world().get::<PvpParticipant>(a).unwrap().id;
    let round = app.world().resource::<Matchmaker>().battles[&id]
        .encounter
        .battle
        .round;
    assert!(
        snapshot(
            app.world(),
            &app.world().resource::<Matchmaker>().battles[&id],
            0
        )
        .initial_selection_required
    );
    assert!(
        advance(
            app.world_mut(),
            a,
            WildPetBattleActionRequest {
                battle_id: id,
                round,
                action: WildPetBattleAction::Ability(1)
            }
        )
        .is_err()
    );
    for (owner, slot) in [(a, 3), (b, 2)] {
        advance(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round,
                action: WildPetBattleAction::Swap(slot),
            },
        )
        .unwrap();
        assert_eq!(
            app.world().resource::<Matchmaker>().battles[&id]
                .encounter
                .battle
                .round,
            round
        );
    }
    let battle = &app.world().resource::<Matchmaker>().battles[&id];
    assert!(!snapshot(app.world(), battle, 0).initial_selection_required);
    assert!(!snapshot(app.world(), battle, 1).initial_selection_required);
    assert_eq!(battle.encounter.battle.teams[0].active, 2);
    assert_eq!(battle.encounter.battle.teams[1].active, 1);
    assert!(
        battle
            .encounter
            .battle
            .teams
            .iter()
            .all(|t| t.pets.iter().all(|p| p.health == p.stats.health))
    );
}

#[test]
fn petbattle_pvp_find_battle_queues_once_and_can_leave() {
    let mut app = app();
    let (owner, _) = player(&mut app, 41);
    queue_request(app.world_mut(), owner, PetBattleQueueRequest::Join).unwrap();
    assert_eq!(app.world().resource::<Matchmaker>().queued, [owner]);
    assert!(queue_request(app.world_mut(), owner, PetBattleQueueRequest::Join).is_err());
    queue_request(app.world_mut(), owner, PetBattleQueueRequest::Leave).unwrap();
    assert!(app.world().resource::<Matchmaker>().queued.is_empty());
}

fn matched(app: &mut App) -> [Entity; 2] {
    let owners = proposed(app);
    let id = app.world().get::<PvpParticipant>(owners[0]).unwrap().id;
    let round = app.world().resource::<Matchmaker>().battles[&id]
        .encounter
        .battle
        .round;
    for owner in owners {
        advance(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round,
                action: WildPetBattleAction::Swap(1),
            },
        )
        .unwrap();
    }
    owners
}

fn proposed(app: &mut App) -> [Entity; 2] {
    let a = player(app, 41).0;
    let b = player(app, 42).0;
    for owner in [a, b] {
        queue_request(app.world_mut(), owner, PetBattleQueueRequest::Join).unwrap();
    }
    propose_matches(app.world_mut());
    let id = *app
        .world()
        .resource::<Matchmaker>()
        .proposals
        .keys()
        .next()
        .unwrap();
    for owner in [a, b] {
        queue_request(
            app.world_mut(),
            owner,
            PetBattleQueueRequest::Accept { proposal_id: id },
        )
        .unwrap();
    }
    receive_queue(app.world_mut());
    [a, b]
}
#[test]
fn petbattle_pvp_two_players_wait_for_both_decisions_and_get_oriented_final_results() {
    let mut app = app();
    let [a, b] = matched(&mut app);
    let id = app.world().get::<PvpParticipant>(a).unwrap().id;
    let round = app.world().resource::<Matchmaker>().battles[&id]
        .encounter
        .battle
        .round;
    let request = WildPetBattleActionRequest {
        battle_id: id,
        round,
        action: WildPetBattleAction::Ability(1),
    };
    advance(app.world_mut(), a, request.clone()).unwrap();
    assert_eq!(
        app.world().resource::<Matchmaker>().battles[&id]
            .encounter
            .battle
            .round,
        round
    );
    assert!(advance(app.world_mut(), a, request.clone()).is_err());
    advance(app.world_mut(), b, request).unwrap();
    let battle = &app.world().resource::<Matchmaker>().battles[&id];
    assert_eq!(battle.encounter.battle.round, round + 1);
    assert!(
        battle
            .encounter
            .battle
            .teams
            .iter()
            .all(|t| t.pets[0].health < t.pets[0].stats.health)
    );
    let a_state = snapshot(app.world(), battle, 0);
    let b_state = snapshot(app.world(), battle, 1);
    assert_eq!(a_state.teams[0], b_state.teams[1]);
    assert_eq!(a_state.teams[1], b_state.teams[0]);
    assert!(!a_state.can_trap && !b_state.can_trap);
    let round = battle.encounter.battle.round;
    advance(
        app.world_mut(),
        a,
        WildPetBattleActionRequest {
            battle_id: id,
            round,
            action: WildPetBattleAction::Forfeit,
        },
    )
    .unwrap();
    assert!(!is_in_battle(app.world(), a) && !is_in_battle(app.world(), b));
    assert!(app.world().resource::<Matchmaker>().battles.is_empty());
    app.update();
    for (owner, outcome) in [
        (a, WildPetBattleOutcome::Forfeited),
        (b, WildPetBattleOutcome::Won),
    ] {
        let connection = connection(app.world_mut(), owner).unwrap();
        let updates: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<WildPetBattleUpdate>>(connection)
            .unwrap()
            .receive()
            .collect();
        assert!(updates.iter().any(|update| matches!(update, WildPetBattleUpdate::End { outcome: result, captured_pet_id: None, .. } if result == &outcome)));
        assert!(
            updates
                .iter()
                .any(|update| matches!(update, WildPetBattleUpdate::State(_)))
        );
    }
}
#[test]
fn petbattle_pvp_decline_requeues_opponent_and_stale_accept_cannot_start() {
    let mut app = app();
    let a = player(&mut app, 41).0;
    let b = player(&mut app, 42).0;
    for owner in [a, b] {
        queue_request(app.world_mut(), owner, PetBattleQueueRequest::Join).unwrap();
    }
    propose_matches(app.world_mut());
    let id = *app
        .world()
        .resource::<Matchmaker>()
        .proposals
        .keys()
        .next()
        .unwrap();
    queue_request(
        app.world_mut(),
        a,
        PetBattleQueueRequest::Decline { proposal_id: id },
    )
    .unwrap();
    assert_eq!(state(app.world(), a), PetBattleQueueState::Idle);
    assert_eq!(state(app.world(), b), PetBattleQueueState::Queued);
    assert!(
        queue_request(
            app.world_mut(),
            b,
            PetBattleQueueRequest::Accept { proposal_id: id }
        )
        .is_err()
    );
}

#[test]
fn petbattle_pvp_disconnect_finishes_opponent_and_releases_account() {
    let mut app = app();
    let [a, b] = matched(&mut app);
    let disconnected = connection(app.world_mut(), a).unwrap();
    app.world_mut().entity_mut(disconnected).remove::<InWorld>();
    remove_abandoned(app.world_mut());
    assert!(!is_in_battle(app.world(), a));
    assert!(!is_in_battle(app.world(), b));
    assert!(app.world().resource::<Matchmaker>().battles.is_empty());
    app.update();
    let connection = connection(app.world_mut(), b).unwrap();
    let delivered: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<WildPetBattleUpdate>>(connection)
        .unwrap()
        .receive()
        .collect();
    assert!(delivered.iter().any(|update| matches!(
        update,
        WildPetBattleUpdate::End {
            outcome: WildPetBattleOutcome::Won,
            ..
        }
    )));
}

fn finish_battle_with_winner(app: &mut App, winner: Entity, loser: Entity) {
    let id = app.world().get::<PvpParticipant>(winner).unwrap().id;
    for _ in 0..100 {
        let Some(battle) = app.world().resource::<Matchmaker>().battles.get(&id) else {
            break;
        };
        let round = battle.encounter.battle.round;
        let team = &battle.encounter.battle.teams[1];
        if team.pets[team.active].health <= 0 {
            let slot = team.pets.iter().position(|p| p.health > 0).unwrap();
            advance(
                app.world_mut(),
                loser,
                WildPetBattleActionRequest {
                    battle_id: id,
                    round,
                    action: WildPetBattleAction::Swap(slot as u8 + 1),
                },
            )
            .unwrap();
        } else {
            advance(
                app.world_mut(),
                winner,
                WildPetBattleActionRequest {
                    battle_id: id,
                    round,
                    action: WildPetBattleAction::Ability(1),
                },
            )
            .unwrap();
            advance(
                app.world_mut(),
                loser,
                WildPetBattleActionRequest {
                    battle_id: id,
                    round,
                    action: WildPetBattleAction::Pass,
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn achievements_criteria_pet_win_credits_actual_winner_once_and_not_the_loser() {
    use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
    use shared::components::{CharacterAppearance, Player};
    let mut app = build_app(true);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("pet-zone-proof.redb");
    app.insert_resource(GameDatabase::new(redb::Database::create(&db_path).unwrap()));
    crate::persistence::create_tables(&app.world().resource::<GameDatabase>().db);
    let characters = [(41, "Petwinner"), (42, "Petloser")].map(|(account, name)| {
        crate::persistence::create_character(
            &app.world().resource::<GameDatabase>().db,
            account,
            name,
            1,
            1,
        )
        .unwrap()
        .0
    });
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(crate::area_table::load_area_table(&conn).unwrap());
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let data = std::env::var_os("GAME_SERVER_GROUND_DIR").expect("real ground asset directory");
    let ground = crate::ground::GroundMap::new(
        std::path::Path::new(&data),
        maps.maps
            .iter()
            .map(|(&id, map)| (id, map.wdt_file_data_id))
            .collect(),
    );
    app.insert_resource(ground);
    app.init_resource::<crate::maps::ServerClock>();
    let [a, b] = matched(&mut app);
    for (owner, character) in [a, b].into_iter().zip(characters) {
        let connection = connection(app.world_mut(), owner).unwrap();
        app.world_mut()
            .get_mut::<InWorld>(connection)
            .unwrap()
            .character_id = character;
    }
    for (owner, name) in [(a, "Petwinner"), (b, "Petloser")] {
        app.world_mut().entity_mut(owner).insert((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            PlayerAchievementState::default(),
            shared::components::Level(1),
            crate::maps::InMap::continent(0),
        ));
        let (x, y, z) = crate::terrain_height::wow_to_bevy(-8833.38, 628.62, 94.0);
        app.world_mut()
            .entity_mut(owner)
            .insert(shared::components::Position { x, y, z });
        let connection = connection(app.world_mut(), owner).unwrap();
        let character_id = app.world().get::<InWorld>(connection).unwrap().character_id;
        app.world_mut().entity_mut(connection).remove::<InWorld>();
        app.world_mut()
            .entity_mut(connection)
            .insert(InWorld { character_id });
    }
    app.update();
    // Nine previous wins are setup; the actual engine win must cross the real tenth-win threshold.
    app.world_mut()
        .get_mut::<PlayerAchievementState>(a)
        .unwrap()
        .criteria_progress
        .insert(19724, 9);
    finish_battle_with_winner(&mut app, a, b);
    assert!(app.world().resource::<Matchmaker>().battles.is_empty());
    app.update();
    let winner = app.world().get::<PlayerAchievementState>(a).unwrap();
    assert_eq!(winner.criteria_progress.get(&19724), Some(&10));
    assert_eq!(winner.criteria_progress.get(&19845), Some(&1));
    assert!(!winner.criteria_progress.contains_key(&19846));
    assert!(!winner.criteria_progress.contains_key(&19849));
    assert!(
        winner
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&6594)
    );
    assert!(!winner.criteria_progress.contains_key(&21526));
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(b)
            .unwrap()
            .criteria_progress
            .contains_key(&19724)
    );
    assert_eq!(
        crate::persistence::load_achievements(
            &app.world().resource::<GameDatabase>().db,
            characters[0]
        )
        .criteria_progress
        .get(&19724),
        Some(&10)
    );
    assert_eq!(
        crate::persistence::load_achievements(
            &app.world().resource::<GameDatabase>().db,
            characters[0]
        )
        .criteria_progress
        .get(&19845),
        Some(&1)
    );
    for (owner, expected) in [
        (a, WildPetBattleOutcome::Won),
        (b, WildPetBattleOutcome::Lost),
    ] {
        let connection = connection(app.world_mut(), owner).unwrap();
        let messages: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<WildPetBattleUpdate>>(connection)
            .unwrap()
            .receive()
            .collect();
        assert!(messages.iter().any(|m| matches!(m, WildPetBattleUpdate::End { outcome, captured_pet_id: None, .. } if outcome == &expected)));
        let terminal = messages
            .iter()
            .rev()
            .find_map(|m| {
                if let WildPetBattleUpdate::State(s) = m {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap();
        let defeated = if owner == a { 1 } else { 0 };
        assert!(terminal.teams[defeated].iter().all(|p| p.health == 0));
        assert_eq!(
            app.world()
                .get::<AccountPetJournal>(owner)
                .unwrap()
                .journal
                .count(),
            3
        );
    }
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(b)
            .unwrap()
            .criteria_progress
            .contains_key(&19845)
    );
    // Consume terminal network messages before advancing their receiver lifetime.
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(a)
            .unwrap()
            .criteria_progress
            .get(&19724),
        Some(&10)
    );
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(a)
            .unwrap()
            .criteria_progress
            .get(&19845),
        Some(&1)
    );
    drop(app);
    let reopened = redb::Database::open(&db_path).unwrap();
    let saved = crate::persistence::load_achievements(&reopened, characters[0]);
    assert_eq!(saved.criteria_progress.get(&19845), Some(&1));
}
