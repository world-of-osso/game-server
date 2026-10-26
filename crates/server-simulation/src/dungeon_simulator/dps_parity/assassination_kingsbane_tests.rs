//! SimC76f8118 sc_rogue.cpp4921–4945: default bugs=1 Kingsbane double dips.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

struct KingsbaneSample {
    direct: f32,
    periodic: f32,
    mastery: f32,
    implacable_direct: f32,
    implacable_periodic: f32,
}

fn sample_kingsbane(disabled: Option<u32>) -> KingsbaneSample {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let native_amount = |spell, index| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell && aura.caster == actor)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount
    };
    let mastery = native_amount(76803, 0);
    let implacable_direct = native_amount(1265386, 1);
    let implacable_periodic = native_amount(1265386, 2);
    if let Some(spell) = disabled {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            RemoveReason::Unlearned,
            |aura| aura.spell_id == spell && aura.caster == actor,
        );
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 385627, Some(target), 1);
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage = |periodic| {
        events
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(385627)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic == periodic
            })
            .expect("actual native Kingsbane direct and periodic damage")
            .amount as f32
    };
    KingsbaneSample {
        direct: damage(false),
        periodic: damage(true),
        mastery,
        implacable_direct,
        implacable_periodic,
    }
}

fn assert_damage_ratio(actual: f32, base: f32, percent: f32) {
    let expected = base * (1.0 + percent / 100.0).powi(2);
    assert!(
        (actual - expected).abs() <= 4.0,
        "native Kingsbane must apply the sourced multiplier twice: {actual}/{expected} (base{base},native{percent}%)"
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_kingsbane_direct_double_dips_native_mastery() {
    let full = sample_kingsbane(None);
    let without = sample_kingsbane(Some(76803));
    assert!(full.mastery > 0.0);
    assert_damage_ratio(full.direct, without.direct, full.mastery);
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_kingsbane_direct_and_periodic_double_dip_native_implacable() {
    let full = sample_kingsbane(None);
    let without = sample_kingsbane(Some(1265386));
    assert!(full.implacable_direct > 0.0 && full.implacable_periodic > 0.0);
    assert_damage_ratio(full.direct, without.direct, full.implacable_direct);
    assert_damage_ratio(full.periodic, without.periodic, full.implacable_periodic);
}
