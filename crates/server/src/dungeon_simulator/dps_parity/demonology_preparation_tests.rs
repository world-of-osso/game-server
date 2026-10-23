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
