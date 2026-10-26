//! Native target-dependent Assassination damage, not reference coefficient substitutions.
use super::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

struct PoisonSample {
    damage: f32,
    dose_percent: f32,
    execute_percent: f32,
}

fn sample_native_poison(debuff: Option<u32>, health_fraction: f32) -> PoisonSample {
    let _setup_seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    if let Some(spell) = debuff {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, spell, Some(target), 1);
        assert!(crate::auras::has_aura(run.app.world(), target, spell));
    }
    let amount = |spell| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount
    };
    let dose_percent = amount(381640);
    let execute_percent = amount(381798);
    {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.max = 1_000_000.0;
        health.current = health.max * health_fraction;
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    // Reset only the impact RNG, so both native outcomes have the same crit/variance.
    let _impact_seed = crate::random::seed_scope(259);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        315585,
        Some(target),
        1
    ));
    // Native training dummies report damage but deliberately lose no Health.
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(315585)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| event.amount as f32)
        .sum::<f32>();
    assert!(
        damage > 0.0,
        "native poison must actually damage its recipient"
    );
    PoisonSample {
        damage,
        dose_percent,
        execute_percent,
    }
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_target_lethal_dose_counts_a_real_owned_nonlethal_poison() {
    let clean = sample_native_poison(None, 0.8);
    let poisoned = sample_native_poison(Some(3409), 0.8);
    let expected = clean.damage * (1.0 + poisoned.dose_percent / 100.0);
    assert!(
        (poisoned.damage - expected).abs() <= 2.0,
        "owned native Crippling Poison must add Lethal Dose: {} vs{expected}",
        poisoned.damage
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_target_zoldyck_uses_real_target_health_and_native_amount() {
    let healthy = sample_native_poison(None, 0.8);
    let executing = sample_native_poison(None, 0.2);
    let expected = healthy.damage * (1.0 + executing.execute_percent / 100.0);
    assert!(
        (executing.damage - expected).abs() <= 2.0,
        "native Zoldyck damage must apply below its health threshold: {} vs{expected}",
        executing.damage
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_target_kingsbane_does_not_count_for_lethal_dose() {
    let clean = sample_native_poison(None, 0.8);
    let kingsbane = sample_native_poison(Some(385627), 0.8);
    assert!(
        (kingsbane.damage - clean.damage).abs() <= 2.0,
        "pinned Kingsbane exclusion must preserve native poison damage: {} vs{}",
        kingsbane.damage,
        clean.damage
    );
}
