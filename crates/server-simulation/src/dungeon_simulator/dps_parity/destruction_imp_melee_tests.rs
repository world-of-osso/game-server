//! The controlled Imp casts Firebolt and never auto-attacks.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_controlled_imp_casts_firebolt_without_melee() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 1).unwrap();
    let owner = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    let imp = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(imp)
            .unwrap()
            .template_id,
        416
    );
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let from_imp = |spell: Option<u32>| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(imp.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && !event.periodic
                    && event.spell_id == spell
            })
            .count()
    };
    assert!(from_imp(Some(3110)) > 0, "ordered Imp must land Firebolts");
    // SimC76f8118 sc_warlock_pets.cpp360–385: imp_pet_t has no melee_attack;
    // Felhunter, Voidwalker and Felguard create one (342/431/503).
    assert_eq!(from_imp(None), 0, "controlled Imp must not auto-attack");
}
