//! Real native out-of-combat preparation and first-pull lifecycle; no literal controller.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

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
