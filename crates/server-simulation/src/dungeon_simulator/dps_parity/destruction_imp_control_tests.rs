//! Actual main-pet engagement through the literal player controller.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Npc, UnitIsPet};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player profile"]
fn destruction_imp_literal_controller_orders_actual_main_pet_firebolts() {
    // SimC76f8118 pets.cpp360–383: native main Imp action list is Firebolt.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 1).unwrap();
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let imp = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("literal precombat must summon its actual configured Imp");
    assert_eq!(run.app.world().get::<Npc>(imp).unwrap().template_id, 416);
    assert!(run.app.world().get::<UnitIsPet>(imp).unwrap().0);
    let reaction = run
        .app
        .world()
        .get::<crate::pet_control::PetCharm>(imp)
        .unwrap()
        .react;
    engage(&mut run);
    for _ in 0..200 {
        run.app.update();
        drive_player(&mut run).unwrap();
    }
    let expected_identity = (
        Some(imp.to_bits()),
        Some(run.targets[0].to_bits()),
        Some(3110),
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let identity = (event.source, event.target, event.spell_id);
            let is_damage = event.kind == CombatLogKind::Damage;
            identity == expected_identity && is_damage && event.amount > 0
        })
        .count();
    assert!(
        hits >= 2,
        "literal controller must order repeated actual3110 casts from its native main Imp; hits={hits}"
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::pet_control::PetCharm>(imp)
            .unwrap()
            .react,
        reaction,
        "an attack order must not replace the actual saved reaction stance"
    );
}
