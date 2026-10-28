//! Native target-dependent Assassination damage, not reference coefficient substitutions.
use super::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_systemic_failure_amplifies_masked_hits_only_on_owned_garrote() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let percent = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 381652 && aura.caster == actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    let capture = |run: &mut Run, spell: u32| {
        run.app
            .insert_resource(FactorCapture::new(actor, target, spell, "systemic", 1));
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            spell,
            Some(target),
            1
        ));
        let receipt = run.app.world().resource::<FactorCapture>().export();
        let factors = receipt["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == spell)
            .unwrap()["applied_multipliers"]
            .as_array()
            .unwrap();
        factors
            .iter()
            .find(|factor| factor["factor"] == "giantkiller")
            .unwrap()["value"]
            .as_f64()
            .unwrap()
    };
    let clean = capture(&mut run, 5374);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 703, Some(target), 1);
    assert!(crate::auras::has_aura(run.app.world(), target, 703));
    let affected = capture(&mut run, 5374);
    assert!(
        (affected / clean - (1.0 + percent / 100.0)).abs() < 0.001,
        "Garrote grants its native target bonus once: {clean} -> {affected}"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 703 && aura.caster == actor,
    );
    let removed = capture(&mut run, 5374);
    assert!(
        (removed - clean).abs() < 0.001,
        "removing Garrote removes the bonus"
    );
}

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

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_rupture_duration_scales_through_seven_combo_points() {
    use shared::components::PowerType;
    for points in [5.0, 6.0, 7.0] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(259, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
                .unwrap();
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
            let combo = pools.get_mut(PowerType::ComboPoints).unwrap();
            assert!(combo.max >= points);
            combo.current = points;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            actor,
            literal_rotation::execution::Execution::Spell(1943, target, Some(target)),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1943 && a.caster == actor)
            .expect("Rupture must land");
        assert_eq!(aura.duration_ms, (4000.0 * (1.0 + points)) as u32);
    }
}

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_inspiring_strike_requires_envenom_refresh() {
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1250036));
    let fill = |run: &mut Run| {
        let mut pools = run.app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(actor).unwrap();
        for power in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(power).unwrap();
            pool.current = pool.max;
        }
    };
    let cast = |run: &mut Run| {
        literal_rotation::execution::execute(run.app.world_mut(), actor,
            literal_rotation::execution::Execution::Spell(32645, target, Some(target))).unwrap();
        preparation::finish_pending_setup(run).unwrap();
    };
    let bonus = |run: &Run| run.app.world().get::<crate::auras::ActiveAuras>(actor).unwrap().iter()
        .find(|a| a.spell_id == 32645 && a.caster == actor).unwrap().effects.iter()
        .find(|effect| effect.index == 9).unwrap().amount;
    fill(&mut run);
    cast(&mut run);
    let first = bonus(&run);
    let first_white = run.app.world().get::<crate::auras::AuraStatMods>(actor).unwrap().autoattack_damage_mult;
    for _ in 0..25 { run.app.update(); }
    fill(&mut run);
    cast(&mut run);
    let refreshed = bonus(&run);
    let refreshed_white = run.app.world().get::<crate::auras::AuraStatMods>(actor).unwrap().autoattack_damage_mult;
    assert!((refreshed_white / first_white - 1.05).abs() < 0.001, "refresh must increase actual auto-attack damage by 5%");
    assert!((refreshed - (first + 5.0 * (1.0 + first / 100.0))).abs() < 0.001,
        "the native 5% Inspiring Strike multiplier applies only after refresh: {first} -> {refreshed}");
    for _ in 0..200 { run.app.update(); }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 32645));
    fill(&mut run);
    cast(&mut run);
    assert!((bonus(&run) - first).abs() < 0.001, "a fresh Envenom cannot retain Inspiring Strike");
}
