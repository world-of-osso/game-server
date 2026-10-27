//! SimC76f8118 sc_shaman.cpp3265–3267/6795/7323–7324.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use crate::auras::RemoveReason;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(spell: u32, retain_flowing: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, 1296626).is_some(),
        "native MID2 set equipped"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    cast(&mut run, 188196);
    cast(&mut run, 188196);
    let aura = own_aura(&run, 1300219).expect("two real Stormkeeper uses earn Flowing Elements");
    assert_eq!(aura.stacks, 2);
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    assert_eq!(percent, 25.0, "native70448 Flowing Elements amount");
    let instance = aura.instance_id;
    if !retain_flowing {
        crate::auras::remove_instance(
            run.app.world_mut(),
            run.actor,
            instance,
            RemoveReason::Consumed,
        );
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _damage_seed = crate::random::seed_scope(70448);
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
        .expect("actual native overload damage");
    (hit.amount as f64, percent)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_flowing_elements_empowers_real_bolt_and_lava_overloads() {
    for spell in [45284, 285466] {
        let (ordinary, _) = sample(spell, false);
        let (empowered, percent) = sample(spell, true);
        let expected = 1.0 + percent / 100.0;
        let spread = 0.025;
        let ratio = empowered / ordinary;
        assert!(
            ratio >= expected * (1.0 - spread) / (1.0 + spread)
                && ratio <= expected * (1.0 + spread) / (1.0 - spread),
            "overload{spell}: {empowered}/{ordinary} expected {expected}"
        );
    }
}
