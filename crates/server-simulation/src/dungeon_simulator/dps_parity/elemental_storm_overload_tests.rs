//! SimC76f8118 sc_shaman.cpp3254–3257,6315–6322,7320–7327.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use crate::auras::RemoveReason;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(spell: u32, retain_stormkeeper: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        1
    ));
    settle(&mut run);
    let stormkeeper = own_aura(&run, 191634).expect("real native Stormkeeper");
    let instance = stormkeeper.instance_id;
    let percent = stormkeeper
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount as f64;
    assert_eq!(percent, 150.0, "native70448 Stormkeeper effect1");
    if !retain_stormkeeper {
        crate::auras::remove_instance(
            run.app.world_mut(),
            run.actor,
            instance,
            RemoveReason::Consumed,
        );
        // Ending Stormkeeper separately earns Flowing Elements; exclude that
        // independent modifier from the no-Stormkeeper control.
        if let Some(flowing) = own_aura(&run, 1300219) {
            let instance = flowing.instance_id;
            crate::auras::remove_instance(
                run.app.world_mut(),
                run.actor,
                instance,
                RemoveReason::Consumed,
            );
        }
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _impact_seed = crate::random::seed_scope(70448);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..80 {
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
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .expect("native overload impact");
    (hit.amount as f64, percent)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_stormkeeper_empowers_real_lightning_overloads() {
    for spell in [45284, 45297] {
        let (ordinary, _) = sample(spell, false);
        let (empowered, percent) = sample(spell, true);
        // SimC explicitly sets Stormkeeper for Lightning Bolt Overload. Chain
        // Overload relies on its native mask, which does not match this effect.
        let expected = if spell == 45284 {
            1.0 + percent / 100.0
        } else {
            1.0
        };
        // Native overload damage has 5% variance; bound both independent rolls.
        let spread = 0.025;
        let ratio = empowered / ordinary;
        assert!(
            ratio >= expected * (1.0 - spread) / (1.0 + spread)
                && ratio <= expected * (1.0 + spread) / (1.0 - spread),
            "spell{spell} native overload Stormkeeper: {empowered}/{ordinary} expected {expected}"
        );
    }
}
