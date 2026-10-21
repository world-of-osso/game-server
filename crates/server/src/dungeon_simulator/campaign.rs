//! Full-population route controller and observable run results.
use super::{graph::Graph, runtime};
use bevy::prelude::*;
use serde::Serialize;
use shared::components::{CombatStatus, Health, Mana, Position};
use std::collections::BTreeMap;

const MAX_TICKS: usize = 72_000;
const STALL_SECONDS: f64 = 60.0;

#[derive(Component)]
pub(super) struct MoveIntent(pub Vec3);

#[derive(Serialize)]
pub(super) struct MemberSample {
    entity: u64,
    exists: bool,
    name: Option<String>,
    health: Option<f32>,
    max_health: Option<f32>,
    mana: Option<f32>,
    max_mana: Option<f32>,
    position: Option<[f32; 3]>,
}

#[derive(Serialize)]
pub(super) struct Sample {
    at: f64,
    members: Vec<MemberSample>,
}

#[derive(Serialize)]
pub(super) struct PullDecision {
    pub at: f64,
    pub pack: u32,
    pub decision: String,
}

#[derive(Serialize)]
pub(super) struct Report {
    pub seed: u64,
    pub status: String,
    pub failed_pack: Option<u32>,
    pub bosses_killed: Vec<u32>,
    pub simulated_seconds: f64,
    pub wall_seconds: f64,
    pub detail: String,
    pub samples: Vec<Sample>,
    pub pulls: Vec<PullDecision>,
    pub performance: serde_json::Value,
    pub terminal_units: serde_json::Value,
}

#[test]
#[ignore = "explicit Azure2515/205 environment qualification"]
fn azure_simulator_uses_requested_follower_copy_and_native_population() {
    let run = spawn_run(1, &mut None).unwrap();
    let world = run.app.world();
    let copy = *world.get::<crate::maps::InMap>(run.party.owner).unwrap();
    assert_eq!(
        copy.map_id, 2515,
        "requested Azure map, never Stockade substitution"
    );
    assert_eq!(
        world
            .get::<crate::maps::DungeonDifficulty>(run.party.owner)
            .unwrap()
            .0,
        205
    );
    for entry in [186644, 186739, 186737, 186738] {
        assert!(
            run.actors.values().any(|actor| world
                .get::<shared::components::Npc>(*actor)
                .is_some_and(|npc| npc.template_id == entry)),
            "native boss{entry} must spawn"
        );
    }
}

#[test]
#[ignore = "explicit Azure2515/205 environment qualification"]
fn azure_book_moves_genuine_native_party_without_resource_or_copy_changes() {
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(
        run.graph.map_id, 2515,
        "explicit Azure qualification environment"
    );
    let owner = run.party.owner;
    let book = run.actors[&29001868];
    let endpoint = *run
        .app
        .world()
        .get::<Position>(run.actors[&29001870])
        .unwrap();
    let source = *run.app.world().get::<Position>(book).unwrap();
    let members: Vec<_> = run.members().collect();
    let before: Vec<_> = members
        .iter()
        .map(|unit| {
            let world = run.app.world();
            (
                *world.get::<crate::maps::InMap>(*unit).unwrap(),
                world.get::<Health>(*unit).unwrap().current,
                world.get::<Mana>(*unit).map(|mana| mana.current),
            )
        })
        .collect();
    run.app.world_mut().entity_mut(owner).insert(source);
    assert_eq!(
        crate::npc_interaction::azure_translocation::traverse(run.app.world_mut(), owner, book),
        Ok(5)
    );
    for (unit, (copy, health, mana)) in members.into_iter().zip(before) {
        let world = run.app.world();
        assert_eq!(world.get::<Position>(unit), Some(&endpoint));
        assert_eq!(world.get::<crate::maps::InMap>(unit), Some(&copy));
        assert_eq!(world.get::<Health>(unit).unwrap().current, health);
        assert_eq!(world.get::<Mana>(unit).map(|pool| pool.current), mana);
    }
}

#[test]
#[ignore = "explicit Azure2515/205 native sight/history qualification"]
fn azure_stalker_ingress_during_authored_opening_has_native_proximity_receipts() {
    let subscriber = bevy::log::tracing_subscriber::fmt()
        .with_env_filter("game_server::creature_sight=debug,game_server::creature_aggro=debug")
        .with_test_writer()
        .finish();
    let _logger = bevy::log::tracing::subscriber::set_default(subscriber);
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    let stalker = run.actors[&29001806];
    let owner = run.party.owner;
    let geometry =
        super::diagnostics::inspect_aggro_pair(run.app.world_mut(), stalker, owner).unwrap();
    println!(
        "STALKER_INGRESS source29001806={} owner={} native_geometry={geometry}",
        stalker.to_bits(),
        owner.to_bits()
    );
    let members: Vec<_> = run.members().collect();
    for _ in 0..600 {
        run.control().unwrap();
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
                && event.source == Some(stalker.to_bits())
        }) {
            break;
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
                && event.source == Some(stalker.to_bits())
                && event
                    .target
                    .is_some_and(|target| members.iter().any(|member| member.to_bits() == target))
        ),
        "actual authored movement must reach the observed stalker damage boundary"
    );
}

#[test]
#[ignore = "explicit Azure2515/205 native history qualification"]
fn azure_unattackable_sight_source_does_not_hold_a_cleared_party_forever() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    let stalker = run.actors[&29001806];
    let owner = run.party.owner;
    let tank = run.party.tank;
    let members: Vec<_> = run.members().collect();
    let guids = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == run.graph.route[0].pack)
        .unwrap()
        .spawns
        .iter()
        .map(|spawn| spawn.guid)
        .collect();
    crate::followers::simulator::pull(run.app.world_mut(), tank, &guids).unwrap();
    crate::combat::deal_direct_damage(run.app.world_mut(), stalker, owner, 100.0);
    for member in &members {
        assert!(!crate::spell_cast::targets::relation_matches(
            run.app.world(),
            *member,
            stalker,
            crate::spell_cast::targets::Relation::Enemy
        ));
    }
    let valid = run.actors[&29001791];
    assert!(
        crate::spell_cast::targets::relation_matches(
            run.app.world(),
            tank,
            valid,
            crate::spell_cast::targets::Relation::Enemy
        ),
        "native clearable-enemy control"
    );
    crate::combat::deal_direct_damage(run.app.world_mut(), tank, valid, 100.0);
    assert!(
        crate::followers::simulator::has_uncleared_fight(run.app.world(), run.party.owner),
        "clearable enemy positive control"
    );
    crate::combat::kill_unit(run.app.world_mut(), tank, valid);
    assert!(crate::spell_cast::targets::is_alive(
        run.app.world(),
        stalker
    ));
    assert!(
        !crate::followers::simulator::has_uncleared_fight(run.app.world(), run.party.owner),
        "alive but invalid attack targets cannot keep a cleared party held forever"
    );
    let pending = run.actors[&29001801];
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), tank, pending);
    assert!(
        crate::followers::simulator::has_uncleared_fight(run.app.world(), owner),
        "valid native pre-impact combat must still hold, even before a damage receipt"
    );
}

#[test]
fn simulator_party_environment_overrides_keep_native_profiles_and_level() {
    let mut expected = [209057, 209072, 209053, 209059, 209065];
    if let Ok(entries) = std::env::var("DUNGEON_SIMULATOR_PARTY_ENTRIES") {
        expected = entries
            .split(',')
            .map(|entry| entry.trim().parse::<u32>().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .expect("five requested native profiles");
    }
    if let Ok(healer) = std::env::var("DUNGEON_SIMULATOR_HEALER_ENTRY") {
        expected[1] = healer.parse().unwrap();
    }
    let level: u8 = std::env::var("DUNGEON_SIMULATOR_LEVEL")
        .map(|value| value.parse().unwrap())
        .unwrap_or(32);
    let run = spawn_run(1, &mut None).unwrap();
    for (unit, entry) in run.party.members.iter().zip(expected[..4].iter()) {
        let follower = run
            .app
            .world()
            .get::<crate::followers::Follower>(*unit)
            .unwrap();
        assert_eq!(
            follower.profile.entry, *entry,
            "actual spawned native follower profile"
        );
    }
    let leader = run
        .app
        .world()
        .resource::<crate::followers::FollowerCatalog>()
        .profiles
        .iter()
        .find(|profile| profile.entry == expected[4])
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<crate::class_progression::ActiveSpec>(run.party.owner)
            .unwrap()
            .0,
        leader.spec_id
    );
    for unit in run.members() {
        assert_eq!(
            run.app
                .world()
                .get::<shared::components::Level>(unit)
                .unwrap()
                .0,
            level
        );
        assert_eq!(
            run.app
                .world()
                .get::<shared::components::UnitLevel>(unit)
                .unwrap()
                .0,
            level
        );
        assert!(
            run.app.world().get::<Health>(unit).unwrap().max > 0.0,
            "real native stat initialization"
        );
    }
}

#[test]
fn spawned_normal_population_allows_existing_fight_tank_movement() {
    let _rng_scope = crate::random::seed_scope(34);
    let mut run = spawn_run(34, &mut None).unwrap();
    let target = run.actors[&20375718];
    let position = *run.app.world().get::<Position>(target).unwrap();
    let tank = run.party.tank;
    let members: Vec<_> = run.members().collect();
    for unit in members {
        run.app.world_mut().entity_mut(unit).insert(Position {
            x: position.x - 2.0,
            ..position
        });
    }
    {
        let mut data = run
            .app
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        let index = data
            .spawns
            .iter()
            .position(|spawn| spawn.guid == 20375718)
            .unwrap();
        data.spawns_by_map.insert(34, vec![index]);
    }
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(Some(target)));
    crate::creature_aggro::add_threat(run.app.world_mut(), target, tank, 1.0);
    crate::followers::update_ai(run.app.world_mut());
    assert_eq!(
        crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), tank),
        Ok(Some(true)),
        "native existing-fight admission must consume the spawned Normal population without missing source metadata"
    );
}

#[test]
fn authored_route_order_does_not_change_when_the_tank_is_nearer_another_pack() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let elsewhere = *run
        .app
        .world()
        .get::<Position>(run.actors[&20375760])
        .unwrap();
    run.app
        .world_mut()
        .entity_mut(run.party.tank)
        .insert(elsewhere);
    assert_eq!(
        run.next_pack().unwrap().0,
        20375759,
        "fixed opening route must win over nearest living pack"
    );
}

#[test]
fn authored_route_skips_observed_accidental_deaths_without_reordering_the_rest() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let bytes = std::fs::read(super::graph::graph_path()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let steps = value["route"].as_array().expect("authored Stockade route");
    assert!(steps.len() >= 4);
    let ids: Vec<_> = steps
        .iter()
        .map(|step| step["pack"].as_u64().unwrap() as u32)
        .collect();
    let owner = run.party.owner;
    crate::combat::kill_unit(run.app.world_mut(), owner, run.actors[&ids[0]]);
    crate::combat::kill_unit(run.app.world_mut(), owner, run.actors[&ids[2]]);
    assert_eq!(run.next_pack().unwrap().0, ids[1]);
    crate::combat::kill_unit(run.app.world_mut(), owner, run.actors[&ids[1]]);
    assert_eq!(
        run.next_pack().unwrap().0,
        ids[3],
        "already dead accidental pack must not be pulled again"
    );
}

#[test]
fn native_hogger_bleed_after_death_does_not_reopen_a_cleared_fight() {
    let _rng_scope = crate::random::seed_scope(2);
    let mut run = spawn_run(2, &mut None).unwrap();
    let boss = run.actors[&20375720];
    for (&guid, &actor) in &run.actors {
        if guid != 20375720 {
            run.app.world_mut().despawn(actor);
        }
    }
    let at = *run.app.world().get::<Position>(boss).unwrap();
    let members: Vec<_> = run.members().collect();
    for member in members {
        run.app.world_mut().entity_mut(member).insert(Position {
            x: at.x - 2.0,
            ..at
        });
    }
    let tank = run.party.tank;
    crate::spell_cast::creature_cast(run.app.world_mut(), boss, 86604, Some(tank), true)
        .expect("native Hogger Vicious Slice");
    assert!(crate::auras::has_aura(run.app.world(), tank, 86604));
    crate::combat::kill_unit(run.app.world_mut(), tank, boss);
    assert!(!crate::followers::simulator::has_uncleared_fight(
        run.app.world(),
        run.party.owner
    ));
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let death = events
        .iter()
        .position(|event| {
            event.kind == shared::protocol::CombatLogKind::Death
                && event.target == Some(boss.to_bits())
        })
        .expect("actual boss Death");
    assert!(
        events[death + 1..].iter().any(|event| event.kind
            == shared::protocol::CombatLogKind::Damage
            && event.source == Some(boss.to_bits())
            && event.target == Some(tank.to_bits())
            && event.spell_id == Some(86604)
            && event.periodic
            && event.amount > 0),
        "real lingering bleed must still damage after its source died"
    );
    assert!(
        !crate::followers::simulator::has_uncleared_fight(run.app.world(), run.party.owner),
        "a dead boss's lingering damage must not reinsert it into observed active enemies"
    );
}

fn spawn_isolated_moloch(seed: u64) -> Run {
    let mut run = spawn_run(seed, &mut None).unwrap();
    let boss = run.actors[&20375766];
    for (&guid, &actor) in &run.actors {
        if guid != 20375766 {
            run.app.world_mut().despawn(actor);
        }
    }
    {
        let mut data = run
            .app
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        let index = data
            .spawns
            .iter()
            .position(|spawn| spawn.guid == 20375766)
            .unwrap();
        data.spawns_by_map.insert(34, vec![index]);
    }
    for _ in 0..400 {
        run.app.update();
    }
    let at = *run.app.world().get::<Position>(boss).unwrap();
    let members: Vec<_> = run.members().collect();
    for member in members {
        run.app.world_mut().entity_mut(member).insert(Position {
            x: at.x - 2.0,
            ..at
        });
    }
    run
}

#[test]
fn native_scripted_vanish_is_not_a_cleared_party_or_another_pull() {
    let _rng_scope = crate::random::seed_scope(2);
    let mut run = spawn_isolated_moloch(2);
    let boss = run.actors[&20375766];
    let max = run.app.world().get::<Health>(boss).unwrap().max;
    crate::combat::deal_direct_damage(run.app.world_mut(), run.party.tank, boss, max * 0.35);
    for _ in 0..600 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), boss, 55964) {
            // Native follower regroup calls CombatStop after detection is lost.
            // Drive that actual boundary without relying on concurrent heal-cast timing.
            let members: Vec<_> = run.members().collect();
            for member in members {
                crate::combat::combat_stop(run.app.world_mut(), member);
            }
            assert!(run.members().all(|member| {
                run.app
                    .world()
                    .get::<CombatStatus>(member)
                    .is_some_and(|status| !status.0)
            }));
            assert!(run.app.world().get::<Health>(boss).unwrap().current > 0.0);
            assert!(!crate::spell_cast::targets::relation_matches(
                run.app.world(),
                run.party.tank,
                boss,
                crate::spell_cast::targets::Relation::Enemy
            ));
            let before = run.report.pulls.len();
            run.control().unwrap();
            assert_eq!(
                run.report.pulls.len(),
                before,
                "a living observed enemy's native Vanish must not start a new-pull decision"
            );
            return;
        }
    }
    panic!("native scripted damage boundary did not execute Moloch Vanish");
}

#[test]
fn initialized_simulator_leader_recovers_native_health_after_a_wound() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let attacker = run.actors[&20375759];
    let owner = run.party.owner;
    crate::combat::deal_direct_damage(run.app.world_mut(), attacker, owner, 100.0);
    let wounded = *run.app.world().get::<Health>(owner).unwrap();
    assert!(wounded.current < wounded.max);
    crate::combat::kill_unit(run.app.world_mut(), owner, attacker);
    for _ in 0..100 {
        run.app.update();
    }
    let recovered = run.app.world().get::<Health>(owner).unwrap();
    assert_eq!(
        recovered.current, recovered.max,
        "native Player health regeneration must remain active for the initialized leader"
    );
}

#[test]
fn single_engaged_boss_never_enters_repeating_breakable_control_hold() {
    let _rng_scope = crate::random::seed_scope(4);
    let mut run = spawn_isolated_moloch(4);
    let boss = run.actors[&20375766];
    crate::combat::deal_direct_damage(run.app.world_mut(), run.party.tank, boss, 100.0);
    for _ in 0..1200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
                && event.spell_id == Some(86726)),
        "real scripted boss must create control opportunities: {:?}",
        super::diagnostics::inspect_unit(run.app.world(), Some(20375766), boss)
    );
    let sheep = events
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::CastSuccess
                && event.spell_id == Some(118)
        })
        .count();
    assert_eq!(
        sheep, 0,
        "kit control must not indefinitely suspend the sole engaged enemy and let it regenerate"
    );
}

#[test]
fn stationary_full_party_still_stalls_without_any_progress() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    run.last_progress = run.now();
    assert!(!run.stalled());
    for _ in 0..1201 {
        run.app.update();
    }
    assert!(
        run.stalled(),
        "full resources must not manufacture progress"
    );
}

#[test]
fn native_mana_recovery_is_party_progress_without_damage_or_movement() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    run.last_progress = run.now();
    let healer = run.party.members[1];
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 8936, Some(healer), false)
        .unwrap();
    for _ in 0..1201 {
        run.app.update();
        assert!(
            !run.stalled(),
            "native OOC resource recovery must refresh party progress: {}",
            run.report.detail
        );
    }
}

#[test]
fn scripted_boss_ready_single_pull_does_not_require_a_numeric_forecast() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let boss = std::collections::BTreeSet::from([20375766]);
    let decision = crate::followers::simulator::pull(run.app.world_mut(), tank, &boss);
    assert!(
        matches!(&decision, Ok((true, _))),
        "SCRIPTED_BOSS_PULL_WHEN_READY: {decision:?}"
    );
    let multiple = std::collections::BTreeSet::from([20375766, 20375765]);
    assert!(
        !crate::followers::simulator::pull(run.app.world_mut(), tank, &multiple)
            .unwrap()
            .0,
        "boss admission must not chain an add"
    );
    let healer = run.party.members[1];
    crate::combat::kill_unit(run.app.world_mut(), run.party.owner, healer);
    assert!(
        !crate::followers::simulator::pull(run.app.world_mut(), tank, &boss)
            .unwrap()
            .0,
        "dead admitted member is a real wait, not ready"
    );
}

#[test]
fn cleared_ready_party_pulls_next_single_pack_without_measurements() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let first = run.actors[&20375759];
    crate::combat::deal_direct_damage(run.app.world_mut(), tank, first, 100.0);
    crate::combat::kill_unit(run.app.world_mut(), tank, first);
    run.app.update();
    let next = std::collections::BTreeSet::from([20375765]);
    let decision = crate::followers::simulator::pull(run.app.world_mut(), tank, &next);
    assert!(
        matches!(&decision, Ok((true, _))),
        "CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK: {decision:?}"
    );
}

#[test]
fn non_tank_regroup_crosses_passive_but_not_aggressive_sight() {
    use crate::creature_ai::{CreatureState, FollowOwner};
    use crate::scripted_ai::ReactState;
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    let watcher = run.actors[&20375760];
    let healer = run.party.members[1];
    let owner = run.party.owner;
    let center = *run.app.world().get::<Position>(watcher).unwrap();
    for actor in run.actors.values() {
        run.app
            .world_mut()
            .entity_mut(*actor)
            .insert(ReactState::Passive);
    }
    run.app.world_mut().entity_mut(healer).insert(Position {
        x: center.x - 20.0,
        ..center
    });
    run.app.world_mut().entity_mut(owner).insert(Position {
        x: center.x + 20.0,
        ..center
    });
    for (react, should_follow) in [(ReactState::Aggressive, false), (ReactState::Passive, true)] {
        run.app
            .world_mut()
            .entity_mut(watcher)
            .insert((react, CreatureState::Idle));
        crate::followers::simulator::regroup(run.app.world_mut(), healer, owner);
        assert_eq!(
            run.app.world().get::<FollowOwner>(healer).is_some(),
            should_follow,
            "native20375760 {react:?}: only aggressive idle sight may block regroup"
        );
    }
}

#[test]
fn non_tank_positioning_does_not_enter_unpulled_rowdy_native_sight_radius() {
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    let watcher = run.actors[&20375760];
    let opening = run.actors[&20375759];
    let healer = run.party.members[1];
    let geometry =
        super::diagnostics::inspect_aggro_pair(run.app.world_mut(), watcher, healer).unwrap();
    let radius = geometry["npc_acquisition_radius"].as_f64().unwrap() as f32;
    assert_eq!(
        radius, 16.0,
        "native31-to32 acquisition includes Crenna's3yd reach"
    );
    for _ in 0..4000 {
        run.control().unwrap();
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Death
                && event.target == Some(opening.to_bits())
        }) {
            return;
        }
        let from = *run.app.world().get::<Position>(watcher).unwrap();
        let to = *run.app.world().get::<Position>(healer).unwrap();
        let distance = (from.x - to.x).hypot(from.z - to.z);
        assert!(
            distance >= radius,
            "non-tank healer entered unpulled20375760 native sight: {distance}<{radius} at{}s",
            run.now()
        );
        assert!(
            !run.app
                .world()
                .get::<CombatStatus>(watcher)
                .is_some_and(|status| status.0),
            "healer positioning must not recruit the unpulled watcher"
        );
    }
    panic!("opening pack must actually die, not just hold the complete party forever");
}

#[test]
fn opening_accidental_pack_is_killed_and_skipped_in_authored_route() {
    let subscriber = bevy::log::tracing_subscriber::fmt()
        .with_env_filter("game_server::creature_sight=debug,game_server::creature_aggro=debug")
        .with_test_writer()
        .finish();
    let _logger = bevy::log::tracing::subscriber::set_default(subscriber);
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    let accidental = run.actors[&20375760];
    let mut acquired = false;
    for _ in 0..4000 {
        run.control().unwrap();
        if run.report.pulls.len() == 1 && !acquired {
            // Deliberate external accidental attack still uses native threat/combat;
            // safe autonomous positioning must not erase its recovery coverage.
            crate::creature_aggro::attack_start(
                run.app.world_mut(),
                accidental,
                run.party.members[1],
            );
        }
        run.app.update();
        if !acquired
            && run
                .app
                .world()
                .get::<CombatStatus>(accidental)
                .is_some_and(|status| status.0)
        {
            acquired = true;
            assert!(
                !run.report.pulls.iter().any(|pull| pull.pack == 20375760),
                "native accidental engagement precedes any planned pull for that pack"
            );
            let healer = run.party.members[1];
            let geometry =
                super::diagnostics::inspect_aggro_pair(run.app.world_mut(), accidental, healer)
                    .expect("actual NPC-to-healer acquisition geometry");
            println!(
                "ACCIDENTAL_ACQUISITION at={} geometry={geometry}",
                run.now()
            );
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let died = events.iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Death
                && event.target == Some(accidental.to_bits())
        });
        if !died {
            continue;
        }
        assert!(acquired, "actually observe the accidental fight");
        assert!(
            events.iter().any(
                |event| event.kind == shared::protocol::CombatLogKind::Damage
                    && event.target == Some(accidental.to_bits())
                    && event.amount > event.overflow.max(0)
            ),
            "party must actually fight, not just wait for a missing actor"
        );
        assert_ne!(
            run.next_pack().unwrap().0,
            20375760,
            "an accidental pack's actual Death skips its authored route step"
        );
        assert!(
            !run.report.pulls.iter().any(|pull| pull.pack == 20375760),
            "do not record a planned pull for the already-handled accidental pack"
        );
        return;
    }
    panic!("native accidental opening pack must be fought and killed within200sim_s");
}

#[test]
fn sampling_retains_a_member_after_native_dead_summon_despawn() {
    let mut run = spawn_run(2, &mut None).unwrap();
    let member = run.party.members[1];
    crate::combat::kill_unit(run.app.world_mut(), run.party.owner, member);
    for _ in 0..1202 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(member).is_err(),
        "native corpse decay must actually despawn the dead follower"
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run.sample())).is_ok(),
        "a native dead summon is reportable, not a batch-aborting panic"
    );
    let sample = run.report.samples.last().unwrap();
    assert_eq!(sample.members.len(), 5);
    let absent = sample
        .members
        .iter()
        .find(|state| state.entity == member.to_bits())
        .unwrap();
    assert!(!absent.exists);
    assert!(absent.health.is_none());
    assert!(absent.position.is_none());
}

struct Run {
    app: App,
    graph: Graph,
    party: crate::followers::simulator::Party,
    actors: BTreeMap<u32, Entity>,
    report: Report,
    current_pack: Option<u32>,
    last_progress: f64,
    event_cursor: usize,
    last_owner_position: Vec3,
    last_resources: Vec<(Option<f32>, Option<f32>)>,
}

pub(super) fn run(seed: u64) -> Result<Report, String> {
    run_cached(seed, &mut None)
}

pub(super) fn run_cached(
    seed: u64,
    catalogs: &mut Option<super::catalogs::Catalogs>,
) -> Result<Report, String> {
    let _rng_scope = crate::random::seed_scope(seed);
    let started = std::time::Instant::now();
    let mut run = spawn_run(seed, catalogs)?;
    let setup = started.elapsed();
    let mut timings = [std::time::Duration::ZERO; 3];
    let mut ticks = 0;
    for tick in 0..MAX_TICKS {
        let observe = std::time::Instant::now();
        if tick % 10 == 0 {
            run.sample();
        }
        if run.finished() {
            break;
        }
        timings[0] += observe.elapsed();
        let control = std::time::Instant::now();
        if let Err(error) = run.control() {
            run.report.status = "blocked".into();
            run.report.detail = error;
            break;
        }
        timings[1] += control.elapsed();
        let update = std::time::Instant::now();
        run.app.update();
        timings[2] += update.elapsed();
        ticks += 1;
        if tick % 1000 == 0 {
            run.write_progress(ticks, started, setup, &timings)?;
        }
        if run.stalled() {
            break;
        }
    }
    run.sample();
    run.report.performance = run.performance(ticks, started, setup, &timings);
    run.report.performance["tank_movement_admission"] = serde_json::json!(
        crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), run.party.tank)
    );
    run.report.performance["combat_events"] = serde_json::to_value(
        &run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0,
    )
    .map_err(|error| format!("serialize native combat trace: {error}"))?;
    run.report.terminal_units = run.inspect_units();
    run.report.simulated_seconds = run.now();
    run.report.wall_seconds = started.elapsed().as_secs_f64();
    run.report.failed_pack = (run.report.status != "cleared")
        .then_some(run.current_pack)
        .flatten();
    if run.report.status == "running" {
        run.report.status = "timeout".into();
        run.report.detail = "one-hour simulated bound reached".into();
    }
    *catalogs = Some(super::catalogs::Catalogs::take(run.app.world_mut()));
    Ok(run.report)
}

fn spawn_run(seed: u64, catalogs: &mut Option<super::catalogs::Catalogs>) -> Result<Run, String> {
    let graph = super::graph::read_graph()?;
    let mut app = runtime::build_cached_app(catalogs);
    let copy = runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
    )?;
    let actors = runtime::spawn_population(&mut app, copy);
    let missing: Vec<_> = graph
        .packs
        .iter()
        .flat_map(|pack| &pack.spawns)
        .filter(|spawn| !actors.contains_key(&spawn.guid))
        .map(|spawn| spawn.guid)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "frozen graph contains sources absent from production Normal population: {missing:?}"
        ));
    }
    let last_owner_position =
        Vec3::new(graph.entrance[0], graph.entrance_height, graph.entrance[1]);
    Ok(Run {
        app,
        graph,
        party,
        actors,
        current_pack: None,
        last_progress: 0.0,
        event_cursor: 0,
        last_owner_position,
        last_resources: Vec::new(),
        report: Report {
            seed,
            status: "running".into(),
            failed_pack: None,
            bosses_killed: Vec::new(),
            simulated_seconds: 0.0,
            wall_seconds: 0.0,
            detail: String::new(),
            samples: Vec::new(),
            pulls: Vec::new(),
            performance: serde_json::Value::Null,
            terminal_units: serde_json::Value::Null,
        },
    })
}

impl Run {
    fn performance(
        &self,
        ticks: usize,
        started: std::time::Instant,
        setup: std::time::Duration,
        timings: &[std::time::Duration; 3],
    ) -> serde_json::Value {
        let elapsed = started.elapsed();
        let steady = elapsed.saturating_sub(setup).as_secs_f64();
        serde_json::json!({"ticks": ticks, "sim_seconds": self.now(),
            "setup_wall_seconds": setup.as_secs_f64(), "wall_seconds": elapsed.as_secs_f64(),
            "sim_per_wall": self.now() / elapsed.as_secs_f64(),
            "steady_sim_per_wall": self.now() / steady,
            "observation_wall_seconds": timings[0].as_secs_f64(),
            "control_wall_seconds": timings[1].as_secs_f64(),
            "update_wall_seconds": timings[2].as_secs_f64(),
            "allocated_entity_slots": self.app.world().entities().len(),
            "live_entities": self.app.world().entities().count_spawned(),
            "source_map_actors": self.actors.len(), "map_id": self.graph.map_id,
            "difficulty_id":self.graph.difficulty_id(),
            "party_levels":self.members().map(|unit|self.app.world().get::<shared::components::Level>(unit).map(|level|level.0)).collect::<Vec<_>>(),
            "observed_events": self.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len(),
            "samples": self.report.samples.len(), "memory": super::diagnostics::read_process_memory()})
    }

    fn inspect_units(&self) -> serde_json::Value {
        let units: Vec<_> = self
            .actors
            .iter()
            .map(|(guid, actor)| (Some(*guid), *actor))
            .chain(self.members().map(|actor| (None, actor)))
            .map(|(guid, actor)| super::diagnostics::inspect_unit(self.app.world(), guid, actor))
            .collect();
        serde_json::json!(units)
    }

    fn write_progress(
        &mut self,
        ticks: usize,
        started: std::time::Instant,
        setup: std::time::Duration,
        timings: &[std::time::Duration; 3],
    ) -> Result<(), String> {
        let mut performance = self.performance(ticks, started, setup, timings);
        performance["tank_movement_admission"] =
            serde_json::json!(crate::followers::simulator::inspect_tank_movement(
                self.app.world_mut(),
                self.party.tank
            ));
        println!(
            "PROGRESS seed={} ticks={} sim_s={:.2} ratio={} steady_ratio={} entities={} events={}",
            self.report.seed,
            ticks,
            self.now(),
            performance["sim_per_wall"],
            performance["steady_sim_per_wall"],
            performance["live_entities"],
            performance["observed_events"]
        );
        let directory = super::runtime::output_directory();
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = directory.join(format!("progress-{}.json", self.report.seed));
        let checkpoint = serde_json::json!({"seed": self.report.seed, "current_pack": self.current_pack,
            "performance": performance, "last_sample": self.report.samples.last(), "units": self.inspect_units()});
        std::fs::write(&path, serde_json::to_vec_pretty(&checkpoint).unwrap())
            .map_err(|error| format!("write {}: {error}", path.display()))
    }

    fn now(&self) -> f64 {
        self.app
            .world()
            .resource::<Time<Virtual>>()
            .elapsed_secs_f64()
    }

    fn members(&self) -> impl Iterator<Item = Entity> + '_ {
        self.party.members.iter().copied().chain([self.party.owner])
    }

    fn sample(&mut self) {
        let members = self
            .members()
            .map(|entity| {
                let world = self.app.world();
                let health = world.get::<Health>(entity);
                let mana = world.get::<Mana>(entity);
                let position = world.get::<Position>(entity);
                let name = world
                    .get::<shared::components::Npc>(entity)
                    .map(|npc| npc.name.clone())
                    .or_else(|| {
                        world
                            .get::<shared::components::Player>(entity)
                            .map(|player| player.name.clone())
                    });
                MemberSample {
                    entity: entity.to_bits(),
                    exists: world.get_entity(entity).is_ok(),
                    name,
                    health: health.map(|health| health.current),
                    max_health: health.map(|health| health.max),
                    mana: mana.map(|mana| mana.current),
                    max_mana: mana.map(|mana| mana.max),
                    position: position.map(|position| [position.x, position.y, position.z]),
                }
            })
            .collect();
        self.report.samples.push(Sample {
            at: self.now(),
            members,
        });
    }

    fn finished(&mut self) -> bool {
        let world = self.app.world();
        self.report.bosses_killed = self
            .graph
            .boss_entries()
            .iter()
            .copied()
            .filter(|entry| {
                self.graph
                    .packs
                    .iter()
                    .flat_map(|pack| &pack.spawns)
                    .any(|spawn| {
                        spawn.entry == *entry
                            && world
                                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                                .0
                                .iter()
                                .any(|event| {
                                    event.kind == shared::protocol::CombatLogKind::Death
                                        && event.target == Some(self.actors[&spawn.guid].to_bits())
                                })
                    })
            })
            .collect();
        if self.report.bosses_killed.len() == self.graph.boss_entries().len() {
            self.report.status = "cleared".into();
            self.report.detail = format!(
                "all{} source boss actors died",
                self.graph.boss_entries().len()
            );
            return true;
        }
        if self
            .members()
            .all(|member| !crate::spell_cast::targets::is_alive(world, member))
        {
            self.report.status = "wiped".into();
            self.report.detail = "all five bots died".into();
            return true;
        }
        false
    }

    fn control(&mut self) -> Result<(), String> {
        if crate::followers::simulator::has_uncleared_fight(self.app.world(), self.party.owner)
            || self.members().any(|member| {
                self.app
                    .world()
                    .get::<CombatStatus>(member)
                    .is_some_and(|status| status.0)
            })
        {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(());
        }
        let Some((pack_id, target)) = self.next_pack() else {
            return Err(
                "no living hostile source pack remains, but boss deaths are incomplete".into(),
            );
        };
        self.current_pack = Some(pack_id);
        let target_position = *self.app.world().get::<Position>(target).unwrap();
        if self.approach_translocation(target_position)? {
            return Ok(());
        }
        let tank_position = *self.app.world().get::<Position>(self.party.tank).ok_or(
            "admitted tank is absent; route anchor remains known, but no native opener can cast",
        )?;
        let (_, range) =
            crate::followers::simulator::opening_spell(self.app.world(), self.party.tank);
        let step = self
            .graph
            .route
            .iter()
            .find(|step| step.pack == pack_id)
            .unwrap();
        let from = Vec3::new(tank_position.x, tank_position.y, tank_position.z);
        let to = Vec3::new(target_position.x, target_position.y, target_position.z);
        if from.distance(to) > range {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(Vec3::new(
                    step.anchor[0],
                    self.graph
                        .packs
                        .iter()
                        .find(|pack| pack.id == pack_id)
                        .unwrap()
                        .height,
                    step.anchor[1],
                )));
            return Ok(());
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<MoveIntent>();
        let guids = self
            .graph
            .packs
            .iter()
            .find(|pack| pack.id == pack_id)
            .unwrap()
            .spawns
            .iter()
            .map(|spawn| spawn.guid)
            .collect();
        let (allowed, decision) = match crate::followers::simulator::pull(
            self.app.world_mut(),
            self.party.tank,
            &guids,
        ) {
            Ok(decision) => decision,
            Err(error) => {
                self.record_pull(pack_id, format!("Unavailable: {error}"));
                return Ok(());
            }
        };
        self.record_pull(pack_id, decision);
        if !allowed {
            return Ok(());
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .insert(crate::networking::CombatTarget(Some(target)));
        match crate::followers::simulator::cast_opening(
            self.app.world_mut(),
            self.party.tank,
            target,
        ) {
            Ok(()) => Ok(()),
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress,
            ) => Ok(()),
            Err(error) => Err(format!(
                "native tank opening failed on pack{pack_id}: {error:?}"
            )),
        }
    }

    fn approach_translocation(&mut self, target: Position) -> Result<bool, String> {
        if self.graph.teleports.is_empty() {
            return Ok(false);
        }
        let from = *self
            .app
            .world()
            .get::<Position>(self.party.owner)
            .ok_or("translocation reader absent")?;
        let edge = self
            .app
            .world()
            .resource::<super::Space>()
            .next_translocation(
                Vec3::new(from.x, from.y, from.z),
                Vec3::new(target.x, target.y, target.z),
            )?;
        let Some((source, destination)) = edge else {
            return Ok(false);
        };
        let book = *self
            .actors
            .get(&source)
            .ok_or("translocation source actor absent")?;
        let at = *self
            .app
            .world()
            .get::<Position>(book)
            .ok_or("translocation book lacks position")?;
        if Vec3::new(from.x, from.y, from.z).distance(Vec3::new(at.x, at.y, at.z)) > 0.1 {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(Vec3::new(at.x, at.y, at.z)));
            return Ok(true);
        }
        crate::npc_interaction::azure_translocation::traverse(
            self.app.world_mut(),
            self.party.owner,
            book,
        )?;
        super::movement::forget_party_travel(self.app.world_mut(), self.party.owner);
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<MoveIntent>();
        self.record_pull(
            source,
            format!("AZURE_TRANSLOCATION_BOOK_ENDPOINTS {source}->{destination}"),
        );
        Ok(true)
    }

    fn next_pack(&self) -> Option<(u32, Entity)> {
        let world = self.app.world();
        self.graph.route.iter().find_map(|step| {
            let pack = self.graph.packs.iter().find(|pack| pack.id == step.pack)?;
            let target = pack
                .spawns
                .iter()
                .map(|spawn| self.actors[&spawn.guid])
                .find(|actor| {
                    crate::spell_cast::targets::is_alive(world, *actor)
                        && world
                            .get::<crate::creature_aggro::AggroRange>(*actor)
                            .is_some()
                        && crate::spell_cast::targets::is_hostile(world, self.party.owner, *actor)
                })?;
            Some((pack.id, target))
        })
    }

    fn record_pull(&mut self, pack: u32, decision: String) {
        if self
            .report
            .pulls
            .last()
            .is_some_and(|last| last.pack == pack && last.decision == decision)
        {
            return;
        }
        self.report.pulls.push(PullDecision {
            at: self.now(),
            pack,
            decision,
        });
    }

    fn observe_resource_gain(&mut self) -> bool {
        let world = self.app.world();
        let current: Vec<_> = self
            .members()
            .map(|member| {
                (
                    world.get::<Health>(member).map(|health| health.current),
                    world.get::<Mana>(member).map(|mana| mana.current),
                )
            })
            .collect();
        let increased = |before: Option<f32>, after: Option<f32>| {
            before
                .zip(after)
                .is_some_and(|(before, after)| after > before)
        };
        let gained = self
            .last_resources
            .iter()
            .zip(&current)
            .any(|(before, after)| increased(before.0, after.0) || increased(before.1, after.1));
        self.last_resources = current;
        gained
    }

    fn stalled(&mut self) -> bool {
        let resource_gain = self.observe_resource_gain();
        let world = self.app.world();
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let members: Vec<_> = self.members().collect();
        let damage = events[self.event_cursor..].iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > event.overflow.max(0)
                && event
                    .source
                    .and_then(Entity::try_from_bits)
                    .is_some_and(|source| {
                        members.contains(&crate::spell_cast::combat_event_owner(world, source))
                    })
        });
        self.event_cursor = events.len();
        let position = world.get::<Position>(self.party.owner).unwrap();
        let position = Vec3::new(position.x, position.y, position.z);
        if damage || position != self.last_owner_position || resource_gain {
            self.last_progress = self.now();
        }
        self.last_owner_position = position;
        if self.now() - self.last_progress < STALL_SECONDS {
            return false;
        }
        self.report.status = "stalled".into();
        self.report.detail = self.report.pulls.last().map_or_else(
            || "no party damage, actual leader movement or HP/mana gain for 60s".into(),
            |pull| {
                format!(
                    "no party damage, actual leader movement or HP/mana gain for 60s; last pull: {}",
                    pull.decision
                )
            },
        );
        true
    }
}
