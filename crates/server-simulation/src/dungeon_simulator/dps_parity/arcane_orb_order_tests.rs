//! Real Orb Barrage execution must fund damage, but not change frozen recipients.
use super::arcane_tests::{expected_charge_bonus, grant, normalize_charge_hit_rolls, start_cast};
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::{ObservedCombatEvents, ObservedTriggeredCasts};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const SEEDS: &[u64] = &[70448, 1, 2, 3, 4, 5, 6, 7];

struct BarrageSample {
    damage: f32,
    per_charge: f32,
    recipients: usize,
}

fn load_orb_barrage_fixture(charges: f32, targets: usize) -> Run {
    let mut run = prepare_literal(62, targets).unwrap();
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        PowerType::ArcaneCharges,
        charges,
    )
    .unwrap();
    // Keep native chance, cap and effect amounts. Both comparison cases begin
    // at the same real Salvo cap, so Orb's Salvo grant cannot alter their ratio.
    grant(&mut run, 1242974, 20);
    run.app.insert_resource(ObservedTriggeredCasts::default());
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        44425,
        "orb_barrage_order",
        1,
    ));
    run
}

fn sample_orb_barrage(charges: f32, targets: usize, seed: u64) -> Option<BarrageSample> {
    let mut run = load_orb_barrage_fixture(charges, targets);
    let per_charge = expected_charge_bonus(&run, 44425);
    let _seed = crate::random::seed_scope(seed);
    start_cast(&mut run, 44425);
    preparation::finish_pending_setup(&mut run).unwrap();
    let orb_executed = run
        .app
        .world()
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .any(|&(caster, spell, _)| caster == run.actor && spell == 153626);
    if !orb_executed {
        return None;
    }
    Some(read_barrage_sample(&run, per_charge))
}

fn read_barrage_sample(run: &Run, per_charge: f32) -> BarrageSample {
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(44425)
                && event.kind == CombatLogKind::Damage
        })
        .collect::<Vec<_>>();
    let primary = hits
        .iter()
        .find(|event| event.target == Some(run.targets[0].to_bits()))
        .expect("ordinary Barrage must hit its actual primary");
    BarrageSample {
        damage: normalize_charge_hit_rolls(run, primary.amount),
        per_charge,
        recipients: hits.len(),
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_orb_order_barrage_damage_includes_the_orb_root_charge() {
    // SimC76f8118 sc_mage.cpp2659–2662/2722–2731: the proc Orb root
    // grants one charge before Barrage snapshots damage and spends charges.
    // Later Orb bolt impacts are separate; they are not this launch-time grant.
    for &seed in SEEDS {
        let Some(zero) = sample_orb_barrage(0.0, 1, seed) else {
            continue;
        };
        let Some(one) = sample_orb_barrage(1.0, 1, seed) else {
            continue;
        };
        assert_eq!(zero.per_charge, one.per_charge);
        let expected = zero.damage * (1.0 + 2.0 * zero.per_charge) / (1.0 + zero.per_charge);
        assert!(
            (one.damage - expected).abs() <= expected * 0.005 + 2.0,
            "native Orb proc: before0/1 must snapshot1/2; seed={seed}, zero={}, one={}, per_charge={}, expected={expected}",
            zero.damage,
            one.damage,
            zero.per_charge
        );
        return;
    }
    panic!("no native Orb Barrage proc observed in the bounded seeded cases");
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_orb_order_barrage_recipients_keep_pre_orb_charges() {
    // sc_mage.cpp2711–2714/2722–2728 freezes recipients before the Orb,
    // even though that same Orb must increase the later damage snapshot.
    for &seed in SEEDS {
        if let Some(sample) = sample_orb_barrage(0.0, 3, seed) {
            assert_eq!(sample.recipients, 1, "Orb must not add a Barrage recipient");
            return;
        }
    }
    panic!("no native Orb Barrage proc observed in the bounded seeded cases");
}
