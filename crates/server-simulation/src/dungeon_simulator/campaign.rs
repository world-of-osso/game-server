//! Full-population route controller and observable run results.
use super::{graph::Graph, runtime};
use bevy::prelude::*;
use serde::Serialize;
use shared::components::{CombatStatus, Health, Mana, Position};
use std::collections::BTreeMap;

pub(super) const MAX_TICKS: usize = 72_000;
pub(super) const STALL_SECONDS: f64 = 60.0;
const COHESION_CHECK_SECONDS: f64 = 1.0;
pub(super) const DAWNBREAKER_DECK_START_MS: u32 = 86_000;
pub(super) const DAWNBREAKER_COMMANDER_DEATH_MS: u32 = 162_380;
pub(super) const DAWNBREAKER_RADIANT_GRANT_MS: u32 = 166_807;
const DECK_CARRY_SECONDS: f64 =
    (DAWNBREAKER_COMMANDER_DEATH_MS - DAWNBREAKER_DECK_START_MS) as f64 / 1000.0;
const DECK_GRANT_SECONDS: f64 =
    (DAWNBREAKER_RADIANT_GRANT_MS - DAWNBREAKER_DECK_START_MS) as f64 / 1000.0;

#[path = "azure_positioning_tests.rs"]
mod azure_positioning_tests;
#[path = "azure_throughput_tests.rs"]
mod azure_throughput_tests;
#[path = "havoc_setup_tests.rs"]
mod havoc_setup_tests;
#[path = "keystone_cohesion_tests.rs"]
mod keystone_cohesion_tests;
#[path = "party_buff_tests.rs"]
mod party_buff_tests;
#[path = "positioning_tests.rs"]
mod positioning_tests;
#[path = "recovery_admission_tests.rs"]
mod recovery_admission_tests;

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
    pets: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub(super) struct PullDecision {
    pub at: f64,
    pub pack: u64,
    pub decision: String,
}

#[derive(Serialize)]
pub(super) struct Report {
    pub seed: u64,
    pub status: String,
    pub failed_pack: Option<u64>,
    pub bosses_killed: Vec<u32>,
    pub encounters_killed: usize,
    pub encounters_total: usize,
    pub deaths: usize,
    pub simulated_seconds: f64,
    pub wall_seconds: f64,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deck_population_unsourced: Option<serde_json::Value>,
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
    let enemies: Vec<_> = run
        .actors
        .values()
        .copied()
        .filter(|actor| {
            crate::spell_cast::targets::relation_matches(
                run.app.world(),
                owner,
                *actor,
                crate::spell_cast::targets::Relation::Enemy,
            )
        })
        .collect();
    for enemy in enemies {
        run.app.world_mut().despawn(enemy);
    }
    for &unit in &members {
        run.app.world_mut().entity_mut(unit).insert(source);
    }
    assert_eq!(
        crate::npc_interaction::azure_translocation::traverse(run.app.world_mut(), owner, book),
        Ok(1)
    );
    for (&unit, &(copy, health, mana)) in members.iter().zip(&before) {
        let world = run.app.world();
        assert_eq!(world.get::<crate::maps::InMap>(unit), Some(&copy));
        assert_eq!(world.get::<Health>(unit).unwrap().current, health);
        assert_eq!(world.get::<Mana>(unit).map(|pool| pool.current), mana);
    }
    for &unit in &run.party.members {
        assert_eq!(
            run.app.world().get::<Position>(unit),
            Some(&source),
            "owner interaction must not teleport followers"
        );
    }
    let mut arrived = std::collections::HashSet::new();
    for _ in 0..2000 {
        run.app.update();
        for (&unit, &(copy, _, _)) in members.iter().zip(&before) {
            let world = run.app.world();
            let position = world.get::<Position>(unit).unwrap();
            if Vec3::new(position.x, position.y, position.z)
                .distance(Vec3::new(endpoint.x, endpoint.y, endpoint.z))
                < 3.0
            {
                arrived.insert(unit);
            }
            assert_eq!(world.get::<crate::maps::InMap>(unit), Some(&copy));
        }
        if arrived.len() == members.len() {
            break;
        }
    }
    assert_eq!(
        arrived.len(),
        members.len(),
        "each follower must independently use the book"
    );
}

#[test]
#[ignore = "requires extracted Den Mythic graph and native navigation"]
fn keystone_owner_opens_first_hostile_pack() {
    use shared::protocol::CombatLogKind;
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    let owner = run.party.owner;
    let targets: std::collections::HashSet<_> =
        run.actors.values().map(|unit| unit.to_bits()).collect();
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..6000 {
        run.control().unwrap();
        run.app.update();
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        if let Some(event) = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.target.is_some_and(|target| targets.contains(&target))
            })
        {
            assert_eq!(
                event.source,
                Some(owner.to_bits()),
                "the owner must land the first hostile action, not a follower"
            );
            return;
        }
    }
    panic!("owner did not open a hostile pack");
}

#[test]
fn chain_approach_stops_where_the_walk_route_leaves_tank_range() {
    let clipped = clip_route_to_tank_range(
        Vec3::ZERO,
        Vec3::ZERO,
        10.0,
        [Vec3::new(3.0, 0.0, 4.0), Vec3::new(20.0, 0.0, 4.0)],
    );
    assert!((clipped.length() - 10.0).abs() < 0.001);
    assert!((clipped.z - 4.0).abs() < 0.001);
}

#[test]
#[ignore = "requires extracted Den Mythic graph and native navigation"]
fn den_owner_tags_each_pack_while_previous_pack_is_alive() {
    use shared::protocol::CombatLogKind;
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    let owner = run.party.owner;
    let group = run.chain_groups.as_ref().unwrap()[0].clone();
    assert!(group.len() >= 2);
    // Keep the first native target alive long enough to exercise a chained tag.
    let first_target = run.actors[&group[0]];
    let mut health = run.app.world_mut().get_mut::<Health>(first_target).unwrap();
    health.max = 20_000_000.0;
    health.current = health.max;
    let actor_packs: std::collections::HashMap<_, _> = run
        .graph
        .packs
        .iter()
        .filter(|pack| group.contains(&pack.id))
        .flat_map(|pack| {
            pack.spawns.iter().filter_map(|spawn| {
                run.actors
                    .get(&spawn.guid)
                    .map(|unit| (unit.to_bits(), pack.id))
            })
        })
        .collect();
    let mut first_hits = std::collections::HashMap::new();
    let mut proved_chain = false;
    let mut cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..10000 {
        let previous_pulls = run.report.pulls.len();
        run.control().unwrap();
        if run.report.pulls[previous_pulls..]
            .iter()
            .any(|pull| pull.decision == "Owner chain pull")
        {
            assert!(
                run.tank_holds_most_enemies(),
                "tank must hold most live enemies before a chain tag"
            );
        }
        if run
            .app
            .world()
            .get::<server_follower_apl::DirectedOwnerTarget>(owner)
            .is_some()
            && crate::followers::simulator::has_uncleared_fight(run.app.world(), owner)
        {
            assert!(
                run.tank_holds_most_enemies(),
                "the pending tag must only execute while the tank holds most of the current pull"
            );
        }
        run.app.update();
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[cursor..] {
            if event.target == Some(run.actors[&group[1]].to_bits()) {
                let owner_action =
                    event
                        .source
                        .and_then(Entity::try_from_bits)
                        .is_some_and(|source| {
                            source == owner
                                || run
                                    .app
                                    .world()
                                    .get::<shared::components::UnitSummonedBy>(source)
                                    .is_some_and(|summon| summon.0 == owner.to_bits())
                        });
                let hostile_action = event.kind == CombatLogKind::CastSuccess
                    || (event.kind == CombatLogKind::Damage && event.amount > 0);
                let attempted_chain = run
                    .report
                    .pulls
                    .iter()
                    .any(|pull| pull.pack == group[1] && pull.decision == "Owner chain pull");
                if owner_action && hostile_action && attempted_chain && !proved_chain {
                    let other_live =
                        crate::followers::simulator::active_enemies(run.app.world(), owner)
                            .into_iter()
                            .filter(|unit| Some(unit.to_bits()) != event.target)
                            .collect::<Vec<_>>();
                    assert!(
                        !other_live.is_empty(),
                        "an earlier enemy must still be alive at the chain tag: time={}, first_health={:?}",
                        run.now(),
                        run.app.world().get::<Health>(first_target)
                    );
                    proved_chain = true;
                }
            }
            if event.kind != CombatLogKind::Damage || event.amount <= 0 {
                continue;
            }
            let Some(pack) = event
                .target
                .and_then(|target| actor_packs.get(&target))
                .copied()
            else {
                continue;
            };
            if let std::collections::hash_map::Entry::Vacant(entry) = first_hits.entry(pack) {
                let source = event.source.and_then(Entity::try_from_bits);
                let source_detail = source
                    .map(|unit| super::diagnostics::inspect_unit(run.app.world(), None, unit));
                let owner_controlled = source.is_some_and(|unit| {
                    unit == owner
                        || run
                            .app
                            .world()
                            .get::<shared::components::UnitSummonedBy>(unit)
                            .is_some_and(|summon| summon.0 == owner.to_bits())
                });
                assert!(
                    owner_controlled,
                    "pack {pack} must be tagged by the owner or an owner-controlled unit: event={event:?} source={source_detail:?}"
                );
                entry.insert(event.target.unwrap());
            }
        }
        cursor = events.len();
        if crate::followers::simulator::has_uncleared_fight(run.app.world(), owner) {
            let owner_position = run.app.world().get::<Position>(owner).unwrap();
            let tank_position = run.app.world().get::<Position>(run.party.tank).unwrap();
            let distance = Vec3::new(owner_position.x, owner_position.y, owner_position.z)
                .distance(Vec3::new(tank_position.x, tank_position.y, tank_position.z));
            let (_, range) =
                crate::followers::simulator::opening_spell(run.app.world(), run.party.tank);
            assert!(
                distance <= range + 0.001,
                "chain owner separated {distance:.2} yd from the tank's {range:.2} yd range"
            );
        }
        if proved_chain && !run.chain_returning && run.chain_tag_target.is_none() {
            let world = run.app.world();
            let owner_position = world.get::<Position>(owner).unwrap();
            let tank_position = world.get::<Position>(run.party.tank).unwrap();
            let distance = Vec3::new(owner_position.x, owner_position.y, owner_position.z)
                .distance(Vec3::new(tank_position.x, tank_position.y, tank_position.z));
            let reach =
                crate::scripted_ai::model_reach::melee_range(world, owner, run.party.tank).unwrap();
            assert!(
                distance <= reach,
                "owner must return to the tank after tagging the next pack"
            );
            return;
        }
    }
    panic!("owner did not tag and return from a second live chain pack");
}

#[test]
#[ignore = "full Den keystone; requires Mythic graph and native navigation"]
fn den_each_node_first_party_damage_is_owner_controlled() {
    use shared::protocol::CombatLogKind;
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    let owner = run.party.owner;
    let members: std::collections::HashSet<_> = run.members().collect();
    let targets: std::collections::HashSet<_> =
        run.actors.values().map(|unit| unit.to_bits()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    run.last_progress = run.now();
    for _ in 0..MAX_TICKS {
        if run.finished() {
            assert!(!seen.is_empty(), "the run must observe party damage");
            return;
        }
        run.control().unwrap();
        run.app.update();
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
        let world = run.app.world();
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[cursor..] {
            if event.kind != CombatLogKind::Damage || event.amount <= 0 {
                continue;
            }
            let Some(target) = event.target.filter(|target| targets.contains(target)) else {
                continue;
            };
            let Some(source) = event.source.and_then(Entity::try_from_bits) else {
                continue;
            };
            let controller = world
                .get::<shared::components::UnitSummonedBy>(source)
                .and_then(|summon| Entity::try_from_bits(summon.0))
                .unwrap_or(source);
            if members.contains(&controller) && seen.insert(target) {
                assert_eq!(
                    controller, owner,
                    "a follower initiated damage on graph node {target}: {event:?}"
                );
            }
        }
        cursor = events.len();
        assert!(
            !run.stalled(),
            "Den stalled before every-node proof: {}",
            run.report.detail
        );
    }
    panic!("Den exceeded the simulator bound before every-node proof");
}

#[test]
#[ignore = "requires extracted Den Mythic graph and native navigation"]
fn den_objective_chooses_a_castable_npc_offering() {
    use server::test_support::objectives::InteractionStep;
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    let owner = run.party.owner;
    let boss = run.actors[&31000640];
    let apple = run.actors[&31000492];
    let at = *run.app.world().get::<Position>(apple).unwrap();
    run.app.world_mut().entity_mut(owner).insert(at);
    let step = server::test_support::objectives::next(run.app.world_mut(), owner, boss).unwrap();
    let Some(InteractionStep::Approach { target }) = step else {
        panic!("the nearest apple NPC cannot receive its gameobject-targeted spell: {step:?}");
    };
    assert_ne!(target, apple);
    let source = run
        .app
        .world()
        .get::<shared::components::Npc>(target)
        .unwrap()
        .template_id;
    assert!(matches!(source, 244941 | 245750 | 245638 | 245698));
    let position = *run.app.world().get::<Position>(target).unwrap();
    run.app.world_mut().entity_mut(owner).insert(position);
    assert!(
        matches!(server::test_support::objectives::next(run.app.world_mut(), owner, boss).unwrap(),
        Some(InteractionStep::Interact { target: chosen }) if chosen == target)
    );
    server::test_support::objectives::interact(run.app.world_mut(), owner, target)
        .expect("the chosen native NPC-targeted offering spell must start");
}

#[test]
#[ignore = "requires extracted Den Mythic graph and native navigation"]
fn den_boss_objective_waits_for_live_party_combat() {
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    for _ in 0..400 {
        run.app.update();
        let mut key = run.keystone.take().unwrap();
        key.update(run.app.world_mut()).unwrap();
        run.keystone = Some(key);
    }
    let owner = run.party.owner;
    let tank = run.party.tank;
    let enemy = run.actors[&31000519];
    let source = run.actors[&31000492];
    let position = *run.app.world().get::<Position>(source).unwrap();
    run.app.world_mut().entity_mut(owner).insert(position);
    run.app.world_mut().entity_mut(tank).insert(position);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, enemy);
    run.app.update();
    assert!(crate::followers::simulator::has_uncleared_fight(
        run.app.world(),
        owner
    ));
    run.chain_groups = Some(vec![vec![31000640]]);
    run.chain_group = 0;
    run.control()
        .expect("active combat must defer the boss objective, not fail its spellclick");
    assert_eq!(run.current_pack, Some(31000640));
    assert!(run.app.world().get::<MoveIntent>(owner).is_none());
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_none()
    );
    assert!(crate::spell_cast::targets::is_alive(run.app.world(), enemy));
}

#[test]
#[ignore = "requires extracted Den Mythic graph and native navigation"]
fn den_pyre_cohesion_waits_for_tank_to_use_transport() {
    let mut run = spawn_run_mode(
        1,
        &mut None,
        Some(super::keystone::Options {
            map_id: 2825,
            level: 2,
            specs: [66, 105, 262, 63, 253],
        }),
    )
    .unwrap();
    assert_eq!(run.graph.map_id, 2825);
    let owner = run.party.owner;
    let edge = run
        .graph
        .transports
        .iter()
        .find(|edge| edge.from == 3276416805)
        .unwrap()
        .clone();
    let pyre = run.objects[&edge.source_guid];
    let source = *run.app.world().get::<Position>(pyre).unwrap();
    let enemies: Vec<_> = run
        .actors
        .values()
        .copied()
        .filter(|actor| {
            crate::spell_cast::targets::relation_matches(
                run.app.world(),
                owner,
                *actor,
                crate::spell_cast::targets::Relation::Enemy,
            )
        })
        .collect();
    for enemy in enemies {
        run.app.world_mut().despawn(enemy);
    }
    for unit in run.members().collect::<Vec<_>>() {
        run.app.world_mut().entity_mut(unit).insert(source);
    }
    server::test_support::transport::activate(run.app.world_mut(), owner, Some(pyre), &edge)
        .unwrap();
    let destination = *run.app.world().get::<Position>(owner).unwrap();
    assert!(
        run.wait_for_tank().unwrap(),
        "leader must wait for the tank's pyre route"
    );
    for _ in 0..2000 {
        run.app.update();
        if !run.wait_for_tank().unwrap() {
            let tank = run.app.world().get::<Position>(run.party.tank).unwrap();
            assert!(
                Vec3::new(tank.x, tank.y, tank.z).distance(Vec3::new(
                    destination.x,
                    destination.y,
                    destination.z
                )) < 20.0
            );
            return;
        }
        assert_eq!(run.app.world().get::<Position>(owner), Some(&destination));
    }
    panic!("tank did not independently use the Den pyre");
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
    let chasing_party = |world: &World| {
        matches!(world.get::<crate::creature_ai::CreatureState>(stalker),
        Some(crate::creature_ai::CreatureState::Pursuing {target}) if members.iter().any(|member|member.to_bits()==*target))
    };
    for _ in 0..600 {
        run.control().unwrap();
        run.app.update();
        if chasing_party(run.app.world()) {
            break;
        }
    }
    assert!(
        chasing_party(run.app.world()),
        "authored movement must produce native pursuit of a party member"
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
fn unattackable_objective_is_approached_using_its_live_position() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let boss = run.actors[&20375720];
    let pack = run
        .graph
        .route
        .iter()
        .find(|step| step.pack == 20375720)
        .unwrap()
        .clone();
    run.graph.route = vec![pack];
    let at = *run.app.world().get::<Position>(boss).unwrap();
    let moved = Position {
        x: at.x + 2.0,
        ..at
    };
    run.app.world_mut().entity_mut(boss).insert((
        moved,
        shared::components::UnitFlags(shared::components::UnitFlags::IMMUNE_TO_PC),
    ));
    run.control().unwrap();
    let target = run
        .app
        .world()
        .get::<MoveIntent>(run.party.owner)
        .expect("walk toward gated objective");
    assert_eq!(target.0, Vec3::new(moved.x, moved.y, moved.z));
    for unit in [run.party.owner, run.party.tank] {
        run.app.world_mut().entity_mut(unit).insert(moved);
    }
    run.control().unwrap();
    run.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs(61));
    assert!(run.stalled());
    assert!(run.report.detail.contains("OBJECTIVE_NOT_ATTACKABLE"));
}

#[test]
fn off_graph_source_is_approached_at_its_anchor_when_translocations_exist() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let source = 20375720;
    let neighbour = run
        .graph
        .edges
        .iter()
        .find_map(|edge| (edge.from == source).then_some(edge.to))
        .unwrap();
    let anchor = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == neighbour)
        .unwrap()
        .point();
    run.graph
        .edges
        .retain(|edge| edge.from != source && edge.to != source);
    let ledge = {
        let pack = run
            .graph
            .packs
            .iter_mut()
            .find(|pack| pack.id == source)
            .unwrap();
        pack.position[0] += 500.0;
        pack.point()
    };
    // A self-loop enables translocation planning without lying on any shortest path.
    run.graph.teleports = vec![server::test_support::graph::Edge {
        from: neighbour,
        to: neighbour,
        yards: 0.0,
    }];
    run.app
        .world_mut()
        .insert_resource(super::Space(run.graph.clone()));
    run.graph.route = vec![server::test_support::graph::RouteStep {
        pack: source,
        anchor: anchor.to_array(),
    }];
    let boss = run.actors[&source];
    run.app.world_mut().entity_mut(boss).insert(Position {
        x: ledge.x,
        y: ledge.y,
        z: ledge.z,
    });
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        anchor
    );
}

#[test]
fn locked_gated_boss_on_an_off_graph_perch_holds_until_it_lands() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let source = 20375720;
    let landing = run
        .graph
        .edges
        .iter()
        .find_map(|edge| (edge.from == source).then_some(edge.to))
        .unwrap();
    let landing = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == landing)
        .unwrap()
        .point();
    run.graph
        .edges
        .retain(|edge| edge.from != source && edge.to != source);
    let perch = {
        let pack = run
            .graph
            .packs
            .iter_mut()
            .find(|pack| pack.id == source)
            .unwrap();
        pack.position[0] += 500.0;
        pack.point()
    };
    run.graph.teleports = vec![server::test_support::graph::Edge {
        from: source,
        to: source,
        yards: 0.0,
    }];
    run.app
        .world_mut()
        .insert_resource(super::Space(run.graph.clone()));
    run.graph.route = vec![server::test_support::graph::RouteStep {
        pack: source,
        anchor: perch.to_array(),
    }];
    // Crawth (Academy2526) stays immune on its perch until the Alpha Eagles die and it lands.
    run.graph.map_id = 2526;
    let boss = run.actors[&source];
    let unlocked = *run
        .app
        .world()
        .get::<shared::components::UnitFlags>(boss)
        .unwrap();
    run.app.world_mut().entity_mut(boss).insert((
        shared::components::Npc {
            template_id: 191736,
            name: "Crawth".into(),
        },
        Position {
            x: perch.x,
            y: perch.y,
            z: perch.z,
        },
        shared::components::UnitFlags(shared::components::UnitFlags::IMMUNE_TO_PC),
    ));
    run.control().unwrap();
    assert!(run.app.world().get::<MoveIntent>(run.party.owner).is_none());
    assert_eq!(run.waiting_objective.map(|(pack, _)| pack), Some(source));
    run.app.world_mut().entity_mut(boss).insert((
        Position {
            x: landing.x,
            y: landing.y,
            z: landing.z,
        },
        unlocked,
    ));
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        landing
    );
}

#[test]
fn reached_anchor_that_did_not_engage_gives_way_to_the_live_source() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let source = 20375720;
    let boss = run.actors[&source];
    let at = *run.app.world().get::<Position>(boss).unwrap();
    let live = Vec3::new(at.x, at.y, at.z);
    let anchor = run
        .graph
        .packs
        .iter()
        .map(|pack| pack.point())
        .find(|point| (40.0..200.0).contains(&point.distance(live)))
        .unwrap();
    run.graph.route = vec![server::test_support::graph::RouteStep {
        pack: source,
        anchor: anchor.to_array(),
    }];
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        anchor
    );
    let standing = Position {
        x: anchor.x,
        y: anchor.y,
        z: anchor.z,
    };
    for unit in [run.party.owner, run.party.tank] {
        run.app.world_mut().entity_mut(unit).insert(standing);
    }
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        live,
        "an anchor that left the source idle must not hold the party there"
    );
    let step = Position {
        x: anchor.x + 2.0,
        ..standing
    };
    run.app.world_mut().entity_mut(run.party.owner).insert(step);
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        live,
        "leaving the reached anchor must not send the party back to it"
    );
}

#[test]
fn relocated_attackable_objective_without_aggro_uses_live_position() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let boss = run.actors[&20375720];
    let step = run
        .graph
        .route
        .iter()
        .find(|step| step.pack == 20375720)
        .unwrap()
        .clone();
    run.graph.route = vec![step];
    let at = *run.app.world().get::<Position>(boss).unwrap();
    let moved = Position {
        x: at.x + 2.0,
        ..at
    };
    run.app
        .world_mut()
        .entity_mut(boss)
        .insert(moved)
        .remove::<crate::creature_aggro::AggroRange>();
    run.control().unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<MoveIntent>(run.party.owner)
            .unwrap()
            .0,
        Vec3::new(moved.x, moved.y, moved.z)
    );
    for unit in [run.party.owner, run.party.tank] {
        run.app.world_mut().entity_mut(unit).insert(moved);
    }
    run.control().unwrap();
    assert!(
        !run.report.pulls.is_empty(),
        "attackable objective must attempt native admission without AggroRange"
    );
    assert!(run.waiting_objective.is_none());
}

#[test]
#[ignore = "requires extracted Darkflame follower graph"]
fn darkflame_withheld_darkness_does_not_abort_initialization() {
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2651);
    assert!(!run.actors.contains_key(&63651001));
    let map = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.party.owner)
        .unwrap();
    assert!(crate::scripted_ai::instance_script::set_boss_state(
        run.app.world_mut(),
        map,
        0,
        crate::scripted_ai::instance_script::EncounterState::Done,
    ));
    run.refresh_population();
    let darkness = run.actors[&63651001];
    let capture = run.app.world().resource::<super::healer_intake::Capture>();
    assert_eq!(capture.sources[&darkness].0, 63651001);
    assert!(capture.source_snapshots.contains_key(&63651001));
}

#[test]
#[ignore = "requires extracted Murder Row follower graph with journal actors"]
fn completed_kystia_encounter_skips_living_nibbles() {
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2813);
    let map = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.party.owner)
        .unwrap();
    let nibbles = run.actors[&31000648];
    assert!(crate::spell_cast::targets::is_alive(
        run.app.world(),
        nibbles
    ));
    run.graph.route.retain(|step| step.pack == 31000648);
    assert_eq!(run.next_pack().unwrap().1, nibbles);
    run.app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .update_instance_lock(map, &[], 1, Some(0), 0);
    assert!(
        run.next_pack().is_none(),
        "Nibbles belongs to completed encounter 3101"
    );
}

#[test]
#[ignore = "requires extracted Uldaman follower graph"]
fn completed_dwarf_encounter_is_skipped_even_if_olaf_is_alive() {
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2451);
    let copy = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.party.owner)
        .unwrap();
    let olaf = run.actors[&29004388];
    assert!(crate::spell_cast::targets::is_alive(run.app.world(), olaf));
    run.graph.route.retain(|step| step.pack == 29004388);
    assert_eq!(run.next_pack().unwrap().1, olaf);
    run.app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .update_instance_lock(copy, &[], 1, Some(0), 0);
    assert!(
        run.next_pack().is_none(),
        "completed encounter must not revisit an immune living actor"
    );
}

#[test]
#[ignore = "requires extracted Neltharus follower graph"]
fn dead_leader_releases_at_dungeon_entrance_and_resumes_walking() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let owner = run.party.owner;
    assert_eq!(run.graph.map_id, 2519);
    let corpse = Position {
        x: -257.5076,
        y: 124.9674,
        z: 488.93933,
    };
    run.app.world_mut().entity_mut(owner).insert(corpse);
    let attacker = *run.actors.values().next().unwrap();
    crate::combat::kill_unit(run.app.world_mut(), attacker, owner);
    for _ in 0..10 {
        run.app.update();
    }
    assert!(server::test_support::player_death_recorded(
        run.app.world(),
        owner
    ));
    run.control().unwrap();
    assert!(crate::spell_cast::targets::is_alive(run.app.world(), owner));
    let released = *run.app.world().get::<Position>(owner).unwrap();
    assert_ne!(
        released, corpse,
        "native release must not teleport to the corpse"
    );
    for _ in 0..30 {
        run.control().unwrap();
        run.app.update();
    }
    let after = *run.app.world().get::<Position>(owner).unwrap();
    assert_ne!(
        after, released,
        "the driver must resume walking after release"
    );
}

#[test]
#[ignore = "requires Azure Vault keystone data and class follower gear"]
fn keystone_death_time_starts_after_native_countdown() {
    let options = super::keystone::Options {
        map_id: 2515,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let mut run = spawn_run_mode(1, &mut None, Some(options)).unwrap();
    let owner = run.party.owner;
    for _ in 0..240 {
        run.app.update();
        run.keystone
            .as_mut()
            .unwrap()
            .update(run.app.world_mut())
            .unwrap();
    }
    let attacker = *run.actors.values().next().unwrap();
    crate::combat::kill_unit(run.app.world_mut(), attacker, owner);
    run.app.update();
    run.keystone
        .as_mut()
        .unwrap()
        .update(run.app.world_mut())
        .unwrap();
    let state = run.keystone.take().unwrap();
    let record = state
        .finish(
            run.app.world(),
            1,
            options,
            "blocked".into(),
            "test".into(),
            serde_json::Value::Null,
        )
        .unwrap();
    assert_eq!(record.deaths.len(), 1);
    let death = record.deaths[0].elapsed_ms;
    assert!((1_500..3_000).contains(&death), "death elapsed {death}");
    assert!(record.outcome.elapsed_ms.unwrap().abs_diff(death) <= 1000);
}

#[test]
#[ignore = "requires Azure Vault keystone data and class follower gear"]
fn keystone_two_pulls_do_not_repeat_precombat_setup() {
    const CHILD: &str = "AZURE_TWO_PULL_FIXTURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "dungeon_simulator::campaign::keystone_two_pulls_do_not_repeat_precombat_setup",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("DUNGEON_SIMULATOR_MAP", "2515")
            .env_remove("DUNGEON_SIMULATOR_GRAPH")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "Azure fixture child failed or ran no test:\n{}\n{}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for seed in 1..=3 {
        check_azure_precombat(seed);
    }
}

fn check_azure_precombat(seed: u64) {
    let _seed = crate::random::seed_scope(seed);
    let options = super::keystone::Options {
        map_id: 2515,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let mut run = spawn_run_mode(seed, &mut None, Some(options)).unwrap();
    let owner = run.party.owner;
    let mage = run
        .party
        .members
        .iter()
        .copied()
        .find(|unit| {
            run.app
                .world()
                .get::<shared::components::ActiveSpec>(*unit)
                .is_some_and(|spec| spec.0 == 63)
        })
        .unwrap();
    // Preparation can cast while spawn_run_mode walks to the Font. Include
    // those actual events when counting once-only precombat setup.
    let cursor = 0;
    for _ in 0..2000 {
        run.control().unwrap();
        run.app.update();
        run.keystone
            .as_mut()
            .unwrap()
            .update(run.app.world_mut())
            .unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        let kills = events
            .iter()
            .filter(|event| {
                event.kind == shared::protocol::CombatLogKind::Death
                    && event.target.is_some_and(|target| {
                        run.actors.values().any(|actor| actor.to_bits() == target)
                    })
            })
            .count();
        let pulls = run
            .report
            .pulls
            .iter()
            .filter(|pull| pull.decision.starts_with("Allow"))
            .count();
        if kills >= 5 && pulls >= 5 {
            let casts = |actor: Entity, spell| {
                events
                    .iter()
                    .filter(|event| {
                        event.kind == shared::protocol::CombatLogKind::CastSuccess
                            && event.source == Some(actor.to_bits())
                            && event.spell_id == Some(spell)
                    })
                    .count()
            };
            assert_eq!(
                casts(owner, 883),
                1,
                "Call Pet must not repeat between pulls"
            );
            assert_eq!(
                casts(mage, 1459),
                1,
                "Arcane Intellect must not repeat between pulls"
            );
            assert_eq!(
                casts(run.party.tank, 465),
                1,
                "Devotion Aura must not repeat when Consecration expires"
            );
            return;
        }
    }
    panic!("five native source deaths and pulls were not observed for seed {seed}");
}

#[test]
#[ignore = "requires Azure Vault keystone data and class follower gear"]
fn keystone_driver_waits_for_countdown_before_pulling() {
    let options = super::keystone::Options {
        map_id: 2515,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let mut run = spawn_run_mode(1, &mut None, Some(options)).unwrap();
    let initial_deaths = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == shared::protocol::CombatLogKind::Death)
        .count();
    while run
        .keystone
        .as_ref()
        .unwrap()
        .waiting_for_start(run.app.world())
        .unwrap()
    {
        run.control().unwrap();
        assert!(
            run.report.pulls.is_empty(),
            "route admitted a pull during countdown"
        );
        run.app.update();
        run.keystone
            .as_mut()
            .unwrap()
            .update(run.app.world_mut())
            .unwrap();
    }
    let deaths = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == shared::protocol::CombatLogKind::Death)
        .count();
    assert_eq!(deaths, initial_deaths, "creature died during countdown");
    for _ in 0..800 {
        run.control().unwrap();
        if run
            .report
            .pulls
            .iter()
            .any(|pull| pull.decision.starts_with("Allow"))
        {
            return;
        }
        run.app.update();
        run.keystone
            .as_mut()
            .unwrap()
            .update(run.app.world_mut())
            .unwrap();
    }
    panic!("driver never admitted a pull after countdown");
}

#[test]
#[ignore = "requires Azure Vault keystone data and class follower gear"]
fn azure_rune_does_not_attack_owner_while_hostile_does() {
    let options = super::keystone::Options {
        map_id: 2515,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let mut run = spawn_run_mode(1, &mut None, Some(options)).unwrap();
    let owner = run.party.owner;
    crate::followers::simulator::stop_leader_driver(run.app.world_mut(), owner);
    let rune = run.actors[&29001892];
    // The Crystal Fury's inferred faction comes from its Crystal Thrasher formation leader.
    let hostile = run.actors[&29001878];
    let location = *run.app.world().get::<Position>(rune).unwrap();
    run.app.world_mut().entity_mut(owner).insert(location);
    run.app.world_mut().entity_mut(hostile).insert(Position {
        x: location.x + 2.0,
        ..location
    });
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        run.app.update();
        assert!(
            run.app
                .world()
                .get::<shared::components::CombatStatus>(rune)
                .is_none_or(|state| !state.0),
            "rune entered combat"
        );
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let melee = |source: Entity| {
        events.iter().any(|event| {
            matches!(
                event.kind,
                shared::protocol::CombatLogKind::Damage | shared::protocol::CombatLogKind::Miss(_)
            ) && event.source == Some(source.to_bits())
                && event.target == Some(owner.to_bits())
                && event.spell_id.is_none()
        })
    };
    assert!(!melee(rune), "rune attempted to melee the owner");
    assert!(melee(hostile), "nearby hostile did not melee the owner");
    let observed = super::diagnostics::inspect_unit(run.app.world(), Some(29001878), hostile);
    assert!(
        observed["creature_swing_range"].is_array(),
        "missing native swing range: {observed}"
    );
    assert_eq!(observed["keystone_damage_multiplier"], 1.0);
}

#[test]
#[ignore = "requires extracted Altar graph, world data and native ground"]
fn altar_automatic_drop_discards_upper_floor_travel_before_landing() {
    let options = super::keystone::Options {
        map_id: 2993,
        level: 2,
        specs: [104, 256, 254, 260, 262],
    };
    let mut run = spawn_run_mode(1, &mut None, Some(options)).unwrap();
    let owner = run.party.owner;
    crate::followers::simulator::stop_leader_driver(run.app.world_mut(), owner);
    let edge = run
        .graph
        .transports
        .iter()
        .find(|edge| edge.action == server::test_support::transport::Action::AltarDrop)
        .unwrap()
        .clone();
    let source = Vec3::from_array(edge.departure.unwrap());
    run.app.world_mut().entity_mut(owner).insert((
        Position {
            x: 1620.0,
            y: 15.6,
            z: -2127.0,
        },
        MoveIntent(source),
    ));
    let mut saw_travel = false;
    let mut saw_transport = false;
    for _ in 0..400 {
        run.app.update();
        saw_travel |= !super::movement::inspect_travel(run.app.world(), owner).is_null();
        if server::test_support::transport::in_progress(run.app.world(), owner) {
            saw_transport = true;
        } else if saw_transport {
            break;
        }
    }
    assert!(
        saw_travel && saw_transport,
        "did not walk to and enter the native drop"
    );
    assert!(run.app.world().get::<MoveIntent>(owner).is_none());
    assert!(super::movement::inspect_travel(run.app.world(), owner).is_null());
    let landing = *run.app.world().get::<Position>(owner).unwrap();
    assert!(landing.y < -60.0, "did not land: {landing:?}");
    for _ in 0..10 {
        run.app.update();
    }
    let after = *run.app.world().get::<Position>(owner).unwrap();
    assert!(
        (after.y - landing.y).abs() < 0.1,
        "stale upper-floor route moved leader: {landing:?} -> {after:?}"
    );
}

#[test]
#[ignore = "requires extracted Azure Vault follower graph and native navigation"]
fn released_leader_rejoins_party_through_azure_book() {
    let mut run = spawn_run(1, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 2515);
    let owner = run.party.owner;
    let tank = run.party.tank;
    let book = run.actors[&29001868];
    let position = *run.app.world().get::<Position>(book).unwrap();
    run.app.world_mut().entity_mut(owner).insert(position);
    let transport = run
        .graph
        .transports
        .iter()
        .find(|edge| edge.source_guid == 29001868)
        .unwrap()
        .clone();
    server::test_support::transport::activate(run.app.world_mut(), owner, Some(book), &transport)
        .unwrap();
    for member in run.party.members.clone() {
        server::test_support::transport::activate(
            run.app.world_mut(),
            member,
            Some(book),
            &transport,
        )
        .unwrap();
    }
    super::movement::forget_party_travel(run.app.world_mut(), owner);
    let cleared: Vec<_> = run
        .actors
        .values()
        .copied()
        .filter(|actor| {
            crate::spell_cast::targets::relation_matches(
                run.app.world(),
                owner,
                *actor,
                crate::spell_cast::targets::Relation::Enemy,
            )
        })
        .collect();
    for actor in cleared {
        run.app.world_mut().despawn(actor);
    }
    crate::combat::kill_unit(run.app.world_mut(), owner, owner);
    for _ in 0..10 {
        run.app.update();
    }
    let target = *run.app.world().get::<Position>(tank).unwrap();
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(MoveIntent(Vec3::new(target.x, target.y, target.z)));
    run.control().unwrap();
    assert!(crate::spell_cast::targets::is_alive(run.app.world(), owner));
    assert!(
        run.app.world().get::<MoveIntent>(owner).is_none(),
        "release must discard the pre-death movement target"
    );
    let positions_before_return: Vec<_> = run
        .party
        .members
        .iter()
        .map(|member| (*member, *run.app.world().get::<Position>(*member).unwrap()))
        .collect();
    // The normal run loop advances once immediately after release, before the next control call.
    run.app.update();
    for (member, position) in &positions_before_return {
        assert_eq!(run.app.world().get::<Position>(*member).unwrap(), position);
    }
    let mut reunited = false;
    for _ in 0..7000 {
        assert!(
            crate::spell_cast::targets::is_alive(run.app.world(), owner),
            "leader died again while returning at {:?}",
            run.app.world().get::<Position>(owner)
        );
        if !run.return_to_party().unwrap() {
            reunited = true;
            break;
        }
        run.app.update();
        for (member, position) in &positions_before_return {
            assert_eq!(
                *run.app.world().get::<Position>(*member).unwrap(),
                *position,
                "living followers must hold throughout the leader's return"
            );
        }
    }
    assert!(
        reunited,
        "released leader did not rejoin the tank: owner={:?}, tank={:?}, intent={:?}, decisions={:?}",
        run.app.world().get::<Position>(owner),
        run.app.world().get::<Position>(tank),
        run.app
            .world()
            .get::<MoveIntent>(owner)
            .map(|intent| intent.0),
        run.report
            .pulls
            .iter()
            .map(|pull| &pull.decision)
            .collect::<Vec<_>>()
    );
    let owner_position = *run.app.world().get::<Position>(owner).unwrap();
    let tank_position = *run.app.world().get::<Position>(tank).unwrap();
    assert!(
        Vec3::new(owner_position.x, owner_position.y, owner_position.z).distance(Vec3::new(
            tank_position.x,
            tank_position.y,
            tank_position.z
        )) < 40.0
    );
    assert!(
        run.report
            .pulls
            .iter()
            .any(|event| event.decision.starts_with("REGISTERED_TRANSPORT"))
    );
}

fn spawn_azure_gate_after_first_member_death() -> Run {
    let mut run = spawn_run(3, &mut None).unwrap();
    let first = run.actors[&29001816];
    let remaining = run.actors[&29001823];
    let gate = run
        .graph
        .route
        .iter()
        .position(|step| step.pack == 29001816)
        .unwrap();
    run.graph.route.drain(..gate);
    // Production admission initializes this party's run history before any Death.
    let guids = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == 29001816)
        .unwrap()
        .spawns
        .iter()
        .map(|spawn| spawn.guid)
        .collect();
    crate::followers::simulator::pull(run.app.world_mut(), run.party.tank, &guids).unwrap();
    let at = *run.app.world().get::<Position>(first).unwrap();
    let members: Vec<_> = run.members().collect();
    for member in members {
        run.app.world_mut().entity_mut(member).insert(at);
    }
    crate::combat::kill_unit(run.app.world_mut(), run.party.owner, first);
    run.app.update();
    assert!(!crate::spell_cast::targets::is_alive(
        run.app.world(),
        first
    ));
    assert!(crate::spell_cast::targets::is_alive(
        run.app.world(),
        remaining
    ));
    assert_eq!(run.next_pack().unwrap().1, remaining);
    run
}

#[test]
#[ignore = "explicit Azure70 native gate-first-member death and remaining formation approach"]
fn azure_corridor_advances_to_the_living_gate_member_after_the_first_member_dies() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_azure_gate_after_first_member_death();
    let remaining = run.actors[&29001823];
    let point = |world: &World, actor| {
        let position = world.get::<Position>(actor).unwrap();
        Vec3::new(position.x, position.y, position.z)
    };
    let target = point(run.app.world(), remaining);
    let before = point(run.app.world(), run.party.owner).distance(target);
    for _ in 0..400 {
        run.control().unwrap();
        run.app.update();
    }
    let after = point(run.app.world(), run.party.owner).distance(target);
    assert!(
        after < before - 2.0,
        "after actual first gate Death, approach the living same-formation member: before={before}, after={after}"
    );
}

#[test]
#[ignore = "explicit Azure70 native first191164 Death then living-member admission"]
fn azure_remaining_formation_member_reaches_native_admission_after_leader_death() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_azure_gate_after_first_member_death();
    let remaining = run.actors[&29001823];
    let at = *run.app.world().get::<Position>(remaining).unwrap();
    let members: Vec<_> = run.members().collect();
    for member in members {
        run.app.world_mut().entity_mut(member).insert(at);
    }
    run.control().unwrap();
    let pull = run
        .report
        .pulls
        .last()
        .expect("native surviving-member admission decision");
    assert_eq!(
        pull.pack, 29001816,
        "source formation identity remains unchanged"
    );
    assert!(
        !pull.decision.starts_with("Unavailable:"),
        "native leader Death must not leave a missing projected GUID in the living member's pull: {}",
        pull.decision
    );
    assert!(crate::spell_cast::targets::is_alive(
        run.app.world(),
        remaining
    ));
}

#[test]
#[ignore = "explicit Stockade32 Holy Priest roster (DUNGEON_SIMULATOR_HEALER_SPEC=257); opening200sim_s native movement reproduction"]
fn stockade_holy_healer_leaves_the_entrance_with_the_native_party() {
    let _rng_scope = crate::random::seed_scope(6);
    let mut run = spawn_run(6, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 34);
    let healer = run.party.members[1];
    assert_eq!(
        run.app
            .world()
            .get::<crate::followers::Follower>(healer)
            .unwrap()
            .profile
            .spec_id,
        257,
        "qualify the explicit Holy Priest roster, never silently test Restoration Druid"
    );
    let origin = *run.app.world().get::<Position>(healer).unwrap();
    let started = std::time::Instant::now();
    // Frozen cohort seed6 first separates the native tank by40yd at175.5s.
    // Keep the batch observation order and bound the reproduction to200s.
    for tick in 0..4000 {
        if tick % 10 == 0 {
            run.sample();
        }
        assert!(
            !run.finished(),
            "opening fixture ended: {}",
            run.report.detail
        );
        run.control().unwrap();
        run.app.update();
        if tick % 1000 == 0 {
            run.write_progress(
                tick + 1,
                started,
                std::time::Duration::ZERO,
                &[std::time::Duration::ZERO; 3],
            )
            .unwrap();
        }
    }
    write_stockade_follower_combat(&run).unwrap();
    let tank = super::diagnostics::inspect_unit(run.app.world(), None, run.party.tank);
    let state = super::diagnostics::inspect_unit(run.app.world(), None, healer);
    println!(
        "NATIVE_HEALER_MOVEMENT at={} tank={tank} healer={state}",
        run.now()
    );
    assert!(
        !run.report.pulls.is_empty(),
        "the real route controller must have admitted opening combat"
    );
    assert_ne!(
        run.app.world().get::<Position>(run.party.tank).unwrap(),
        &origin,
        "the tank must have actually left the entrance"
    );
    assert_ne!(
        run.app.world().get::<Position>(healer).unwrap(),
        &origin,
        "Henry must accompany the native route instead of remaining at the entrance while the tank fights out of healing range"
    );
}

fn write_stockade_follower_combat(run: &Run) -> Result<(), String> {
    use shared::protocol::CombatLogKind;
    let world = run.app.world();
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let members: Vec<_> = run
        .party
        .members
        .iter()
        .map(|unit| {
            let follower = world.get::<crate::followers::Follower>(*unit).unwrap();
            let mut casts = BTreeMap::<u32, usize>::new();
            let mut damage = BTreeMap::<u32, i64>::new();
            for event in events
                .iter()
                .filter(|event| event.source == Some(unit.to_bits()))
            {
                let spell = event.spell_id.unwrap_or(0);
                match event.kind {
                    CombatLogKind::CastSuccess => *casts.entry(spell).or_default() += 1,
                    CombatLogKind::Damage => {
                        *damage.entry(spell).or_default() += i64::from(event.amount)
                    }
                    _ => {}
                }
            }
            let total: i64 = damage.values().sum();
            serde_json::json!({"spec": follower.profile.spec_id,
            "casts": casts, "damage_by_spell": damage, "damage": total,
            "dps": total as f64 / run.now()})
        })
        .collect();
    let report = serde_json::json!({"seed": run.report.seed, "seconds": run.now(),
        "map": run.graph.map_id, "followers": members});
    let directory = runtime::output_directory();
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    std::fs::write(
        directory.join("native-book-stockade-combat.json"),
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[test]
fn simulator_leader_does_not_pursue_through_unpulled_native_sight() {
    let _rng_scope = crate::random::seed_scope(28);
    let mut run = spawn_run(28, &mut None).unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let owner = run.party.owner;
    let origin = *run.app.world().get::<Position>(owner).unwrap();
    let enemy = run.actors[&20375735];
    // Native source coordinates put idle20375698/20375677 on this approach.
    // Engage the actual Slag Fury without moving the party or granting spells.
    crate::creature_aggro::attack_start(run.app.world_mut(), enemy, owner);
    for _ in 0..40 {
        run.app.update();
    }
    println!(
        "LEADER_APPROACH {}",
        super::diagnostics::inspect_unit(run.app.world(), None, owner)
    );
    assert_eq!(
        run.app.world().get::<Position>(owner).unwrap(),
        &origin,
        "the autonomous leader must respect the same idle native sight hold as non-tank followers"
    );
}

#[test]
#[ignore = "explicit current Stockade32 seed28 native Slag Fury isolated-leader Death"]
fn stockade_seed_twenty_eight_keeps_the_native_leader_alive_through_the_route() {
    let report = run(28).unwrap();
    runtime::write_report(&report).unwrap();
    runtime::print_report(&report);
    let owner = report.samples.last().unwrap().members.last().unwrap();
    assert!(
        owner.health.is_some_and(|health| health > 0.0),
        "native owner must not die isolated from the admitted party: {}",
        report.detail
    );
    assert_eq!(report.status, "cleared");
}

#[test]
#[ignore = "explicit Stockade corridor seed2 native isolated leader death reproduction"]
fn stockade_corridor_seed_two_keeps_native_party_with_the_approaching_leader() {
    let _rng_scope = crate::random::seed_scope(2);
    let mut run = spawn_run(2, &mut None).unwrap();
    let earth = run.actors[&20375677];
    let mut diagnosed = false;
    let started = std::time::Instant::now();
    for tick in 0..MAX_TICKS {
        if tick % 10 == 0 {
            run.sample();
        }
        if run.finished() {
            return;
        }
        run.control().unwrap();
        run.app.update();
        if tick % 1000 == 0 {
            run.write_progress(
                tick + 1,
                started,
                std::time::Duration::ZERO,
                &[std::time::Duration::ZERO; 3],
            )
            .unwrap();
        }
        let owner_health = run
            .app
            .world()
            .get::<shared::components::Health>(run.party.owner)
            .unwrap();
        if !diagnosed
            && owner_health.current < owner_health.max / 2.0
            && run
                .app
                .world()
                .get::<CombatStatus>(earth)
                .is_some_and(|status| status.0)
        {
            diagnosed = true;
            let tank = super::diagnostics::inspect_unit(run.app.world(), None, run.party.tank);
            println!("ISOLATED_LEADER at={} tank={tank:?}", run.now());
            println!(
                "TANK_MOVEMENT {:?}",
                crate::followers::simulator::inspect_tank_movement(
                    run.app.world_mut(),
                    run.party.tank
                )
            );
            let decision = crate::followers::simulator::inspect_follower_decision(
                run.app.world_mut(),
                run.party.tank,
            );
            println!("TANK_DECISION {decision}");
            let healer = crate::followers::simulator::inspect_follower_decision(
                run.app.world_mut(),
                run.party.members[1],
            );
            println!("HEALER_DECISION {healer}");
        }
        assert!(
            crate::spell_cast::targets::is_alive(run.app.world(), run.party.owner),
            "native party must accompany the corridor approach, not leave the leader dying alone"
        );
        assert!(!run.stalled(), "native run stalled: {}", run.report.detail);
    }
    panic!("seed2 must complete within the unchanged one-hour bound");
}

#[test]
#[ignore = "explicit Stockade32 roster with native230861 Demonology follower"]
fn native_pet_last_hit_evade_finishes_frozen_home_travel_and_native_arrival() {
    let _rng_scope = crate::random::seed_scope(12);
    let mut run = spawn_run(12, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 34);
    let enemy = run.actors[&20375744];
    let warlock = *run
        .party
        .members
        .iter()
        .find(|unit| {
            run.app
                .world()
                .get::<crate::followers::Follower>(**unit)
                .unwrap()
                .profile
                .entry
                == 230861
        })
        .expect("explicit native Demonology roster");
    crate::spell_cast::creature_cast(run.app.world_mut(), warlock, 30146, Some(warlock), false)
        .unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), warlock).expect("native Felguard");
    let spawn = run
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(enemy)
        .unwrap();
    let home = run
        .app
        .world()
        .get::<crate::creature_ai::HomePosition>(enemy)
        .map_or(
            Position {
                x: spawn.x,
                y: spawn.y,
                z: spawn.z,
            },
            |home| home.position,
        );
    let displaced = Position {
        x: home.x - 10.0,
        ..home
    };
    run.app.world_mut().entity_mut(enemy).insert(displaced);
    run.app.world_mut().entity_mut(pet).insert(displaced);
    crate::combat::deal_direct_damage(run.app.world_mut(), pet, enemy, 100.0);
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(
                |event| event.kind == shared::protocol::CombatLogKind::Damage
                    && event.source == Some(pet.to_bits())
                    && event.target == Some(enemy.to_bits())
                    && event.amount > 0
            )
    );
    crate::creature_aggro::enter_evade_mode(run.app.world_mut(), enemy);
    assert_eq!(
        run.app
            .world()
            .get::<crate::creature_ai::CreatureState>(enemy),
        Some(&crate::creature_ai::CreatureState::Returning)
    );
    let entrance = Position {
        x: run.graph.entrance[0],
        y: run.graph.entrance_height,
        z: run.graph.entrance[1],
    };
    run.app.world_mut().entity_mut(pet).insert(entrance);
    crate::pet_control::order_companion_pet(run.app.world_mut(), warlock, None);
    for _ in 0..800 {
        run.app.update();
        if run
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(enemy)
            == Some(&crate::creature_ai::CreatureState::Idle)
        {
            break;
        }
    }
    assert_eq!(
        run.app
            .world()
            .get::<crate::creature_ai::CreatureState>(enemy),
        Some(&crate::creature_ai::CreatureState::Idle),
        "native evade must finish instead of remaining Returning forever"
    );
    let at = *run.app.world().get::<Position>(enemy).unwrap();
    assert!(
        Vec3::new(at.x, at.y, at.z).distance(Vec3::new(home.x, home.y, home.z)) < 0.0001,
        "actual frozen travel must reach home"
    );
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::CreatureMotion>(enemy),
        Some(&shared::components::CreatureMotion::Still),
        "native home arrival must stop replicated movement"
    );
    assert!(
        !run.app
            .world()
            .get::<CombatStatus>(enemy)
            .is_some_and(|combat| combat.0)
    );
}

#[test]
fn simulator_party_environment_overrides_keep_native_profiles_and_level() {
    let mut expected = crate::followers::simulator::DEFAULT_PARTY;
    if let Ok(specs) = std::env::var("DUNGEON_SIMULATOR_PARTY_SPECS") {
        expected = specs
            .split(',')
            .map(|spec| spec.trim().parse::<u32>().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .expect("five requested class specs");
    }
    if let Ok(healer) = std::env::var("DUNGEON_SIMULATOR_HEALER_SPEC") {
        expected[1] = healer.parse().unwrap();
    }
    let level: u8 = std::env::var("DUNGEON_SIMULATOR_LEVEL")
        .map(|value| value.parse().unwrap())
        .unwrap_or(32);
    let run = spawn_run(1, &mut None).unwrap();
    let world = run.app.world();
    let spawned: Vec<_> = run
        .party
        .members
        .iter()
        .chain([&run.party.owner])
        .map(|&unit| {
            assert!(
                world.get::<crate::followers::AplControlled>(unit).is_some(),
                "every simulator actor casts through its spec APL"
            );
            world
                .get::<crate::class_progression::ActiveSpec>(unit)
                .unwrap()
                .0
        })
        .collect();
    assert_eq!(spawned, expected, "actual spawned class specs, leader last");
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
    // Every Stockade spawn stands inside another pack's aggro radius. Slag Fury
    // 20375676 has one neighbour within 18.5 yards, Enraged Fire Elemental
    // 20375742 at 13 yards; an earlier pull cleared it, so this fight stands
    // outside every idle pack's sight.
    let target = run.actors[&20375676];
    let owner = run.party.owner;
    crate::combat::kill_unit(run.app.world_mut(), owner, run.actors[&20375742]);
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
            .position(|spawn| spawn.guid == 20375676)
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
        .map(|step| step["pack"].as_u64().unwrap())
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
fn forces_complete_skips_unengaged_trash_and_selects_remaining_boss() {
    let run = spawn_run(2, &mut None).unwrap();
    let normal = run.next_pack_with_forces(None, false, false).unwrap();
    let boss = run.next_pack_with_forces(None, false, true).unwrap();
    assert_ne!(normal.0, boss.0);
    let npc = run
        .app
        .world()
        .get::<shared::components::Npc>(boss.1)
        .unwrap();
    assert!(run.graph.boss_entries().contains(&npc.template_id));
    assert!(crate::spell_cast::targets::is_alive(
        run.app.world(),
        normal.1
    ));
}

#[test]
fn living_unattackable_required_boss_does_not_block_another_pullable_pack() {
    let _rng_scope = crate::random::seed_scope(2);
    let mut run = spawn_run(2, &mut None).unwrap();
    let boss = run.actors[&20375766];
    run.app
        .world_mut()
        .entity_mut(boss)
        .insert(shared::components::UnitFlags(
            shared::components::UnitFlags::NOT_SELECTABLE,
        ));
    let next = run.next_pack().expect("other native packs remain pullable");
    assert_ne!(next.1, boss);
    run.control().unwrap();
    assert_eq!(run.current_pack, Some(next.0));
    assert!(run.app.world().get::<MoveIntent>(run.party.owner).is_some());
    assert!(run.report.bosses_killed.is_empty());
}

#[test]
fn generated_multi_actor_encounter_counts_native_completion_once() {
    let mut run = spawn_run(1, &mut None).unwrap();
    run.graph.map_id = 2451;
    run.graph.follower_boss_entries = vec![184580, 184581, 184582, 184018];
    run.graph.follower_boss_bits = [(184580, 0), (184581, 0), (184582, 0), (184018, 1)].into();
    assert!(!run.finished());
    assert_eq!(run.report.encounters_total, 2);
    let copy = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.party.owner)
        .unwrap();
    run.app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .update_instance_lock(copy, &[], 1, Some(0), 0);
    assert!(!run.finished());
    assert_eq!(run.report.encounters_killed, 1);
    assert_eq!(run.report.bosses_killed, [184580, 184581, 184582]);
    run.app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .update_instance_lock(copy, &[], 3, Some(1), 0);
    assert!(run.finished());
    assert_eq!(run.report.encounters_killed, 2);
    assert_eq!(run.report.status, "cleared");
}

#[test]
fn absent_required_boss_without_a_death_is_not_a_clear_or_an_endless_hold() {
    let _rng_scope = crate::random::seed_scope(2);
    let mut run = spawn_isolated_moloch(2);
    let boss = run.actors[&20375766];
    run.app.world_mut().despawn(boss);
    assert!(run.next_pack().is_none());
    assert!(run.control().is_err());
    assert!(run.report.bosses_killed.is_empty());
    assert!(!run.finished());
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
fn leader_waits_on_native_walk_graph_until_tank_catches_up() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let actors: Vec<_> = run.actors.values().copied().collect();
    for actor in actors {
        run.app.world_mut().despawn(actor);
    }
    let owner = run.party.owner;
    let tank = run.party.tank;
    let original_owner = *run.app.world().get::<Position>(owner).unwrap();
    let original_tank = *run.app.world().get::<Position>(tank).unwrap();
    let off_graph = Position {
        x: 100000.0,
        y: 100000.0,
        z: 100000.0,
    };
    run.app.world_mut().entity_mut(owner).insert(off_graph);
    run.app.world_mut().entity_mut(tank).insert(off_graph);
    assert!(
        !run.wait_for_tank().unwrap(),
        "colocated party must not require a graph route"
    );
    run.app.world_mut().entity_mut(owner).insert(original_owner);
    run.app.world_mut().entity_mut(tank).insert(original_tank);
    run.next_cohesion_check = 0.0;
    let target = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == 20375706)
        .unwrap()
        .point();
    run.app.world_mut().entity_mut(owner).insert(Position {
        x: target.x,
        y: target.y,
        z: target.z,
    });
    let initial_tank = *run.app.world().get::<Position>(tank).unwrap();
    assert!(run.wait_for_tank().unwrap());
    for _ in 0..3000 {
        run.app.update();
        let position = *run.app.world().get::<Position>(owner).unwrap();
        assert_eq!(Vec3::new(position.x, position.y, position.z), target);
        if !run.wait_for_tank().unwrap() {
            assert_ne!(
                *run.app.world().get::<Position>(tank).unwrap(),
                initial_tank
            );
            return;
        }
    }
    panic!("tank did not regroup along the native graph");
}

#[test]
fn follower_making_native_route_progress_prevents_a_false_stall() {
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let start = *run.app.world().get::<Position>(tank).unwrap();
    let target = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == 20375720)
        .unwrap()
        .point();
    run.app.world_mut().entity_mut(tank).insert((
        MoveIntent(target),
        crate::creature_ai::SpawnPoint {
            x: start.x,
            y: start.y,
            z: start.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 0.005,
        },
    ));
    run.last_progress = run.now();
    for _ in 0..1300 {
        run.app.update();
        assert!(
            !run.stalled(),
            "a follower advancing on its native route must not stall"
        );
    }
    let end = *run.app.world().get::<Position>(tank).unwrap();
    assert_ne!(start, end);
}

#[test]
fn follower_progress_after_owner_moves_far_prevents_false_stall() {
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let owner = run.party.owner;
    let start = *run.app.world().get::<Position>(tank).unwrap();
    let owner_near = Position {
        x: start.x + 5.0,
        ..start
    };
    run.app.world_mut().entity_mut(owner).insert((
        owner_near,
        MoveIntent(Vec3::new(owner_near.x, owner_near.y, owner_near.z)),
    ));
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert((
            crate::creature_ai::FollowOwner::new(owner),
            crate::creature_ai::CreatureState::Idle,
            crate::creature_ai::SpawnPoint {
                x: start.x,
                y: start.y,
                z: start.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 0.005,
            },
        ))
        .remove::<MoveIntent>();
    for _ in 0..20 {
        run.app.update();
        run.stalled();
    }
    let near_best = run.best_routes.get(&tank).expect("near follow route").1;
    let far = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == 20375720)
        .unwrap()
        .point();
    run.app.world_mut().entity_mut(owner).insert((
        Position {
            x: far.x,
            y: far.y,
            z: far.z,
        },
        MoveIntent(far),
    ));
    run.last_progress = run.now();
    let before = *run.app.world().get::<Position>(tank).unwrap();
    for _ in 0..1300 {
        run.app.update();
        assert!(
            !run.stalled(),
            "follower advancing after its owner moved must not stall; near best={near_best}"
        );
    }
    assert_ne!(*run.app.world().get::<Position>(tank).unwrap(), before);
}

#[test]
fn follower_changing_goals_without_route_progress_still_stalls() {
    let mut run = spawn_run(1, &mut None).unwrap();
    let others: Vec<_> = run
        .actors
        .values()
        .copied()
        .chain(
            run.party
                .members
                .iter()
                .copied()
                .filter(|unit| *unit != run.party.tank),
        )
        .collect();
    for unit in others {
        run.app.world_mut().despawn(unit);
    }
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let start = *run.app.world().get::<Position>(tank).unwrap();
    let points: Vec<_> = [20375706, 20375674]
        .into_iter()
        .map(|id| {
            run.graph
                .packs
                .iter()
                .find(|pack| pack.id == id)
                .unwrap()
                .point()
        })
        .collect();
    run.last_progress = run.now();
    let mut moved = false;
    for tick in 0..1300 {
        run.app
            .world_mut()
            .entity_mut(tank)
            .insert(MoveIntent(points[tick % 2]));
        run.app.update();
        moved |= *run.app.world().get::<Position>(tank).unwrap() != start;
        if run.stalled() {
            assert!(
                moved,
                "the follower must actually move while changing goals"
            );
            return;
        }
    }
    panic!("goal changes without decreasing a stable route must still stall");
}

#[test]
fn stationary_full_party_still_stalls_without_any_progress() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    // Keep the fixture's mana full: native buff upkeep can otherwise spend mana
    // between samples, so its later regeneration is legitimate progress.
    let members: Vec<_> = run.members().collect();
    let fill_mana = |run: &mut Run| {
        for &member in &members {
            if let Some(mut mana) = run.app.world_mut().get_mut::<Mana>(member) {
                mana.current = mana.max;
            }
        }
    };
    fill_mana(&mut run);
    run.last_progress = run.now();
    assert!(!run.stalled());
    for _ in 0..1201 {
        run.app.update();
        fill_mana(&mut run);
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
    let before = run.app.world().get::<Mana>(healer).unwrap().current;
    // An already-full normal direct heal is intentionally cancelled by healer AI.
    // Spend through a learned native instant buff instead, without damage or injury.
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 1126, Some(healer), false)
        .unwrap();
    assert!(
        run.app.world().get::<Mana>(healer).unwrap().current < before,
        "native recovery fixture must actually spend mana before measuring progress"
    );
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
fn recovery_wait_drinks_native_item_and_reaches_healer_mana_gate() {
    use shared::components::{Mana, PlayerStandState, StandState};
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let healer = run.party.members[1];
    let tank = run.party.tank;
    let pack = std::collections::BTreeSet::from([20375766]);
    let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
    mana.current = mana.max * 0.30;
    let (allowed, reason) =
        crate::followers::simulator::pull(run.app.world_mut(), tank, &pack).unwrap();
    assert!(!allowed && reason.contains("Recovery"), "{reason}");
    let mut started_at = None;
    let mut reached_at = None;
    for frame in 0..400 {
        run.app.update();
        let world = run.app.world_mut();
        let _ = crate::followers::simulator::pull(world, tank, &pack).unwrap();
        if crate::auras::has_aura(world, healer, 1291791) {
            started_at.get_or_insert(frame);
            assert_eq!(
                world.get::<PlayerStandState>(healer).unwrap().0,
                StandState::Sit
            );
        }
        let mana = world.get::<Mana>(healer).unwrap();
        if mana.current >= mana.max * 0.81 {
            reached_at = Some(frame);
            break;
        }
    }
    let start = started_at.expect("native drink aura was never applied");
    let end = reached_at.expect("healer did not reach the recovery gate");
    assert!(
        end - start <= 220,
        "recovery took {} frames after drinking began",
        end - start
    );
}

#[test]
fn recovery_drink_is_interrupted_by_follower_movement() {
    use shared::components::{Mana, PlayerStandState, StandState};
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let healer = run.party.members[1];
    let tank = run.party.tank;
    let pack = std::collections::BTreeSet::from([20375766]);
    let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
    mana.current = mana.max * 0.30;
    for _ in 0..100 {
        crate::followers::simulator::pull(run.app.world_mut(), tank, &pack).unwrap();
        if crate::auras::has_aura(run.app.world(), healer, 1291791) {
            break;
        }
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), healer, 1291791));
    let mut to = *run.app.world().get::<Position>(healer).unwrap();
    to.x += 1.0;
    crate::creature_ai::move_follower(run.app.world_mut(), healer, to, 3.0);
    assert_eq!(*run.app.world().get::<Position>(healer).unwrap(), to);
    run.app.update();
    assert!(!crate::auras::has_aura(run.app.world(), healer, 1291791));
    assert_eq!(
        run.app.world().get::<PlayerStandState>(healer).unwrap().0,
        StandState::Stand
    );
}

#[test]
fn recovery_drink_retries_and_interrupts_on_cast_and_combat() {
    use shared::components::{CombatStatus, Mana};
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let healer = run.party.members[1];
    let tank = run.party.tank;
    let pack = std::collections::BTreeSet::from([20375766]);
    let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
    mana.current = mana.max * 0.30;
    for _ in 0..100 {
        crate::followers::simulator::pull(run.app.world_mut(), tank, &pack).unwrap();
        if crate::auras::has_aura(run.app.world(), healer, 1291791) {
            break;
        }
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), healer, 1291791));
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        healer,
        774,
        crate::spell_cast::CastTargets {
            unit: healer,
            primary: Some(healer),
            destination: None,
        },
    )
    .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), healer, 1291791),
        "casting must interrupt drinking"
    );
    for _ in 0..100 {
        run.app.update();
        crate::followers::simulator::pull(run.app.world_mut(), tank, &pack).unwrap();
        if crate::auras::has_aura(run.app.world(), healer, 1291791) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), healer, 1291791),
        "remaining low mana must trigger another item use"
    );
    let enemy = run.actors[&20375766];
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), healer, enemy);
    assert!(run.app.world().get::<CombatStatus>(healer).unwrap().0);
    crate::followers::update_ai(run.app.world_mut());
    assert!(
        !crate::auras::has_aura(run.app.world(), healer, 1291791),
        "combat must interrupt drinking"
    );
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
        if react == ReactState::Passive {
            // The first update clears the previous aggressive watcher's trail;
            // the next update resumes ordinary following.
            crate::followers::simulator::regroup(run.app.world_mut(), healer, owner);
        }
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
        radius, 14.5,
        "native31-to32 acquisition includes the actual 1.5yd creature reach"
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
    // Active followers now receive resurrection/release handling; detach this
    // dead summon so native corpse decay can exercise the missing-member sample.
    run.app
        .world_mut()
        .entity_mut(member)
        .remove::<crate::followers::Follower>();
    for _ in 0..1202 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(member).is_err(),
        "native corpse decay must actually despawn the dead summon"
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

#[test]
#[ignore = "explicit alternate-roster qualification; use Bo/Lantha party entries"]
fn alternate_roster_attackers_participate_in_the_first_native_fight() {
    let _scope = crate::random::seed_scope(11);
    let mut run = spawn_run(11, &mut None).unwrap();
    for _ in 0..600 {
        run.control().unwrap();
        run.app.update();
    }
    let actors: Vec<_> = run.members().collect();
    for actor in actors {
        println!(
            "ACTOR {}",
            super::diagnostics::inspect_unit(run.app.world(), None, actor)
        );
        if run
            .app
            .world()
            .get::<crate::followers::Follower>(actor)
            .is_some()
        {
            println!(
                "DECISION {}",
                crate::followers::simulator::inspect_follower_decision(run.app.world_mut(), actor)
            );
        }
    }
    let monk = run
        .party
        .members
        .iter()
        .copied()
        .find(|unit| {
            run.app
                .world()
                .get::<crate::followers::Follower>(*unit)
                .is_some_and(|follower| follower.profile.spec_id == 269)
        })
        .expect("fixture requires explicit Bo party entry");
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let dealt_damage = |unit: Entity| {
        events.iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.source == Some(unit.to_bits())
                && event.amount > 0
        })
    };
    assert!(
        dealt_damage(run.party.tank),
        "real native engagement must occur before asserting attacker participation"
    );
    assert!(
        dealt_damage(monk),
        "Bo must pursue and execute native attacks in the actual opening fight, not remain idle for the whole campaign"
    );
}

fn deck_replay_provenance(replay_executed: bool) -> serde_json::Value {
    serde_json::json!({
        "report": "x3HzDRn6yVjqCbt8", "fight": 7, "replay_executed": replay_executed,
        "transport_start_ms": DAWNBREAKER_DECK_START_MS,
        "commander_death_ms": DAWNBREAKER_COMMANDER_DEATH_MS,
        "radiant_light_ms": DAWNBREAKER_RADIANT_GRANT_MS, "altitude": "inferred",
        "boundary_positions": "interpolated from logged cast positions",
    })
}

struct DeckReplay {
    started_at: f64,
    start: Vec3,
    end: Vec3,
}

impl DeckReplay {
    fn position(&self, now: f64) -> Vec3 {
        let elapsed = now - self.started_at;
        self.start.lerp(
            self.end,
            (elapsed / DECK_CARRY_SECONDS).clamp(0.0, 1.0) as f32,
        )
    }

    fn grant_due(&self, now: f64) -> bool {
        now - self.started_at >= DECK_GRANT_SECONDS
    }
}

/// Replay WCL XY/timing only; transport-backed deck population is not sourced.
/// Replace this phase when server transport support lands and 214761/214762 are sourced.
fn advance_deck_frame(
    world: &mut World,
    members: &[Entity],
    deck: &DeckReplay,
    now: f64,
) -> Result<bool, String> {
    let point = deck.position(now);
    let position = Position {
        x: point.x,
        y: point.y,
        z: point.z,
    };
    let pets: Vec<_> = members
        .iter()
        .flat_map(|member| crate::hunter_pet::active_owned_pets(world, *member))
        .collect();
    for &member in members {
        if !crate::spell_cast::targets::is_alive(world, member) {
            return Err("deck replay party member died; recorded phase cannot continue".into());
        }
        world.entity_mut(member).insert(position);
    }
    for pet in pets {
        world.entity_mut(pet).insert(position);
    }
    if !deck.grant_due(now) {
        return Ok(true);
    }
    grant_deck_radiant_light(world, members)?;
    Ok(false)
}

fn grant_deck_radiant_light(world: &mut World, members: &[Entity]) -> Result<(), String> {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(449042)
        .ok_or("Radiant Light 449042 is missing from the spell catalog")?;
    let effect_mask = crate::auras::aura_effect_mask(spell);
    for &member in members {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: member,
                target: member,
                spell_id: 449042,
                effect_mask,
                source: crate::auras::AuraSource::Cast,
            },
        );
        if !crate::auras::has_aura(world, member, 449042) {
            return Err(format!("Radiant Light did not apply to {member}"));
        }
    }
    Ok(())
}

#[test]
fn deck_replay_preserves_logged_carry_and_grant_times() {
    let deck = DeckReplay {
        started_at: 5.0,
        start: Vec3::new(1421.0, 3.0, 331.0),
        end: Vec3::new(1973.0, 1104.0, -289.0),
    };
    assert_eq!(deck_replay_provenance(false)["replay_executed"], false);
    assert_eq!(deck_replay_provenance(true)["commander_death_ms"], 162380);
    assert_eq!(deck.position(5.0), deck.start);
    assert!(
        deck.position(43.19)
            .distance(deck.start.lerp(deck.end, 0.5))
            < 0.001
    );
    assert_eq!(deck.position(81.38), deck.end);
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[449042], 1)
        .expect("real Radiant Light catalog");
    let member = h.caster;
    let pet = h
        .app
        .world_mut()
        .spawn((
            crate::hunter_pet::HunterPet {
                owner: member,
                slot: 0,
            },
            crate::maps::InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            Position {
                x: deck.start.x,
                y: deck.start.y,
                z: deck.start.z,
            },
        ))
        .id();
    assert!(advance_deck_frame(h.app.world_mut(), &[member], &deck, 81.38).unwrap());
    assert_eq!(
        *h.app.world().get::<Position>(member).unwrap(),
        Position {
            x: deck.end.x,
            y: deck.end.y,
            z: deck.end.z
        }
    );
    assert_eq!(
        h.app.world().get::<Position>(pet),
        h.app.world().get::<Position>(member)
    );
    assert!(!crate::auras::has_aura(h.app.world(), member, 449042));
    assert!(advance_deck_frame(h.app.world_mut(), &[member], &deck, 85.806).unwrap());
    assert!(!crate::auras::has_aura(h.app.world(), member, 449042));
    assert!(!advance_deck_frame(h.app.world_mut(), &[member], &deck, 85.807).unwrap());
    assert!(crate::auras::has_aura(h.app.world(), member, 449042));
}

struct Run {
    app: App,
    graph: Graph,
    party: crate::followers::simulator::Party,
    actors: BTreeMap<u64, Entity>,
    objects: BTreeMap<u64, Entity>,
    report: Report,
    current_pack: Option<u64>,
    waiting_objective: Option<(u64, f64)>,
    deck_replay: Option<DeckReplay>,
    /// Packs whose frozen route anchor the owner reached without engaging them.
    anchors_reached: std::collections::BTreeSet<u64>,
    last_progress: f64,
    event_cursor: usize,
    best_routes: BTreeMap<Entity, (super::movement::RouteGoal, f32)>,
    next_cohesion_check: f64,
    waiting_for_tank: bool,
    returning_to_party: bool,
    last_resources: Vec<(Option<f32>, Option<f32>)>,
    healer_metrics: super::healer_metrics::Recorder,
    pet_recorder: super::diagnostics::PetRecorder,
    keystone: Option<super::keystone::State>,
    chain_groups: Option<Vec<Vec<u64>>>,
    chain_group: usize,
    chain_tag_target: Option<Entity>,
    chain_tag_cursor: usize,
    chain_returning: bool,
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
    let run = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        spawn_run(seed, catalogs)
    })) {
        Ok(Ok(run)) => run,
        Ok(Err(error)) => return Ok(aborted_initialization(seed, started, error)),
        Err(panic) => return Ok(aborted_initialization(seed, started, panic_reason(panic))),
    };
    let setup = started.elapsed();
    run_prepared(run, catalogs, started, setup)
}

pub(super) fn run_keystone(
    seed: u64,
    options: super::keystone::Options,
) -> super::keystone_record::RunRecord {
    run_keystone_ticks(seed, options, MAX_TICKS)
}

/// `run_keystone` stopped after `max_ticks` fixed updates.
pub(super) fn run_keystone_ticks(
    seed: u64,
    options: super::keystone::Options,
    max_ticks: usize,
) -> super::keystone_record::RunRecord {
    let _rng_scope = crate::random::seed_scope(seed);
    let mut planned_route = None;
    let initialized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        spawn_run_mode_record(seed, &mut None, Some(options), &mut planned_route)
    }));
    let mut run = match initialized {
        Ok(Ok(run)) => run,
        Ok(Err(error)) => return failed_keystone_record(seed, options, error, planned_route),
        Err(panic) => {
            return failed_keystone_record(seed, options, panic_reason(panic), planned_route);
        }
    };
    run.last_progress = run.now();
    let mut progress = super::keystone::Progress::default();
    let outcome =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
            for _ in 0..max_ticks {
                if run.finished() {
                    break;
                }
                if let Err(error) = run.control() {
                    run.report.status = "blocked".into();
                    run.report.detail = error;
                    break;
                }
                run.app.update();
                run.healer_metrics.observe_frame(run.app.world());
                let mut key = run.keystone.take().ok_or("keystone state disappeared")?;
                let result = key.update(run.app.world_mut()).and_then(|()| {
                    key.report_progress(
                        &mut progress,
                        run.app.world(),
                        run.current_pack,
                        run.party.tank,
                    )
                });
                run.keystone = Some(key);
                result?;
                if run.stalled() {
                    break;
                }
            }
            Ok(())
        }));
    let error = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(panic) => Some(panic_reason(panic)),
    };
    if error.is_none() && run.report.status == "running" {
        run.finished();
    }
    if let Some(error) = error {
        run.report.status = "aborted".into();
        run.report.detail = error;
    } else if run.report.status == "running" {
        run.report.status = "timeout".into();
        run.report.detail = "one-hour simulated bound reached".into();
    }
    let status = if run.report.status == "cleared" {
        "completed".into()
    } else {
        run.report.status.clone()
    };
    let diagnostics = serde_json::json!({
        "current_pack": run.current_pack,
        "active_enemies": crate::followers::simulator::active_enemies(run.app.world(), run.party.owner).into_iter().map(Entity::to_bits).collect::<Vec<_>>(),
        "uncleared_fight": crate::followers::simulator::has_uncleared_fight(run.app.world(), run.party.owner),
        "tank_transport_pending": server::test_support::transport::follower_routing(run.app.world(), run.party.tank),
        "returning_to_party": run.returning_to_party,
        "waiting_for_tank": run.waiting_for_tank,
        "owner_move_intent": run.app.world().get::<MoveIntent>(run.party.owner).map(|intent| intent.0.to_array()),
        "cleared_sources": crate::followers::simulator::observed_cleared_sources(run.app.world(), run.party.owner),
        "terminal_units": run.inspect_units(),
        "pull_decisions": &run.report.pulls,
    });
    let state = run.keystone.take().expect("keystone state after run");
    state
        .finish(
            run.app.world(),
            seed,
            options,
            status,
            run.report.detail,
            diagnostics,
        )
        .unwrap_or_else(|error| failed_keystone_record(seed, options, error, planned_route))
}

fn start_with_recorded_route<T>(
    saved: &mut Option<super::keystone_route::PlannedRoute>,
    plan: super::keystone_route::PlannedRoute,
    start: impl FnOnce(super::keystone_route::PlannedRoute) -> Result<T, String>,
) -> Result<T, String> {
    *saved = Some(plan.clone());
    start(plan)
}

#[test]
fn font_initialization_failure_retains_the_planned_force_route() {
    let options = super::keystone::Options {
        map_id: 2825,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let plan = super::keystone_route::PlannedRoute {
        steps: Vec::new(),
        added_packs: vec![17, 21],
        planned_forces: 734,
    };
    let mut saved = None;
    let error =
        start_with_recorded_route(&mut saved, plan, |_| Err::<(), _>("missing Font".into()))
            .unwrap_err();
    let record = failed_keystone_record(1, options, error, saved);
    let json = serde_json::to_value(record).unwrap();
    assert_eq!(json["outcome"]["status"], "aborted");
    assert_eq!(json["outcome"]["reason"], "missing Font");
    assert_eq!(json["route"]["planned_forces"], 734);
    assert_eq!(json["route"]["added_packs"], serde_json::json!([17, 21]));
}

fn failed_keystone_record(
    seed: u64,
    options: super::keystone::Options,
    reason: String,
    planned_route: Option<super::keystone_route::PlannedRoute>,
) -> super::keystone_record::RunRecord {
    super::keystone_record::RunRecord {
        schema_version: 1,
        seed,
        schedule_epoch: super::keystone::schedule_epoch().ok(),
        requested_specs: options.specs,
        map_id: options.map_id,
        challenge_id: None,
        key_level: options.level,
        affixes: None,
        party: Vec::new(),
        route: planned_route,
        scaling: None,
        outcome: super::keystone_record::Outcome {
            status: "aborted".into(),
            reason,
            elapsed_ms: None,
            timer_ms: None,
            in_time: None,
            upgrade_levels: None,
            resulting_key_level: None,
            forces: None,
            forces_required: None,
            bosses: None,
        },
        deaths: Vec::new(),
        pulls: Vec::new(),
        actors: Vec::new(),
        diagnostics: serde_json::Value::Null,
    }
}

fn run_prepared(
    mut run: Run,
    catalogs: &mut Option<super::catalogs::Catalogs>,
    started: std::time::Instant,
    setup: std::time::Duration,
) -> Result<Report, String> {
    let mut timings = [std::time::Duration::ZERO; 3];
    let mut ticks = 0;
    let outcome =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
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
                run.healer_metrics.observe_frame(run.app.world());
                timings[2] += update.elapsed();
                ticks += 1;
                if tick % 1000 == 0 {
                    run.write_progress(ticks, started, setup, &timings)?;
                }
                if run.stalled() {
                    break;
                }
            }
            Ok(())
        }));
    let abort = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(panic) => Some(panic_reason(panic)),
    };
    if let Some(reason) = abort {
        run.report.status = "aborted".into();
        run.report.detail = format!("gameplay execution aborted: {reason}");
    }
    run.sample();
    run.report.performance = run.performance(ticks, started, setup, &timings);
    run.report.performance["tank_movement_admission"] = serde_json::json!(
        crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), run.party.tank)
    );
    let members: std::collections::BTreeSet<_> = run.members().map(Entity::to_bits).collect();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    run.report.deaths = events
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Death
                && event.target.is_some_and(|target| members.contains(&target))
        })
        .count();
    run.report.performance["combat_events"] = serde_json::to_value(events)
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

fn panic_reason(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(reason) = panic.downcast_ref::<String>() {
        return reason.clone();
    }
    if let Some(reason) = panic.downcast_ref::<&str>() {
        return (*reason).into();
    }
    "non-text panic payload".into()
}

fn aborted_initialization(seed: u64, started: std::time::Instant, reason: String) -> Report {
    Report {
        seed,
        status: "aborted".into(),
        failed_pack: None,
        bosses_killed: Vec::new(),
        encounters_killed: 0,
        encounters_total: 0,
        deaths: 0,
        simulated_seconds: 0.0,
        wall_seconds: started.elapsed().as_secs_f64(),
        detail: format!("gameplay initialization aborted: {reason}"),
        deck_population_unsourced: None,
        samples: Vec::new(),
        pulls: Vec::new(),
        performance: serde_json::json!({"combat_events": [], "initialization_complete": false}),
        terminal_units: serde_json::json!([]),
    }
}

fn spawn_run(seed: u64, catalogs: &mut Option<super::catalogs::Catalogs>) -> Result<Run, String> {
    spawn_run_mode(seed, catalogs, None)
}

fn spawn_run_mode(
    seed: u64,
    catalogs: &mut Option<super::catalogs::Catalogs>,
    options: Option<super::keystone::Options>,
) -> Result<Run, String> {
    spawn_run_mode_record(seed, catalogs, options, &mut None)
}

fn spawn_run_mode_record(
    seed: u64,
    catalogs: &mut Option<super::catalogs::Catalogs>,
    options: Option<super::keystone::Options>,
    planned_route: &mut Option<super::keystone_route::PlannedRoute>,
) -> Result<Run, String> {
    let mut graph = super::graph::read_graph()?;
    if let Some(options) = options
        && options.map_id != graph.map_id
    {
        return Err(format!(
            "selected map{} does not match graph map{}",
            options.map_id, graph.map_id
        ));
    }
    let planned = if options.is_some() {
        Some(super::keystone::prepare_challenge(&graph)?)
    } else {
        None
    };
    let mut app = runtime::build_cached_app(catalogs);
    app.insert_resource(super::Space(graph.clone()));
    let copy = if options.is_some() {
        runtime::create_copy_at_difficulty(&mut app, 23)?
    } else {
        runtime::create_copy(&mut app)
    };
    let entrance = Position {
        x: graph.entrance[0],
        y: graph.entrance_height,
        z: graph.entrance[1],
    };
    let party = match (options, planned.as_ref()) {
        (Some(options), Some((_, level))) => crate::followers::simulator::spawn_keystone_party(
            app.world_mut(),
            copy,
            entrance,
            *level,
            options.specs,
            options.level,
        )?,
        (None, None) => crate::followers::simulator::spawn_party(app.world_mut(), copy, entrance)?,
        _ => return Err("keystone planning state is inconsistent".into()),
    };
    server::test_support::enable_headless_resurrection(app.world_mut(), party.owner);
    if !graph.flights.is_empty() {
        server::test_support::flight::configure_leader(app.world_mut(), party.owner)?;
    }
    let actors = runtime::spawn_population(&mut app, copy);
    let object_guids: Vec<_> = graph
        .transports
        .iter()
        .filter(|transport| {
            matches!(
                transport.action,
                server::test_support::transport::Action::ObjectGossip
                    | server::test_support::transport::Action::ObjectScript
            )
        })
        .map(|transport| {
            u32::try_from(transport.source_guid).map_err(|_| {
                format!(
                    "transport object GUID {} exceeds u32",
                    transport.source_guid
                )
            })
        })
        .collect::<Result<_, _>>()?;
    let objects = if object_guids.is_empty() {
        BTreeMap::new()
    } else {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|error| format!("open transport objects: {error}"))?;
        server::test_support::transport::spawn_objects(app.world_mut(), &db, copy, &object_guids)?
            .into_iter()
            .map(|(guid, entity)| (u64::from(guid), entity))
            .collect()
    };
    let missing: Vec<_> = graph
        .packs
        .iter()
        .flat_map(|pack| &pack.spawns)
        .filter(|spawn| !actors.contains_key(&spawn.guid))
        .filter(|spawn| {
            !super::graph::corridor_inputs::source_is_withheld(
                app.world().resource::<crate::world_data::WorldData>(),
                copy,
                app.world()
                    .resource::<crate::maps::InstanceManager>()
                    .copy_difficulty(copy)
                    .expect("simulator copy")
                    .difficulty_id,
                spawn.guid,
            )
        })
        .map(|spawn| spawn.guid)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "frozen graph contains sources absent from the selected production population: {missing:?}"
        ));
    }
    if options.is_some() {
        let source_ids: std::collections::BTreeSet<_> = graph
            .packs
            .iter()
            .flat_map(|pack| pack.spawns.iter().map(|spawn| spawn.guid))
            .collect();
        let extra: Vec<_> = actors
            .keys()
            .filter(|guid| !source_ids.contains(guid))
            .copied()
            .collect();
        if !extra.is_empty() {
            return Err(format!(
                "selected Mythic population is absent from the frozen graph: {extra:?}"
            ));
        }
    }
    let mut chain_groups = None;
    let keystone = match (options, planned) {
        (Some(options), Some((definition, _))) => {
            let hostile_sources =
                super::keystone_route::attackable_sources(app.world(), party.owner, &actors);
            let plan = super::keystone_route::plan(
                &graph,
                &definition.creature_weights,
                definition.forces_required,
                &hostile_sources,
            )?;
            graph.route = plan.steps.clone();
            chain_groups =
                super::keystone_route::wcl_chain_groups(&graph, &definition, &hostile_sources)?;
            app.insert_resource(super::Space(graph.clone()));
            let nav = std::env::var_os("DUNGEON_SIMULATOR_NAVMESH")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| super::graph::extraction::asset_root().join("navmesh"));
            server::test_support::install_navigation(&mut app, &nav);
            Some(start_with_recorded_route(planned_route, plan, |plan| {
                super::keystone::start(&mut app, &party, plan, options)
            })?)
        }
        (None, None) => None,
        _ => return Err("keystone planning state is inconsistent".into()),
    };
    let healer_metrics = super::healer_metrics::start(app.world_mut(), &graph, &party, &actors);
    let deck_provenance = (graph.map_id == 2662).then(|| deck_replay_provenance(false));
    Ok(Run {
        app,
        graph,
        party,
        actors,
        objects,
        current_pack: None,
        waiting_objective: None,
        deck_replay: None,
        anchors_reached: Default::default(),
        last_progress: 0.0,
        event_cursor: 0,
        best_routes: BTreeMap::new(),
        next_cohesion_check: 0.0,
        waiting_for_tank: false,
        returning_to_party: false,
        last_resources: Vec::new(),
        healer_metrics,
        pet_recorder: Default::default(),
        keystone,
        chain_groups,
        chain_group: 0,
        chain_tag_target: None,
        chain_tag_cursor: 0,
        chain_returning: false,
        report: Report {
            seed,
            status: "running".into(),
            failed_pack: None,
            bosses_killed: Vec::new(),
            encounters_killed: 0,
            encounters_total: 0,
            deaths: 0,
            simulated_seconds: 0.0,
            wall_seconds: 0.0,
            detail: String::new(),
            deck_population_unsourced: deck_provenance,
            samples: Vec::new(),
            pulls: Vec::new(),
            performance: serde_json::Value::Null,
            terminal_units: serde_json::Value::Null,
        },
    })
}

fn clip_route_to_tank_range(
    origin: Vec3,
    tank: Vec3,
    range: f32,
    points: impl IntoIterator<Item = Vec3>,
) -> Vec3 {
    let mut previous = origin;
    for point in points {
        if point.distance_squared(tank) > range * range {
            let offset = previous - tank;
            let leg = point - previous;
            let a = leg.length_squared();
            if a == 0.0 {
                return previous;
            }
            let b = 2.0 * offset.dot(leg);
            let c = offset.length_squared() - range * range;
            let distance =
                ((-b + (b * b - 4.0 * a * c).max(0.0).sqrt()) / (2.0 * a)).clamp(0.0, 1.0);
            return previous.lerp(point, distance);
        }
        previous = point;
    }
    previous
}

/// Drive the persistent owner rotation for the current opening target.
pub(crate) fn drive_owner_apl_opening(
    world: &mut World,
    owner: Entity,
    target: Entity,
) -> Result<(), String> {
    let mut apl = match world
        .entity_mut(owner)
        .take::<server_follower_apl::FollowerApl>()
    {
        Some(apl) => apl,
        None => {
            let spec = world
                .get::<shared::components::ActiveSpec>(owner)
                .ok_or("keystone owner lacks an active specialization")?
                .0;
            let rotation = server_follower_apl::registry::load(world, owner, spec)?;
            server_follower_apl::FollowerApl::new_for_world(world, rotation)
        }
    };
    let result = apl.tick(world, owner, &[target]);
    world.entity_mut(owner).insert(apl);
    result.map_err(|error| format!("keystone owner opening failed: {error}"))
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
            "source_map_actors": self.actors.len(),
            "healer_metrics": self.healer_metrics.report(self.app.world()), "map_id": self.graph.map_id,
            "difficulty_id":self.graph.difficulty_id(),
            "party_levels":self.members().map(|unit|self.app.world().get::<shared::components::Level>(unit).map(|level|level.0)).collect::<Vec<_>>(),
            "observed_events": self.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len(),
            "holy_metrics": self.app.world().resource::<crate::spell_cast::holy_metrics::HolyMetrics>(),
            "samples": self.report.samples.len(), "memory": super::diagnostics::read_process_memory()})
    }

    fn inspect_units(&self) -> serde_json::Value {
        let units: Vec<_> = self
            .actors
            .iter()
            .map(|(guid, actor)| (Some(*guid), *actor))
            .chain(self.members().map(|actor| (None, actor)))
            .chain(self.pet_recorder.units().map(|actor| (None, actor)))
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
            "PROGRESS seed={} ticks={} sim_s={:.2} ratio={} steady_ratio={} entities={} events={} pack={:?} owner_intent={:?} owner_state={:?} tank_state={:?}",
            self.report.seed,
            ticks,
            self.now(),
            performance["sim_per_wall"],
            performance["steady_sim_per_wall"],
            performance["live_entities"],
            performance["observed_events"],
            self.current_pack,
            self.app
                .world()
                .get::<MoveIntent>(self.party.owner)
                .map(|intent| intent.0),
            self.app
                .world()
                .get::<crate::creature_ai::CreatureState>(self.party.owner),
            self.app
                .world()
                .get::<crate::creature_ai::CreatureState>(self.party.tank),
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
        let entities: Vec<_> = self.members().collect();
        let pets = self
            .pet_recorder
            .sample(self.app.world(), &entities, &self.actors);
        let members = entities
            .into_iter()
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
            pets,
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
                if let Some(bit) = self.graph.follower_boss_bits.get(entry) {
                    let Some(copy) = world.get::<crate::maps::InMap>(self.party.owner) else {
                        return false;
                    };
                    return world
                        .resource::<crate::maps::InstanceManager>()
                        .completed_encounters(*copy)
                        & (1 << bit)
                        != 0;
                }
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
        if self.graph.follower_boss_bits.is_empty() {
            self.report.encounters_killed = self.report.bosses_killed.len();
            self.report.encounters_total = self.graph.boss_entries().len();
        } else {
            let all: std::collections::BTreeSet<_> =
                self.graph.follower_boss_bits.values().collect();
            let killed: std::collections::BTreeSet<_> = self
                .report
                .bosses_killed
                .iter()
                .filter_map(|entry| self.graph.follower_boss_bits.get(entry))
                .collect();
            self.report.encounters_killed = killed.len();
            self.report.encounters_total = all.len();
        }
        if self
            .keystone
            .as_ref()
            .is_some_and(|key| key.completed(world))
        {
            self.report.status = "cleared".into();
            self.report.detail = "native keystone criteria completed".into();
            return true;
        }
        if self.keystone.is_none()
            && self.report.encounters_total > 0
            && self.report.encounters_killed == self.report.encounters_total
        {
            self.report.status = "cleared".into();
            self.report.detail = if self.graph.follower_boss_bits.is_empty() {
                format!(
                    "all{} source boss actors died",
                    self.graph.boss_entries().len()
                )
            } else {
                format!(
                    "all{} source encounters completed",
                    self.report.encounters_total
                )
            };
            return true;
        }
        if self.keystone.is_none()
            && self
                .members()
                .all(|member| !crate::spell_cast::targets::is_alive(world, member))
        {
            self.report.status = "wiped".into();
            self.report.detail = "all five bots died".into();
            return true;
        }
        false
    }

    fn refresh_population(&mut self) {
        if let Some(copy) = self
            .app
            .world()
            .get::<crate::maps::InMap>(self.party.owner)
            .copied()
        {
            let live = runtime::live_population(self.app.world_mut(), copy);
            let changed = live
                .iter()
                .any(|(guid, actor)| self.actors.get(guid) != Some(actor));
            self.actors.extend(live);
            if changed {
                self.healer_metrics.register_sources(
                    self.app.world_mut(),
                    &self.graph,
                    &self.actors,
                );
            }
        }
    }

    fn wait_for_tank(&mut self) -> Result<bool, String> {
        let world = self.app.world();
        if server::test_support::transport::follower_routing(world, self.party.tank) {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(true);
        }
        if !crate::spell_cast::targets::is_alive(world, self.party.tank)
            || server::test_support::transport::in_progress(world, self.party.tank)
        {
            self.waiting_for_tank = false;
            return Ok(false);
        }
        if self.now() >= self.next_cohesion_check {
            let owner = *world
                .get::<Position>(self.party.owner)
                .ok_or("cohesion owner absent")?;
            let tank = *world
                .get::<Position>(self.party.tank)
                .ok_or("cohesion tank absent")?;
            let from = Vec3::new(tank.x, tank.y, tank.z);
            let to = Vec3::new(owner.x, owner.y, owner.z);
            let (_, opening_range) =
                crate::followers::simulator::opening_spell(world, self.party.tank);
            let nearby =
                from.distance(to) <= opening_range && (from.y - to.y).abs() <= opening_range;
            self.waiting_for_tank = if nearby {
                false
            } else {
                let route = world
                    .resource::<super::native_movement::NativeMovement>()
                    .walk_legs(from, to, "cohesion")
                    .map_err(|error| format!("tank cannot follow leader: {error}"))?;
                let yards: f32 = route.iter().map(|(_, distance)| distance).sum();
                yards > opening_range
            };
            self.next_cohesion_check = self.now() + COHESION_CHECK_SECONDS;
        }
        if self.waiting_for_tank {
            let owner = *self
                .app
                .world()
                .get::<Position>(self.party.owner)
                .ok_or("cohesion owner absent")?;
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(Vec3::new(owner.x, owner.y, owner.z)));
        }
        Ok(self.waiting_for_tank)
    }

    fn return_to_party(&mut self) -> Result<bool, String> {
        if !self.returning_to_party {
            return Ok(false);
        }
        let world = self.app.world();
        let Some(member) = self
            .party
            .members
            .iter()
            .copied()
            .find(|member| crate::spell_cast::targets::is_alive(world, *member))
        else {
            self.returning_to_party = false;
            return Ok(false);
        };
        let target = *world
            .get::<Position>(member)
            .ok_or("party member position absent")?;
        let owner = *world
            .get::<Position>(self.party.owner)
            .ok_or("returning leader position absent")?;
        let target = Vec3::new(target.x, target.y, target.z);
        let from = Vec3::new(owner.x, owner.y, owner.z);
        if self.approach_translocation(target)? {
            return Ok(true);
        }
        let route = self
            .app
            .world()
            .resource::<super::native_movement::NativeMovement>()
            .walk_legs(from, target, "reunion")
            .map_err(|error| format!("released leader cannot reach party: {error}"))?;
        let yards: f32 = route.iter().map(|(_, distance)| distance).sum();
        let (_, range) = crate::followers::simulator::opening_spell(self.app.world(), member);
        if yards > range {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(target));
            return Ok(true);
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<MoveIntent>();
        self.returning_to_party = false;
        self.next_cohesion_check = 0.0;
        Ok(false)
    }

    fn advance_deck_replay(&mut self) -> Result<bool, String> {
        let Some(deck) = self.deck_replay.take() else {
            return Ok(false);
        };
        let now = self.now();
        let members: Vec<_> = self.members().collect();
        let active = advance_deck_frame(self.app.world_mut(), &members, &deck, now)?;
        self.last_progress = now;
        if active {
            self.deck_replay = Some(deck);
        }
        Ok(active)
    }

    fn control(&mut self) -> Result<(), String> {
        if let Some(keystone) = &self.keystone
            && keystone.waiting_for_start(self.app.world())?
        {
            return Ok(());
        }
        if self.advance_deck_replay()? {
            return Ok(());
        }
        self.refresh_population();
        if server::test_support::transport::in_progress(self.app.world(), self.party.owner) {
            return Ok(());
        }
        if !crate::spell_cast::targets::is_alive(self.app.world(), self.party.owner) {
            self.chain_tag_target = None;
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<server_follower_apl::DirectedOwnerTarget>();
            if server::test_support::player_death_recorded(self.app.world(), self.party.owner) {
                if server::test_support::accept_resurrection(self.app.world_mut(), self.party.owner)
                {
                    return Ok(());
                }
                if crate::followers::simulator::has_uncleared_fight(
                    self.app.world(),
                    self.party.owner,
                ) || crate::followers::resurrection_pending(
                    self.app.world_mut(),
                    self.party.owner,
                ) {
                    return Ok(());
                }
                server::test_support::release_dungeon_player(
                    self.app.world_mut(),
                    self.party.owner,
                )?;
                super::movement::forget_party_travel(self.app.world_mut(), self.party.owner);
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .remove::<MoveIntent>();
                self.returning_to_party = true;
            }
            return Ok(());
        }
        if super::movement::flight_in_progress(self.app.world(), self.party.owner) {
            return Ok(());
        }
        if self.return_to_party()? {
            return Ok(());
        }
        let fighting =
            crate::followers::simulator::has_uncleared_fight(self.app.world(), self.party.owner);
        if self.chain_groups.is_some() && self.return_after_chain_tag()? {
            return Ok(());
        }
        if self.chain_groups.is_some() && fighting {
            self.next_cohesion_check = 0.0;
            if self.wait_for_tank()? {
                self.move_owner_to_tank()?;
                return Ok(());
            }
            if !self.tank_holds_most_enemies() {
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .remove::<(MoveIntent, server_follower_apl::DirectedOwnerTarget)>();
                return Ok(());
            }
            if let Some(target) = self.chain_tag_target {
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .insert(server_follower_apl::DirectedOwnerTarget(target));
            }
        } else if fighting {
            if self.wait_for_tank()? {
                // The engaged tank cannot come back; bring the leader to it.
                let tank = *self
                    .app
                    .world()
                    .get::<Position>(self.party.tank)
                    .ok_or("cohesion tank absent")?;
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .insert(MoveIntent(Vec3::new(tank.x, tank.y, tank.z)));
                return Ok(());
            }
        }
        if self.chain_groups.is_none() && fighting {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(());
        }
        let next = if let Some(target) = self.chain_tag_target {
            self.current_pack.map(|pack| (pack, target))
        } else if self.chain_groups.is_some() {
            self.next_chain_pack(fighting)
        } else {
            self.next_pack()
        };
        if next.is_none() && fighting {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(());
        }
        let Some((pack_id, target)) = next else {
            if self.has_living_required_boss() {
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .remove::<MoveIntent>();
                return Ok(());
            }
            return Err(if self.keystone.is_some() {
                "planned route exhausted before native keystone criteria completed".into()
            } else {
                "no living hostile source pack remains, but boss deaths are incomplete".into()
            });
        };
        self.current_pack = Some(pack_id);
        if self.chain_groups.is_some() && fighting && self.is_required_boss(target) {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(());
        }
        let target_position = *self.app.world().get::<Position>(target).unwrap();
        let to = Vec3::new(target_position.x, target_position.y, target_position.z);
        let attackable = crate::spell_cast::targets::relation_matches(
            self.app.world(),
            self.party.owner,
            target,
            crate::spell_cast::targets::Relation::Enemy,
        );
        if !attackable && self.is_gated_boss(target) {
            // Production unlocks a gated boss after its gate packs die and may relocate it
            // (Crawth descends from an off-graph perch), so its spawn needs no path.
            self.hold_for_objective(pack_id);
            return Ok(());
        }
        let approach = self.approach_point(pack_id, target, to);
        if self.approach_translocation(approach)? {
            return Ok(());
        }
        let (opener, range) = if self.keystone.is_some() {
            let range = crate::followers::simulator::leader_attack_range(
                self.app.world(),
                self.party.owner,
            )
            .ok_or("keystone owner has no native hostile action range")?;
            (self.party.owner, range)
        } else {
            let (_, range) =
                crate::followers::simulator::opening_spell(self.app.world(), self.party.tank);
            (self.party.tank, range)
        };
        let position = *self
            .app
            .world()
            .get::<Position>(opener)
            .ok_or("native pull opener is absent")?;
        let from = Vec3::new(position.x, position.y, position.z);
        if !attackable {
            if let Some(step) = server::test_support::objectives::next(
                self.app.world_mut(),
                self.party.owner,
                target,
            )? {
                self.waiting_objective = None;
                match step {
                    server::test_support::objectives::InteractionStep::Approach {
                        target, ..
                    } => {
                        let at = *self
                            .app
                            .world()
                            .get::<Position>(target)
                            .ok_or("interaction target lacks position")?;
                        self.app
                            .world_mut()
                            .entity_mut(self.party.owner)
                            .insert(MoveIntent(Vec3::new(at.x, at.y, at.z)));
                    }
                    server::test_support::objectives::InteractionStep::Interact { target } => {
                        self.app
                            .world_mut()
                            .entity_mut(self.party.owner)
                            .remove::<MoveIntent>();
                        server::test_support::objectives::interact(
                            self.app.world_mut(),
                            self.party.owner,
                            target,
                        )?;
                    }
                    server::test_support::objectives::InteractionStep::Wait => {
                        self.app
                            .world_mut()
                            .entity_mut(self.party.owner)
                            .remove::<MoveIntent>();
                    }
                }
                return Ok(());
            }
            let owner = *self
                .app
                .world()
                .get::<Position>(self.party.owner)
                .ok_or("objective owner absent")?;
            let owner_position = Vec3::new(owner.x, owner.y, owner.z);
            if owner_position.distance(to) > 1.0 {
                self.waiting_objective = None;
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .insert(MoveIntent(to));
            } else {
                self.hold_for_objective(pack_id);
            }
            return Ok(());
        }
        self.waiting_objective = None;
        if from.distance(to) > range {
            let destination = if self.chain_groups.is_some() && fighting {
                self.limit_chain_approach(approach)?
            } else {
                approach
            };
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(destination));
            return Ok(());
        }
        if !fighting && self.wait_for_tank()? {
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
            .filter(|spawn| {
                self.actors.get(&spawn.guid).is_some_and(|actor| {
                    crate::spell_cast::targets::is_alive(self.app.world(), *actor)
                })
            })
            .map(|spawn| spawn.guid)
            .collect();
        if self.chain_groups.is_some() && (fighting || self.chain_tag_target.is_some()) {
            self.record_pull(pack_id, "Owner chain pull".into());
            if self.chain_tag_target != Some(target) {
                self.chain_tag_cursor = self
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0
                    .len();
                self.chain_tag_target = Some(target);
            }
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(server_follower_apl::DirectedOwnerTarget(target));
            return Ok(());
        }
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
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<crate::followers::AdmittedPull>();
            return Ok(());
        }
        let mut prospective = vec![target];
        prospective.extend(
            guids
                .iter()
                .filter_map(|guid| self.actors.get(guid).copied())
                .filter(|actor| *actor != target),
        );
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .insert(crate::followers::AdmittedPull(prospective));
        if self
            .party
            .members
            .iter()
            .copied()
            .chain(std::iter::once(self.party.owner))
            .any(|unit| {
                self.app
                    .world()
                    .get::<crate::followers::AplControlled>(unit)
                    .is_some()
                    && self
                        .app
                        .world()
                        .get::<server_follower_apl::FollowerApl>(unit)
                        .is_none_or(|apl| apl.in_precombat())
            })
        {
            return Ok(());
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<crate::followers::AdmittedPull>();
        if self.keystone.is_some() {
            return self.drive_owner_opening(target);
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

    fn tank_holds_most_enemies(&self) -> bool {
        let world = self.app.world();
        let enemies: Vec<_> = crate::followers::simulator::active_enemies(world, self.party.owner)
            .into_iter()
            .filter(|enemy| Some(*enemy) != self.chain_tag_target)
            .collect();
        let held = enemies
            .iter()
            .filter(|&&enemy| {
                world
                    .get::<crate::networking::CombatTarget>(enemy)
                    .and_then(|target| target.0)
                    == Some(self.party.tank)
            })
            .count();
        !enemies.is_empty() && held * 2 > enemies.len()
    }

    fn move_owner_to_tank(&mut self) -> Result<(), String> {
        let position = *self
            .app
            .world()
            .get::<Position>(self.party.tank)
            .ok_or("chain tank has no position")?;
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .insert(MoveIntent(Vec3::new(position.x, position.y, position.z)));
        Ok(())
    }

    fn return_after_chain_tag(&mut self) -> Result<bool, String> {
        let world = self.app.world();
        if self.chain_tag_target.is_some_and(|target| {
            !crate::spell_cast::targets::is_alive(world, target)
                || world
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0[self.chain_tag_cursor..]
                    .iter()
                    .any(|event| {
                        let owner_action = event
                            .source
                            .and_then(Entity::try_from_bits)
                            .is_some_and(|source| {
                                source == self.party.owner
                                    || world
                                        .get::<shared::components::UnitSummonedBy>(source)
                                        .is_some_and(|owner| owner.0 == self.party.owner.to_bits())
                            });
                        let hostile_action = event.kind
                            == shared::protocol::CombatLogKind::CastSuccess
                            || (event.kind == shared::protocol::CombatLogKind::Damage
                                && event.amount > 0);
                        event.target == Some(target.to_bits()) && owner_action && hostile_action
                    })
        }) {
            self.chain_tag_target = None;
            self.chain_returning = true;
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<server_follower_apl::DirectedOwnerTarget>();
        }
        if !self.chain_returning {
            return Ok(false);
        }
        let world = self.app.world();
        if !crate::spell_cast::targets::is_alive(world, self.party.tank) {
            self.chain_returning = false;
            return Ok(false);
        }
        let owner = world
            .get::<Position>(self.party.owner)
            .ok_or("chain owner has no position")?;
        let tank = world
            .get::<Position>(self.party.tank)
            .ok_or("chain tank has no position")?;
        let distance =
            Vec3::new(owner.x, owner.y, owner.z).distance(Vec3::new(tank.x, tank.y, tank.z));
        let reach =
            crate::scripted_ai::model_reach::melee_range(world, self.party.owner, self.party.tank)
                .ok_or("chain reunion lacks native melee reach")?;
        if distance <= reach {
            self.chain_returning = false;
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(false);
        }
        self.move_owner_to_tank()?;
        Ok(true)
    }

    fn limit_chain_approach(&self, destination: Vec3) -> Result<Vec3, String> {
        let world = self.app.world();
        let owner = world
            .get::<Position>(self.party.owner)
            .ok_or("chain owner has no position")?;
        let tank = world
            .get::<Position>(self.party.tank)
            .ok_or("chain tank has no position")?;
        let origin = Vec3::new(owner.x, owner.y, owner.z);
        let center = Vec3::new(tank.x, tank.y, tank.z);
        let (_, range) = crate::followers::simulator::opening_spell(world, self.party.tank);
        let route = world
            .resource::<super::native_movement::NativeMovement>()
            .walk_legs(origin, destination, "chain")?;
        Ok(clip_route_to_tank_range(
            origin,
            center,
            range,
            route.into_iter().map(|(point, _)| point),
        ))
    }

    fn drive_owner_opening(&mut self, target: Entity) -> Result<(), String> {
        drive_owner_apl_opening(self.app.world_mut(), self.party.owner, target)
    }

    fn is_required_boss(&self, target: Entity) -> bool {
        self.app
            .world()
            .get::<shared::components::Npc>(target)
            .is_some_and(|npc| {
                self.graph.boss_entries().iter().any(|entry| {
                    self.graph
                        .follower_boss_sources
                        .get(entry)
                        .copied()
                        .unwrap_or(*entry)
                        == npc.template_id
                })
            })
    }

    fn is_gated_boss(&self, target: Entity) -> bool {
        self.app
            .world()
            .get::<shared::components::Npc>(target)
            .is_some_and(|npc| {
                server::test_support::objectives::boss_gate_entries(self.graph.map_id)
                    .contains_key(&npc.template_id)
            })
    }

    /// Stops the party at a source that is not attackable yet; `stalled` reports it if it stays so.
    fn hold_for_objective(&mut self, pack_id: u64) {
        if self
            .waiting_objective
            .is_none_or(|(pack, _)| pack != pack_id)
        {
            self.waiting_objective = Some((pack_id, self.now()));
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<MoveIntent>();
    }

    /// An unmoved source is approached at its frozen route anchor, which lies on the walk graph
    /// even when the source itself stands off-mesh; a moved source is approached where it is.
    /// A reached anchor that did not engage the source (its contact member may be dead) gives
    /// way to the live source.
    fn approach_point(&mut self, pack_id: u64, target: Entity, live: Vec3) -> Vec3 {
        let unmoved = self.actors.get(&pack_id) == Some(&target)
            && self
                .graph
                .packs
                .iter()
                .find(|pack| pack.id == pack_id)
                .is_some_and(|pack| pack.point().distance(live) <= 1.0);
        let owner = self
            .app
            .world()
            .get::<Position>(self.party.owner)
            .map(|at| Vec3::new(at.x, at.y, at.z));
        let anchor = self
            .graph
            .route
            .iter()
            .find(|step| step.pack == pack_id)
            .map(|step| Vec3::from_array(step.anchor));
        if anchor
            .zip(owner)
            .is_some_and(|(anchor, owner)| owner.distance(anchor) <= 1.0)
        {
            self.anchors_reached.insert(pack_id);
        }
        let reached = self.anchors_reached.contains(&pack_id);
        match anchor {
            Some(anchor) if unmoved && !reached => anchor,
            _ => live,
        }
    }

    fn approach_translocation(&mut self, target: Vec3) -> Result<bool, String> {
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
            .next_translocation(Vec3::new(from.x, from.y, from.z), target)?;
        let Some((source, destination)) = edge else {
            return Ok(false);
        };
        if let Some(flight) = self
            .graph
            .flights
            .iter()
            .find(|edge| edge.from == source && edge.to == destination)
            .cloned()
        {
            let start = Vec3::from_array(*flight.points.first().ok_or("empty registered flight")?);
            if Vec3::new(from.x, from.y, from.z).distance(start) > 0.001 {
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .insert(MoveIntent(start));
                return Ok(true);
            }
            super::movement::forget_party_travel(self.app.world_mut(), self.party.owner);
            super::movement::begin_flight(self.app.world_mut(), self.party.owner, &flight.points)?;
            self.record_pull(source, format!("REGISTERED_FLIGHT {source}->{destination}"));
            return Ok(true);
        }
        if let Some(transport) = self
            .graph
            .transports
            .iter()
            .find(|edge| edge.from == source && edge.to == destination)
            .cloned()
        {
            if transport.action == server::test_support::transport::Action::RecordedDawnbreakerDeck
            {
                let start =
                    Vec3::from_array(transport.departure.ok_or("deck replay lacks departure")?);
                let here = Vec3::new(from.x, from.y, from.z);
                if here.distance(start) > transport.radius {
                    return Err("deck replay party is not at its logged starting point".into());
                }
                super::movement::forget_party_travel(self.app.world_mut(), self.party.owner);
                self.deck_replay = Some(DeckReplay {
                    started_at: self.now(),
                    start,
                    end: Vec3::from_array(transport.landing),
                });
                self.report.deck_population_unsourced = Some(deck_replay_provenance(true));
                self.record_pull(source, "deck_population_unsourced: WCL x3HzDRn6yVjqCbt8 fight 7, 86000–162380ms; grant 166807ms".into());
                return Ok(true);
            }
            let source_objects = matches!(
                transport.action,
                server::test_support::transport::Action::ObjectGossip
                    | server::test_support::transport::Action::ObjectScript
            );
            let sourceless = matches!(
                transport.action,
                server::test_support::transport::Action::CollisionDrop
                    | server::test_support::transport::Action::GatewayProximity
                    | server::test_support::transport::Action::TaxiProximity
            );
            let actors = if source_objects {
                &self.objects
            } else {
                &self.actors
            };
            let npc = if sourceless {
                None
            } else {
                Some(
                    *actors
                        .get(&transport.source_guid)
                        .ok_or("registered transport source actor absent")?,
                )
            };
            let position = if let Some(npc) = npc {
                let at = self
                    .app
                    .world()
                    .get::<Position>(npc)
                    .ok_or("registered transport source lacks position")?;
                Vec3::new(at.x, at.y, at.z)
            } else {
                self.graph
                    .packs
                    .iter()
                    .find(|pack| pack.id == source)
                    .ok_or("registered transport source waypoint absent")?
                    .point()
            };
            if Vec3::new(from.x, from.y, from.z).distance(position) > transport.radius {
                self.app
                    .world_mut()
                    .entity_mut(self.party.owner)
                    .insert(MoveIntent(position));
                return Ok(true);
            }
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            server::test_support::transport::activate(
                self.app.world_mut(),
                self.party.owner,
                npc,
                &transport,
            )?;
            super::movement::forget_party_travel(self.app.world_mut(), self.party.owner);
            self.record_pull(
                source,
                format!("REGISTERED_TRANSPORT {source}->{destination}"),
            );
            return Ok(true);
        }
        Err(format!("unregistered transport {source}->{destination}"))
    }

    fn next_chain_pack(&mut self, fighting: bool) -> Option<(u64, Entity)> {
        loop {
            let group = self.chain_groups.as_ref()?.get(self.chain_group)?;
            if let Some(next) = self.next_pack_in(Some(group), true) {
                return Some(next);
            }
            if fighting || self.next_pack_in(Some(group), false).is_some() {
                return None;
            }
            self.chain_group += 1;
        }
    }

    fn next_pack(&self) -> Option<(u64, Entity)> {
        self.next_pack_in(None, false)
    }

    fn next_pack_in(&self, group: Option<&[u64]>, skip_engaged: bool) -> Option<(u64, Entity)> {
        let boss_only = self
            .keystone
            .as_ref()
            .is_some_and(|key| key.forces_complete(self.app.world()));
        self.next_pack_with_forces(group, skip_engaged, boss_only)
    }

    fn next_pack_with_forces(
        &self,
        group: Option<&[u64]>,
        skip_engaged: bool,
        boss_only: bool,
    ) -> Option<(u64, Entity)> {
        let world = self.app.world();
        let engaged = if skip_engaged {
            crate::followers::simulator::active_enemies(world, self.party.owner)
        } else {
            Vec::new()
        };
        let completed = world
            .get::<crate::maps::InMap>(self.party.owner)
            .map(|map| {
                world
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(*map)
            })
            .unwrap_or(0);
        self.graph.route.iter().find_map(|step| {
            if group.is_some_and(|group| !group.contains(&step.pack)) {
                return None;
            }
            let pack = self.graph.packs.iter().find(|pack| pack.id == step.pack)?;
            let target = pack
                .spawns
                .iter()
                .filter_map(|spawn| {
                    self.actors
                        .get(&spawn.guid)
                        .copied()
                        .map(|actor| (spawn, actor))
                })
                .find(|(spawn, actor)| {
                    let boss_source = self.graph.boss_entries().contains(&spawn.entry)
                        || self
                            .graph
                            .follower_boss_sources
                            .values()
                            .any(|entry| *entry == spawn.entry);
                    if boss_only && !boss_source {
                        return false;
                    }
                    let bits: Vec<_> = self
                        .graph
                        .follower_boss_sources
                        .iter()
                        .filter_map(|(boss, source)| {
                            (*source == spawn.entry)
                                .then(|| self.graph.follower_boss_bits.get(boss))
                                .flatten()
                        })
                        .collect();
                    let pending = (bits.is_empty()
                        || bits.iter().any(|bit| completed & (1 << **bit) == 0))
                        && self
                            .graph
                            .follower_encounter_actor_bits
                            .get(&spawn.entry)
                            .is_none_or(|bit| completed & (1 << *bit) == 0);
                    pending
                        && !engaged.contains(actor)
                        && crate::spell_cast::targets::is_alive(world, *actor)
                        && (boss_source
                            || crate::spell_cast::targets::is_hostile(
                                world,
                                self.party.owner,
                                *actor,
                            ))
                })?
                .1;
            Some((pack.id, target))
        })
    }

    fn has_living_required_boss(&self) -> bool {
        let world = self.app.world();
        self.actors.values().any(|actor| {
            crate::spell_cast::targets::is_alive(world, *actor)
                && world
                    .get::<shared::components::Npc>(*actor)
                    .is_some_and(|npc| self.graph.boss_entries().contains(&npc.template_id))
        })
    }

    fn record_pull(&mut self, pack: u64, decision: String) {
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

    fn observe_route_progress(&mut self) -> bool {
        let routes: BTreeMap<_, _> = self
            .members()
            .filter_map(|unit| {
                super::movement::remaining_route(self.app.world(), unit).map(|route| (unit, route))
            })
            .collect();
        let mut progressed = false;
        for (&unit, &(goal, remaining)) in &routes {
            match self.best_routes.get_mut(&unit) {
                Some((previous_goal, best)) if *previous_goal == goal => {
                    if remaining < *best {
                        *best = remaining;
                        progressed = true;
                    }
                }
                _ => {
                    self.best_routes.insert(unit, (goal, remaining));
                }
            }
        }
        self.best_routes.retain(|unit, _| routes.contains_key(unit));
        progressed
    }

    fn stalled(&mut self) -> bool {
        if let Some((pack, since)) = self.waiting_objective {
            if self.now() - since >= STALL_SECONDS {
                self.report.status = "stalled".into();
                self.report.detail =
                    format!("OBJECTIVE_NOT_ATTACKABLE: pack{pack} remained gated for 60s");
                return true;
            }
            return false;
        }
        let resource_gain = self.observe_resource_gain();
        let route_progress = self.observe_route_progress();
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
        if damage || route_progress || resource_gain {
            self.last_progress = self.now();
        }
        if self.now() - self.last_progress < STALL_SECONDS {
            return false;
        }
        self.report.status = "stalled".into();
        self.report.detail = self.report.pulls.last().map_or_else(
            || "no party damage, decreasing native route distance or HP/mana gain for 60s".into(),
            |pull| {
                format!(
                    "no party damage, decreasing native route distance or HP/mana gain for 60s; last pull: {}",
                    pull.decision
                )
            },
        );
        true
    }
}
