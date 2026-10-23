//! Real native out-of-combat preparation and first-pull lifecycle; no literal controller.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native level90 actual Hand of Guldan imp attack boundary; private guardian-template DB required"]
fn dps_parity_demo_native_guldan_imps_attack_the_selected_dummy() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare(266, 3).unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    engage(&mut run);
    let mut observed = std::collections::HashSet::new();
    for tick in 0..1200 {
        run.app.update();
        let world = run.app.world_mut();
        let imps: Vec<_> = world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter(|(_, summon)| summon.owner == run.actor && summon.spell_id == 104317)
            .map(|(entity, _)| entity)
            .collect();
        for imp in imps {
            if observed.insert(imp) {
                let spell = world
                    .resource::<crate::spell_info::SpellStore>()
                    .get(104318)
                    .unwrap()
                    .clone();
                let owner_target = world
                    .get::<crate::networking::CombatTarget>(run.actor)
                    .and_then(|target| target.0);
                let admission =
                    crate::spell_cast::read_native_cast_start(world, imp, &spell, run.targets[0]);
                let costs = crate::spell_cast::applicable_costs(world, imp, &spell);
                println!(
                    "DEMO_IMP tick={tick} entity={imp:?} owner_target={owner_target:?} admission={admission:?} costs={costs:?}"
                );
                println!(
                    "DEMO_IMP_STATE level={:?} owner_level={:?} faction={:?} map={:?} target_map={:?} enemy={}",
                    world.get::<shared::components::UnitLevel>(imp),
                    world.get::<shared::components::Level>(run.actor),
                    world.get::<shared::components::UnitFactionTemplate>(imp),
                    world.get::<crate::maps::InMap>(imp),
                    world.get::<crate::maps::InMap>(run.targets[0]),
                    crate::spell_cast::targets::relation_matches(
                        world,
                        imp,
                        run.targets[0],
                        crate::spell_cast::targets::Relation::Enemy
                    )
                );
            }
        }
    }
    assert!(
        !observed.is_empty(),
        "ordinary native Hand of Guldan must actually spawn Wild Imps"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(104318)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hits > 0,
        "actual level90 native Wild Imps must attack the selected dummy; observed {} actors",
        observed.len()
    );
}

#[test]
#[ignore = "native first10s ST/3T engagement regression; private spec266 inputs required"]
fn dps_parity_demo_native_first_pull_survives_inflight_precombat_cast() {
    let _seed = crate::random::seed_scope(70448);
    let mut outcomes = Vec::new();
    for count in [1, 3] {
        let mut run = prepare(266, count).unwrap();
        engage(&mut run);
        let started = crate::spell_cast::now(run.app.world());
        let mut events_seen = 0;
        let mut casts = Vec::new();
        let mut samples = Vec::new();
        for tick in 1..=200 {
            run.app.update();
            let world = run.app.world();
            let now = crate::spell_cast::now(world) - started;
            let events = &world.resource::<ObservedCombatEvents>().0;
            casts.extend(events[events_seen..].iter().filter_map(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::CastStart | CombatLogKind::CastSuccess
                )
                .then(|| json!({"at": now, "event": event}))
            }));
            events_seen = events.len();
            if tick % 20 == 0 {
                let owner = fixture_player(&run);
                samples.push(json!({"at": now,
                    "cast": format!("{:?}", world.get::<shared::casting::CastState>(run.actor)),
                    "target": world.get::<crate::networking::CombatTarget>(run.actor).and_then(|target| target.0).map(Entity::to_bits),
                    "owner_reference": crate::combat_refs::has_active_reference_to(world, owner, run.targets[0]),
                    "actor_reference": crate::combat_refs::has_active_reference_to(world, run.actor, run.targets[0]),
                    "pet": crate::hunter_pet::summoned_pet(world, run.actor).map(Entity::to_bits)}));
            }
        }
        let damage = report::combat(run.app.world(), &run.damage_targets, 10.0)["total_damage"]
            .as_u64()
            .unwrap();
        outcomes
            .push(json!({"targets": count, "damage": damage, "casts": casts, "samples": samples}));
    }
    let path = inputs::directory()
        .unwrap()
        .join("266.first-engagement-10s.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&outcomes).unwrap()).unwrap();
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome["damage"].as_u64().unwrap() > 0),
        "native first pull must reach actual damage in ST and3T; trace {}",
        path.display()
    );
}

fn advance_until_cast_finished(run: &mut Run, unit: Entity) {
    for _ in 0..400 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(unit)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("native resource-control cast never finished");
}

fn cast_and_finish(run: &mut Run, unit: Entity, spell: u32) {
    let target = run.targets[0];
    for _ in 0..400 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            unit,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                advance_until_cast_finished(run, unit);
                return;
            }
            Err(
                crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(error) => panic!("native resource-control spell{spell}: {error:?}"),
        }
    }
    panic!("native resource-control spell{spell} never became ready");
}

#[test]
#[ignore = "native class-follower and Felguard regen control; requires private DB and spec266 profile"]
fn dps_parity_demo_native_follower_and_felguard_keep_resource_regeneration() {
    use shared::components::{Mana, Position, PowerType};
    let mut run = prepare(266, 1).unwrap();
    let controller = fixture_player(&run);
    run.app.world_mut().entity_mut(controller).insert(Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    for _ in 0..200 {
        run.app.update();
    }
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::pet_control::PetCharm>(pet)
        .unwrap()
        .react = shared::protocol::REACT_PASSIVE;
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, None);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    // Exclusive fixture controller prevents unrelated casts from spending the
    // pools while ordinary native spells/regeneration exercise both real roles.
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 686);
    let before = run.app.world().get::<Mana>(actor).unwrap().current;
    assert!(before < run.app.world().get::<Mana>(actor).unwrap().max);
    for _ in 0..20 {
        run.app.update();
    }
    let after = run.app.world().get::<Mana>(actor).unwrap().current;
    assert!(
        after > before,
        "real class-follower mana must regenerate after native Shadow Bolt"
    );
    // This control isolates regeneration, not movement: place the real pet at
    // its real target before spending Energy through native Legion Strike.
    let target_position = *run.app.world().get::<Position>(run.targets[0]).unwrap();
    run.app.world_mut().entity_mut(pet).insert(target_position);
    cast_and_finish(&mut run, pet, 30213);
    let (before, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Energy).unwrap();
    assert!(
        before < maximum,
        "native Legion Strike must spend Felguard Energy"
    );
    let rate = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Energy)
        .unwrap()
        .regen_combat;
    for _ in 0..20 {
        run.app.update();
    }
    let after = crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Energy)
        .unwrap()
        .0;
    assert!(
        (after - before - rate).abs() < 0.01,
        "real controlled Felguard keeps its native per-second Energy rate: before{before}, after{after}, rate{rate}"
    );
}

fn summon_count(world: &World, owner: Entity) -> usize {
    world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(owner.to_bits())
                && event.spell_id == Some(30146)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count()
}

#[test]
#[ignore = "native Demo OOC pet preparation and first pull; requires private DB and spec266 profile"]
fn dps_parity_demo_native_prepares_felguard_once_and_attacks_first_pull() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare(266, 1).unwrap();
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_none());
    for _ in 0..200 {
        run.app.update();
    }
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("native kit must finish Felguard preparation out of combat");
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252,
        "authentic Felguard, not a substituted guardian"
    );
    assert_eq!(summon_count(run.app.world(), run.actor), 1);
    engage(&mut run);
    for _ in 0..400 {
        run.app.update();
    }
    assert_eq!(
        crate::hunter_pet::summoned_pet(run.app.world(), run.actor),
        Some(pet)
    );
    assert_eq!(
        summon_count(run.app.world(), run.actor),
        0,
        "engage clears precombat logs; native kit must not resummon its existing pet"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage: i64 = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
                && event.source == Some(run.actor.to_bits())
        })
        .map(|event| i64::from(event.amount))
        .sum();
    assert!(
        damage > 0,
        "prepared native Demo must damage the first pull"
    );
}
