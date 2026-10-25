use super::*;

fn cast_native(run: &mut Run, spell: u32, target: Entity) {
    super::super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        super::super::literal_rotation::execution::Execution::Spell(
            spell,
            target,
            Some(run.targets[0]),
        ),
    )
    .expect("normal native cast");
    super::super::preparation::finish_pending_setup(run).unwrap();
}

#[test]
#[ignore = "requires exact Arms71 player profile and native selected Deep Wounds"]
fn arms_deep_wounds_still_applies_from_native_mortal_strike() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_literal(71, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Rage, 1300.0)
        .unwrap();
    cast_native(&mut run, 12294, target);
    assert!(
        crate::auras::has_aura(run.app.world(), target, 262115),
        "Fury's proc replacement must not disable Arms Mortal Strike's native producer"
    );
}

#[test]
#[ignore = "requires exact Fury72 player profile and native selected Deep Wounds"]
fn fury_deep_wounds_is_applied_by_execute_not_whirlwind() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    cast_native(&mut run, 190411, actor);
    for _ in 0..40 {
        run.app.update();
    }
    let target = run.targets[0];
    assert!(
        !crate::auras::has_aura(run.app.world(), target, 262115),
        "Fury Deep Wounds is an Execute impact relationship, not a Whirlwind proc"
    );
    let maximum = run
        .app
        .world()
        .get::<shared::components::Health>(target)
        .unwrap()
        .max;
    run.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .current = maximum * 0.1;
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Rage, 1300.0)
        .unwrap();
    cast_native(&mut run, 280735, target);
    assert!(
        crate::auras::has_aura(run.app.world(), target, 262115),
        "actual Fury Execute main-hand must retain native Deep Wounds"
    );
}
