use super::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;

#[test]
fn solo_shuffle_admits_only_source_roles_solo_bracket_and_balanced_rosters() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut world = World::new();
    world.insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let players: Vec<_> = (1..=6)
        .map(|id| {
            world
                .spawn((
                    ClientId(id),
                    ActiveSpec(71),
                    Player {
                        name: format!("Player{id}"),
                        class: 1,
                        race: 1,
                        appearance: Default::default(),
                    },
                ))
                .id()
        })
        .collect();
    let catalog = arena::Catalog::load(&db).unwrap();
    let mut state = PvpServerState::default();
    for id in 1..=6 {
        arena::queue_player(
            &mut PlayerPvpState::default(),
            &mut state,
            &catalog,
            id,
            90,
            ArenaQueueMode::SoloShuffle,
        )
        .unwrap();
    }
    assert!(
        form_match(&mut world, &mut state, &catalog).is_none(),
        "six damage players are not a Shuffle roster"
    );
    for &player in &players[..2] {
        world.get_mut::<Player>(player).unwrap().class = 5;
        world.entity_mut(player).insert(ActiveSpec(257));
        assert_eq!(role(&world, player), Ok(HEALER));
    }
    let match_id = form_match(&mut world, &mut state, &catalog).unwrap();
    let matched = &state.battleground_matches[&match_id];
    assert_eq!(matched.instance.team_a_players, vec![1, 3, 4]);
    assert_eq!(matched.instance.team_b_players, vec![2, 5, 6]);
    assert!(state.arena_queues[&ArenaQueueMode::SoloShuffle].is_empty());
    assert!(matched.arena.as_ref().unwrap().rated.is_none());
    let player = players[2];
    world.entity_mut(player).insert(ActiveSpec(73));
    assert!(
        validate_entrant(&world, player, 3)
            .unwrap_err()
            .contains("healer or damage")
    );
    world.entity_mut(player).remove::<ActiveSpec>();
    assert!(
        validate_entrant(&world, player, 3)
            .unwrap_err()
            .contains("active specialization")
    );
    world.entity_mut(player).insert(ActiveSpec(71));
    for level in [59, 91] {
        assert!(
            arena::queue_player(
                &mut PlayerPvpState::default(),
                &mut state,
                &catalog,
                7,
                level,
                ArenaQueueMode::SoloShuffle
            )
            .unwrap_err()
            .contains("60–90")
        );
    }
    let online: OnlineView = players
        .iter()
        .enumerate()
        .map(|(index, player)| {
            (
                index as u64 + 1,
                OnlineMember {
                    entity: player.to_bits(),
                    name: format!("Player{}", index + 1),
                    class: 1,
                    level: 90,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.sync_presence(&online);
    groups.invite(3, "Player4", &online);
    groups.respond_invite(4, true);
    world.insert_resource(groups);
    assert!(
        validate_entrant(&world, player, 3)
            .unwrap_err()
            .contains("solo entrants")
    );
}
