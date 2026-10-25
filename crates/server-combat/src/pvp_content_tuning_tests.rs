use super::*;
use crate::achievement::PlayerAchievementState;
use crate::maps::InMap;
use shared::battleground::{BgInstance, BgMatch, BgPhase, BgTeam};

fn participation(world: &World, player: Entity, criterion: u32) -> Option<u64> {
    world
        .get::<PlayerAchievementState>(player)
        .unwrap()
        .criteria_progress
        .get(&criterion)
        .copied()
}

#[test]
fn achievements_criteria_content268_full_hundred_settlements_grant_both_factions() {
    let template = shared::battleground::warsong_gulch();
    let (mut world, players, file) = native_flag_match(&template);
    for index in [0, 1] {
        world.entity_mut(players[index].1).insert(Level(90));
    }
    world
        .get_mut::<shared::components::Player>(players[1].1)
        .unwrap()
        .race = 2;
    let matched = BgMatch {
        bg_id: template.id,
        bracket: template.bracket,
        team_a: players.iter().step_by(2).map(|p| p.0).collect(),
        team_b: players.iter().skip(1).step_by(2).map(|p| p.0).collect(),
    };
    for count in 1..=100 {
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let invitation = state.battleground_matches.get_mut(&17).unwrap();
            invitation.instance = BgInstance::from_match(&matched, &template);
            invitation.preparation = Default::default();
            invitation.flags = shared::battleground::flag_rules(template.map_id)
                .map(shared::battleground::FlagBattle::new);
        }
        advance_native_preparation(&mut world);
        assert_eq!(
            world.resource::<PvpServerState>().battleground_matches[&17]
                .instance
                .phase,
            BgPhase::InProgress
        );
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let invitation = state.battleground_matches.get_mut(&17).unwrap();
            // Controlled terminal baseline, followed by a genuine native flag capture each time.
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
        pvp_objective::update_battleground_objectives(&mut world);
        assert_eq!(
            world.resource::<PvpServerState>().battleground_matches[&17]
                .instance
                .phase,
            BgPhase::Ended
        );
        for (index, earned, other) in [(0, 908, 909), (1, 909, 908)] {
            assert_eq!(participation(&world, players[index].1, 45044), Some(count));
            let state = world
                .get::<PlayerAchievementState>(players[index].1)
                .unwrap();
            let account = state.account.as_ref().unwrap();
            assert_eq!(account.earned_ids.contains(&earned), count == 100);
            assert!(
                !account.earned_ids.contains(&other),
                "opposite faction cannot earn"
            );
        }
        pvp_objective::update_battleground_objectives(&mut world);
        assert_eq!(
            participation(&world, players[0].1, 45044),
            Some(count),
            "no idle replay"
        );
    }
    let dates: Vec<_> = [0, 1]
        .into_iter()
        .map(|index| {
            world
                .get::<PlayerAchievementState>(players[index].1)
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .earned_at
                .clone()
        })
        .collect();
    drop(world.remove_resource::<GameDatabase>().unwrap());
    let reopened = redb::Database::open(file.path()).unwrap();
    for index in [0, 1] {
        let stored = crate::persistence::load_achievements(&reopened, players[index].0);
        assert_eq!(stored.criteria_progress.get(&45044), Some(&100));
        world.entity_mut(players[index].1).insert(stored);
    }
    world.insert_resource(GameDatabase::new(reopened));
    for (index, earned) in [(0, 908), (1, 909)] {
        crate::achievement::record_player_event(
            &mut world,
            players[index].1,
            crate::achievement::CriteriaEvent::Login,
        );
        let state = world
            .get::<PlayerAchievementState>(players[index].1)
            .unwrap();
        let account = state.account.as_ref().unwrap();
        assert!(account.earned_ids.contains(&earned));
        assert_eq!(account.earned_at[&earned], dates[index][&earned]);
        assert!(account.earned_at[&earned] > 0);
    }
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
    // Missing Level excludes the unit from the objective roster; remove only after native readiness.
    world.entity_mut(players[3].1).remove::<Level>();
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
    assert_eq!(
        participation(&world, players[2].1, 45067),
        Some(1),
        "level89 participated; only268 excludes its qualified credit"
    );
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
