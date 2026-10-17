use super::*;
use shared::battleground::{BgOutcome, BgPhase, BgTeam, BgTemplate, node_rules};
use shared::protocol::BattlegroundSnapshot;

struct EnteredMatch {
    h: Harness,
    players: Vec<Traveler>,
    teams: Vec<BgTeam>,
    match_id: u64,
    copy: InMap,
    origin: Vec3,
}

#[test]
fn warsong_channel_zero_capture_timeout_gets_only_end_honor() {
    let template = shared::battleground::warsong_gulch();
    let rules = shared::battleground::flag_rules(template.map_id).unwrap();
    let mut entered = enter_source_match(&template, rules.starts);
    use_second_frames(&mut entered.h);
    assert_team_honor(&entered, 0, 0);
    entered
        .h
        .advance((shared::battleground::FLAG_MATCH_MS / 1000 + 1) as usize);
    let snapshot = battleground(&entered.h, &entered.players[0]);
    assert_eq!(snapshot.phase, BgPhase::Ended);
    assert_eq!(snapshot.outcome, Some(BgOutcome::Draw));
    assert_eq!([snapshot.score_a, snapshot.score_b], [0, 0]);
    assert_team_honor(&entered, 59, 59);
    entered.h.advance(30);
    assert_team_honor(&entered, 59, 59);
    let world = entered.h.app.world();
    let character = world.get::<ClientId>(entered.players[0].player).unwrap().0;
    let database = world.resource::<crate::persistence::GameDatabase>();
    assert_eq!(
        crate::persistence::load_currencies(&database.db, character).get(1792),
        59
    );
    assert_eq!(
        crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
        59
    );
    leave_and_return(&mut entered);
}

#[test]
fn flag_channel_capture_honor_uses_source_brackets_and_both_assigned_teams() {
    for (template, bonus, winner, final_awards) in [
        (
            shared::battleground::warsong_gulch(),
            59,
            BgTeam::A,
            [286, 118],
        ),
        (
            shared::battleground::warsong_gulch(),
            59,
            BgTeam::B,
            [118, 286],
        ),
        (
            shared::battleground::twin_peaks(),
            121,
            BgTeam::A,
            [363, 121],
        ),
        (
            shared::battleground::twin_peaks(),
            121,
            BgTeam::B,
            [121, 363],
        ),
    ] {
        let rules = shared::battleground::flag_rules(template.map_id).unwrap();
        let mut entered = enter_source_match(&template, rules.starts);
        let mut captures = [0, 0];
        let opponent = if winner == BgTeam::A {
            BgTeam::B
        } else {
            BgTeam::A
        };
        for team in [winner, opponent, winner, winner] {
            let index = usize::from(team == BgTeam::B);
            let actor_index = entered
                .teams
                .iter()
                .position(|value| *value == team)
                .unwrap();
            let actor = &entered.players[actor_index];
            let (enemy_entry, home_entry) = if team == BgTeam::A {
                (
                    shared::protocol::WARSONG_HORDE_FLAG_ENTRY,
                    shared::protocol::WARSONG_ALLIANCE_FLAG_ENTRY,
                )
            } else {
                (
                    shared::protocol::WARSONG_ALLIANCE_FLAG_ENTRY,
                    shared::protocol::WARSONG_HORDE_FLAG_ENTRY,
                )
            };
            let (_, home) = objects_in(&mut entered.h, entered.copy, home_entry)[0];
            let (flag, at) = objects_in(&mut entered.h, entered.copy, enemy_entry)[0];
            entered.h.walk_to(actor, at);
            use_object(&mut entered.h, actor, flag);
            assert_team_honor(&entered, captures[0] * bonus, captures[1] * bonus);
            entered.h.walk_to(actor, home);
            entered.h.receive_pvp_updates(5);
            captures[index] += 1;
            let snapshot = battleground(&entered.h, actor);
            assert_eq!([snapshot.score_a, snapshot.score_b], captures);
            let expected = if snapshot.phase == BgPhase::Ended {
                final_awards
            } else {
                [captures[0] * bonus, captures[1] * bonus]
            };
            assert_team_honor(&entered, expected[0], expected[1]);
            entered.h.receive_pvp_updates(5);
            assert_team_honor(&entered, expected[0], expected[1]);
        }
        let snapshot = battleground(&entered.h, &entered.players[0]);
        assert_eq!(snapshot.phase, BgPhase::Ended);
        assert_eq!(snapshot.outcome, Some(BgOutcome::Winner(winner)));
        for (actor, team) in entered.players.iter().zip(&entered.teams) {
            let expected = final_awards[usize::from(*team == BgTeam::B)];
            let world = entered.h.app.world();
            let character = world.get::<ClientId>(actor.player).unwrap().0;
            let database = world.resource::<crate::persistence::GameDatabase>();
            assert_eq!(
                crate::persistence::load_currencies(&database.db, character).get(1792),
                expected
            );
            assert_eq!(
                crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
                u64::from(expected)
            );
        }
        leave_and_return(&mut entered);
    }
}

#[test]
fn alterac_channel_cross_faction_team_fights_enemy_not_own_captain_and_restores_race() {
    let template = shared::battleground::alterac_valley();
    let mut entered = enter_source_match_with(&template, [611, 610], |app| {
        app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        app.add_plugins(crate::combat::CombatPlugin);
    });
    let actor = entered
        .players
        .iter()
        .zip(&entered.teams)
        .find(|(_, team)| **team == BgTeam::B)
        .unwrap()
        .0;
    let h = &mut entered.h;
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CurrentPvpFaction>(actor.player),
        Some(&shared::components::CurrentPvpFaction(1)),
        "Human entered on Horde team"
    );
    h.app.world_mut().entity_mut(actor.connection).insert((
        pair::<shared::protocol::AttackSwing>(),
        pair::<shared::protocol::AttackStop>(),
        pair::<shared::protocol::AttackStart>(),
        pair::<shared::protocol::AttackStopped>(),
        pair::<shared::protocol::CombatEvent>(),
    ));
    let (enemy, before) = find_alterac_source_npc(h, entered.copy, 11949);
    let at = wow(h.app.world().get::<Position>(enemy).unwrap());
    h.walk_to(actor, at);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::AttackSwing>>(actor.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::AttackSwing {
            target: enemy.to_bits(),
        });
    h.advance(20);
    let after = h.app.world().get::<Health>(enemy).unwrap().current;
    assert!(
        after < before,
        "Human assigned Horde must hit Alliance captain: {before} -> {after}"
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::AttackStop>>(actor.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::AttackStop);
    h.advance(5);
    let (friendly, before) = find_alterac_source_npc(h, entered.copy, 11947);
    let at = wow(h.app.world().get::<Position>(friendly).unwrap());
    h.walk_to(actor, at);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::AttackSwing>>(actor.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::AttackSwing {
            target: friendly.to_bits(),
        });
    h.advance(20);
    assert_eq!(
        h.app.world().get::<Health>(friendly).unwrap().current,
        before
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(actor.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground {
            match_id: entered.match_id,
        });
    h.receive_pvp_updates(5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFactionTemplate>(actor.player)
            .unwrap()
            .0,
        1
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CurrentPvpFaction>(actor.player),
        Some(&shared::components::CurrentPvpFaction(2)),
        "successful return restores racial Alliance"
    );
}

#[test]
fn alterac_channel_last_leave_with_retargeting_npc_settles_target_mirror_before_unload() {
    let template = shared::battleground::alterac_valley();
    let mut entered = enter_source_match_with(&template, [611, 610], |app| {
        app.add_plugins(crate::unit_target::UnitTargetPlugin);
    });
    let (captain, _) = find_alterac_source_npc(&mut entered.h, entered.copy, 11947);
    let target = entered.players[0].player.to_bits();
    entered
        .h
        .app
        .world_mut()
        .entity_mut(captain)
        .remove::<crate::grid_activity::GridDormant>();
    entered.h.app.add_systems(
        Update,
        move |mut states: Query<&mut crate::creature_ai::CreatureState>| {
            if let Ok(mut state) = states.get_mut(captain) {
                *state = match *state {
                    crate::creature_ai::CreatureState::Pursuing { .. } => {
                        crate::creature_ai::CreatureState::Idle
                    }
                    _ => crate::creature_ai::CreatureState::Pursuing { target },
                };
            }
        },
    );
    for actor in &entered.players {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(actor.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: entered.match_id,
            });
    }
    entered.h.receive_pvp_updates(5);
    assert!(entered.h.app.world().get_entity(captain).is_err());
    for actor in &entered.players {
        let returns = entered.h.new_worlds(actor);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].map_id, 0);
        entered.h.ack(actor);
        assert!(close(entered.h.position(actor), entered.origin));
        assert_eq!(
            entered.h.app.world().get::<InMap>(actor.player),
            Some(&STORMWIND)
        );
        assert!(
            entered
                .h
                .app
                .world()
                .get::<crate::pvp::PlayerPvpState>(actor.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
}

#[test]
fn alterac_channel_last_leave_with_fresh_npc_death_unloads_without_loot_panic() {
    let template = shared::battleground::alterac_valley();
    let mut entered = enter_source_match_with(&template, [611, 610], |app| {
        app.init_resource::<crate::merchant::VendorCatalog>();
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.add_plugins(crate::loot::LootPlugin);
    });
    let actor = &entered.players[0];
    let (victim, health) = find_alterac_source_npc(&mut entered.h, entered.copy, 11947);
    crate::combat::deal_direct_damage(
        entered.h.app.world_mut(),
        actor.player,
        victim,
        health + 1.0,
    );
    assert!(
        entered
            .h
            .app
            .world()
            .get::<crate::combat::DeathState>(victim)
            .is_some()
    );
    // The real death is still fresh when every accepted participant's normal
    // Leave request arrives; no intermediate frame may settle corpse commands.
    for who in &entered.players {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: entered.match_id,
            });
    }
    entered.h.receive_pvp_updates(5);
    assert!(entered.h.app.world().get_entity(victim).is_err());
    let world = entered.h.app.world_mut();
    let mut maps = world.query::<&InMap>();
    assert!(!maps.iter(world).any(|map| *map == entered.copy));
    for who in &entered.players {
        assert!(
            world
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
}

#[test]
fn alterac_channel_captain_general_honor_uses_source_bracket_and_single_death() {
    for (winner, captain_entry, general_entry, outside_entry) in [
        (BgTeam::A, 11947, 11946, 11949),
        (BgTeam::B, 11949, 11948, 11947),
    ] {
        let template = shared::battleground::alterac_valley();
        let mut entered = enter_source_match(&template, [611, 610]);
        use_second_frames(&mut entered.h);
        assert_team_honor(&entered, 0, 0);
        let actor_index = entered
            .teams
            .iter()
            .position(|team| *team == winner)
            .unwrap();
        let actor = &entered.players[actor_index];
        let assert_rewards = |entered: &EnteredMatch, amount| {
            let expected = if winner == BgTeam::A {
                [amount, 0]
            } else {
                [0, amount]
            };
            assert_team_honor(entered, expected[0], expected[1]);
        };
        let other_copy = InMap {
            instance_id: entered.copy.instance_id + 1,
            ..entered.copy
        };
        let (outside_captain, _) =
            find_alterac_source_npc(&mut entered.h, entered.copy, outside_entry);
        entered
            .h
            .app
            .world_mut()
            .entity_mut(outside_captain)
            .insert(other_copy);
        kill_alterac_source_npc(&mut entered.h, actor, other_copy, outside_entry);
        entered.h.receive_pvp_updates(5);
        let snapshot = battleground(&entered.h, actor);
        assert_eq!([snapshot.score_a, snapshot.score_b], [700, 700]);
        assert_team_honor(&entered, 0, 0);

        let (captain, health) =
            find_alterac_source_npc(&mut entered.h, entered.copy, captain_entry);
        let dead_index = entered
            .teams
            .iter()
            .enumerate()
            .find_map(|(index, team)| (*team == winner && index != actor_index).then_some(index))
            .unwrap();
        let dead_teammate = entered.players[dead_index].player;
        crate::combat::deal_direct_damage(entered.h.app.world_mut(), captain, dead_teammate, 1.0e7);
        entered.h.receive_pvp_updates(5);
        assert_eq!(
            entered
                .h
                .app
                .world()
                .get::<Health>(dead_teammate)
                .unwrap()
                .current,
            0.0
        );
        assert_team_honor(&entered, 0, 0);
        // A normal NPC-caused death does not disqualify the accepted team member from Honor.
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            captain,
            health + 1.0,
        );
        entered.h.receive_pvp_updates(5);
        let captain_scores = if winner == BgTeam::A {
            [699, 600]
        } else {
            [600, 699]
        };
        let snapshot = battleground(&entered.h, actor);
        assert_eq!([snapshot.score_a, snapshot.score_b], captain_scores);
        assert_eq!(snapshot.phase, BgPhase::InProgress);
        // Source20–79 uses79, not player20; round the whole3-kill bonus once.
        assert_rewards(&entered, 368);
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            captain,
            health + 1.0,
        );
        entered.h.receive_pvp_updates(5);
        let snapshot = battleground(&entered.h, actor);
        assert_eq!([snapshot.score_a, snapshot.score_b], captain_scores);
        assert_rewards(&entered, 368);

        let (general, health) =
            find_alterac_source_npc(&mut entered.h, entered.copy, general_entry);
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            general,
            health + 1.0,
        );
        entered.h.receive_pvp_updates(5);
        for who in &entered.players {
            let snapshot = battleground(&entered.h, who);
            assert_eq!(snapshot.phase, BgPhase::Ended);
            assert_eq!(snapshot.outcome, Some(BgOutcome::Winner(winner)));
        }
        // General4=490 + captain3=368; survivor OnEnd adds ten equivalents1225.
        let completed = if winner == BgTeam::A {
            [2083, 980]
        } else {
            [980, 2083]
        };
        assert_team_honor(&entered, completed[0], completed[1]);
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            general,
            health + 1.0,
        );
        entered.h.receive_pvp_updates(5);
        assert_team_honor(&entered, completed[0], completed[1]);
        let world = entered.h.app.world();
        let character = world.get::<ClientId>(actor.player).unwrap().0;
        let database = world.resource::<crate::persistence::GameDatabase>();
        assert_eq!(
            crate::persistence::load_currencies(&database.db, character).get(1792),
            2083
        );
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            2083
        );
        leave_and_return(&mut entered);
    }
}

#[test]
fn alterac_channel_survivor_honor_runs_once_on_tower_and_player_depletion() {
    for tower_end in [true, false] {
        let mut entered = enter_source_match(&shared::battleground::alterac_valley(), [611, 610]);
        use_second_frames(&mut entered.h);
        let actor = &entered.players[0];
        let victim = entered
            .players
            .iter()
            .zip(&entered.teams)
            .find(|(_, team)| **team == BgTeam::B)
            .unwrap()
            .0;
        crate::pvp::seed_terminal_reinforcements(
            entered.h.app.world_mut(),
            entered.match_id,
            BgTeam::B,
            if tower_end { 75 } else { 1 },
        );
        let expected = if tower_end {
            let (banner, at) = objects_in(&mut entered.h, entered.copy, 178946)[0];
            entered.h.walk_to(actor, at);
            use_object(&mut entered.h, actor, banner);
            entered.h.advance(8);
            entered.h.advance(240);
            [1593, 980] // Tower368 +10 survivor equivalents1225; loser8 equivalents980.
        } else {
            let (captain, _) = find_alterac_source_npc(&mut entered.h, entered.copy, 11949);
            crate::combat::deal_direct_damage(
                entered.h.app.world_mut(),
                captain,
                victim.player,
                1.0e7,
            );
            entered.h.receive_pvp_updates(5);
            assert_eq!(
                entered
                    .h
                    .app
                    .world()
                    .get::<Health>(victim.player)
                    .unwrap()
                    .current,
                0.0
            );
            crate::combat::deal_direct_damage(
                entered.h.app.world_mut(),
                captain,
                victim.player,
                1.0e7,
            );
            [1225, 1225] // Both teams retain four controlled towers and a living captain.
        };
        let snapshot = battleground(&entered.h, actor);
        assert_eq!(snapshot.phase, BgPhase::Ended);
        assert_eq!(snapshot.outcome, Some(BgOutcome::Winner(BgTeam::A)));
        assert_team_honor(&entered, expected[0], expected[1]);
        entered.h.advance(300);
        assert_team_honor(&entered, expected[0], expected[1]);
        let world = entered.h.app.world();
        let character = world.get::<ClientId>(actor.player).unwrap().0;
        let database = world.resource::<crate::persistence::GameDatabase>();
        assert_eq!(
            crate::persistence::load_currencies(&database.db, character).get(1792),
            expected[0]
        );
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            u64::from(expected[0])
        );
        leave_and_return(&mut entered);
    }
}

#[test]
fn alterac_channel_survivor_honor_counts_controlled_towers_and_living_captains() {
    for (team, own_captain, enemy_general, tower, contested_tower) in [
        (BgTeam::A, 11949, 11946, 178946, 178945),
        (BgTeam::B, 11947, 11948, 178925, 178929),
    ] {
        let mut entered = enter_source_match(&shared::battleground::alterac_valley(), [611, 610]);
        use_second_frames(&mut entered.h);
        let actor_index = entered
            .teams
            .iter()
            .position(|value| *value == team)
            .unwrap();
        let opponent_index = entered
            .teams
            .iter()
            .position(|value| *value != team)
            .unwrap();
        let actor = &entered.players[actor_index];
        let opponent = &entered.players[opponent_index];
        let (banner, at) = objects_in(&mut entered.h, entered.copy, tower)[0];
        entered.h.walk_to(actor, at);
        use_object(&mut entered.h, actor, banner);
        entered.h.advance(8);
        entered.h.advance(240);
        let (banner, at) = objects_in(&mut entered.h, entered.copy, contested_tower)[0];
        entered.h.walk_to(actor, at);
        use_object(&mut entered.h, actor, banner);
        entered.h.advance(8);
        kill_alterac_source_npc(&mut entered.h, opponent, entered.copy, own_captain);
        entered.h.receive_pvp_updates(5);
        assert_team_honor(&entered, 368, 368);
        let (general, health) =
            find_alterac_source_npc(&mut entered.h, entered.copy, enemy_general);
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            general,
            health + 1.0,
        );
        entered.h.receive_pvp_updates(5);
        let snapshot = battleground(&entered.h, actor);
        assert_eq!(snapshot.phase, BgPhase::Ended);
        assert_eq!(snapshot.outcome, Some(BgOutcome::Winner(team)));
        // Attacker: tower368 + general490 +4 controlled towers*2 equivalents980.
        // Defender: captain368 + (2 controlled towers + living captain)*2 equivalents735.
        let expected = if team == BgTeam::A {
            [1838, 1103]
        } else {
            [1103, 1838]
        };
        assert_team_honor(&entered, expected[0], expected[1]);
        crate::combat::deal_direct_damage(
            entered.h.app.world_mut(),
            actor.player,
            general,
            health + 1.0,
        );
        entered.h.advance(300);
        assert_team_honor(&entered, expected[0], expected[1]);
        let world = entered.h.app.world();
        let character = world.get::<ClientId>(actor.player).unwrap().0;
        let database = world.resource::<crate::persistence::GameDatabase>();
        assert_eq!(
            crate::persistence::load_currencies(&database.db, character).get(1792),
            1838
        );
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            1838
        );
        leave_and_return(&mut entered);
    }
}

#[test]
fn alterac_channel_tower_honor_requires_destruction_and_rounds_each_event() {
    for (team, tower_entry, contested_entry, second_tower, graveyard) in [
        (BgTeam::A, 178946, 178940, 178945, 179483),
        (BgTeam::B, 178925, 179435, 178929, 179284),
    ] {
        let template = shared::battleground::alterac_valley();
        let mut entered = enter_source_match(&template, [611, 610]);
        use_second_frames(&mut entered.h);
        let actor_index = entered
            .teams
            .iter()
            .position(|value| *value == team)
            .unwrap();
        let defender_index = entered
            .teams
            .iter()
            .position(|value| *value != team)
            .unwrap();
        let actor = &entered.players[actor_index];
        let defender = &entered.players[defender_index];
        let (banner, at) = objects_in(&mut entered.h, entered.copy, tower_entry)[0];
        entered.h.walk_to(actor, at);
        use_object(&mut entered.h, actor, banner);
        entered
            .h
            .app
            .world_mut()
            .entity_mut(actor.player)
            .insert(crate::networking::Moving(true));
        entered.h.advance(1);
        entered
            .h
            .app
            .world_mut()
            .entity_mut(actor.player)
            .insert(crate::networking::Moving(false));
        entered.h.advance(8);
        assert_eq!(
            objects_in(&mut entered.h, entered.copy, tower_entry).len(),
            1
        );
        assert_team_honor(&entered, 0, 0);

        use_object(&mut entered.h, actor, banner);
        entered.h.advance(8);
        entered.h.advance(200);
        assert_team_honor(&entered, 0, 0);
        let (contested, at) = objects_in(&mut entered.h, entered.copy, contested_entry)[0];
        entered.h.walk_to(defender, at);
        use_object(&mut entered.h, defender, contested);
        entered.h.advance(8);
        entered.h.advance(240);
        assert_eq!(
            objects_in(&mut entered.h, entered.copy, tower_entry).len(),
            1
        );
        assert_team_honor(&entered, 0, 0);

        for (entry, amount, remaining) in [(tower_entry, 368, 625), (second_tower, 736, 550)] {
            let (banner, at) = objects_in(&mut entered.h, entered.copy, entry)[0];
            entered.h.walk_to(actor, at);
            use_object(&mut entered.h, actor, banner);
            entered.h.advance(8);
            entered.h.advance(240);
            let snapshot = battleground(&entered.h, actor);
            let enemy_reinforcements = if team == BgTeam::A {
                snapshot.score_b
            } else {
                snapshot.score_a
            };
            assert_eq!(
                enemy_reinforcements, remaining,
                "tower destruction: team={team:?}, entry={entry}"
            );
            assert_eq!(snapshot.phase, BgPhase::InProgress);
            let expected = if team == BgTeam::A {
                [amount, 0]
            } else {
                [0, amount]
            };
            assert_team_honor(&entered, expected[0], expected[1]);
            entered.h.advance(240);
            assert_team_honor(&entered, expected[0], expected[1]);
        }
        let (banner, at) = objects_in(&mut entered.h, entered.copy, graveyard)[0];
        entered.h.walk_to(actor, at);
        use_object(&mut entered.h, actor, banner);
        entered.h.advance(8);
        entered.h.advance(240);
        let expected = if team == BgTeam::A {
            [736, 0]
        } else {
            [0, 736]
        };
        assert_team_honor(&entered, expected[0], expected[1]);
        let world = entered.h.app.world();
        let character = world.get::<ClientId>(actor.player).unwrap().0;
        let database = world.resource::<crate::persistence::GameDatabase>();
        assert_eq!(
            crate::persistence::load_currencies(&database.db, character).get(1792),
            736
        );
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            736
        );
        leave_and_return(&mut entered);
    }
}

#[test]
fn alterac_channel_source_opening_cancel_tower_captain_general_and_origin_return() {
    let template = shared::battleground::alterac_valley();
    let mut entered = enter_source_match(&template, [611, 610]);
    assert_eq!(entered.players.len(), 20);
    let victim = entered
        .players
        .iter()
        .zip(&entered.teams)
        .find(|(_, team)| **team == BgTeam::B)
        .unwrap()
        .0;
    let actor = &entered.players[0];
    let h = &mut entered.h;
    use_second_frames(h);
    assert_eq!(battleground(h, actor).score_a, 700);
    assert_eq!(battleground(h, actor).score_b, 700);
    let (banner, at) = objects_in(h, entered.copy, 178946)[0];
    h.walk_to(actor, at);
    let other_copy = InMap {
        instance_id: entered.copy.instance_id + 1,
        ..entered.copy
    };
    h.app.world_mut().entity_mut(banner).insert(other_copy);
    use_object(h, actor, banner);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    h.app.world_mut().entity_mut(banner).insert(entered.copy);
    use_object(h, actor, banner);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_some()
    );
    assert!(objects_in(h, entered.copy, 178940).is_empty());
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(true));
    h.advance(1);
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(false));
    h.advance(8);
    assert!(objects_in(h, entered.copy, 178940).is_empty());
    use_object(h, actor, banner);
    h.advance(8);
    assert!(objects_in(h, entered.copy, 178946).is_empty());
    assert_eq!(objects_in(h, entered.copy, 178940).len(), 1);
    h.advance(220);
    assert_eq!(battleground(h, actor).score_b, 700);
    h.advance(20);
    assert_eq!(battleground(h, actor).score_b, 625);
    assert!(objects_in(h, entered.copy, 178940).is_empty());
    crate::combat::deal_direct_damage(h.app.world_mut(), actor.player, victim.player, 1000.0);
    h.advance(2);
    assert_eq!(battleground(h, actor).score_b, 624);
    h.app
        .world_mut()
        .write_message(crate::death::PlayerDeathTransition {
            player: victim.player,
        });
    h.advance(2);
    assert_eq!(battleground(h, actor).score_b, 624);
    assert_eq!(
        battleground(h, actor)
            .players
            .iter()
            .find(|row| row.character_id == h.app.world().get::<ClientId>(victim.player).unwrap().0)
            .unwrap()
            .deaths,
        1
    );
    let (other_captain, _) = find_alterac_source_npc(h, entered.copy, 11949);
    h.app
        .world_mut()
        .entity_mut(other_captain)
        .insert(other_copy);
    kill_alterac_source_npc(h, actor, other_copy, 11949);
    h.receive_pvp_updates(5);
    assert_eq!(battleground(h, actor).score_a, 700);
    assert!(battleground(h, actor).world_states.contains(&(1351, 1)));
    kill_alterac_source_npc(h, actor, entered.copy, 11947);
    h.receive_pvp_updates(5);
    assert_eq!(battleground(h, actor).score_b, 524);
    assert!(battleground(h, actor).world_states.contains(&(1352, 0)));
    kill_alterac_source_npc(h, actor, entered.copy, 11946);
    h.receive_pvp_updates(5);
    assert_eq!(battleground(h, actor).phase, BgPhase::Ended);
    let row = battleground(h, actor)
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert!(row.objective_scores.contains(&(61, 1)));
    win_leave_and_return(&mut entered, 700);
}

/// TDB1210 spawn groups 1041-1173 with their source-33 conditions: only the owner's
/// tier-0 defenders, neutral mine crews and intact-tower warmasters exist at the start;
/// a destroyed tower despawns its warmaster (flag 0x20) and keeps its flag-0 bowmen.
#[test]
fn alterac_channel_spawns_only_condition_met_source_spawn_groups() {
    let template = shared::battleground::alterac_valley();
    let mut entered = enter_source_match(&template, [611, 610]);
    let actor = &entered.players[0];
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    // Aid Station/Stormpike/Stonehearth Alliance and Iceblood/Frostwolf/Hut Horde
    // graveyards (1325/1333/1302, 1347/1338/1330) each spawn four tier-0 guards;
    // neutral Snowfall (1966) spawns none.
    assert_eq!(creatures_in(h, copy, 12050), 12, "Stormpike Defender");
    assert_eq!(creatures_in(h, copy, 12053), 12, "Frostwolf Guardian");
    for upgraded in [13326, 13328, 13331, 13332, 13421, 13422] {
        assert_eq!(
            creatures_in(h, copy, upgraded),
            0,
            "defender tier {upgraded}"
        );
    }
    // Neutral Irondeep (1360, group 1170) and not the Alliance crew (1358, group 1124).
    assert_eq!(creatures_in(h, copy, 10987), 50, "neutral Irondeep Trogg");
    assert_eq!(creatures_in(h, copy, 13396), 0, "Alliance Irondeep Miner");
    assert_eq!(creatures_in(h, copy, 14773), 1, "Iceblood Warmaster");
    assert_eq!(creatures_in(h, copy, 13359), 16, "Frostwolf Bowman");
    // Banner variants are TDB spawn-group objects too: neutral Snowfall (group 1053,
    // world state 1966) and the Horde-owned Iceblood tower banner (group 1100, 1385).
    assert_eq!(
        objects_in(h, copy, 180418).len(),
        1,
        "neutral Snowfall banner"
    );
    assert!(
        objects_in(h, copy, 178365).is_empty(),
        "Alliance Snowfall banner"
    );
    let (banner, at) = objects_in(h, copy, 178946)[0];
    h.walk_to(actor, at);
    use_object(h, actor, banner);
    h.advance(8);
    h.advance(240);
    assert_eq!(battleground(h, actor).score_b, 625);
    assert_eq!(
        creatures_in(h, copy, 14773),
        0,
        "destroyed Iceblood despawns its warmaster"
    );
    assert_eq!(creatures_in(h, copy, 13359), 16, "flag-0 tower guards stay");
}

#[test]
fn isle_channel_match_team_hostility_overrides_race_without_changing_open_world() {
    use crate::spell_cast::targets::{Relation, relation_matches};
    use shared::components::UnitFactionTemplate;
    let mut entered = enter_source_match_with(
        &shared::battleground::isle_of_conquest(),
        [1299, 1245],
        |app| {
            app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        },
    );
    for team in [BgTeam::A, BgTeam::B] {
        let mut allies = entered
            .players
            .iter()
            .zip(&entered.teams)
            .filter(|(_, assigned)| **assigned == team)
            .map(|(player, _)| player.player);
        let actor = allies.next().unwrap();
        let ally = allies.next().unwrap();
        let enemy = entered
            .players
            .iter()
            .zip(&entered.teams)
            .find(|(_, assigned)| **assigned != team)
            .unwrap()
            .0
            .player;
        for actor_race in [1, 2] {
            for target_race in [1, 2] {
                let world = entered.h.app.world_mut();
                world.entity_mut(actor).insert(UnitFactionTemplate(
                    crate::faction::faction_template_for_race(actor_race),
                ));
                for target in [ally, enemy] {
                    world.entity_mut(target).insert(UnitFactionTemplate(
                        crate::faction::faction_template_for_race(target_race),
                    ));
                }
                assert!(
                    relation_matches(world, actor, enemy, Relation::Enemy),
                    "accepted opposing team is attackable: {team:?}, races{actor_race}/{target_race}"
                );
                assert!(!relation_matches(world, actor, ally, Relation::Enemy));
                assert!(relation_matches(world, actor, ally, Relation::Ally));
                assert!(!relation_matches(world, actor, enemy, Relation::Ally));
            }
        }
        let foreign = InMap {
            instance_id: entered.copy.instance_id + 1,
            ..entered.copy
        };
        let world = entered.h.app.world_mut();
        world.entity_mut(enemy).insert(foreign);
        assert!(!relation_matches(world, actor, enemy, Relation::Enemy));
        world.entity_mut(enemy).insert(entered.copy);
        let snapshot = world
            .get::<crate::pvp::PlayerPvpState>(enemy)
            .unwrap()
            .clone();
        world
            .get_mut::<crate::pvp::PlayerPvpState>(enemy)
            .unwrap()
            .battleground
            .as_mut()
            .unwrap()
            .accepted = false;
        assert!(!relation_matches(world, actor, enemy, Relation::Enemy));
        world.entity_mut(enemy).insert(snapshot.clone());
        world
            .get_mut::<crate::pvp::PlayerPvpState>(enemy)
            .unwrap()
            .battleground
            .as_mut()
            .unwrap()
            .phase = BgPhase::Waiting;
        assert!(
            !relation_matches(world, actor, enemy, Relation::Enemy),
            "a player outside the active phase is not attackable"
        );
        world.entity_mut(enemy).insert(snapshot.clone());
        world
            .get_mut::<crate::pvp::PlayerPvpState>(enemy)
            .unwrap()
            .battleground
            .as_mut()
            .unwrap()
            .match_id += 1;
        assert!(
            !relation_matches(world, actor, enemy, Relation::Enemy),
            "different matches cannot attack even with matching physical map IDs"
        );
        world.entity_mut(enemy).insert(snapshot);
    }
    let first = entered.h.spawn(9001, STORMWIND, entered.origin);
    let second = entered.h.spawn(9002, STORMWIND, entered.origin);
    let world = entered.h.app.world_mut();
    world
        .entity_mut(first.player)
        .insert(UnitFactionTemplate(1));
    world
        .entity_mut(second.player)
        .insert(UnitFactionTemplate(2));
    assert!(
        !relation_matches(world, first.player, second.player, Relation::Enemy),
        "existing unflagged open-world player PvP stays rejected"
    );
    world
        .entity_mut(second.player)
        .insert(UnitFactionTemplate(1));
    assert!(relation_matches(
        world,
        first.player,
        second.player,
        Relation::Ally
    ));
}

fn enter_isle_bomb_match() -> EnteredMatch {
    let template = shared::battleground::isle_of_conquest();
    enter_source_match_with(&template, [1299, 1245], |app| {
        let db = &app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let (owner, _) = crate::persistence::create_character(db, 1, "Bombowner", 1, 1).unwrap();
        assert_eq!(
            owner, 1,
            "native inventory owner matches the first channel actor"
        );
        assert!(crate::persistence::inventory_store::load_inventory_record(db, owner).is_some());
        let mut character = crate::persistence::load_character(db, owner).unwrap();
        character.level = u16::from(TRAVELER_LEVEL);
        crate::persistence::save_character(db, owner, &character);
        let path = crate::test_data::world_db_path();
        let source = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let templates = crate::item_templates::load_runtime_item_templates(&path).unwrap();
        assert!(crate::inventory::free_bag_cells(db, &templates, owner) > 0);
        app.insert_resource(templates);
        app.insert_resource(crate::merchant::load_vendor_catalog(&source).unwrap());
        app.insert_resource(crate::quest_data::load_quest_db(&path).unwrap());
        app.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
        app.insert_resource(crate::toy_catalog::load(&source).unwrap());
        app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        app.insert_resource(crate::game_objects::summoned::load_templates(&source).unwrap());
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.add_systems(Update, crate::inventory::process_use_requests);
        let data = crate::spell_cast::test_harness::world_db().unwrap();
        app.insert_resource(crate::spell_info::SpellStore::fixture(
            [21651, 66265, 66268, 66676, 66669, 66674, 66672]
                .into_iter()
                .map(|id| data.store.get(id).unwrap().clone())
                .collect(),
        ));
    })
}

/// TC npc_four_car_garage / source34775 spellclick66245: aura236 boards the
/// Demolisher's Vehicle509 driver seat5766 after normal Workshop capture.
fn enter_and_board_isle_demolisher() -> (EnteredMatch, Entity) {
    let template = shared::battleground::isle_of_conquest();
    let mut entered = enter_source_match_with(&template, [1299, 1245], |app| {
        let source = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let click: (u32, u8, u16) = source
            .query_row(
                "SELECT spell_id,cast_flags,user_type FROM tdb_npc_spellclick_spells WHERE npc_entry=34775",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(click, (66245, 1, 0));
        // Isolate the runtime aura boundary from the separately missing publication.
        let fixture = Connection::open_in_memory().unwrap();
        fixture.execute_batch("CREATE TABLE content_npc_spellclick_spells (npc_entry INTEGER,spell_id INTEGER,cast_flags INTEGER,user_type INTEGER)").unwrap();
        fixture
            .execute(
                "INSERT INTO content_npc_spellclick_spells VALUES (34775,?1,?2,?3)",
                rusqlite::params![click.0, click.1, click.2],
            )
            .unwrap();
        app.insert_resource(crate::npc_interaction::load_spellclick_catalog(&fixture).unwrap());
        app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        let data = crate::spell_cast::test_harness::world_db().unwrap();
        app.insert_resource(crate::spell_info::SpellStore::fixture(
            [21651, 66245]
                .into_iter()
                .map(|id| data.store.get(id).unwrap().clone())
                .collect(),
        ));
        crate::vehicle::tests::install_source_vehicle_runtime(app);
    });
    let actor = &entered.players[0];
    let h = &mut entered.h;
    use_second_frames(h);
    let (banner, position) = objects_in(h, entered.copy, 195133)[0];
    h.walk_to(actor, position);
    use_object(h, actor, banner);
    h.advance(8);
    h.advance(60);
    let (demolisher, _) = find_alterac_source_npc(h, entered.copy, 34775);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFactionTemplate>(demolisher)
            .unwrap()
            .0,
        1732,
        "TC OnCreatureCreate must apply this Alliance spawn's authored StringId faction"
    );
    let position = wow(h.app.world().get::<Position>(demolisher).unwrap());
    h.walk_to(actor, position);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: demolisher.to_bits(),
        });
    h.receive_pvp_updates(5);
    let source_aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(demolisher)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 66245 && aura.caster == actor.player)
        })
        .expect("the actual spellclick must apply the rider's source aura to the vehicle");
    assert!(
        source_aura
            .effects
            .iter()
            .any(|effect| effect.aura_type == 236)
    );
    let rider = h
        .app
        .world()
        .get::<shared::components::VehiclePassenger>(actor.player)
        .expect("normal authored Demolisher spellclick must board its driver seat");
    assert_eq!(rider.driver, demolisher.to_bits());
    assert_eq!(rider.seat_index, 0);
    assert_eq!(rider.seat_id, 5766);
    (entered, demolisher)
}

#[test]
fn isle_channel_workshop_demolisher_driver_seat_exits_and_leaves_through_real_requests() {
    let (mut entered, demolisher) = enter_and_board_isle_demolisher();
    let actor = &entered.players[0];
    let h = &mut entered.h;
    let position = wow(h.app.world().get::<Position>(demolisher).unwrap());
    assert!(
        h.app
            .world()
            .get::<MovementControl>(actor.player)
            .unwrap()
            .controlled
    );
    let vehicle_position = h.app.world().get::<Position>(demolisher).unwrap();
    let rider_position = h.app.world().get::<Position>(actor.player).unwrap();
    let yaw = h.app.world().get::<Rotation>(demolisher).unwrap().y;
    let relative = Quat::from_rotation_y(-yaw)
        * Vec3::new(
            rider_position.x - vehicle_position.x,
            rider_position.y - vehicle_position.y,
            rider_position.z - vehicle_position.z,
        );
    // Source123367 M2 attachment39=(-3.84265995,0,6.19728518), DB2scale0.75;
    // VehicleSeat5766 attachment13 maps to39 with zero offset, model aligns -90deg.
    assert!(
        relative.distance(Vec3::new(0.0, 4.647964, -2.881995)) < 0.002,
        "{relative:?}"
    );
    h.app
        .world_mut()
        .entity_mut(actor.connection)
        .insert(pair::<shared::protocol::ExitVehicle>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::ExitVehicle>>(actor.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::ExitVehicle);
    h.receive_pvp_updates(5);
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(actor.player)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(actor.player)
            .unwrap()
            .controlled
    );
    assert!(!crate::auras::has_aura(h.app.world(), demolisher, 66245));

    // Reentry must reuse the freed driver seat, not a later passenger seat.
    h.walk_to(actor, position);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: demolisher.to_bits(),
        });
    h.receive_pvp_updates(5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(actor.player)
            .unwrap()
            .seat_index,
        0
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(actor.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground {
            match_id: entered.match_id,
        });
    h.receive_pvp_updates(5);
    h.ack(actor);
    h.receive_pvp_updates(5);
    assert_eq!(h.map_of(actor), Some(STORMWIND));
    assert!(close(h.position(actor), entered.origin));
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(actor.player)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(actor.player)
            .unwrap()
            .controlled
    );
    assert!(!crate::auras::has_aura(h.app.world(), demolisher, 66245));
}

#[test]
fn isle_channel_workshop_demolisher_seats_keep_independent_riders_and_refuse_overfill() {
    let (mut entered, demolisher) = enter_and_board_isle_demolisher();
    let team = entered.teams[0];
    let allies: Vec<_> = entered
        .teams
        .iter()
        .enumerate()
        .filter(|(index, candidate)| *index != 0 && **candidate == team)
        .take(3)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(allies.len(), 3);
    let driver = &entered.players[0];
    let h = &mut entered.h;
    let position = wow(h.app.world().get::<Position>(demolisher).unwrap());
    for (index, expected_seat) in [(allies[0], 2), (allies[1], 3)] {
        let rider = &entered.players[index];
        h.walk_to(rider, position);
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::InteractNpc>>(rider.connection)
            .unwrap()
            .send::<InteractionChannel>(shared::protocol::InteractNpc {
                npc: demolisher.to_bits(),
            });
        h.receive_pvp_updates(5);
        let seated = h
            .app
            .world()
            .get::<shared::components::VehiclePassenger>(rider.player)
            .expect("another friendly rider must occupy the next source seat");
        assert_eq!(seated.driver, demolisher.to_bits());
        assert_eq!(seated.seat_index, expected_seat);
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::VehiclePassenger>(driver.player)
                .unwrap()
                .seat_index,
            0
        );
    }
    let overflow = &entered.players[allies[2]];
    h.walk_to(overflow, position);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(overflow.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: demolisher.to_bits(),
        });
    h.receive_pvp_updates(5);
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(overflow.player)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(overflow.player)
            .unwrap()
            .controlled
    );
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(demolisher)
        .unwrap();
    assert_eq!(
        auras.iter().filter(|aura| aura.spell_id == 66245).count(),
        3
    );
    assert!(
        !auras
            .iter()
            .any(|aura| aura.spell_id == 66245 && aura.caster == overflow.player)
    );

    let leaving = &entered.players[allies[0]];
    h.app
        .world_mut()
        .entity_mut(leaving.connection)
        .insert(pair::<shared::protocol::ExitVehicle>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::ExitVehicle>>(leaving.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::ExitVehicle);
    h.receive_pvp_updates(5);
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(leaving.player)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(driver.player)
            .unwrap()
            .seat_index,
        0
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(overflow.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: demolisher.to_bits(),
        });
    h.receive_pvp_updates(5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(overflow.player)
            .unwrap()
            .seat_index,
        2
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(entered.players[allies[1]].player)
            .unwrap()
            .seat_index,
        3
    );
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(demolisher)
        .unwrap();
    assert_eq!(
        auras.iter().filter(|aura| aura.spell_id == 66245).count(),
        3
    );
    assert!(
        !auras
            .iter()
            .any(|aura| aura.spell_id == 66245 && aura.caster == leaving.player)
    );
}

#[test]
fn isle_channel_workshop_demolisher_death_releases_its_living_driver() {
    let (mut entered, demolisher) = enter_and_board_isle_demolisher();
    let enemy_index = entered
        .teams
        .iter()
        .position(|team| *team != entered.teams[0])
        .unwrap();
    let actor = &entered.players[0];
    let enemy = &entered.players[enemy_index];
    let h = &mut entered.h;
    let health = h.app.world().get::<Health>(demolisher).unwrap().current;
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy.player, demolisher, health + 1.0);
    h.advance(2);
    assert_eq!(
        h.app.world().get::<Health>(demolisher).unwrap().current,
        0.0
    );
    assert!(h.app.world().get::<Health>(actor.player).unwrap().current > 0.0);
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(actor.player)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(actor.player)
            .unwrap()
            .controlled
    );
    assert!(!crate::auras::has_aura(h.app.world(), demolisher, 66245));
}

#[test]
fn isle_channel_workshop_demolisher_driver_death_releases_the_living_vehicle() {
    let (mut entered, demolisher) = enter_and_board_isle_demolisher();
    let enemy_index = entered
        .teams
        .iter()
        .position(|team| *team != entered.teams[0])
        .unwrap();
    let actor = &entered.players[0];
    let enemy = &entered.players[enemy_index];
    let h = &mut entered.h;
    let health = h.app.world().get::<Health>(actor.player).unwrap().current;
    let vehicle_health = h.app.world().get::<Health>(demolisher).unwrap().current;
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy.player, actor.player, health + 1.0);
    h.advance(2);
    assert_eq!(
        h.app.world().get::<Health>(actor.player).unwrap().current,
        0.0
    );
    assert_eq!(
        h.app.world().get::<Health>(demolisher).unwrap().current,
        vehicle_health
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::VehiclePassenger>(actor.player)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(actor.player)
            .unwrap()
            .controlled
    );
    assert!(!crate::auras::has_aura(h.app.world(), demolisher, 66245));
}

/// TC a352b1fa GameObject::Use goober Data10 spell (GameObject.cpp:2930-2932, 3563-3567):
/// a captured Workshop's TDB group1227/1230 Seaforium Bombs195237 cast66265, which creates
/// item46847 for the user in that battleground copy only.
#[test]
fn isle_channel_normal_seaforium_pickup_creates_owned_item_only_in_its_copy() {
    let mut entered = enter_isle_bomb_match();
    let actor = &entered.players[0];
    use_second_frames(&mut entered.h);
    // TDB groups1227/1230 expose195237 only after normal Workshop capture.
    // Template195232 has no authored628 spawn in this source database.
    let (banner, position) = objects_in(&mut entered.h, entered.copy, 195133)[0];
    entered.h.walk_to(actor, position);
    use_object(&mut entered.h, actor, banner);
    entered.h.advance(8);
    entered.h.advance(60);
    let (pickup, position) = objects_in(&mut entered.h, entered.copy, 195237)[0];
    entered.h.walk_to(actor, position);
    let count = |h: &Harness| {
        let db = &h
            .app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let character_id = h.app.world().get::<ClientId>(actor.player).unwrap().0;
        crate::persistence::load_inventory(db, character_id)
            .items
            .iter()
            .filter(|item| item.item_id == 46847)
            .map(|item| item.stack_count)
            .sum::<u32>()
    };
    assert_eq!(count(&entered.h), 0);
    let foreign_copy = InMap {
        map_id: entered.copy.map_id,
        instance_id: entered.copy.instance_id + 1,
    };
    entered
        .h
        .app
        .world_mut()
        .entity_mut(actor.player)
        .insert(foreign_copy);
    use_object(&mut entered.h, actor, pickup);
    assert_eq!(
        count(&entered.h),
        0,
        "another copy cannot pick up the source bomb"
    );
    entered
        .h
        .app
        .world_mut()
        .entity_mut(actor.player)
        .insert(entered.copy);
    use_object(&mut entered.h, actor, pickup);
    assert_eq!(
        count(&entered.h),
        1,
        "normal authored goober use must create item46847"
    );
}

/// TC EffectTransmitted50: normal item46847 use casts66268 and owns trap195235.
#[test]
fn isle_channel_normal_seaforium_use_arms_then_detonates_once_in_its_copy() {
    assert_isle_bomb_lifecycle(false, false);
}

#[test]
fn isle_channel_normal_seaforium_disarm_cancels_the_owned_fuse_only_in_its_copy() {
    assert_isle_bomb_lifecycle(false, true);
}

#[test]
fn isle_channel_huge_seaforium_use_arms_then_detonates_once_in_its_copy() {
    assert_isle_bomb_lifecycle(true, false);
}

#[test]
fn isle_channel_huge_seaforium_disarm_cancels_the_owned_fuse_only_in_its_copy() {
    assert_isle_bomb_lifecycle(true, true);
}

fn assert_isle_bomb_disarm(
    h: &mut Harness,
    copy: InMap,
    bomb: (u32, Entity),
    victim: (&Traveler, f32),
    foreign: (&Traveler, f32),
) {
    let (bomb_entry, object) = bomb;
    let (victim, before) = victim;
    let (foreign, foreign_before) = foreign;
    for user in [foreign, victim] {
        h.app
            .world_mut()
            .get_mut::<MessageSender<UseGameObject>>(user.connection)
            .unwrap()
            .send::<InteractionChannel>(UseGameObject {
                object: object.to_bits(),
            });
        let mut errors = Vec::new();
        for _ in 0..3 {
            h.advance(1);
            errors.extend(
                h.app
                    .world_mut()
                    .get_mut::<MessageReceiver<shared::protocol::InteractionFailed>>(
                        user.connection,
                    )
                    .unwrap()
                    .receive(),
            );
        }
        if user.player == foreign.player {
            assert_eq!(
                errors.len(),
                1,
                "the foreign request must actually be processed before the control is accepted"
            );
            assert_eq!(
                errors[0].error,
                shared::protocol::InteractionError::InvalidTarget
            );
            assert_eq!(
                objects_in(h, copy, bomb_entry).len(),
                1,
                "another copy cannot disarm the bomb"
            );
        } else {
            assert!(
                errors.is_empty(),
                "normal in-copy disarm rejected: {errors:?}"
            );
        }
    }
    assert!(
        objects_in(h, copy, bomb_entry).is_empty(),
        "normal in-copy bomb use must run source disarm during the arming period"
    );
    h.advance(12);
    assert_eq!(
        h.app.world().get::<Health>(victim.player).unwrap().current,
        before,
        "disarm cancels the pending explosion, not only its model"
    );
    assert_eq!(
        h.app.world().get::<Health>(foreign.player).unwrap().current,
        foreign_before
    );
}

fn assert_isle_bomb_lifecycle(huge: bool, disarm: bool) {
    let mut entered = enter_isle_bomb_match();
    let actor = &entered.players[0];
    let owner_team = entered.teams[0];
    let (item_id, pickup_entry, bomb_entry, deployment_spell, damage_percent) = if huge {
        (
            47030,
            if owner_team == BgTeam::A {
                195332
            } else {
                195333
            },
            195331,
            66674,
            60.0,
        )
    } else {
        (46847, 195237, 195235, 66268, 25.0)
    };
    let mut opponents = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter_map(|(player, team)| (*team != owner_team).then_some(player));
    let victim = opponents.next().unwrap();
    let foreign = opponents.next().unwrap();
    use_second_frames(&mut entered.h);
    let (banner, position) = objects_in(&mut entered.h, entered.copy, 195133)[0];
    entered.h.walk_to(actor, position);
    use_object(&mut entered.h, actor, banner);
    entered.h.advance(8);
    entered.h.advance(60);
    let (pickup, position) = objects_in(&mut entered.h, entered.copy, pickup_entry)[0];
    entered.h.walk_to(actor, position);
    use_object(&mut entered.h, actor, pickup);
    let h = &mut entered.h;
    // Both targets stand inside the source blast radius before the item cast begins.
    h.walk_to(victim, position);
    h.walk_to(foreign, position);
    h.app.world_mut().entity_mut(foreign.player).insert(InMap {
        map_id: entered.copy.map_id,
        instance_id: entered.copy.instance_id + 1,
    });
    let inventory = crate::inventory::load_model(
        &h.app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        1,
    )
    .unwrap()
    .inventory_snapshot();
    let location = inventory
        .bags
        .iter()
        .find_map(|bag| {
            bag.items
                .iter()
                .find(|slot| slot.item.item_id == item_id)
                .map(|slot| ItemLocation::Bag {
                    bag: bag.bag,
                    slot: slot.slot,
                })
        })
        .expect("authored pile pickup creates the owned bomb used below");
    h.app.world_mut().entity_mut(actor.connection).insert((
        pair::<UseItem>(),
        pair::<InventoryError>(),
        pair::<CastFailed>(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseItem>>(actor.connection)
        .unwrap()
        .send::<InventoryChannel>(UseItem {
            location,
            target: None,
        });
    let before = h.app.world().get::<Health>(victim.player).unwrap().current;
    let foreign_before = h.app.world().get::<Health>(foreign.player).unwrap().current;
    let mut inventory_errors = Vec::new();
    let mut cast_errors = Vec::new();
    for _ in 0..5 {
        h.advance(1);
        inventory_errors.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<InventoryError>>(actor.connection)
                .unwrap()
                .receive(),
        );
        cast_errors.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<CastFailed>>(actor.connection)
                .unwrap()
                .receive(),
        );
        if !objects_in(h, entered.copy, bomb_entry).is_empty() {
            break;
        }
    }
    assert!(
        inventory_errors.is_empty(),
        "normal item use: {inventory_errors:?}"
    );
    assert!(
        cast_errors.is_empty(),
        "normal deployment cast: {cast_errors:?}"
    );
    let remaining = crate::persistence::load_inventory(
        &h.app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db,
        1,
    )
    .items
    .iter()
    .filter(|item| item.item_id == item_id)
    .map(|item| item.stack_count)
    .sum::<u32>();
    assert_eq!(
        remaining, 0,
        "the authored expendable item is consumed by normal use"
    );
    let bombs = objects_in(h, entered.copy, bomb_entry);
    assert_eq!(
        bombs.len(),
        1,
        "normal owned item use must deploy one authored bomb"
    );
    let summoned = h
        .app
        .world()
        .get::<crate::game_objects::summoned::SummonedObject>(bombs[0].0)
        .unwrap();
    assert_eq!(summoned.owner, Some(actor.player));
    assert_eq!(summoned.spell_id, deployment_spell);
    assert_eq!(
        h.app.world().get::<Health>(victim.player).unwrap().current,
        before,
        "planting does not detonate immediately"
    );
    if disarm {
        assert_isle_bomb_disarm(
            h,
            entered.copy,
            (bomb_entry, bombs[0].0),
            (victim, before),
            (foreign, foreign_before),
        );
        return;
    }
    h.advance(9);
    assert_eq!(
        objects_in(h, entered.copy, bomb_entry).len(),
        1,
        "no detonation before ten seconds"
    );
    assert_eq!(
        h.app.world().get::<Health>(victim.player).unwrap().current,
        before
    );
    h.advance(1);
    assert!(
        objects_in(h, entered.copy, bomb_entry).is_empty(),
        "authored bomb deactivates after detonation"
    );
    // Native effect165 is25%/60% max health; its0.001 variance rounds at100HP.
    let after = h.app.world().get::<Health>(victim.player).unwrap().current;
    assert_eq!(
        after,
        before * (100.0 - damage_percent) / 100.0,
        "normal timed source explosion damages the actual enemy"
    );
    assert_eq!(
        h.app.world().get::<Health>(foreign.player).unwrap().current,
        foreign_before,
        "another map copy is untouched"
    );
    h.advance(5);
    assert_eq!(
        h.app.world().get::<Health>(victim.player).unwrap().current,
        after,
        "explosion happens once"
    );
}

/// Isle of Conquest (TC a352b1fa battleground_isle_of_conquest.cpp, TDB spawn groups
/// 1201-1239): a normal Opening assaults the neutral Quarry, its contested banner captures
/// one minute later with the Alliance miners, owned Quarry ticks one reinforcement per
/// 45 s and each death costs one. The keep bosses only spawn once a keep gate is breached
/// (groups 1238/1239, world states 4320-4325), so with intact gates the Horde loses by
/// running out of its 400 reinforcements.
#[test]
fn isle_channel_source_demolition_breaches_gate_and_releases_keep_banner() {
    let template = shared::battleground::isle_of_conquest();
    let mut entered = enter_source_match_with(&template, [1299, 1245], |app| {
        let data = crate::spell_cast::test_harness::world_db().unwrap();
        app.insert_resource(crate::spell_info::SpellStore::fixture(
            [21651, 66672]
                .into_iter()
                .map(|id| data.store.get(id).unwrap().clone())
                .collect(),
        ));
    });
    let actor = entered
        .players
        .iter()
        .zip(&entered.teams)
        .find(|(_, team)| **team == BgTeam::A)
        .unwrap()
        .0;
    use_second_frames(&mut entered.h);
    let gate = objects_in(&mut entered.h, entered.copy, 195494)[0].0;
    let position = *entered.h.app.world().get::<Position>(gate).unwrap();
    // The normal bomb callback is a triggered cast from its physical GO position.
    // This test covers that landing -> destruction boundary, not item activation.
    let bomb = entered
        .h
        .app
        .world_mut()
        .spawn((
            position,
            entered.copy,
            shared::protocol::GameObjectInfo {
                entry: 195331,
                go_type: 6,
                display_id: 9061,
                name: "Huge Seaforium Bomb".into(),
                scale: 1.0,
            },
        ))
        .id();
    for _ in 0..100 {
        crate::spell_cast::cast_triggered(entered.h.app.world_mut(), bomb, 66672, None, 0);
    }
    assert!(
        entered
            .h
            .app
            .world()
            .get::<shared::protocol::GameObjectCollision>(gate)
            .unwrap()
            .0
    );
    assert!(
        battleground(&entered.h, actor)
            .world_states
            .contains(&(4317, 1))
    );
    crate::spell_cast::cast_triggered(entered.h.app.world_mut(), bomb, 66672, None, 0);
    entered.h.receive_pvp_updates(5);
    assert!(
        !entered
            .h
            .app
            .world()
            .get::<shared::protocol::GameObjectCollision>(gate)
            .unwrap()
            .0
    );
    assert!(
        battleground(&entered.h, actor)
            .world_states
            .contains(&(4317, 0))
    );
    assert!(
        battleground(&entered.h, actor)
            .world_states
            .contains(&(4322, 1))
    );
    assert_eq!(battleground(&entered.h, actor).score_a, 400);
    assert_eq!(battleground(&entered.h, actor).score_b, 400);
    let door = objects_in(&mut entered.h, entered.copy, 195223)[0].0;
    assert!(
        !entered
            .h
            .app
            .world()
            .get::<shared::protocol::GameObjectCollision>(door)
            .unwrap()
            .0
    );
    let (banner, at) = objects_in(&mut entered.h, entered.copy, 195393)[0];
    entered.h.walk_to(actor, at);
    use_object(&mut entered.h, actor, banner);
    entered.h.advance(8);
    entered.h.receive_pvp_updates(5);
    assert!(
        battleground(&entered.h, actor)
            .world_states
            .contains(&(4347, 1))
    );
    assert_team_honor(&entered, 0, 0);
    let (commander, health) = find_alterac_source_npc(&mut entered.h, entered.copy, 34922);
    crate::combat::deal_direct_damage(
        entered.h.app.world_mut(),
        actor.player,
        commander,
        health + 1.0,
    );
    entered.h.receive_pvp_updates(5);
    assert_eq!(battleground(&entered.h, actor).phase, BgPhase::Ended);
    // TC OnUnitKilled pays a literal500, not a bracket-scaled kill equivalent.
    assert_team_honor(&entered, 500, 0);
    crate::combat::deal_direct_damage(
        entered.h.app.world_mut(),
        actor.player,
        commander,
        health + 1.0,
    );
    entered.h.receive_pvp_updates(5);
    assert_team_honor(&entered, 500, 0);
    let world = entered.h.app.world();
    let character = world.get::<ClientId>(actor.player).unwrap().0;
    let database = world.resource::<crate::persistence::GameDatabase>();
    assert_eq!(
        crate::persistence::load_currencies(&database.db, character).get(1792),
        500
    );
    assert_eq!(
        crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
        500
    );
}

#[test]
fn isle_channel_resource_honor_follows_owned_nodes_contests_and_both_teams() {
    let template = shared::battleground::isle_of_conquest();
    let mut entered = enter_source_match(&template, [1299, 1245]);
    let alliance: Vec<_> = entered
        .teams
        .iter()
        .enumerate()
        .filter_map(|(index, team)| (*team == BgTeam::A).then_some(index))
        .collect();
    let horde = entered
        .teams
        .iter()
        .position(|team| *team == BgTeam::B)
        .unwrap();
    let copy = entered.copy;
    use_second_frames(&mut entered.h);
    assert_team_honor(&entered, 0, 0);
    // Two independent, normal eight-second Openings, then the source60s capture clocks.
    for (index, entry) in [(alliance[0], 195343), (alliance[1], 195338)] {
        let (banner, at) = objects_in(&mut entered.h, copy, entry)[0];
        entered.h.walk_to(&entered.players[index], at);
        use_object(&mut entered.h, &entered.players[index], banner);
    }
    entered.h.advance(68);
    assert_eq!(
        objects_in(&mut entered.h, copy, 195339).len(),
        1,
        "Alliance Refinery"
    );
    assert_eq!(
        objects_in(&mut entered.h, copy, 195334).len(),
        1,
        "Alliance Quarry"
    );
    let score = battleground(&entered.h, &entered.players[alliance[0]]).score_a;
    let before_a = read_honor_wallet(&entered.h, entered.players[alliance[0]].player);
    let before_b = read_honor_wallet(&entered.h, entered.players[horde].player);
    entered.h.advance(45);
    assert_eq!(
        battleground(&entered.h, &entered.players[alliance[0]]).score_a,
        score + 2
    );
    assert_team_honor(&entered, before_a + 24, before_b);

    let (refinery, at) = objects_in(&mut entered.h, copy, 195339)[0];
    entered.h.walk_to(&entered.players[horde], at);
    use_object(&mut entered.h, &entered.players[horde], refinery);
    entered.h.advance(8);
    assert_eq!(
        objects_in(&mut entered.h, copy, 195342).len(),
        1,
        "Horde contested Refinery"
    );
    let score = battleground(&entered.h, &entered.players[alliance[0]]).score_a;
    let before_a = read_honor_wallet(&entered.h, entered.players[alliance[0]].player);
    let before_b = read_honor_wallet(&entered.h, entered.players[horde].player);
    entered.h.advance(45);
    assert_eq!(
        battleground(&entered.h, &entered.players[alliance[0]]).score_a,
        score + 1
    );
    assert_team_honor(&entered, before_a + 12, before_b);

    entered.h.advance(15);
    assert_eq!(
        objects_in(&mut entered.h, copy, 195341).len(),
        1,
        "Horde Refinery"
    );
    let snapshot = battleground(&entered.h, &entered.players[alliance[0]]);
    let scores = [snapshot.score_a, snapshot.score_b];
    let before_a = read_honor_wallet(&entered.h, entered.players[alliance[0]].player);
    let before_b = read_honor_wallet(&entered.h, entered.players[horde].player);
    entered.h.advance(45);
    let snapshot = battleground(&entered.h, &entered.players[alliance[0]]);
    assert_eq!(
        [snapshot.score_a, snapshot.score_b],
        [scores[0] + 1, scores[1] + 1]
    );
    assert_team_honor(&entered, before_a + 12, before_b + 12);
    let world = entered.h.app.world();
    let database = world.resource::<crate::persistence::GameDatabase>();
    for index in [alliance[0], horde] {
        let player = entered.players[index].player;
        let character = world.get::<ClientId>(player).unwrap().0;
        let amount = read_honor_wallet(&entered.h, player);
        let saved = crate::persistence::load_currencies(&database.db, character);
        assert_eq!(saved.get(1792), amount);
        assert_eq!(saved.week_earned(1792), 0);
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            u64::from(amount)
        );
    }
}

fn read_honor_wallet(h: &Harness, player: Entity) -> u32 {
    h.app
        .world()
        .get::<crate::currency::PlayerCurrencies>(player)
        .unwrap()
        .0
        .get(1792)
}

#[test]
fn isle_channel_quarry_capture_tick_gated_bosses_reinforcement_victory_and_origin_return() {
    let template = shared::battleground::isle_of_conquest();
    let mut entered = enter_source_match(&template, [1299, 1245]);
    let actor_index = entered
        .teams
        .iter()
        .position(|team| *team == BgTeam::A)
        .unwrap();
    let victim_index = entered
        .teams
        .iter()
        .position(|team| *team == BgTeam::B)
        .unwrap();
    {
        let EnteredMatch {
            h, players, copy, ..
        } = &mut entered;
        let (actor, victim, copy) = (&players[actor_index], &players[victim_index], *copy);
        use_second_frames(h);
        assert_eq!(battleground(h, actor).score_a, 400);
        assert_eq!(
            objects_in(h, copy, 195338).len(),
            1,
            "neutral Quarry banner"
        );
        assert!(
            objects_in(h, copy, 195334).is_empty(),
            "Alliance Quarry banner"
        );
        assert_eq!(objects_in(h, copy, 195396).len(), 1, "Alliance Keep banner");
        assert_eq!(creatures_in(h, copy, 61696), 0, "Alliance Quarry miners");
        assert_eq!(
            creatures_in(h, copy, 34922),
            0,
            "Overlord Agmar behind intact gates"
        );
        assert_eq!(
            creatures_in(h, copy, 34924),
            0,
            "High Commander Halford behind intact gates"
        );
        let (banner, at) = objects_in(h, copy, 195338)[0];
        h.walk_to(actor, at);
        use_object(h, actor, banner);
        assert!(
            objects_in(h, copy, 195335).is_empty(),
            "Opening is not instant"
        );
        h.advance(8);
        assert!(objects_in(h, copy, 195338).is_empty());
        assert_eq!(
            objects_in(h, copy, 195335).len(),
            1,
            "Alliance contested Quarry"
        );
        assert!(battleground(h, actor).world_states.contains(&(4310, 1)));
        h.advance(60);
        assert_eq!(
            objects_in(h, copy, 195334).len(),
            1,
            "Alliance Quarry banner"
        );
        assert_eq!(creatures_in(h, copy, 61696), 13, "Alliance Quarry miners");
        h.advance(45);
        assert_eq!(battleground(h, actor).score_a, 401);
        assert!(battleground(h, actor).world_states.contains(&(4226, 401)));
        crate::combat::deal_direct_damage(h.app.world_mut(), actor.player, victim.player, 1.0e7);
        h.advance(2);
        assert_eq!(battleground(h, actor).score_b, 399);
        for _ in 1..400 {
            crate::death::resurrect_player(h.app.world_mut(), victim.player);
            crate::combat::deal_direct_damage(
                h.app.world_mut(),
                actor.player,
                victim.player,
                1.0e7,
            );
            h.advance(1);
        }
        h.receive_pvp_updates(5);
        assert_eq!(battleground(h, actor).score_b, 0);
        let row = battleground(h, actor)
            .players
            .iter()
            .find(|row| row.character_id == h.app.world().get::<ClientId>(actor.player).unwrap().0)
            .unwrap()
            .clone();
        assert!(row.objective_scores.contains(&(245, 1)));
    }
    let score = battleground(&mut entered.h, &entered.players[actor_index]).score_a;
    assert!(score > 401, "the owned Quarry kept ticking: {score}");
    win_leave_and_return(&mut entered, score);
}

fn creatures_in(h: &mut Harness, copy: InMap, entry: u32) -> usize {
    let world = h.app.world_mut();
    let mut query = world.query::<(&shared::components::Npc, &InMap)>();
    query
        .iter(world)
        .filter(|(npc, map)| npc.template_id == entry && **map == copy)
        .count()
}

fn kill_alterac_source_npc(h: &mut Harness, actor: &Traveler, copy: InMap, entry: u32) {
    let (victim, health) = find_alterac_source_npc(h, copy, entry);
    crate::combat::deal_direct_damage(h.app.world_mut(), actor.player, victim, health + 1.0);
}

fn find_alterac_source_npc(h: &mut Harness, copy: InMap, entry: u32) -> (Entity, f32) {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Health)>();
    query
        .iter(world)
        .find(|(_, npc, map, health)| {
            npc.template_id == entry && **map == copy && health.current > 0.0
        })
        .map(|(entity, _, _, health)| (entity, health.current))
        .expect("authored living objective NPC")
}

#[test]
fn deephaul_channel_real_carts_crystal_drop_capture_victory_and_origin_return() {
    let template = shared::battleground::deephaul_ravine();
    let mut entered = enter_source_match(&template, shared::battleground::DEEPHAUL_STARTS);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    assert_eq!(deephaul_carts_in(h, copy).len(), 2);
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY).is_empty());
    h.advance(15);
    let actor = alliance[0];
    let (flag, home) = objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY)[0];
    h.walk_to(actor, home);
    h.app.world_mut().entity_mut(flag).insert(InMap {
        instance_id: copy.instance_id + 1,
        ..copy
    });
    use_object(h, actor, flag);
    assert!(battleground(h, actor).world_states.contains(&(1545, 1)));
    h.app.world_mut().entity_mut(flag).insert(copy);
    use_object(h, actor, flag);
    assert!(battleground(h, actor).world_states.contains(&(1545, 2)));
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let dropped = objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY);
    assert_eq!(dropped.len(), 1);
    assert!(close(dropped[0].1, home));
    assert_eq!(
        h.app
            .world()
            .get::<GameObjectInfo>(dropped[0].0)
            .unwrap()
            .go_type,
        37
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 100.0;
    let teammate = alliance[1];
    h.walk_to(teammate, dropped[0].1);
    use_object(h, teammate, dropped[0].0);
    assert!(battleground(h, teammate).world_states.contains(&(1545, 2)));
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).is_empty());
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(teammate.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    assert_eq!(
        objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).len(),
        1
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(teammate.player)
        .unwrap()
        .current = 100.0;
    h.advance(10);
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).is_empty());
    let (flag, home) = objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY)[0];
    h.walk_to(actor, home);
    use_object(h, actor, flag);
    h.walk_to(
        actor,
        Vec3::from_array(shared::battleground::DEEPHAUL_CAPTURE_TRIGGERS[0]),
    );
    h.advance(1);
    assert!(
        battleground(h, actor)
            .players
            .iter()
            .find(|row| row.character_id == 1)
            .unwrap()
            .objective_scores
            .contains(&(1020, 1))
    );
    h.advance(30);
    assert_eq!(
        objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY).len(),
        1
    );
    let mut delivered = false;
    let mut respawned = false;
    for _ in 0..2000 {
        let carts = deephaul_carts_in(h, copy);
        for (index, who) in alliance.iter().enumerate() {
            if let Some((_, at)) = carts.get(index % 2) {
                h.app.world_mut().entity_mut(who.player).insert(bevy(*at));
            }
        }
        h.advance(1);
        let count = deephaul_carts_in(h, copy).len();
        delivered |= count == 0;
        respawned |= delivered && count == 2;
        if battleground(h, actor).phase == BgPhase::Ended {
            break;
        }
    }
    assert!(
        delivered && respawned,
        "source paired delivery and two-second replacement"
    );
    assert!(battleground(h, actor).players.iter().any(|row| {
        row.team == BgTeam::A
            && row
                .objective_scores
                .iter()
                .any(|(stat, value)| *stat == 1021 && *value > 0)
    }));
    win_leave_and_return(&mut entered, 1500);
}

fn deephaul_carts_in(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, npc, map, _)| [214690, 217346].contains(&npc.template_id) && **map == copy)
        .map(|(entity, _, _, at)| (entity, wow(at)))
        .collect()
}

#[test]
fn seething_channel_authored_entry_then_ground_gathering_cancellation_victory_and_return() {
    let template = shared::battleground::seething_shore();
    let mut entered = enter_source_match(&template, shared::battleground::SEETHING_STARTS);
    let actor = &entered.players[0];
    let h = &mut entered.h;
    use_second_frames(h);
    h.advance(42);
    let nodes = seething_objects(h, entered.copy);
    assert_eq!(nodes.len(), 3);
    let (object, at) = nodes[0];
    // Fixture travel to the actual fissure uses normal near-teleport cleanup, not
    // a position overwrite that leaves the player attached to its moving ship.
    let orientation = h
        .app
        .world()
        .get::<shared::components::Rotation>(actor.player)
        .unwrap()
        .y
        - std::f32::consts::FRAC_PI_2;
    assert!(crate::maps::teleport_to(
        h.app.world_mut(),
        actor.player,
        crate::maps::WorldLocation {
            map_id: 1803,
            position: at,
            orientation
        },
        crate::maps::TeleportOptions::default()
    ));
    h.advance(2);
    assert_eq!(
        h.map_of(actor),
        Some(entered.copy),
        "same-map travel retains the reserved battleground copy"
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::TransportPassenger>(actor.player)
            .is_none()
    );
    h.app.world_mut().entity_mut(object).insert(InMap {
        instance_id: entered.copy.instance_id + 1,
        ..entered.copy
    });
    use_object(h, actor, object);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert_eq!(battleground(h, actor).score_a, 0);
    h.app.world_mut().entity_mut(object).insert(entered.copy);
    use_object(h, actor, object);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_some()
    );
    assert_eq!(battleground(h, actor).score_a, 0);
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(true));
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(false));
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 0);
    use_object(h, actor, object);
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 100);
    use_object(h, actor, object);
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 100);
    for _ in 0..20 {
        h.advance(42);
        for (object, at) in seething_objects(h, entered.copy) {
            if battleground(h, actor).phase != BgPhase::InProgress {
                break;
            }
            h.walk_to(actor, at);
            use_object(h, actor, object);
            h.advance(6);
        }
        if battleground(h, actor).phase == BgPhase::Ended {
            break;
        }
    }
    let result = battleground(h, actor);
    let own = result
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert!(own.objective_scores.contains(&(744, 15)));
    win_leave_and_return(&mut entered, 1500);
}

#[test]
fn seething_channel_authored_ship_entry_and_carry_use_transport_frames() {
    let template = shared::battleground::seething_shore();
    let mut entered = enter_source_match(&template, shared::battleground::SEETHING_STARTS);
    let world = entered.h.app.world_mut();
    let mut objects = world.query::<(&shared::protocol::GameObjectInfo, &InMap)>();
    let ships = objects
        .iter(world)
        .filter(|(info, map)| **map == entered.copy && info.go_type == 15)
        .count();
    assert_eq!(
        ships, 2,
        "both authored airships exist only in their match copy"
    );
    let before: Vec<_> = entered
        .players
        .iter()
        .map(|player| {
            let at = *world.get::<Position>(player.player).unwrap();
            assert!(
                at.y > 290.0,
                "source ship deck arrival, not a ground substitute: {at:?}"
            );
            let passenger = *world
                .get::<shared::components::TransportPassenger>(player.player)
                .unwrap();
            (at, passenger)
        })
        .collect();
    use_second_frames(&mut entered.h);
    entered.h.advance(30);
    for (index, player) in entered.players.iter().enumerate() {
        let world = entered.h.app.world();
        let at = world.get::<Position>(player.player).unwrap();
        let passenger = world
            .get::<shared::components::TransportPassenger>(player.player)
            .unwrap();
        assert_eq!(passenger.transport, before[index].1.transport);
        for axis in 0..3 {
            assert!(
                (passenger.position[axis] - before[index].1.position[axis]).abs() < 0.002,
                "ship-local pose preserves feet within world-space float precision: {passenger:?}"
            );
        }
        assert!((passenger.orientation - before[index].1.orientation).abs() < 0.001);
        let distance = Vec3::new(
            at.x - before[index].0.x,
            at.y - before[index].0.y,
            at.z - before[index].0.z,
        )
        .length();
        assert!(
            distance > 5.0,
            "source ship carries idle player: {distance}"
        );
        let ship = world
            .get::<shared::components::TransportState>(Entity::from_bits(passenger.transport))
            .unwrap();
        let expected = if entered.teams[index] == BgTeam::A {
            35
        } else {
            36
        };
        assert_eq!(ship.spawn_id, expected);
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(player.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: entered.match_id,
            });
    }
    entered.h.receive_pvp_updates(5);
    for player in &entered.players {
        assert!(
            entered
                .h
                .app
                .world()
                .get::<shared::components::TransportPassenger>(player.player)
                .is_none()
        );
        assert_eq!(entered.h.new_worlds(player).len(), 1);
        entered.h.ack(player);
        assert_eq!(entered.h.map_of(player), Some(STORMWIND));
        assert!(close(entered.h.position(player), entered.origin));
    }
}

#[test]
fn seething_channel_refuses_missing_authored_transport_catalog() {
    let template = shared::battleground::seething_shore();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(crate::faction::FactionTemplates::default());
        app.insert_resource(crate::game_objects::GameObjectSpawns::default());
        app.add_plugins(crate::pvp::PvpPlugin);
    })
    .unwrap();
    let players: Vec<_> = (1..=10)
        .map(|id| h.spawn(id, STORMWIND, STOCKADE_ENTRANCE_TRIGGER))
        .collect();
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            shared::components::Level(40),
            shared::components::CurrentPvpFaction(2),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<PvpStateUpdate>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: template.id,
            });
    }
    h.receive_pvp_updates(5);
    let who = players
        .iter()
        .find(|who| battleground(&h, who).team == BgTeam::B)
        .unwrap();
    let match_id = battleground(&h, who).match_id;
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RespondToBattlegroundInvite {
            match_id,
            accepted: true,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates[&who.connection].iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("transport 36"))),
        "{:#?}",
        updates[&who.connection]
    );
    assert!(h.new_worlds(who).is_empty());
    assert_eq!(h.map_of(who), Some(STORMWIND));
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CurrentPvpFaction>(who.player),
        Some(&shared::components::CurrentPvpFaction(2)),
        "refused Horde entry retains racial Alliance"
    );
}

fn seething_objects(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    shared::battleground::SEETHING_OBJECTS
        .into_iter()
        .flat_map(|entry| objects_in(h, copy, entry))
        .collect()
}

/// Source-minimum roster queue, invitation, acceptance and safe-location entry over the channel.
fn enter_source_match(template: &BgTemplate, starts: [u32; 2]) -> EnteredMatch {
    enter_source_match_with(template, starts, |_| {})
}

fn enter_source_match_with(
    template: &BgTemplate,
    starts: [u32; 2],
    setup: impl FnOnce(&mut App),
) -> EnteredMatch {
    let objects = if template.map_id == 998 {
        source_kotmogu_objects()
    } else if matches!(template.map_id, 727 | 1803) {
        crate::game_objects::GameObjectSpawns::default()
    } else {
        source_battleground_objects(template.map_id)
    };
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(objects);
        if template.map_id == 727 {
            let (world_data, spellclick) = silvershard_source_world();
            app.insert_resource(world_data);
            app.insert_resource(spellclick);
            let data = crate::spell_cast::test_harness::world_db().unwrap();
            app.insert_resource(crate::spell_info::SpellStore::fixture(
                [124491, 124492, 97372]
                    .into_iter()
                    .map(|id| data.store.get(id).unwrap().clone())
                    .collect(),
            ));
            app.insert_resource(crate::spell_cast::test_harness::power_table(data));
            app.add_plugins((crate::spell_cast::SpellCastPlugin, crate::auras::AuraPlugin));
        }
        if template.map_id == 1803 {
            let (world_data, templates) = seething_source_world();
            app.insert_resource(crate::transport_catalog::fixture_catalog());
            app.insert_resource(world_data);
            app.insert_resource(templates);
            app.insert_resource(crate::ground::GroundMap::new(
                &std::env::var_os("GAME_SERVER_GROUND_DIR")
                    .map(std::path::PathBuf::from)
                    .expect("source Seething terrain root"),
                std::collections::HashMap::from([(1803, 1686724)]),
            ));
            let data = crate::spell_cast::test_harness::world_db().unwrap();
            app.insert_resource(crate::spell_info::SpellStore::fixture(vec![
                data.store.get(261857).unwrap().clone(),
            ]));
            app.insert_resource(crate::spell_cast::test_harness::power_table(data));
            app.add_plugins((crate::spell_cast::SpellCastPlugin, crate::auras::AuraPlugin));
        }
        if matches!(template.map_id, 30 | 628) {
            app.insert_resource(promoted_source_world(template.map_id));
            if template.map_id == 628 {
                app.insert_resource(
                    crate::pvp::isle_siege::Catalog::load(
                        &crate::pvp::isle_siege::tests::source_catalog(),
                    )
                    .unwrap(),
                );
            }
            let data = crate::spell_cast::test_harness::world_db().unwrap();
            app.insert_resource(crate::spell_info::SpellStore::fixture(vec![
                data.store.get(21651).unwrap().clone(),
            ]));
            app.insert_resource(crate::spell_cast::test_harness::power_table(data));
            app.add_plugins((
                crate::spell_cast::SpellCastPlugin,
                crate::auras::AuraPlugin,
                crate::death::DeathPlugin,
            ));
        }
        if template.map_id == 2656 {
            let (world_data, templates) = deephaul_source_world();
            app.insert_resource(world_data);
            app.insert_resource(templates);
        }
        app.insert_resource(crate::faction::FactionTemplates::default());
        app.add_plugins(crate::pvp::PvpPlugin);
        // Match production ordering: PvP registers before target-mirror systems.
        setup(app);
        app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
    })
    .unwrap();
    let origin = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let count = u64::from(template.min_players_per_team) * 2;
    // BattlemasterList's broad admission minimum need not have a PVPDifficulty row
    // (Deephaul allows19–90 there, but its first authored match bracket is80).
    let level = (template.bracket.min_level..=template.bracket.max_level)
        .find(|&level| shared::battleground::source_level_bracket(template.map_id, level).is_some())
        .expect("source fixture needs an authored PVPDifficulty level");
    let players: Vec<_> = (1..=count)
        .map(|id| h.spawn(id, STORMWIND, origin))
        .collect();
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            crate::currency::PlayerCurrencies::default(),
            shared::components::Level(level),
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            shared::components::CurrentPvpFaction(2),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<LeaveBattleground>(),
            pair::<UseGameObject>(),
            pair::<shared::protocol::InteractNpc>(),
            pair::<shared::protocol::InteractionFailed>(),
            pair::<PvpStateUpdate>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: template.id,
            });
    }
    h.receive_pvp_updates(5);
    let invitations: Vec<_> = players
        .iter()
        .map(|who| battleground(&h, who).clone())
        .collect();
    let match_id = invitations[0].match_id;
    for (who, invitation) in players.iter().zip(&invitations) {
        assert_eq!(invitation.match_id, match_id);
        assert_eq!(invitation.map_id, template.map_id);
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::CurrentPvpFaction>(who.player),
            Some(&shared::components::CurrentPvpFaction(2)),
            "invitation is not entry"
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: true,
            });
    }
    let updates = h.receive_pvp_updates(5);
    for (who, invitation) in players.iter().zip(&invitations) {
        assert!(
            updates[&who.connection]
                .iter()
                .all(|update| update.error.is_none()),
            "{:?}",
            updates[&who.connection]
        );
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1);
        let start = starts[if invitation.team == BgTeam::A { 0 } else { 1 }];
        let store = h.app.world().resource::<MapStore>();
        let mut source = store.safe_locs[&start];
        if let Some(spawn_id) = store.safe_loc_transports.get(&start) {
            let ships = h
                .app
                .world()
                .resource::<crate::transport_catalog::TransportCatalog>();
            let world_pose = ships.0[spawn_id].route.pose_at(0).local_to_world(
                shared::transport_route::TransportPose {
                    position: source.position.to_array(),
                    orientation: source.orientation,
                },
            );
            source.position = Vec3::from_array(world_pose.position);
        }
        assert_eq!(destinations[0].map_id, source.map_id);
        let position = destinations[0].position;
        assert!(close(
            wow(&Position {
                x: position[0],
                y: position[1],
                z: position[2]
            }),
            source.position
        ));
        h.ack(who);
    }
    h.receive_pvp_updates(5);
    let copy = h.map_of(&players[0]).unwrap();
    assert!(players.iter().all(|who| h.map_of(who) == Some(copy)));
    EnteredMatch {
        h,
        teams: invitations
            .iter()
            .map(|invitation| invitation.team)
            .collect(),
        players,
        match_id,
        copy,
        origin,
    }
}

fn battleground<'a>(h: &'a Harness, who: &Traveler) -> &'a BattlegroundSnapshot {
    h.app
        .world()
        .get::<crate::pvp::PlayerPvpState>(who.player)
        .unwrap()
        .battleground
        .as_ref()
        .expect("authoritative battleground state")
}

/// One-second frames so source heartbeats, ticks and capture clocks run on whole seconds.
fn use_second_frames(h: &mut Harness) {
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(1));
    h.app
        .world_mut()
        .resource_mut::<Time<Fixed>>()
        .set_timestep(Duration::from_secs(1));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(1),
        ));
}

fn objects_in(h: &mut Harness, copy: InMap, entry: u32) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &GameObjectInfo, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, info, map, _)| info.entry == entry && **map == copy)
        .map(|(entity, _, _, position)| (entity, wow(position)))
        .collect()
}

fn use_object(h: &mut Harness, who: &Traveler, object: Entity) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseGameObject>>(who.connection)
        .unwrap()
        .send::<InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
    h.receive_pvp_updates(5);
}

/// Every player ends on Team A's victory, leaves, returns to its origin and the copy unloads.
fn win_leave_and_return(entered: &mut EnteredMatch, score: u32) {
    let EnteredMatch {
        h, players, copy, ..
    } = entered;
    for who in players.iter() {
        let result = battleground(h, who);
        assert_eq!(result.phase, BgPhase::Ended);
        assert_eq!(result.outcome, Some(BgOutcome::Winner(BgTeam::A)));
        assert_eq!(result.score_a, score);
        assert_eq!(result.honor_reward, None);
        assert_eq!(result.players.len(), players.len());
        let score_state = match copy.map_id {
            30 => 3127,
            628 => 4226,
            998 => 6303,
            727 => 6437,
            1803 => 13845,
            2656 => 24958,
            _ => 1776,
        };
        assert!(result.world_states.contains(&(score_state, score as i32)));
    }
    leave_and_return(entered);
}

fn leave_and_return(entered: &mut EnteredMatch) {
    let EnteredMatch {
        h,
        players,
        match_id,
        copy,
        origin,
        ..
    } = entered;
    for who in players.iter() {
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: *match_id,
            });
    }
    h.receive_pvp_updates(5);
    for who in players.iter() {
        let returns = h.new_worlds(who);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].map_id, 0);
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), *origin));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
    assert!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(*copy)
            .is_none()
    );
}

fn node_match(template: BgTemplate) {
    let rules = node_rules(template.map_id).unwrap();
    let source_points = source_battleground_objects(template.map_id);
    assert_eq!(source_points.0.len(), rules.nodes.len());
    let mut entered = enter_source_match(&template, rules.starts);
    let (h, copy) = (&mut entered.h, entered.copy);
    let actor = &entered.players[0];
    for point in source_points.0 {
        let (object, _) = objects_in(h, copy, point.info.entry)[0];
        h.walk_to(actor, point.position);
        use_object(h, actor, object);
    }
    let before = battleground(h, actor);
    assert_eq!(before.phase, BgPhase::InProgress);
    assert_eq!(before.score_a, 0, "assaults are not instant captures");
    assert!(before.world_states.contains(&(rules.nodes[0].poi, 3)));
    let score = before
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert!(
        score
            .objective_scores
            .contains(&(rules.assault_stat, rules.nodes.len() as u32))
    );
    assert!(score.objective_scores.contains(&(rules.defense_stat, 0)));
    use_second_frames(h);
    h.advance(120);
    if template.map_id == 2107 {
        // Source max19 gives30: nine resource thresholds270 + winner/end60, other end30.
        assert_team_honor(&entered, 330, 30);
    } else if template.map_id == 761 {
        assert_team_honor(&entered, 0, 0); // Gilneas does not inherit Arathi's source producer.
    }
    win_leave_and_return(&mut entered, 1500);
}

fn assert_team_honor(entered: &EnteredMatch, team_a: u32, team_b: u32) {
    let world = entered.h.app.world();
    for (who, team) in entered.players.iter().zip(&entered.teams) {
        let expected = if *team == BgTeam::A { team_a } else { team_b };
        assert_eq!(
            world
                .get::<crate::currency::PlayerCurrencies>(who.player)
                .unwrap()
                .0
                .get(1792),
            expected
        );
        let progress = &world
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap()
            .progress;
        assert_eq!(
            (progress.honor, progress.lifetime_honor_xp),
            (expected, u64::from(expected))
        );
    }
}

#[test]
fn arathi_channel_resource_honor_keeps_team_remainders_through_contests() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match(&template, node_rules(template.map_id).unwrap().starts);
    use_second_frames(&mut entered.h);
    let alliance_index = entered
        .teams
        .iter()
        .position(|team| *team == BgTeam::A)
        .unwrap();
    let horde_index = entered
        .teams
        .iter()
        .position(|team| *team == BgTeam::B)
        .unwrap();
    for (index, entry) in [(alliance_index, 227420), (horde_index, 227536)] {
        let actor = &entered.players[index];
        let (object, at) = objects_in(&mut entered.h, entered.copy, entry)[0];
        entered.h.walk_to(actor, at);
        use_object(&mut entered.h, actor, object);
    }
    entered.h.advance(60);
    advance_arathi_score(&mut entered, BgTeam::A, 158);
    assert_team_honor(&entered, 0, 0);
    entered.h.advance(2);
    assert_team_honor(&entered, 30, 0);
    advance_arathi_score(&mut entered, BgTeam::B, 160);
    assert_team_honor(&entered, 30, 30);
    let alliance = &entered.players[alliance_index];
    let horde = &entered.players[horde_index];
    let (farm, at) = objects_in(&mut entered.h, entered.copy, 227536)[0];
    entered.h.walk_to(alliance, at);
    use_object(&mut entered.h, alliance, farm);
    let paused = battleground(&entered.h, horde).score_b;
    entered.h.advance(20);
    assert_eq!(battleground(&entered.h, horde).score_b, paused);
    assert_team_honor(&entered, 30, 30);
    entered.h.walk_to(horde, at);
    use_object(&mut entered.h, horde, farm);
    advance_arathi_score(&mut entered, BgTeam::B, 318);
    assert_team_honor(&entered, 60, 30);
    entered.h.advance(2);
    assert_team_honor(&entered, 60, 60);
    for actor in &entered.players {
        let world = entered.h.app.world();
        let character = world.get::<ClientId>(actor.player).unwrap().0;
        let database = world.resource::<crate::persistence::GameDatabase>();
        assert_eq!(
            crate::persistence::load_currencies(&database.db, character).get(1792),
            60
        );
        assert_eq!(
            crate::persistence::load_pvp(&database.db, character).lifetime_honor_xp,
            60
        );
    }
    leave_and_return(&mut entered);
}

fn advance_arathi_score(entered: &mut EnteredMatch, team: BgTeam, target: u32) {
    for _ in 0..400 {
        let snapshot = battleground(&entered.h, &entered.players[0]);
        let score = if team == BgTeam::A {
            snapshot.score_a
        } else {
            snapshot.score_b
        };
        if score >= target {
            assert_eq!(score, target);
            return;
        }
        entered.h.advance(1);
    }
    panic!("Arathi source resource clock did not reach {target} for {team:?}");
}

#[test]
fn arathi_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::arathi_basin());
}
#[test]
fn gilneas_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::battle_for_gilneas());
}

fn create_node_achievement_characters(path: &std::path::Path, template: &BgTemplate) {
    let database = redb::Database::create(path).unwrap();
    crate::persistence::create_tables(&database);
    for id in 1..=u64::from(template.min_players_per_team) * 2 {
        let name = format!("Node{}", char::from(b'a' + (id - 1) as u8));
        let (created, mut data) =
            crate::persistence::create_character(&database, id, &name, 1, 1).unwrap();
        assert_eq!(created, id);
        data.level = u16::from(template.bracket.min_level);
        crate::persistence::save_character(&database, id, &data);
    }
}

fn enter_achievement_node_match(path: &std::path::Path, template: &BgTemplate) -> EnteredMatch {
    let database = redb::Database::open(path).unwrap();
    let rules = node_rules(template.map_id).unwrap();
    let mut entered = enter_source_match_with(template, rules.starts, |app| {
        app.insert_resource(crate::persistence::GameDatabase { db: database });
        app.insert_resource(crate::test_data::achievement_catalog());
        app.add_plugins(crate::achievement::AchievementPlugin);
    });
    // Authentication loads this component before gameplay (auth_character.rs468).
    for who in &entered.players {
        let world = entered.h.app.world_mut();
        let character = world.get::<ClientId>(who.player).unwrap().0;
        let state = crate::persistence::load_achievements(
            &world.resource::<crate::persistence::GameDatabase>().db,
            character,
        );
        world.entity_mut(who.player).insert(state);
    }
    entered
}

fn probe_node_count_authority(world: &mut World, player: Entity, expected: [u8; 2]) {
    use crate::pvp::{PlayerPvpState, player_controlled_node_counts};
    assert_eq!(player_controlled_node_counts(world, player), Some(expected));
    let character = world.get::<ClientId>(player).unwrap().0;
    world.entity_mut(player).insert(ClientId(character + 1000));
    assert_eq!(
        player_controlled_node_counts(world, player),
        None,
        "unaccepted identity cannot borrow counts"
    );
    world.entity_mut(player).insert(ClientId(character));
    let map = *world.get::<InMap>(player).unwrap();
    world
        .entity_mut(player)
        .insert(InMap::continent(map.map_id));
    assert_eq!(
        player_controlled_node_counts(world, player),
        None,
        "same map ID in another copy is not the match"
    );
    world.entity_mut(player).insert(map);
    let mut snapshot = world
        .get_mut::<PlayerPvpState>(player)
        .unwrap()
        .battleground
        .take()
        .unwrap();
    assert_eq!(
        player_controlled_node_counts(world, player),
        None,
        "missing battleground context is unknown"
    );
    let match_id = snapshot.match_id;
    let states = std::mem::take(&mut snapshot.world_states);
    snapshot.match_id = u64::MAX;
    world
        .get_mut::<PlayerPvpState>(player)
        .unwrap()
        .battleground = Some(snapshot);
    assert_eq!(
        player_controlled_node_counts(world, player),
        None,
        "stale match identity is not authority"
    );
    world
        .get_mut::<PlayerPvpState>(player)
        .unwrap()
        .battleground
        .as_mut()
        .unwrap()
        .match_id = match_id;
    assert_eq!(
        player_controlled_node_counts(world, player),
        Some(expected),
        "wire-cache contents are not the native node ledger"
    );
    world
        .get_mut::<PlayerPvpState>(player)
        .unwrap()
        .battleground
        .as_mut()
        .unwrap()
        .world_states = states;
}

fn assert_node_win_progress(
    entered: &EnteredMatch,
    expected: [(u64, u64); 2],
    dates: &mut std::collections::BTreeMap<u64, i64>,
) -> Vec<(u64, u64)> {
    use crate::achievement::PlayerAchievementState;
    entered
        .players
        .iter()
        .zip(&entered.teams)
        .map(|(who, team)| {
            let world = entered.h.app.world();
            let character = world.get::<ClientId>(who.player).unwrap().0;
            let index = match team {
                BgTeam::A => 0,
                BgTeam::B => 1,
            };
            let (qualified, wins) = expected[index];
            let state = world.get::<PlayerAchievementState>(who.player).unwrap();
            assert_eq!(
                state.criteria_progress.get(&14943).copied(),
                (qualified > 0).then_some(qualified)
            );
            assert_eq!(
                state.criteria_progress.get(&16513).copied(),
                (wins > 0).then_some(wins),
                "ordinary wins still credit when ownership predicate fails"
            );
            assert!(
                !state.criteria_progress.contains_key(&14944),
                "timed/start/fail criterion stays disabled"
            );
            let account = state.account.as_ref().unwrap();
            assert_eq!(
                account.earned_ids.contains(&5253),
                qualified >= 10,
                "authored leaf15057 requires10 genuine wins"
            );
            if qualified >= 10 {
                let date = *account
                    .earned_at
                    .get(&5253)
                    .expect("native grant has a date");
                assert_eq!(
                    *dates
                        .entry(character)
                        .or_insert_with(|| world.resource::<ServerClock>().unix()),
                    date,
                    "later wins retain original award date"
                );
            } else {
                assert!(!account.earned_at.contains_key(&5253));
            }
            (character, qualified)
        })
        .collect()
}

#[test]
fn achievements_criteria_native_gilneas_full_coverage_uses_earned_node_world_state() {
    use crate::achievement::PlayerAchievementState;
    let template = shared::battleground::battle_for_gilneas();
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let required: u64 = source
        .query_row(
            "SELECT Amount FROM criteria_tree WHERE ID=15057 AND CriteriaID=14943",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(required, 10, "installed authored Full Coverage threshold");
    let ordinary: u32 = source.query_row("SELECT COUNT(*) FROM criteria WHERE ID=16513 AND Type=1 AND Asset=761 AND ModifierTreeId=0 AND StartEvent=0 AND StartTimer=0 AND FailEvent=0 AND EligibilityWorldStateID=0", [], |row| row.get(0)).unwrap();
    assert_eq!(
        ordinary, 1,
        "the control must be a sourced unconditional Gilneas win"
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    create_node_achievement_characters(file.path(), &template);
    let mut dates = std::collections::BTreeMap::new();
    // Ten source-qualified Alliance wins, one Horde win, then an unqualified two-base win.
    for round in 1_u64..=required + 2 {
        let winner = if round == required + 1 {
            BgTeam::B
        } else {
            BgTeam::A
        };
        let held = if round == required + 2 { 2 } else { 3 };
        let mut entered = enter_achievement_node_match(file.path(), &template);
        entered
            .h
            .app
            .world_mut()
            .resource_mut::<ServerClock>()
            .offset_secs = i64::try_from(round).unwrap();
        let actor_index = entered
            .teams
            .iter()
            .position(|team| *team == winner)
            .unwrap();
        let actor = &entered.players[actor_index];
        let (h, copy) = (&mut entered.h, entered.copy);
        let initial = h
            .app
            .world()
            .get::<PlayerAchievementState>(actor.player)
            .unwrap()
            .criteria_progress
            .get(&14943)
            .copied();
        for point in source_battleground_objects(template.map_id)
            .0
            .into_iter()
            .take(held)
        {
            let (object, _) = objects_in(h, copy, point.info.entry)[0];
            h.walk_to(actor, point.position);
            use_object(h, actor, object);
        }
        let before = battleground(h, actor);
        assert_eq!(before.phase, BgPhase::InProgress);
        assert_eq!(
            before.score_a + before.score_b,
            0,
            "assaults are not instant captures or wins"
        );
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(actor.player)
                .unwrap()
                .criteria_progress
                .get(&14943)
                .copied(),
            initial
        );
        use_second_frames(h);
        h.advance(if held == 2 { 620 } else { 120 });
        let result = battleground(h, actor);
        assert_eq!(result.phase, BgPhase::Ended);
        assert_eq!(result.outcome, Some(BgOutcome::Winner(winner)));
        let (score, state_id) = match winner {
            BgTeam::A => (result.score_a, 1779),
            BgTeam::B => (result.score_b, 1778),
        };
        assert_eq!(score, 1500, "source node clocks, not seeded scores");
        assert!(result.world_states.contains(&(state_id, held as i32)));
        if round == 1 {
            probe_node_count_authority(h.app.world_mut(), actor.player, [3, 0]);
        }
        let expected = [
            (
                round.min(required),
                round.min(required) + u64::from(round == required + 2),
            ),
            (u64::from(round > required), u64::from(round > required)),
        ];
        assert_node_win_progress(&entered, expected, &mut dates);
        entered.h.advance(3);
        let saved = assert_node_win_progress(&entered, expected, &mut dates);
        leave_and_return(&mut entered);
        drop(entered);
        let database = redb::Database::open(file.path()).unwrap();
        for (character, qualified) in saved {
            assert_eq!(
                crate::persistence::load_achievements(&database, character)
                    .criteria_progress
                    .get(&14943)
                    .copied(),
                (qualified > 0).then_some(qualified)
            );
        }
    }
    let template = shared::battleground::warsong_gulch();
    let rules = shared::battleground::flag_rules(template.map_id).unwrap();
    let entered = enter_source_match(&template, rules.starts);
    assert_eq!(
        crate::pvp::player_controlled_node_counts(entered.h.app.world(), entered.players[0].player),
        None,
        "an actual accepted flag match has no node-count authority"
    );
}

fn source_kotmogu_objects() -> crate::game_objects::GameObjectSpawns {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let objects = shared::battleground::KOTMOGU_ORBS.iter().map(|orb| {
        conn.query_row(
            "SELECT t.type,t.displayId,t.name,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5
             FROM tdb_gameobject_template t JOIN game_object_display_info d ON d.ID=t.displayId WHERE t.entry=?",
            [orb.entry], |row| {
                let scale = row.get::<_, f64>(3)? as f32;
                let number = |index| row.get::<_, f64>(index).map(|value| value as f32);
                Ok(crate::game_objects::GameObjectSpawn {
                    guid: 90_000_000 + orb.entry,
                    map_id: 998,
                    info: GameObjectInfo { entry: orb.entry, go_type: row.get(0)?, display_id: row.get(1)?, name: row.get(2)?, scale },
                    position: Vec3::from_array(orb.position), orientation: 0.0,
                    initial_state: 1,
                    bounds: crate::game_objects::GameObjectBounds {
                        min: Vec3::new(number(4)?,number(5)?,number(6)?),
                        max: Vec3::new(number(7)?,number(8)?,number(9)?),
                        scale, orientation: 0.0,
                    },
                    spawn_difficulties: vec![0], chair: None, quest_object: None,
                    script_name: String::new(),
                })
            },
        ).unwrap()
    }).collect();
    crate::game_objects::GameObjectSpawns(objects)
}

#[test]
fn kotmogu_channel_orbs_region_ticks_death_respawn_victory_and_origin_return() {
    let template = shared::battleground::temple_of_kotmogu();
    let mut entered = enter_source_match(&template, shared::battleground::KOTMOGU_STARTS);
    let actor = &entered.players[0];
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    let orb = &shared::battleground::KOTMOGU_ORBS[0];
    let (object, home) = objects_in(h, copy, orb.entry)[0];
    h.walk_to(actor, home);
    use_object(h, actor, object);
    let taken = battleground(h, actor);
    assert!(taken.world_states.contains(&(6964, 1)));
    assert!(objects_in(h, copy, orb.entry).is_empty());
    let row = battleground(h, actor)
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert_eq!(
        row.objective_scores.iter().find(|(stat, _)| *stat == 418),
        Some(&(418, 1))
    );
    h.walk_to(actor, Vec3::new(1783.41, 1333.56, 6.0));
    h.advance(5);
    let result = battleground(h, actor);
    assert!(result.score_a >= 6);
    assert!(result.world_states.contains(&(6303, result.score_a as i32)));
    let row = result
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert_eq!(
        row.objective_scores
            .iter()
            .find(|(stat, _)| *stat == 419)
            .unwrap()
            .1,
        result.score_a
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let returned = objects_in(h, copy, orb.entry);
    assert_eq!(returned.len(), 1);
    assert!(close(returned[0].1, home));
    assert!(battleground(h, actor).world_states.contains(&(6717, 1)));
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 100.0;
    for orb in &shared::battleground::KOTMOGU_ORBS {
        let (object, home) = objects_in(h, copy, orb.entry)[0];
        h.walk_to(actor, home);
        use_object(h, actor, object);
        // Only one orb: unavailable second pickup leaves the second orb at home.
        if orb.entry != 212094 {
            assert_eq!(objects_in(h, copy, orb.entry).len(), 1);
        }
    }
    h.walk_to(actor, Vec3::new(1783.41, 1333.56, 6.0));
    h.advance(1300);
    win_leave_and_return(&mut entered, 1500);
}

fn promoted_source_world(map_id: u32) -> crate::world_data::WorldData {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    let promoted = std::process::Command::new("python3")
        .args([
            "../../scripts/promote_battlegrounds.py",
            "--maps",
            &map_id.to_string(),
            "--db",
        ])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        promoted.status.success(),
        "{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
    crate::world_data::load_world_data(copy.path())
}

fn deephaul_source_world() -> (
    crate::world_data::WorldData,
    crate::pvp::BattlegroundObjectTemplates,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    for (script, maps) in [
        ("../../scripts/promote_battlegrounds.py", Some("2656")),
        ("../../scripts/add_battleground_summon_templates.py", None),
    ] {
        let mut command = std::process::Command::new("python3");
        command.arg(script);
        if let Some(maps) = maps {
            command.args(["--maps", maps]);
        }
        let output = command.arg("--db").arg(copy.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let conn = Connection::open_with_flags(copy.path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let templates = crate::pvp::load_battleground_objects(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), templates)
}

fn seething_source_world() -> (
    crate::world_data::WorldData,
    crate::pvp::BattlegroundObjectTemplates,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    let promoted = std::process::Command::new("python3")
        .args([
            "../../scripts/promote_battlegrounds.py",
            "--maps",
            "1803",
            "--db",
        ])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        promoted.status.success(),
        "{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
    let conn = Connection::open_with_flags(copy.path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let templates = crate::pvp::load_battleground_objects(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), templates)
}

fn silvershard_source_world() -> (
    crate::world_data::WorldData,
    crate::npc_interaction::SpellClickCatalog,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    let conn = Connection::open(copy.path()).unwrap();
    conn.execute_batch("CREATE TABLE IF NOT EXISTS tdb_npc_spellclick_spells (npc_entry INTEGER,spell_id INTEGER,cast_flags INTEGER,user_type INTEGER,PRIMARY KEY(npc_entry,spell_id));
        INSERT OR REPLACE INTO tdb_npc_spellclick_spells VALUES (60283,124491,1,0);").unwrap();
    drop(conn);
    let promoted = std::process::Command::new("python3")
        .args([
            "../../scripts/promote_battlegrounds.py",
            "--maps",
            "727",
            "--db",
        ])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        promoted.status.success(),
        "{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
    let output = std::process::Command::new("python3")
        .args(["../../scripts/add_battleground_summon_templates.py", "--db"])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let conn = Connection::open(copy.path()).unwrap();
    let catalog = crate::npc_interaction::load_spellclick_catalog(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), catalog)
}

#[test]
fn silvershard_channel_spellclick_waits_three_seconds_then_switches_only_its_copy() {
    let template = shared::battleground::silvershard_mines();
    let mut entered = enter_source_match(&template, shared::battleground::SILVERSHARD_STARTS);
    let actor = &entered.players[0];
    let h = &mut entered.h;
    // Real promoted source4000143, spawned through the production copied-map loader.
    let at = Vec3::new(715.642, 100.165, 320.284);
    let switch = {
        let world = h.app.world_mut();
        let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
        query
            .iter(world)
            .find(|(_, npc, map, position)| {
                npc.template_id == 60283 && **map == entered.copy && close(wow(position), at)
            })
            .map(|(entity, _, _, _)| entity)
            .expect("authentic eastern Track Switch")
    };
    h.walk_to(actor, at);
    // Identical geometry in another copy cannot supply a spellclick target.
    let other_copy = InMap {
        instance_id: entered.copy.instance_id + 1,
        ..entered.copy
    };
    h.app.world_mut().entity_mut(switch).insert(other_copy);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(battleground(h, actor).world_states.contains(&(6467, 2)));
    h.app.world_mut().entity_mut(switch).insert(entered.copy);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_some()
    );
    h.advance(40);
    crate::spell_cast::cancel_cast(h.app.world_mut(), actor.player);
    h.advance(24);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "cancelled cast never changes track"
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    let cast = h
        .app
        .world()
        .get::<shared::casting::CastState>(actor.player)
        .expect("source124492 cast bar");
    assert_eq!(cast.spell_id, 124492);
    assert_eq!(cast.duration, 3.0);
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "not instant"
    );
    h.advance(40);
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "still casting at2s"
    );
    h.advance(24);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 1)),
        "97372 selects and changes eastern switch"
    );
}

fn carts_in(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, npc, map, _)| npc.template_id == 60140 && **map == copy)
        .map(|(entity, _, _, position)| (entity, wow(position)))
        .collect()
}

#[test]
fn silvershard_channel_real_npc_carts_control_delivery_respawn_victory_and_origin_return() {
    let template = shared::battleground::silvershard_mines();
    let mut entered = enter_source_match(&template, shared::battleground::SILVERSHARD_STARTS);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    let initial = carts_in(h, copy);
    assert_eq!(initial.len(), 3);
    for (entity, _) in &initial {
        assert!(h.app.world().get::<Replicate>(*entity).is_some());
    }
    let mut delivered = false;
    let mut respawned = false;
    for _ in 0..3000 {
        let carts = carts_in(h, copy);
        for (who, (_, position)) in alliance.iter().zip(&carts) {
            h.app
                .world_mut()
                .entity_mut(who.player)
                .insert(bevy(*position));
        }
        h.advance(1);
        let current = carts_in(h, copy);
        delivered |= current.len() < 3;
        respawned |= delivered && current.len() == 3;
        if battleground(h, alliance[0]).phase == BgPhase::Ended {
            break;
        }
    }
    assert!(
        delivered && respawned,
        "source depot disappearance and five-second replacement"
    );
    let result = battleground(h, alliance[0]);
    assert!(
        result
            .players
            .iter()
            .filter(|row| row.team == BgTeam::A)
            .all(|row| row
                .objective_scores
                .iter()
                .any(|(id, value)| *id == 428 && *value > 0))
    );
    assert!(result.world_states.contains(&(6437, 1500)));
    win_leave_and_return(&mut entered, 1500);
}

/// Four Alliance players hold the towers by presence; the carrier scores at a held tower's
/// trigger, the flag respawns, a dead carrier drops it and it expires home before 1500 wins.
#[test]
fn eye_of_the_storm_channel_towers_flag_drop_victory_and_origin_return() {
    let template = shared::battleground::eye_of_the_storm();
    let rules = shared::battleground::eye_rules(template.map_id).unwrap();
    let mut entered = enter_source_match(&template, rules.starts);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    for (who, tower) in alliance.iter().zip(&rules.towers) {
        h.app
            .world_mut()
            .entity_mut(who.player)
            .insert(bevy(Vec3::from_array(tower.position)));
    }
    h.advance(40);
    let carrier = alliance[0];
    let held = battleground(h, carrier);
    assert!(
        held.world_states.contains(&(2752, 4)),
        "{:?}",
        held.world_states
    );
    assert!(held.world_states.contains(&(2726, 1)), "Fel Reaver held");

    let (flag, home) = objects_in(h, copy, rules.flag_entry)[0];
    h.walk_to(carrier, home);
    use_object(h, carrier, flag);
    let taken = battleground(h, carrier);
    assert!(taken.world_states.contains(&(8863, 2)) && taken.world_states.contains(&(9808, 2)));
    let before = taken.score_a;
    assert!(objects_in(h, copy, rules.flag_entry).is_empty());
    // Areatrigger spawn 38 beside Fel Reaver Ruins (TDB1210).
    h.walk_to(carrier, Vec3::new(2043.95, 1729.78, 1190.04));
    let scored = battleground(h, carrier);
    assert!(
        scored.score_a >= before + 500,
        "four bases: 500 per capture"
    );
    let row = scored.players.iter().find(|row| row.character_id == 1);
    assert_eq!(row.unwrap().objective_scores, [(183, 1)]);
    assert!(scored.world_states.contains(&(8863, 4)));
    h.advance(21);
    assert!(close(objects_in(h, copy, rules.flag_entry)[0].1, home));

    let (flag, _) = objects_in(h, copy, rules.flag_entry)[0];
    h.walk_to(carrier, home);
    use_object(h, carrier, flag);
    let fallen = home + Vec3::new(10.0, 0.0, 0.0);
    h.walk_to(carrier, fallen);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(carrier.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let dropped = objects_in(h, copy, rules.flag_entry);
    assert_eq!(dropped.len(), 1);
    assert!(close(dropped[0].1, fallen));
    let states = &battleground(h, carrier).world_states;
    assert!(states.contains(&(8863, 3)) && states.contains(&(9809, 1)));
    h.advance(10);
    assert!(close(objects_in(h, copy, rules.flag_entry)[0].1, home));
    assert!(battleground(h, carrier).world_states.contains(&(8863, 1)));

    h.advance(300);
    // Source fixture enters at20: maximum29 gives45 per bonus, including the dead carrier.
    assert_team_honor(&entered, 90, 45);
    win_leave_and_return(&mut entered, 1500);
}
