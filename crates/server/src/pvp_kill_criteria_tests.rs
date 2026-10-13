use super::*;
use crate::achievement::PlayerAchievementState;
use crate::maps::InMap;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Health, Position};

#[test]
fn achievements_criteria_native_player_map_some_credits_only_actual_warsong_killing_blow() {
    let (mut world, players, file) = kill_match();
    let killer = players[0].1;
    let victim = players[1].1;
    assert_eq!(world.get::<InMap>(killer).unwrap().map_id, 2106);
    damage(&mut world, killer, victim, 1.0);
    assert!(
        !world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .contains_key(&5538)
    );
    damage(&mut world, killer, victim, 1000.0);
    assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
    for (index, (_, player)) in players.iter().enumerate() {
        assert_eq!(
            world
                .get::<PlayerAchievementState>(*player)
                .unwrap()
                .criteria_progress
                .get(&5538),
            (index == 0).then_some(&1),
            "only killer, not eligible teammate or victim {index}"
        );
    }
    damage(&mut world, killer, victim, 1000.0);
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(
        world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .get(&5538),
        Some(&1),
        "dead/idle replay"
    );
    drop(world);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::persistence::load_achievements(&reopened, players[0].0)
            .criteria_progress
            .get(&5538),
        Some(&1),
        "native killing blow survives storage reopen"
    );
}

fn player_kills(world: &World, player: Entity) -> Option<u64> {
    world
        .get::<PlayerAchievementState>(player)
        .unwrap()
        .criteria_progress
        .get(&26004)
        .copied()
}

fn damage(world: &mut World, caster: Entity, target: Entity, amount: f32) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let ctx = EffectCtx {
        caster,
        spell: data.store.get(133).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(world, &ctx, target, amount, false, false);
}

fn kill_match() -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    let template = shared::battleground::warsong_gulch();
    let (mut world, players, file) = native_flag_match(&template);
    world.init_resource::<Time>();
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    world.init_resource::<Messages<crate::death::PlayerDeathTransition>>();
    world.init_resource::<crate::death::DeathServerState>();
    world.init_resource::<crate::death::DeathTransitionCounters>();
    pvp_objective::update_battleground_objectives(&mut world);
    (world, players, file)
}

#[test]
fn achievements_criteria_native_battleground_kill_rejects_world_self_and_unaccepted_or_inactive_matches()
 {
    for control in [
        "world",
        "self",
        "waiting",
        "ended",
        "killer_pending",
        "victim_pending",
    ] {
        let (mut world, players, _file) = kill_match();
        let killer = players[0].1;
        let mut victim = players[1].1;
        match control {
            "world" => {
                world
                    .resource_mut::<PvpServerState>()
                    .battleground_matches
                    .clear();
                world.entity_mut(killer).insert(InMap::continent(0));
                world.entity_mut(victim).insert(InMap::continent(0));
            }
            "self" => victim = killer,
            "waiting" => {
                world
                    .resource_mut::<PvpServerState>()
                    .battleground_matches
                    .get_mut(&17)
                    .unwrap()
                    .instance
                    .phase = shared::battleground::BgPhase::Waiting
            }
            "ended" => {
                world
                    .resource_mut::<PvpServerState>()
                    .battleground_matches
                    .get_mut(&17)
                    .unwrap()
                    .instance
                    .phase = shared::battleground::BgPhase::Ended
            }
            pending => {
                let index = if pending == "killer_pending" { 0 } else { 1 };
                world
                    .resource_mut::<PvpServerState>()
                    .battleground_matches
                    .get_mut(&17)
                    .unwrap()
                    .accepted
                    .remove(&players[index].0);
            }
        }
        damage(&mut world, killer, victim, 1000.0);
        assert_eq!(
            world.get::<Health>(victim).unwrap().current,
            0.0,
            "native lethal damage {control}"
        );
        for (_, player) in &players {
            assert_eq!(player_kills(&world, *player), None, "no reward {control}");
            // TC Unit::Kill11449–11453 credits56 independently of BG reward70,
            // including self/unfinished matches; only PlayerMap32 gates5538 here.
            let expected_blow = (*player == killer && control != "world").then_some(&1);
            assert_eq!(
                world
                    .get::<PlayerAchievementState>(*player)
                    .unwrap()
                    .criteria_progress
                    .get(&5538),
                expected_blow,
                "killing blow is not a battleground kill reward {control}"
            );
        }
    }
}

#[test]
fn achievements_criteria_native_battleground_kill_credits_killer_and_team_at_reward_distance() {
    let template = shared::battleground::warsong_gulch();
    let (mut world, players, file) = kill_match();
    let killer = players[0].1;
    let victim = players[1].1;
    // The killer is credited even if its current position moved beyond reward distance.
    world.get_mut::<Position>(killer).unwrap().x = 100.0;
    // Living player reach1.5 + victim reach1.5 makes77 yards exactly reward distance74.
    world.get_mut::<Position>(players[2].1).unwrap().x = 77.0;
    world.get_mut::<Position>(players[6].1).unwrap().x = 77.01;
    world.entity_mut(players[8].1).insert(InMap {
        map_id: template.map_id,
        instance_id: 18,
    });
    // TC uses a dead teammate's corpse, not their remote ghost position (corpse reach0).
    let ghost = players[4].1;
    world.get_mut::<Position>(ghost).unwrap().x = 500.0;
    world.get_mut::<Health>(ghost).unwrap().current = 0.0;
    world
        .entity_mut(ghost)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Ghost,
            corpse: Some(shared::death::Corpse {
                player: players[4].0,
                map_id: template.map_id as u16,
                x: 75.5,
                y: 0.0,
                z: 0.0,
                created_at: 0,
            }),
            graveyard: None,
        });
    damage(&mut world, killer, victim, 1.0);
    assert!(world.get::<Health>(victim).unwrap().current > 0.0);
    assert_eq!(player_kills(&world, killer), None, "nonlethal damage");
    damage(&mut world, killer, victim, 1000.0);
    assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
    assert!(
        !world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .contains_key(&11491),
        "start/fail-gated Isle criterion remains disabled"
    );
    assert!(
        !world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .contains_key(&20124),
        "Power Orb aura requirement remains conjunctive"
    );
    assert_eq!(
        world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .get(&50458),
        Some(&1),
        "BG map-type killing blow"
    );
    assert!(
        !world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .contains_key(&50457),
        "BG cannot count as arena"
    );
    world
        .run_system_once(crate::death::process_player_death_transitions)
        .unwrap();
    assert_eq!(
        world
            .get::<PlayerAchievementState>(victim)
            .unwrap()
            .criteria_progress
            .get(&50460),
        Some(&1),
        "actual BG death transition"
    );
    for index in [0, 2, 4] {
        assert_eq!(
            player_kills(&world, players[index].1),
            Some(1),
            "eligible killer/team{index}"
        );
    }
    for index in [1, 3, 5, 6, 7, 8, 9] {
        assert_eq!(
            player_kills(&world, players[index].1),
            None,
            "enemy/far/other-copy{index}"
        );
    }
    damage(&mut world, killer, victim, 1000.0);
    assert_eq!(player_kills(&world, killer), Some(1), "dead-target replay");
    pvp_objective::update_battleground_objectives(&mut world);
    assert_eq!(player_kills(&world, killer), Some(1), "idle replay");
    world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .get_mut(&17)
        .unwrap()
        .accepted
        .remove(&players[2].0);
    damage(&mut world, killer, players[3].1, 1000.0);
    assert_eq!(player_kills(&world, killer), Some(2));
    assert_eq!(player_kills(&world, ghost), Some(2));
    assert_eq!(
        player_kills(&world, players[2].1),
        Some(1),
        "unaccepted teammate cannot borrow roster membership"
    );
    drop(world);
    let reopened = redb::Database::open(file.path()).unwrap();
    for index in [0, 2, 4] {
        assert_eq!(
            crate::persistence::load_achievements(&reopened, players[index].0)
                .criteria_progress
                .get(&26004),
            Some(&(if index == 2 { 1 } else { 2 })),
            "reopened team{index}"
        );
    }
}
