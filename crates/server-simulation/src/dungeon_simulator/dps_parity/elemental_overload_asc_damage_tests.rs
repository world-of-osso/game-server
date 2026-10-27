//! SimC76f8118 sc_shaman.cpp4884–4891: Ascendance increases elemental overload damage.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(ascendance: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let percent = if ascendance {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            1219480,
            Some(run.actor),
            false,
        )
        .unwrap();
        settle(&mut run);
        own_aura(&run, 1219480)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 7)
            .unwrap()
            .amount as f64
    } else {
        0.0
    };
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _cast_seed = crate::random::seed_scope(70448);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        45284,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..30 {
        run.app.update();
    }
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(45284)
                && event.kind == CombatLogKind::Damage
        })
        .expect("native Lightning Overload hit");
    (hit.amount as f64, percent)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_overload_ascendance_uses_native_damage_bonus() {
    let (ordinary, _) = sample(false);
    let (empowered, percent) = sample(true);
    let ratio = empowered / ordinary;
    let expected = 1.0 + percent / 100.0;
    assert!(
        (ratio - expected).abs() < 0.02,
        "native Ascendance overload bonus: {empowered}/{ordinary} = {ratio}; expected {expected}"
    );
}
