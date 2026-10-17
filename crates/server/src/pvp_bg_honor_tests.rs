//! Real native-objective terminal tick, not a direct reward-function call.
use super::*;
use shared::battleground::{BgPhase, BgTeam, LevelBracket, NodeBattle, arathi_basin, node_rules};
use shared::components::Health;

#[test]
fn arathi_completion_mirrors_two_separate_source_bonuses_and_reopens_without_replay() {
    for (level, range, one_bonus) in [(30, (30, 39), 61_u32), (90, (90, 90), 124)] {
        let template = arathi_basin();
        let (mut world, players, file) = native_flag_match(&template);
        world.init_resource::<Time>();
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
        {
            let mut state = world.resource_mut::<PvpServerState>();
            let matched = state.battleground_matches.get_mut(&17).unwrap();
            matched.instance.bracket = LevelBracket::new(range.0, range.1);
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
