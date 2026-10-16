use super::*;
use crate::achievement::PlayerAchievementState;
use crate::maps::InMap;
use shared::battleground::{BgPhase, BgTeam};

fn participation(world: &World, player: Entity, criterion: u32) -> Option<u64> {
    world
        .get::<PlayerAchievementState>(player)
        .unwrap()
        .criteria_progress
        .get(&criterion)
        .copied()
}

#[test]
fn achievements_criteria_content268_native_settlement_uses_authored_max_level_range() {
    let template = shared::battleground::warsong_gulch();
    assert_eq!(template.map_id, 2106);
    let (mut world, players, file) = native_flag_match(&template);
    // Controlled accepted rosters are the fixture baseline, not a queue/admission certificate.
    for index in [0, 1, 8] {
        world.entity_mut(players[index].1).insert(Level(90));
    }
    world.entity_mut(players[2].1).insert(Level(89));
    world.entity_mut(players[3].1).remove::<Level>();
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&17]
            .instance
            .phase,
        BgPhase::InProgress
    );
    assert_eq!(
        participation(&world, players[0].1, 45044),
        None,
        "not settled"
    );
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let invitation = state.battleground_matches.get_mut(&17).unwrap();
        // Same terminal-boundary baseline as the existing native flag-settlement proof.
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
    world.entity_mut(players[8].1).insert(InMap::continent(0));
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&17]
            .instance
            .phase,
        BgPhase::Ended
    );
    // Real winning capture and both present teams' unconditional native participation occurred.
    assert_eq!(participation(&world, players[0].1, 45067), Some(1));
    assert_eq!(participation(&world, players[1].1, 45067), Some(1));
    for index in [2, 3, 8] {
        assert_eq!(
            participation(&world, players[index].1, 45044),
            None,
            "not in current authored level range / unknown / left copy: {index}"
        );
    }
    for index in [0, 1] {
        assert_eq!(
            participation(&world, players[index].1, 45044),
            Some(1),
            "settled native max-level participation, winner and loser"
        );
    }
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(
        participation(&world, players[0].1, 45044),
        Some(1),
        "no idle replay"
    );
    drop(world);
    let reopened = redb::Database::open(file.path()).unwrap();
    for index in [0, 1] {
        let stored = crate::persistence::load_achievements(&reopened, players[index].0);
        assert_eq!(stored.criteria_progress.get(&45044), Some(&1));
    }
}
