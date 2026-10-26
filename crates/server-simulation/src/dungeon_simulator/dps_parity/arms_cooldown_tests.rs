//! Real paid casts and earned Slayer states must drive native Arms recovery.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use shared::components::{Health, PowerType};

fn fund_rage(run: &mut Run) {
    run.app
        .world_mut()
        .get_mut::<PowerPools>(run.actor)
        .unwrap()
        .get_mut(PowerType::Rage)
        .unwrap()
        .current = 1000.0;
}

fn advance_frames(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn cooldown_ms(run: &Run, action: u32) -> u64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(action)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_anger_management_reduces_avatar_and_smash_from_actual_paid_rage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(107574, actor, Some(target)),
    )
    .unwrap();
    advance_frames(&mut run, 40);
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(167105, target, Some(target)),
    )
    .unwrap();
    advance_frames(&mut run, 40);
    fund_rage(&mut run);
    let before = [cooldown_ms(&run, 107574), cooldown_ms(&run, 167105)];
    assert!(before.iter().all(|remaining| *remaining > 0));
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(12294, target, Some(target)),
    )
    .unwrap();
    let remaining_rage = run
        .app
        .world()
        .get::<PowerPools>(actor)
        .unwrap()
        .get(PowerType::Rage)
        .unwrap()
        .current;
    let paid = 1000.0 - remaining_rage;
    assert!(paid > 0.0, "the native Mortal Strike must really pay Rage");
    let units = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Rage)
        .unwrap()
        .display_modifier;
    let divisor =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 152278, 0);
    let expected_ms = f64::from(paid / units as f32 / divisor) * 1000.0;
    for (action, before) in [107574, 167105].into_iter().zip(before) {
        let reduction = before as f64 - cooldown_ms(&run, action) as f64;
        assert!(
            (reduction - expected_ms).abs() <= 1.0,
            "paid Rage must reduce native{action}: got{reduction}ms expected{expected_ms}ms"
        );
    }
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sudden_death_onslaught_reduces_bladestorm_and_applies_overwhelmed() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(227847, actor, Some(target)),
    )
    .unwrap();
    advance_frames(&mut run, 200);
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 445584 | 52437),
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 445836,
    );
    for _ in 0..3 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 445579, Some(target), 1);
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 52437),
        "three actual Slayer executions earn SD"
    );
    let executioner_stacks = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 445584 && aura.caster == actor)
        .unwrap()
        .stacks;
    assert!(executioner_stacks > 0);
    {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * 0.1;
    }
    fund_rage(&mut run);
    let before = cooldown_ms(&run, 446035);
    assert!(before > 0, "actual Bladestorm started its cooldown");
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(163201, target, Some(target)),
    )
    .unwrap();
    let seconds_per_stack =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 444780, 0);
    let expected_ms = (seconds_per_stack * f32::from(executioner_stacks) * 1000.0) as u64;
    assert_eq!(
        before - cooldown_ms(&run, 446035),
        expected_ms,
        "earned SD Execute must reduce live Bladestorm by native seconds per Executioner stack"
    );
    let overwhelmed = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 445836 && aura.caster == actor)
        .map_or(0, |aura| aura.stacks);
    assert!(
        overwhelmed >= executioner_stacks,
        "Onslaught applies native Overwhelmed stacks to the actual primary"
    );
}

fn used_overpower_charges(run: &Run, category: u32) -> u8 {
    crate::spell_cast::test_harness::charge_recovery(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap(),
        category,
    )
    .0
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_tactician_actual_paid_slam_restores_spent_overpower_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let category = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(7384)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(7384, target, Some(target)),
    )
    .unwrap();
    advance_frames(&mut run, 40);
    let mut resets = 0;
    let mut opportunist_seen = false;
    for _ in 0..40 {
        if used_overpower_charges(&run, category) == 0 {
            execute(
                run.app.world_mut(),
                actor,
                Execution::Spell(7384, target, Some(target)),
            )
            .unwrap();
            advance_frames(&mut run, 40);
        }
        fund_rage(&mut run);
        let before = used_overpower_charges(&run, category);
        assert!(
            before > 0,
            "a real Overpower charge is spent before the paid Slam"
        );
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(1464, target, Some(target)),
        )
        .unwrap();
        resets += usize::from(used_overpower_charges(&run, category) < before);
        opportunist_seen |= crate::auras::has_aura(run.app.world(), actor, 456120);
        advance_frames(&mut run, 40);
    }
    assert!(
        resets > 0,
        "native Tactician must synchronously restore spent Overpower charges after actual paid Slams"
    );
    assert!(
        opportunist_seen,
        "selected Opportunist is earned by Tactician, not a fixture grant"
    );
}
