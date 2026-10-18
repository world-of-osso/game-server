use super::*;
use crate::persistence::GameDatabase;

#[path = "pvp_bg_honor_tests.rs"]
mod bg_honor;
#[path = "pvp_content_tuning_tests.rs"]
mod content_tuning;
#[path = "pvp_honor_area_tests.rs"]
mod honor_area;
#[path = "pvp_honor_class_tests.rs"]
mod honor_class;
#[path = "pvp_honor_tests.rs"]
mod honor_progress;
#[path = "pvp_honor_race_tests.rs"]
mod honor_race;
#[path = "pvp_kill_criteria_tests.rs"]
mod kill_criteria;
#[path = "pvp_lifetime_honor_tests.rs"]
mod lifetime_honor;
use crate::pvp::pvp_support::build_pvp_snapshot;
use shared::currency::{CurrencyWallet, currency_by_id};
use shared::protocol::CurrencyStateUpdate;
use shared::protocol::{PvpQueueKindSnapshot, PvpQueueSnapshot, QueueForRatedPvp};

#[test]
fn arena_skirmish_pairing_requires_four_or_six_unique_eligible_players() {
    use shared::protocol::ArenaQueueMode;
    let mut database = rusqlite::Connection::open_in_memory().unwrap();
    database.execute_batch("CREATE TABLE arena_map(battlemaster_id INTEGER,map_id INTEGER,name TEXT,start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,weight INTEGER,wdt_fdid INTEGER); INSERT INTO arena_map VALUES(8,572,'Ruins of Lordaeron',1258,1259,10,90,1,789984);").unwrap();
    let catalog = arena::Catalog::load(&mut database).unwrap();
    for (mode, count) in [
        (ArenaQueueMode::Skirmish2v2, 4),
        (ArenaQueueMode::Skirmish3v3, 6),
    ] {
        let mut state = PvpServerState::default();
        let mut players = vec![PlayerPvpState::default(); count];
        assert!(arena::queue_player(&mut players[0], &mut state, &catalog, 1, 19, mode).is_err());
        for (index, player) in players.iter_mut().enumerate() {
            arena::queue_player(player, &mut state, &catalog, index as u64 + 1, 90, mode).unwrap();
            if index + 1 < count {
                assert!(arena::form_match(&mut state, &catalog, mode).is_none());
            }
        }
        assert!(arena::queue_player(&mut players[0], &mut state, &catalog, 1, 90, mode).is_err());
        let id = arena::form_match(&mut state, &catalog, mode).unwrap();
        let matched = &state.battleground_matches[&id];
        assert_eq!(matched.instance.team_a_players.len(), count / 2);
        assert_eq!(matched.instance.team_b_players.len(), count / 2);
        assert!(matched.accepted.is_empty());
        assert_eq!(
            matched.instance.phase,
            shared::battleground::BgPhase::Waiting
        );
        assert_eq!(matched.arena.as_ref().unwrap().source.map_id, 572);
        assert!(arena::form_match(&mut state, &catalog, mode).is_none());
    }
}

#[test]
fn arena_preparation_waits_for_worldport_then_runs_sixty_seconds_and_eliminates() {
    use shared::components::{Health, Player, Position};
    use shared::protocol::ArenaQueueMode;
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE arena_map(battlemaster_id INTEGER,map_id INTEGER,name TEXT,start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,weight INTEGER,wdt_fdid INTEGER); INSERT INTO arena_map VALUES(8,572,'Ruins of Lordaeron',1258,1259,10,90,1,789984);").unwrap();
    let catalog = arena::Catalog::load(&db).unwrap();
    let mut state = PvpServerState::default();
    for id in 1..=4 {
        arena::queue_player(
            &mut PlayerPvpState::default(),
            &mut state,
            &catalog,
            id,
            90,
            ArenaQueueMode::Skirmish2v2,
        )
        .unwrap();
    }
    let id = arena::form_match(&mut state, &catalog, ArenaQueueMode::Skirmish2v2).unwrap();
    let map = crate::maps::InMap {
        map_id: 572,
        instance_id: 1,
    };
    let matched = state.battleground_matches.get_mut(&id).unwrap();
    matched.map = Some(map);
    matched.accepted.extend(1..=4);
    let mut world = World::new();
    world.insert_resource(state);
    world.insert_resource(Time::<()>::default());
    world.insert_resource(crate::game_objects::GameObjectSpawns::default());
    db.execute_batch("CREATE TABLE arena_gate(map_id INTEGER,entry INTEGER,action TEXT,despawn_secs INTEGER); INSERT INTO arena_gate VALUES(572,185918,'open',5);").unwrap();
    world.insert_resource(arena_objects::GateCatalog::load(&db).unwrap());
    world.insert_resource(arena_objects::SightCatalog::default());
    world.insert_resource(arena_objects::SightTimers::default());
    let gate = world
        .spawn((
            map,
            shared::protocol::GameObjectInfo {
                entry: 185918,
                go_type: 0,
                display_id: 0,
                name: "source door".into(),
                scale: 1.0,
            },
        ))
        .id();
    let mut actors = Vec::new();
    for player in 1..=4 {
        let entity = world
            .spawn((
                crate::persistence::ClientId(player),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Player {
                    name: format!("fb_arena_{player}"),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                Level(90),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ))
            .id();
        if player != 4 {
            world.entity_mut(entity).insert(map);
        }
        actors.push(entity);
    }
    arena::update_matches(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .arena
            .as_ref()
            .unwrap()
            .snapshot
            .preparation_remaining_secs,
        None
    );
    assert_eq!(
        world
            .get::<shared::protocol::GameObjectAnimation>(gate)
            .unwrap()
            .animation_id,
        147
    );
    world.entity_mut(actors[3]).insert(map);
    arena::update_matches(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .arena
            .as_ref()
            .unwrap()
            .snapshot
            .preparation_remaining_secs,
        Some(60)
    );
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(59));
    arena::update_matches(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .instance
            .phase,
        shared::battleground::BgPhase::Waiting
    );
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    arena::update_matches(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .instance
            .phase,
        shared::battleground::BgPhase::InProgress
    );
    assert_eq!(
        world
            .get::<shared::protocol::GameObjectAnimation>(gate)
            .unwrap()
            .animation_id,
        148
    );
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(5));
    arena::update_matches(&mut world);
    assert!(
        world.get_entity(gate).is_err(),
        "source door despawns five seconds after opening"
    );
    world.get_mut::<Health>(actors[1]).unwrap().current = 0.0;
    arena::update_matches(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .instance
            .phase,
        shared::battleground::BgPhase::InProgress
    );
    world.get_mut::<Health>(actors[3]).unwrap().current = 0.0;
    arena::update_matches(&mut world);
    let matched = &world.resource::<PvpServerState>().battleground_matches[&id];
    assert_eq!(
        matched.instance.outcome,
        Some(shared::battleground::BgOutcome::Winner(
            shared::battleground::BgTeam::A
        ))
    );
    assert_eq!(
        matched
            .scoreboard
            .rows
            .iter()
            .filter(|row| row.team == shared::battleground::BgTeam::B)
            .map(|row| row.deaths)
            .sum::<u32>(),
        2
    );
}

fn arena_player_state(
    team: shared::battleground::BgTeam,
    phase: shared::battleground::BgPhase,
) -> PlayerPvpState {
    PlayerPvpState {
        battleground: Some(shared::protocol::BattlegroundSnapshot {
            match_id: 1,
            battleground_id: 8,
            name: "Ruins of Lordaeron".into(),
            map_id: 572,
            team,
            phase,
            accepted: true,
            invitation_time_left_secs: None,
            score_a: 0,
            score_b: 0,
            flags: Default::default(),
            world_states: Vec::new(),
            outcome: None,
            players: Vec::new(),
            honor_reward: None,
            arena: Some(shared::protocol::ArenaMatchSnapshot {
                mode: shared::protocol::ArenaQueueMode::Skirmish2v2,
                preparation_remaining_secs: None,
                shadow_sight_available: false,
                units: Vec::new(),
                rating_changes: Vec::new(),
                shuffle: None,
            }),
        }),
        ..Default::default()
    }
}

#[test]
fn arena_opponents_of_same_race_attack_only_after_preparation_and_never_attack_teammates() {
    use crate::spell_cast::targets::{Relation, is_friendly, is_hostile, relation_matches};
    use shared::battleground::{BgPhase, BgTeam};
    use shared::components::{Health, Player, UnitFactionTemplate};
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1).unwrap();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .main_hand = crate::item_stats::WeaponDamage::UNARMED;
    let map = crate::maps::InMap {
        map_id: 572,
        instance_id: 1,
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((map, arena_player_state(BgTeam::A, BgPhase::Waiting)));
    h.app.world_mut().entity_mut(h.npc).insert((
        map,
        UnitFactionTemplate(1),
        Player {
            name: "fb_arena_enemy".into(),
            race: 1,
            class: 1,
            appearance: Default::default(),
        },
        arena_player_state(BgTeam::B, BgPhase::Waiting),
    ));
    let health = h.app.world().get::<Health>(h.npc).unwrap().current;
    h.attack(h.npc);
    h.advance(5);
    assert_eq!(h.app.world().get::<Health>(h.npc).unwrap().current, health);
    for entity in [h.caster, h.npc] {
        h.app
            .world_mut()
            .get_mut::<PlayerPvpState>(entity)
            .unwrap()
            .battleground
            .as_mut()
            .unwrap()
            .phase = BgPhase::InProgress;
    }
    assert!(
        is_hostile(h.app.world(), h.caster, h.npc),
        "same-race arena opponents are hostile"
    );
    assert!(!is_friendly(h.app.world(), h.caster, h.npc));
    assert!(relation_matches(
        h.app.world(),
        h.caster,
        h.npc,
        Relation::Enemy
    ));
    h.attack(h.npc);
    for _ in 0..100 {
        h.advance(5);
        if h.app.world().get::<Health>(h.npc).unwrap().current < health {
            break;
        }
    }
    assert!(
        h.app.world().get::<Health>(h.npc).unwrap().current < health,
        "actual HostClient attack caused damage"
    );
    h.stop_attack();
    h.advance(3);
    h.app
        .world_mut()
        .get_mut::<PlayerPvpState>(h.npc)
        .unwrap()
        .battleground
        .as_mut()
        .unwrap()
        .team = BgTeam::A;
    assert!(is_friendly(h.app.world(), h.caster, h.npc));
    assert!(!relation_matches(
        h.app.world(),
        h.caster,
        h.npc,
        Relation::Enemy
    ));
    let teammate_health = h.app.world().get::<Health>(h.npc).unwrap().current;
    h.attack(h.npc);
    h.advance(20);
    assert_eq!(
        h.app.world().get::<Health>(h.npc).unwrap().current,
        teammate_health
    );
}

#[test]
fn arena_rating_reference_uses_pinned_tc_formula_not_the_retired_elo_simulation() {
    assert_eq!(arena_rating::rating_delta(0, 1500, true), 48);
    assert_eq!(arena_rating::rating_delta(0, 1500, false), 0);
    assert_eq!(arena_rating::rating_delta(1200, 1200, true), 16);
    assert_eq!(arena_rating::rating_delta(1300, 1300, true), 12);
    assert_eq!(arena_rating::rating_delta(1500, 1500, false), -12);
    assert_eq!(arena_rating::matchmaker_delta(1500, 1500, true), 12);
    assert_eq!(arena_rating::matchmaker_delta(1500, 1500, false), -12);
}

#[test]
fn arena_rated_pairing_uses_source_mmr_window_and_discard_age() {
    let team = |leader, mmr, queued_at| arena_rating::QueuedTeam {
        leader,
        members: vec![leader, leader + 1],
        levels: vec![90, 90],
        mmr,
        queued_at,
    };
    let queue = vec![team(1, 1500, 0.0), team(3, 1700, 600.001)];
    assert_eq!(
        arena_rating::find_pair(&queue, 600.001, Some(1700)),
        Some((0, 1))
    );
    assert_eq!(arena_rating::find_pair(&queue, 600.0, None), None);
    assert_eq!(
        arena_rating::find_pair(&[team(1, 1500, 0.0), team(3, 1650, 0.0)], 1.0, None),
        Some((0, 1))
    );
    assert_eq!(
        arena_rating::find_pair(&[team(1, 1500, 0.0), team(3, 1651, 0.0)], 1.0, None),
        None
    );
    assert_eq!(
        arena_rating::find_pair(&[team(1, 1500, 0.0)], 1.0, None),
        None
    );
}

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
fn battleground_admission_retains_source_bracket_without_mixing_queued_players() {
    let mut state = PvpServerState::default();
    queue_for_battleground(&mut PlayerPvpState::default(), &mut state, 1, 29, 1018, 0).unwrap();
    for character in 2..=17 {
        let level = if character % 2 == 0 { 30 } else { 39 };
        queue_for_battleground(
            &mut PlayerPvpState::default(),
            &mut state,
            character,
            level,
            1018,
            0,
        )
        .unwrap();
    }
    let id =
        form_battleground_invitation(&mut state, 1018, &random_bg::Catalog::default()).unwrap();
    let invitation = &state.battleground_matches[&id];
    assert_eq!(
        invitation.instance.bracket,
        shared::battleground::LevelBracket::new(30, 39)
    );
    let mut roster: Vec<_> = invitation
        .instance
        .team_a_players
        .iter()
        .chain(&invitation.instance.team_b_players)
        .copied()
        .collect();
    roster.sort_unstable();
    assert_eq!(roster, (2..=17).collect::<Vec<_>>());
    assert!(state.battleground_queue.is_queued(1));
    assert!(
        form_battleground_invitation(&mut state, 1018, &random_bg::Catalog::default()).is_none()
    );
}

#[test]
fn random_queue_level25_parties_form_configured_level_matching_map() {
    let mut state = PvpServerState::default();
    for leader in [1, 6] {
        state
            .battleground_queue
            .join_group(leader, (leader..leader + 5).collect(), vec![25; 5], 32, 0)
            .unwrap();
    }
    let id = form_battleground_invitation(&mut state, 32, &random_bg::tests::source_catalog(&[]))
        .expect("ordinary Random32 parties receive a configured-map invitation");
    let invitation = &state.battleground_matches[&id];
    let template = template_by_id(invitation.instance.bg_id).unwrap();
    assert!([566, 726, 727, 761, 998, 1803, 2106, 2107, 2656].contains(&template.map_id));
    assert_eq!(invitation.original_queues.len(), 10);
    for character in 1..=10 {
        assert_eq!(invitation.original_queues.get(&character), Some(&32));
    }
    assert_ne!(
        template.map_id, 2656,
        "level25 cannot select Deephaul's80-only bracket"
    );
    assert_eq!(
        invitation.instance.bracket,
        shared::battleground::LevelBracket::new(20, 29)
    );
    assert_eq!(
        (
            invitation.instance.team_a_players.len(),
            invitation.instance.team_b_players.len()
        ),
        (5, 5)
    );
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
    let match_id =
        form_battleground_invitation(&mut state, template.id, &random_bg::Catalog::default())
            .unwrap();
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
        bracket: template.bracket,
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
            original_queues: BTreeMap::new(),
            flags: Default::default(),
            nodes: None,
            eye: None,
            kotmogu: None,
            silvershard: None,
            seething: None,
            deephaul: None,
            alterac: None,
            scoreboard: Scoreboard::default(),
            flag_positions: [None, None],
            arena: None,
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
fn rated_modes_cannot_bypass_full_group_queue_through_solo_skirmish_message() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE arena_map(battlemaster_id INTEGER,map_id INTEGER,name TEXT,start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,weight INTEGER,wdt_fdid INTEGER); INSERT INTO arena_map VALUES(8,572,'Ruins of Lordaeron',1258,1259,10,90,1,789984);").unwrap();
    let catalog = arena::Catalog::load(&db).unwrap();
    let mut player = PlayerPvpState::default();
    let mut state = PvpServerState::default();
    for mode in [
        shared::protocol::ArenaQueueMode::Rated2v2,
        shared::protocol::ArenaQueueMode::Rated3v3,
    ] {
        assert!(
            arena::queue_player(&mut player, &mut state, &catalog, 44, 90, mode)
                .unwrap_err()
                .contains("full group")
        );
    }
    assert!(player.queue.is_none());
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
    app.insert_resource(GameDatabase::new(crate::persistence::temp_database()));
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
            state.queue.is_none(),
            "ungrouped rated request cannot manufacture a team"
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

/// Ten authenticated in-world players accepted into one native flag match (copy17) whose
/// fixture homes use the shared flag entries; returns the world, (character, player) pairs
/// and the redb file backing persistence.
fn native_flag_match(
    template: &shared::battleground::BgTemplate,
) -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    native_flag_match_in_world(template, World::new(), |index| index)
}

fn native_flag_match_in_world(
    template: &shared::battleground::BgTemplate,
    mut world: World,
    account_for_index: impl Fn(u64) -> u64,
) -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
    use crate::auth::{Authenticated, InWorld};
    use crate::game_objects::{GameObjectBounds, GameObjectSpawn, GameObjectSpawns};
    use crate::maps::InMap;
    use crate::networking::PlayerLink;
    use crate::persistence::{self, ClientId};
    use shared::battleground::{BgInstance, BgMatch};
    use shared::components::{Health, Player, Position};
    use shared::protocol::{GameObjectInfo, WARSONG_ALLIANCE_FLAG_ENTRY, WARSONG_HORDE_FLAG_ENTRY};
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    world.insert_resource(load_achievement_catalog(&conn).unwrap());
    world.init_resource::<crate::maps::ServerClock>();
    world.init_resource::<Time<Fixed>>();
    let map = InMap {
        map_id: template.map_id,
        instance_id: 17,
    };
    let mut players = Vec::new();
    for index in 1..=u64::from(template.min_players_per_team) * 2 {
        let name = format!("Bgproof{}", char::from(b'a' + index as u8 - 1));
        let account = account_for_index(index);
        let (character, _) = persistence::create_character(&db, account, &name, 1, 1).unwrap();
        let player = world
            .spawn((
                Player {
                    name,
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                PlayerAchievementState::default(),
                map,
                ClientId(character),
                Level(30),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        world.spawn((
            Authenticated {
                account_id: account,
            },
            PlayerLink(player),
            InWorld {
                character_id: character,
            },
        ));
        players.push((character, player));
    }
    world.insert_resource(GameDatabase::new(db));
    // Fixture flag coordinates are intentionally not an authored spawn-location claim.
    // The final pickup/capture follows the same native flag objective as live matches.
    let homes = [WARSONG_ALLIANCE_FLAG_ENTRY, WARSONG_HORDE_FLAG_ENTRY]
        .into_iter()
        .enumerate()
        .map(|(i, entry)| GameObjectSpawn {
            guid: i as u32 + 1,
            map_id: map.map_id,
            info: GameObjectInfo {
                entry,
                go_type: 24,
                display_id: 0,
                name: "Flag fixture".into(),
                scale: 1.0,
            },
            position: Vec3::new(i as f32 * 100.0, 0.0, 0.0),
            orientation: 0.0,
            bounds: GameObjectBounds {
                min: Vec3::ZERO,
                max: Vec3::ONE,
                scale: 1.0,
                orientation: 0.0,
            },
            spawn_difficulties: vec![0],
            chair: None,
            quest_object: None,
            initial_state: 1,
            script_name: String::new(),
        })
        .collect();
    world.insert_resource(GameObjectSpawns(homes));
    let matched = BgMatch {
        bg_id: template.id,
        bracket: template.bracket,
        team_a: players.iter().step_by(2).map(|p| p.0).collect(),
        team_b: players.iter().skip(1).step_by(2).map(|p| p.0).collect(),
    };
    let invitation = InvitedBattleground {
        instance: BgInstance::from_match(&matched, &template),
        accepted: players.iter().map(|p| p.0).collect(),
        map: Some(map),
        origins: BTreeMap::new(),
        original_queues: BTreeMap::new(),
        flags: shared::battleground::flag_rules(template.map_id)
            .map(shared::battleground::FlagBattle::new),
        nodes: None,
        eye: None,
        kotmogu: None,
        silvershard: None,
        seething: None,
        deephaul: None,
        alterac: None,
        arena: None,
        scoreboard: Scoreboard::default(),
        flag_positions: [None, None],
    };
    let mut state = PvpServerState::default();
    state.battleground_matches.insert(17, invitation);
    world.insert_resource(state);
    (world, players, file)
}

#[test]
fn achievements_criteria_native_flag_capture_credits_present_participants_and_winners_once() {
    use crate::achievement::PlayerAchievementState;
    use crate::maps::InMap;
    use crate::persistence;
    use shared::battleground::{BgPhase, BgTeam};
    let template = shared::battleground::warsong_gulch();
    let (mut world, players, file) = native_flag_match(&template);
    // Native readiness starts the match and builds its authored scoreboard columns.
    pvp_objective::update_battleground_objectives(&mut world);
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let invitation = state.battleground_matches.get_mut(&17).unwrap();
        assert_eq!(invitation.instance.phase, BgPhase::InProgress);
        invitation.instance.add_score(BgTeam::A, 2, &template);
        assert!(
            invitation
                .flags
                .as_mut()
                .unwrap()
                .flags
                .pickup(BgTeam::A, players[0].0)
        );
    }
    // A winning roster member who already left the copy must not receive completion.
    world.entity_mut(players[8].1).insert(InMap::continent(0));
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&17]
            .instance
            .phase,
        BgPhase::Ended
    );
    assert_eq!(
        world
            .get::<PlayerAchievementState>(players[0].1)
            .unwrap()
            .criteria_progress
            .get(&45085),
        Some(&1)
    );
    assert_eq!(
        world
            .get::<PlayerAchievementState>(players[0].1)
            .unwrap()
            .criteria_progress
            .get(&28187),
        Some(&1)
    );
    for i in [1, 8] {
        assert!(
            !world
                .get::<PlayerAchievementState>(players[i].1)
                .unwrap()
                .criteria_progress
                .contains_key(&45085)
        );
    }
    // TrackedWorldStateUIModified45084 is PvpStat928 Flag Captures, credited to the connected
    // capturer only (TC BattlegroundScore.cpp:78-80). Its Capture the Flag199 keeps
    // Instance_ID489 (legacy Warsong); native WSG is map2106, so the native completion-map
    // gate leaves 199 unearned.
    let flag_captures = |world: &World, i: usize| {
        let state = world.get::<PlayerAchievementState>(players[i].1).unwrap();
        let account = state.account.as_deref().expect("account aggregate loaded");
        (
            account.criteria_progress.get(&45084).copied(),
            account.earned_ids.contains(&199),
        )
    };
    assert_eq!(flag_captures(&world, 0), (Some(1), false));
    assert_eq!(flag_captures(&world, 2), (None, false));
    // ParticipateInBattleground45067 ("Warsong Gulch battles"): present winner and loser.
    let battles = |world: &World, i: usize| {
        world
            .get::<PlayerAchievementState>(players[i].1)
            .unwrap()
            .criteria_progress
            .get(&45067)
            .copied()
    };
    assert_eq!(battles(&world, 0), Some(1));
    assert_eq!(battles(&world, 1), Some(1));
    assert_eq!(battles(&world, 8), None, "left the copy before the end");
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(battles(&world, 1), Some(1));
    assert_eq!(flag_captures(&world, 0), (Some(1), false), "idle tick");
    assert_eq!(
        world
            .get::<PlayerAchievementState>(players[0].1)
            .unwrap()
            .criteria_progress
            .get(&45085),
        Some(&1)
    );
    let winner = players[0].0;
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    let reopened = persistence::load_achievements(&db, winner);
    assert_eq!(reopened.criteria_progress.get(&45085), Some(&1));
    assert_eq!(reopened.criteria_progress.get(&45084), Some(&1));
}

/// Two-Timer5210 (map726): TrackedWorldStateUIModified14869 counts PvpStat290 Twin Peaks
/// flag captures; two native captures by one carrier earn it in its own map.
#[test]
fn achievements_criteria_native_twin_peaks_captures_count_pvp_stat_per_credit() {
    use crate::achievement::PlayerAchievementState;
    use crate::persistence;
    use shared::battleground::BgTeam;
    let template = shared::battleground::twin_peaks();
    let (mut world, players, file) = native_flag_match(&template);
    pvp_objective::update_battleground_objectives(&mut world);
    let progress = |world: &World, i: usize| {
        let state = world.get::<PlayerAchievementState>(players[i].1).unwrap();
        state.account.as_deref().map_or((None, false), |account| {
            (
                account.criteria_progress.get(&14869).copied(),
                account.earned_ids.contains(&5210),
            )
        })
    };
    let carry = |world: &mut World| {
        let mut state = world.resource_mut::<PvpServerState>();
        let battle = state.battleground_matches.get_mut(&17).unwrap();
        assert!(
            battle
                .flags
                .as_mut()
                .unwrap()
                .flags
                .pickup(BgTeam::A, players[0].0)
        );
    };
    assert_eq!(progress(&world, 0), (None, false));
    carry(&mut world);
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(progress(&world, 0), (Some(1), false));
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(progress(&world, 0), (Some(1), false), "idle tick");
    carry(&mut world);
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(progress(&world, 0), (Some(2), true));
    for i in [1, 2] {
        assert_eq!(progress(&world, i), (None, false));
    }
    let scores = &world.resource::<PvpServerState>().battleground_matches[&17]
        .scoreboard
        .rows;
    let carrier = scores.iter().find(|row| row.character_id == players[0].0);
    assert_eq!(carrier.unwrap().objective_scores, vec![(290, 2), (291, 0)]);
    let character = players[0].0;
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&14869),
        Some(&2)
    );
}
