//! Real native-objective terminal tick, not a direct reward-function call.
use super::*;
use shared::battleground::{BgPhase, BgTeam, LevelBracket, NodeBattle, arathi_basin, node_rules};
use shared::components::Health;

#[test]
fn arathi_completion_mirrors_two_separate_source_bonuses_and_reopens_without_replay() {
    for (level, range, one_bonus) in [(30, (30, 39), 61_u32), (90, (90, 90), 124)] {
        let template = arathi_basin();
        let (mut world, players, file) = honor_match(&template, level);
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let matched = state.battleground_matches.get_mut(&17).unwrap();
            assert_eq!(
                matched.instance.bracket,
                LevelBracket::new(range.0, range.1)
            );
            matched.nodes = Some(NodeBattle::new(node_rules(2107).unwrap()));
        }
        pvp_objective::update_battleground_objectives(&mut world);
        assert_eq!(
            world.resource::<PvpServerState>().battleground_matches[&17]
                .instance
                .phase,
            BgPhase::InProgress
        );
        // Dead accepted participants still receive the team award; departed/pending do not.
        world.get_mut::<Health>(players[0].1).unwrap().current = 0.0;
        world
            .entity_mut(players[14].1)
            .insert(crate::maps::InMap::continent(0));
        world
            .resource_mut::<PvpServerState>()
            .battleground_matches
            .get_mut(&17)
            .unwrap()
            .accepted
            .remove(&players[15].0);
        let inactive = players[12].1;
        crate::auras::apply_aura(
            &mut world,
            &crate::auras::AuraApplication {
                caster: inactive,
                target: inactive,
                spell_id: 43681,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(
            world
                .get::<crate::auras::ActiveAuras>(inactive)
                .unwrap()
                .has_spell(43681)
        );
        // One winner starts near the authored holding cap; the full XP award remains separate.
        world
            .get_mut::<crate::currency::PlayerCurrencies>(players[2].1)
            .unwrap()
            .0
            .add(
                1792,
                14_990,
                shared::currency::currency_by_id(1792).unwrap(),
            )
            .unwrap();
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let matched = state.battleground_matches.get_mut(&17).unwrap();
            let nodes = matched.nodes.as_mut().unwrap();
            for entry in [227420, 227522, 227536, 227538, 227544] {
                nodes.assault(entry, BgTeam::A).unwrap();
            }
        }
        advance_objective(&mut world, 60_000);
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let matched = state.battleground_matches.get_mut(&17).unwrap();
            assert_eq!(matched.instance.score_a.score, 60);
            // Controlled pre-terminal baseline; the final real resource tick wins.
            matched.instance.add_score(BgTeam::A, 1430, &template);
            assert_eq!(matched.instance.phase, BgPhase::InProgress);
        }
        assert_eq!(wallet(&world, players[0].1), 0);
        advance_objective(&mut world, 2000);
        assert_eq!(
            world.resource::<PvpServerState>().battleground_matches[&17]
                .instance
                .phase,
            BgPhase::Ended
        );
        // TC OnEnd calls GetBonusHonorFromKill(1) twice for a winner, not once with2:
        // ceil(39 *1.55) + ceil(39 *1.55) =122, whereas ceil(39 *1.55 *2) =121.
        for (index, (_, player)) in players.iter().enumerate() {
            let expected_xp = match index {
                12 | 14 | 15 => 0,
                winner if winner % 2 == 0 => one_bonus * 2,
                _ => one_bonus,
            };
            let expected_wallet = if index == 2 { 15_000 } else { expected_xp };
            assert_eq!(
                wallet(&world, *player),
                expected_wallet,
                "wallet recipient{index}"
            );
            let progress = &world.get::<PlayerPvpState>(*player).unwrap().progress;
            assert_eq!(
                progress.lifetime_honor_xp,
                u64::from(expected_xp),
                "XP recipient{index}"
            );
            assert!(progress.honor_level.is_none());
            assert_eq!(progress.conquest, 0);
            assert!(
                !world
                    .get::<crate::achievement::PlayerAchievementState>(*player)
                    .unwrap()
                    .criteria_progress
                    .contains_key(&63413),
                "completion is not a type35 kill"
            );
        }
        advance_objective(&mut world, 2000);
        assert_eq!(
            wallet(&world, players[0].1),
            one_bonus * 2,
            "ended-tick replay"
        );
        assert_eq!(
            world
                .get::<PlayerPvpState>(players[0].1)
                .unwrap()
                .progress
                .lifetime_honor_xp,
            u64::from(one_bonus * 2)
        );
        let character = players[0].0;
        drop(world);
        let db = redb::Database::open(file.path()).unwrap();
        let saved = crate::persistence::load_pvp(&db, character);
        assert_eq!(
            (saved.lifetime_honor_xp, saved.honor),
            (u64::from(one_bonus * 2), one_bonus * 2)
        );
        let saved_wallet = crate::persistence::load_currencies(&db, character);
        assert_eq!(saved_wallet.get(1792), one_bonus * 2);
        assert_eq!(saved_wallet.week_earned(1792), 0);
        assert_eq!(saved_wallet.get(1602), 0);
    }
}

#[test]
fn eye_completion_mirrors_source_bonuses_on_the_real_presence_resource_win() {
    let template = shared::battleground::eye_of_the_storm();
    let (mut world, players, file) = honor_match(&template, 30);
    let rules = shared::battleground::eye_rules(566).unwrap();
    let position = crate::game_objects::bevy_position(Vec3::from_array(rules.towers[0].position));
    for (_, player) in players.iter().step_by(2) {
        world.entity_mut(*player).insert(position);
    }
    world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .get_mut(&17)
        .unwrap()
        .eye = Some(shared::battleground::EyeBattle::new(rules));
    pvp_objective::update_battleground_objectives(&mut world);
    advance_objective(&mut world, 40_000);
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let matched = state.battleground_matches.get_mut(&17).unwrap();
        assert_eq!(matched.instance.phase, BgPhase::InProgress);
        assert!(
            matched.instance.score_a.score > 0,
            "real physical presence captured a source tower"
        );
        let baseline = template.win_score - 1 - matched.instance.score_a.score;
        matched.instance.add_score(BgTeam::A, baseline, &template);
    }
    assert_eq!(wallet(&world, players[0].1), 0);
    advance_objective(&mut world, 2000);
    assert_eq!(
        world.resource::<PvpServerState>().battleground_matches[&17]
            .instance
            .phase,
        BgPhase::Ended
    );
    for (index, (_, player)) in players.iter().enumerate() {
        let expected = if index % 2 == 0 { 122 } else { 61 };
        assert_eq!(wallet(&world, *player), expected);
        assert_eq!(
            world
                .get::<PlayerPvpState>(*player)
                .unwrap()
                .progress
                .lifetime_honor_xp,
            u64::from(expected)
        );
    }
    advance_objective(&mut world, 2000);
    assert_eq!(wallet(&world, players[0].1), 122, "terminal replay");
    let character = players[0].0;
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::persistence::load_currencies(&db, character).get(1792),
        122
    );
    assert_eq!(
        crate::persistence::load_pvp(&db, character).lifetime_honor_xp,
        122
    );
}

#[test]
fn arathi_resource_honor_final_unclamped_tick_excludes_external_score_seed() {
    let template = arathi_basin();
    let (mut world, players, file) = honor_match(&template, 30);
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let matched = state.battleground_matches.get_mut(&17).unwrap();
        let mut nodes = NodeBattle::new(node_rules(2107).unwrap());
        for entry in [227420, 227522, 227536, 227538, 227544] {
            nodes.assault(entry, BgTeam::A).unwrap();
        }
        matched.nodes = Some(nodes);
    }
    pvp_objective::update_battleground_objectives(&mut world);
    advance_objective(&mut world, 60_000);
    advance_objective(&mut world, 2000);
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let matched = state.battleground_matches.get_mut(&17).unwrap();
        assert_eq!(matched.instance.score_a.score, 120);
        matched.instance.add_score(BgTeam::A, 1370, &template);
        assert_eq!(matched.instance.score_a.score, 1490);
        assert_eq!(matched.instance.phase, BgPhase::InProgress);
    }
    assert_eq!(
        wallet(&world, players[0].1),
        0,
        "external score baseline is not resource income"
    );
    advance_objective(&mut world, 2000);
    let instance = &world.resource::<PvpServerState>().battleground_matches[&17].instance;
    assert_eq!(instance.score_a.score, 1500);
    assert_eq!(instance.phase, BgPhase::Ended);
    for (index, (_, player)) in players.iter().enumerate() {
        let expected = if index % 2 == 0 { 183 } else { 61 };
        assert_eq!(wallet(&world, *player), expected);
        assert_eq!(
            world
                .get::<PlayerPvpState>(*player)
                .unwrap()
                .progress
                .lifetime_honor_xp,
            u64::from(expected)
        );
    }
    advance_objective(&mut world, 2000);
    assert_eq!(wallet(&world, players[0].1), 183, "terminal replay");
    let character = players[0].0;
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::persistence::load_currencies(&db, character).get(1792),
        183
    );
    assert_eq!(
        crate::persistence::load_pvp(&db, character).lifetime_honor_xp,
        183
    );
}

fn honor_match(
    template: &shared::battleground::BgTemplate,
    level: u8,
) -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    let (mut world, players, file) = native_flag_match(template);
    world.init_resource::<Time>();
    world.init_resource::<Time<Virtual>>();
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    world.init_resource::<crate::auras::UnhandledAuras>();
    for (_, player) in &players {
        world.entity_mut(*player).insert((
            PlayerPvpState::default(),
            crate::currency::PlayerCurrencies::default(),
            shared::components::Level(level),
        ));
    }
    world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .get_mut(&17)
        .unwrap()
        .instance
        .bracket = shared::battleground::source_level_bracket(template.map_id, level).unwrap();
    (world, players, file)
}

fn wallet(world: &World, player: Entity) -> u32 {
    world
        .get::<crate::currency::PlayerCurrencies>(player)
        .unwrap()
        .0
        .get(1792)
}

fn advance_objective(world: &mut World, millis: u64) {
    world
        .resource_mut::<Time<Fixed>>()
        .advance_by(std::time::Duration::from_millis(millis));
    pvp_objective::update_battleground_objectives(world);
}
