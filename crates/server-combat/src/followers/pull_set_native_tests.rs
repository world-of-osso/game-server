//! Native Windrunner assistance membership at pull admission.
use bevy::prelude::*;
use shared::components::{Health, Level, Position, UnitLevel};
use std::collections::HashMap;

#[test]
#[ignore = "requires Windrunner native world, ground and LOS data"]
fn windrunner_assistance_snapshot_is_planned_and_tank_can_attack_it() {
    let (mut h, tank, profile) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 73,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805115 | 63805164 | 63805165))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 3);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&root.join("los")).expect("native LOS"),
    );
    let mut ground = crate::ground::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 1,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        3
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.advance(2);
    let units: HashMap<_, _> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .map(|(entity, origin)| (origin.guid, entity))
        .collect();
    let caller = units[&63805115];
    let helpers = [units[&63805164], units[&63805165]];
    let here = Position {
        x: 5312.1387,
        y: 126.344864,
        z: 3142.1423,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90), here));
    h.app.world_mut().entity_mut(tank).insert((map, here));
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    for unit in [tank, caller, helpers[0], helpers[1]] {
        let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.max = 1_000_000.0;
        health.current = health.max;
    }
    let planned = super::pull_set::members(h.app.world(), caller, tank);
    assert_eq!(planned.len(), 3);
    assert!(planned.contains(&caller));
    assert!(helpers.iter().all(|helper| planned.contains(helper)));
    assert!(
        !super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "area attacks wait for the first-wave dispatch"
    );
    crate::creature_aggro::attack_start(h.app.world_mut(), caller, tank);
    for _ in 0..60 {
        h.advance(1);
        if helpers.iter().all(|&helper| {
            h.app
                .world()
                .get::<crate::creature_assistance::AssistanceCalled>(helper)
                .is_some()
        }) {
            break;
        }
    }
    assert!(
        helpers
            .iter()
            .all(|&helper| crate::combat_refs::has_active_reference_to(
                h.app.world(),
                helper,
                tank
            )),
        "planned assistants must join the tank's combat at dispatch"
    );
    assert!(
        super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "the tank may use its native chain after planned assistants dispatch"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), tank, 31935, Some(caller), false)
        .expect("the tank's native Avenger's Shield must start against the planned pull");
    for _ in 0..100 {
        h.advance(1);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(31935)
                && helpers
                    .iter()
                    .any(|helper| event.target == Some(helper.to_bits()))
                && event.kind == shared::protocol::CombatLogKind::Damage
        }) {
            break;
        }
    }
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(31935)
                && helpers
                    .iter()
                    .any(|helper| event.target == Some(helper.to_bits()))
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "native Avenger's Shield must actually damage a planned helper"
    );
    let after = super::pull_set::members(h.app.world(), caller, tank);
    assert!(
        helpers.iter().all(|helper| after.contains(helper)),
        "the admission snapshot must survive delayed arrival"
    );
}

#[test]
#[ignore = "requires Windrunner native world, ground and LOS data"]
fn windrunner_assistants_do_not_call_second_wave_and_tank_avoids_it() {
    let (mut h, tank, profile) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 73,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805113 | 63805114 | 63805162))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 3);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&root.join("los")).expect("native LOS"),
    );
    let mut ground = crate::ground::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 1,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        3
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.advance(2);
    let units: HashMap<_, _> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .map(|(entity, origin)| (origin.guid, entity))
        .collect();
    let caller = units[&63805113];
    let helper = units[&63805114];
    let dragonhawk = units[&63805162];
    let here = Position {
        x: 5316.415,
        y: 127.7347,
        z: 3124.3594,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90), here));
    h.app.world_mut().entity_mut(tank).insert((map, here));
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let mut health = h.app.world_mut().get_mut::<Health>(tank).unwrap();
    health.current = health.max;
    drop(health);
    let first = super::pull_set::first_wave(h.app.world(), caller, tank);
    assert!(first.contains(&helper));
    assert!(!first.contains(&dragonhawk));
    let planned = super::pull_set::members(h.app.world(), caller, tank);
    let sources = [
        (63805113, caller),
        (63805114, helper),
        (63805162, dragonhawk),
    ]
    .into_iter()
    .map(|(guid, actor)| {
        (
            guid,
            super::pull_sources::Source {
                guid,
                entry: 0,
                pack_guid: guid,
                position: Vec3::ZERO,
                actor: Some(actor),
            },
        )
    })
    .collect();
    let projected = super::pull_admission::expand_assistance(
        h.app.world(),
        tank,
        &std::collections::BTreeSet::from([63805113]),
        &sources,
    )
    .unwrap();
    assert_eq!(
        projected,
        std::collections::BTreeSet::from([63805113, 63805114])
    );
    assert!(
        !planned.contains(&dragonhawk),
        "an assistant's own potential helper is not in the first-wave plan"
    );
    data.formations.insert(
        63805114,
        crate::world_data::FormationInfo {
            leader_guid: 63805113,
            follow_dist: 0.0,
            follow_angle: 0.0,
            group_ai: 3,
        },
    );
    h.app.insert_resource(data.clone());
    assert!(
        super::pull_set::members(h.app.world(), caller, tank).contains(&dragonhawk),
        "formation members contribute their own immediate assistance snapshot"
    );
    data.formations.remove(&63805114);
    h.app.insert_resource(data);
    assert!(
        super::pull_set::risky_pending_helpers(h.app.world(), caller, tank).contains(&helper),
        "directly hitting the pending helper could call the Dragonhawk"
    );
    assert!(
        !super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "AoE must not hit the helper before dispatch"
    );
    assert!(
        super::spell_safety::safe(h.app.world(), tank, caller, 62124),
        "single-target taunt remains available"
    );
    crate::creature_aggro::attack_start(h.app.world_mut(), caller, tank);
    let pet = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    let faction = *h
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(tank)
        .unwrap();
    h.app.world_mut().entity_mut(pet).insert((
        map,
        here,
        faction,
        crate::hunter_pet::HunterPet {
            owner: tank,
            slot: 0,
        },
        crate::pet_control::PetCharm::new(Default::default(), shared::protocol::REACT_ASSIST),
    ));
    crate::pet_control::attack_start(h.app.world_mut(), pet, caller);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(pet)
            .unwrap()
            .0,
        Some(caller),
        "the pet can attack the caller"
    );
    h.app
        .world_mut()
        .entity_mut(caller)
        .remove::<crate::combat::AutoAttack>();
    h.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::combat::AutoAttack::new(helper));
    let selected = crate::pet_control::select_next_target(h.app.world_mut(), tank, pet);
    assert_eq!(
        selected, None,
        "pet target selection must skip the pending helper"
    );
    h.app
        .world_mut()
        .entity_mut(tank)
        .remove::<crate::combat::AutoAttack>();
    crate::pet_control::attack_start(h.app.world_mut(), pet, helper);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(pet)
            .unwrap()
            .0,
        Some(caller),
        "a follower pet must not attack a pending helper"
    );
    assert!(
        !super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "AoE remains unsafe while assistance is pending"
    );
    for _ in 0..60 {
        h.advance(1);
        if h.app
            .world()
            .get::<crate::creature_assistance::AssistanceCalled>(helper)
            .is_some()
        {
            break;
        }
    }
    assert!(
        h.app
            .world()
            .get::<crate::creature_assistance::AssistanceCalled>(helper)
            .is_some(),
        "the native delayed dispatch must join the helper"
    );
    crate::pet_control::attack_start(h.app.world_mut(), pet, helper);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(pet)
            .unwrap()
            .0,
        Some(helper),
        "the pet can attack the helper after dispatch"
    );
    assert!(
        super::pull_set::risky_pending_helpers(h.app.world(), caller, tank).is_empty(),
        "the first-wave pending restriction ends after native dispatch"
    );
    assert!(
        !super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "the ordinary AoE guard must still protect the unplanned Dragonhawk"
    );
    assert!(crate::combat_refs::has_active_reference_to(
        h.app.world(),
        helper,
        tank
    ));
    // TC 15e0ed76 Creature.cpp:288-300 marks an assistant no-call before engagement.
    // Wait beyond another native 1.5-second delay to catch an unintended second wave.
    h.advance(80);
    assert!(
        crate::combat_refs::active_opponents(h.app.world(), dragonhawk)
            .next()
            .is_none(),
        "the second wave must not enter combat after the first-wave dispatch"
    );
}

#[test]
#[ignore = "requires Windrunner native world, ground and LOS data"]
fn windrunner_hammer_chain_guard_protects_outside_first_wave() {
    let (mut h, tank, profile) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 73,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805160 | 63805200 | 63805206))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 3);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&root.join("los")).expect("native LOS"),
    );
    let mut ground = crate::ground::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 1,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        3
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.advance(2);
    let units: HashMap<_, _> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .map(|(entity, origin)| (origin.guid, entity))
        .collect();
    let caller = units[&63805160];
    let helper = units[&63805200];
    let outside = units[&63805206];
    let here = Position {
        x: 5343.4077,
        y: 101.11535,
        z: 3201.2021,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90), here));
    h.app.world_mut().entity_mut(tank).insert((map, here));
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let mut health = h.app.world_mut().get_mut::<Health>(tank).unwrap();
    health.current = health.max;
    drop(health);
    let first = super::pull_set::members(h.app.world(), caller, tank);
    assert!(first.contains(&helper));
    assert!(!first.contains(&outside));
    crate::creature_aggro::attack_start(h.app.world_mut(), caller, tank);
    for _ in 0..60 {
        h.advance(1);
        if h.app
            .world()
            .get::<crate::creature_assistance::AssistanceCalled>(helper)
            .is_some()
        {
            break;
        }
    }
    assert!(
        h.app
            .world()
            .get::<crate::creature_assistance::AssistanceCalled>(helper)
            .is_some()
    );
    assert!(!super::pull_set::members(h.app.world(), helper, tank).contains(&outside));
    // Native Sweeping Verdict modifies Hammer of Wrath's chain target count.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: tank,
            target: tank,
            spell_id: 1246488,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::KnownSpell,
        },
    );
    let reach =
        super::unintended_reach::spell_unintended_reach(h.app.world(), tank, 1241413, helper)
            .unwrap();
    assert!(
        reach.1 >= 10.0,
        "modified Hammer of Wrath must report chain reach: {reach:?}"
    );
    assert!(
        !super::spell_safety::safe(h.app.world(), tank, helper, 1241413),
        "the native Hammer chain must not reach a creature outside the first wave"
    );
    assert!(
        crate::combat_refs::active_opponents(h.app.world(), outside)
            .next()
            .is_none()
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), tank, 1241413, Some(helper), false)
        .expect("native Hammer of Wrath must cast on the helper");
    for _ in 0..100 {
        h.advance(1);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(1241413)
                && event.target == Some(outside.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
        }) {
            return;
        }
    }
    panic!("native Hammer of Wrath did not damage the outside creature");
}
