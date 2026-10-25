use super::*;
use crate::achievement::{AchievementCatalog, PlayerAchievementState, load_achievement_catalog};
use crate::group::{OnlineMember, OnlineView};
use crate::maps::InMap;
use crate::persistence::{ClientId, GameDatabase};
use shared::battleground::{BgOutcome, BgPhase, BgTeam};
use shared::components::{Health, Player, Position};

fn create_rating_world(file: &tempfile::NamedTempFile) -> World {
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut world = World::new();
    world.insert_resource(GameDatabase::new(db));
    world.insert_resource(load_achievement_catalog(&conn).unwrap());
    world.insert_resource(arena::Catalog::load(&conn).unwrap());
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    world.init_resource::<PvpServerState>();
    world.init_resource::<Time>();
    world.init_resource::<Time<bevy::time::Virtual>>();
    world.init_resource::<Time<Fixed>>();
    world.init_resource::<GroupRegistry>();
    world.init_resource::<crate::maps::ServerClock>();
    world.init_resource::<Messages<crate::death::PlayerDeathTransition>>();
    world.init_resource::<super::super::arena_objects::GateCatalog>();
    world.init_resource::<super::super::arena_objects::SightCatalog>();
    world.init_resource::<super::super::arena_objects::SightTimers>();
    world.init_resource::<super::super::arena_dalaran::Catalog>();
    world.init_resource::<super::super::arena_dalaran::Timers>();
    world.init_resource::<super::super::arena_cage::Catalog>();
    world.init_resource::<super::super::arena_cage::Timers>();
    world.init_resource::<crate::game_objects::GameObjectSpawns>();
    world
}

fn controlled_starting_progress(requested: PvpBracketSnapshot) -> PlayerPvpState {
    let mut player = PlayerPvpState::default();
    let entry = player
        .progress
        .brackets
        .iter_mut()
        .find(|entry| entry.bracket == requested)
        .unwrap();
    // Controlled legitimate pre-match input, never an achievement credit.
    entry.rating = Some(1539);
    entry.matchmaker_rating = Some(1500);
    player
}

fn spawn_rating_players(
    world: &mut World,
    size: usize,
    requested: PvpBracketSnapshot,
) -> Vec<(u64, Entity)> {
    (0..size * 2)
        .map(|index| create_rating_participant(world, index, requested.clone()))
        .collect()
}

fn create_rating_participant(
    world: &mut World,
    index: usize,
    requested: PvpBracketSnapshot,
) -> (u64, Entity) {
    let name = format!("Rating{}", char::from(b'a' + index as u8));
    let pvp = controlled_starting_progress(requested);
    let db = &world.resource::<GameDatabase>().db;
    let (character, _) =
        crate::persistence::create_character(db, index as u64 + 1, &name, 1, 1).unwrap();
    crate::persistence::save_pvp(db, character, &pvp.progress);
    let player = spawn_rating_actor(world, character, name, pvp);
    world.spawn((
        Authenticated {
            account_id: index as u64 + 1,
        },
        InWorld {
            character_id: character,
        },
        PlayerLink(player),
    ));
    (character, player)
}

fn spawn_rating_actor(
    world: &mut World,
    character: u64,
    name: String,
    pvp: PlayerPvpState,
) -> Entity {
    let identity = Player {
        name,
        race: 1,
        class: 1,
        appearance: Default::default(),
    };
    let health = Health {
        current: 1000.0,
        max: 1000.0,
    };
    let position = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    world
        .spawn((
            identity,
            Level(90),
            ClientId(character),
            position,
            InMap::continent(0),
            health,
            pvp,
            PlayerAchievementState::default(),
            crate::death::PlayerDeathState::default(),
        ))
        .id()
}

fn form_real_parties(world: &mut World, players: &[(u64, Entity)], size: usize) {
    let online: OnlineView = players
        .iter()
        .map(|(id, player)| {
            (
                *id,
                OnlineMember {
                    entity: player.to_bits(),
                    name: world.get::<Player>(*player).unwrap().name.clone(),
                    class: 1,
                    level: 90,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = world.resource_mut::<GroupRegistry>();
    for party in players.chunks(size) {
        for (member, _) in &party[1..] {
            groups.invite(party[0].0, &online[member].name, &online);
            groups.respond_invite(*member, true);
        }
        assert_eq!(groups.group_members(party[0].0).len(), size);
    }
}

fn admit_rating_match(
    world: &mut World,
    players: &[(u64, Entity)],
    size: usize,
    requested: &PvpBracketSnapshot,
) -> u64 {
    queue_party(world, players[0].0, requested).unwrap();
    queue_party(world, players[size].0, requested).unwrap();
    let mode = mode(requested).unwrap();
    form_matches(world, mode, None);
    let id = world.resource::<PvpServerState>().next_battleground_match;
    accept_rating_copy(world, id, players);
    run_rating_preparation(world, id);
    id
}

fn accept_rating_copy(world: &mut World, id: u64, players: &[(u64, Entity)]) {
    let map_id = world.resource::<PvpServerState>().battleground_matches[&id]
        .arena
        .as_ref()
        .unwrap()
        .source
        .map_id;
    let map = InMap {
        map_id,
        instance_id: u32::try_from(id).unwrap(),
    };
    for (character, player) in players {
        super::super::respond_to_battleground_invitation(
            &mut world.resource_mut::<PvpServerState>(),
            id,
            *character,
            true,
        )
        .unwrap();
        world.entity_mut(*player).insert(map);
    }
    world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .get_mut(&id)
        .unwrap()
        .map = Some(map);
}

fn run_rating_preparation(world: &mut World, id: u64) {
    arena::update_matches(world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .instance
            .phase,
        BgPhase::Waiting
    );
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(60));
    world
        .resource_mut::<Time<bevy::time::Virtual>>()
        .advance_by(std::time::Duration::from_secs(60));
    arena::update_matches(world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&id]
            .instance
            .phase,
        BgPhase::InProgress
    );
}

fn eliminate_rating_team(world: &mut World, id: u64, loser: BgTeam, players: &[(u64, Entity)]) {
    let instance = &world.resource::<PvpServerState>().battleground_matches[&id].instance;
    let victims = if loser == BgTeam::A {
        instance.team_a_players.clone()
    } else {
        instance.team_b_players.clone()
    };
    let killer = players
        .iter()
        .find(|(id, _)| !victims.contains(id))
        .unwrap()
        .1;
    for victim in victims {
        let player = players.iter().find(|(id, _)| *id == victim).unwrap().1;
        crate::combat::kill_unit(world, killer, player);
        assert_eq!(world.get::<Health>(player).unwrap().current, 0.0);
    }
    arena::update_matches(world);
    assert_settled_rating_match(world, id, loser);
}

fn assert_settled_rating_match(world: &World, id: u64, loser: BgTeam) {
    let matched = &world.resource::<PvpServerState>().battleground_matches[&id];
    assert_eq!(
        matched.instance.outcome,
        Some(BgOutcome::Winner(if loser == BgTeam::A {
            BgTeam::B
        } else {
            BgTeam::A
        }))
    );
    assert!(
        matched
            .arena
            .as_ref()
            .unwrap()
            .rated
            .as_ref()
            .unwrap()
            .applied
    );
}

#[test]
fn achievements_criteria_native_personal_rating_uses_after_settlement_and_preserves_highest() {
    for (requested, size, criterion, other, achievement, statistic) in [
        (PvpBracketSnapshot::Arena2v2, 2, 451, 447, 399, 370),
        (PvpBracketSnapshot::Arena3v3, 3, 447, 451, 402, 595),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut world = create_rating_world(&file);
        let players = spawn_rating_players(&mut world, size, requested.clone());
        form_real_parties(&mut world, &players, size);
        let id = admit_rating_match(&mut world, &players, size, &requested);
        for (_, player) in &players {
            let state = world.get::<PlayerAchievementState>(*player).unwrap();
            assert!(
                !state.criteria_progress.contains_key(&criterion),
                "controlled rating/queue/preparation is not credit"
            );
            assert!(!state.earned_ids.contains(&achievement));
        }
        eliminate_rating_team(&mut world, id, BgTeam::B, &players);
        for (index, (_, player)) in players.iter().enumerate() {
            let after = world
                .get::<PlayerPvpState>(*player)
                .unwrap()
                .progress
                .brackets
                .iter()
                .find(|entry| entry.bracket == requested)
                .unwrap()
                .rating
                .unwrap();
            assert_eq!(after, if index < size { 1551 } else { 1527 });
            let state = world.get::<PlayerAchievementState>(*player).unwrap();
            assert_eq!(
                state.criteria_progress.get(&criterion),
                Some(&u64::from(after)),
                "after, not pre-result1539 or a win count"
            );
            assert!(
                !state.criteria_progress.contains_key(&other),
                "other bracket isolated"
            );
            assert!(
                !state.criteria_progress.contains_key(&452),
                "retired5v5 stays disabled"
            );
            assert_eq!(
                state.earned_ids.contains(&achievement),
                index < size,
                "real1550 crossing"
            );
            let shared::protocol::AchievementCatalogPage::Statistics { achievements, .. } =
                world.resource::<AchievementCatalog>().page(
                    state,
                    shared::protocol::QueryAchievementCatalog::Statistics {
                        category_id: 152,
                        after_id: statistic - 1,
                    },
                    1,
                )
            else {
                panic!("rating statistic reply")
            };
            let row = achievements
                .iter()
                .find(|row| row.achievement_id == statistic)
                .unwrap();
            assert_eq!(
                row.statistic_value,
                Some(u64::from(after)),
                "unclamped numeric rating statistic"
            );
        }
        arena::update_matches(&mut world);
        super::super::arena_rated_results::apply_result(&mut world, id);
        assert_eq!(
            world
                .get::<PlayerAchievementState>(players[0].1)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1551),
            "replay/idle is not a new result"
        );
        // Real second match reverses teams; retained highest must not fall to new current.
        world
            .resource_mut::<PvpServerState>()
            .battleground_matches
            .clear();
        for (_, player) in &players {
            world
                .get_mut::<PlayerPvpState>(*player)
                .unwrap()
                .battleground = None;
        }
        let second = admit_rating_match(&mut world, &players, size, &requested);
        eliminate_rating_team(&mut world, second, BgTeam::A, &players);
        assert!(
            world
                .get::<PlayerPvpState>(players[0].1)
                .unwrap()
                .progress
                .brackets
                .iter()
                .find(|entry| entry.bracket == requested)
                .unwrap()
                .rating
                .unwrap()
                < 1551
        );
        assert_eq!(
            world
                .get::<PlayerAchievementState>(players[0].1)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1551)
        );
        drop(world);
        let reopened = redb::Database::open(file.path()).unwrap();
        let saved = crate::persistence::load_achievements(&reopened, players[0].0);
        assert_eq!(
            saved.criteria_progress.get(&criterion),
            Some(&1551),
            "reopened highest"
        );
        assert!(saved.earned_ids.contains(&achievement));
    }
}
