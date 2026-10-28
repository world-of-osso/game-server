//! SimC76f8118 sc_shaman.cpp4884–4891: Ascendance increases elemental overload damage.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(ascendance: bool) -> (f64, f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1219480,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    let aura = own_aura(&run, 1219480).unwrap();
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 7)
        .unwrap()
        .amount as f64;
    let instance = aura.instance_id;
    if !ascendance {
        crate::auras::remove_instance(
            run.app.world_mut(),
            run.actor,
            instance,
            crate::auras::RemoveReason::Cancelled,
        );
        // Ending Ascendance earns Flowing Elements; remove that independent
        // modifier so this comparison isolates Ascendance's damage bonus.
        if let Some(flowing) = own_aura(&run, 1300219) {
            let instance = flowing.instance_id;
            crate::auras::remove_instance(
                run.app.world_mut(),
                run.actor,
                instance,
                crate::auras::RemoveReason::Consumed,
            );
        }
    }
    let mastery = own_aura(&run, 168534).unwrap().effects.iter()
        .find(|effect| effect.index == 1).unwrap().amount as f64;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _cast_seed = crate::random::seed_scope(70448);
    for _ in 0..100 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            45284,
            Some(run.targets[0]),
            1
        ));
    }
    for _ in 0..30 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(45284)
                && event.kind == CombatLogKind::Damage
                && !event.crit
                && event.amount > 0
        })
        .map(|event| event.amount as f64)
        .collect();
    assert!(
        hits.len() >= 20,
        "native noncritical Lightning Overload sample"
    );
    (hits.iter().sum::<f64>() / hits.len() as f64, percent, mastery)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_overload_ascendance_uses_native_damage_bonus() {
    let (ordinary, _, ordinary_mastery) = sample(false);
    let (empowered, percent, ascended_mastery) = sample(true);
    assert_eq!(ordinary_mastery, ascended_mastery, "Ascendance must not also change overload mastery");
    let ratio = empowered / ordinary;
    let expected = 1.0 + percent / 100.0;
    assert!(
        (ratio - expected).abs() < 0.05,
        "native Ascendance overload bonus: {empowered}/{ordinary} = {ratio}; expected {expected}"
    );
}
