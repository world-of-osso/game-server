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
    .expect("ordinary native warrior cast");
    super::super::preparation::finish_pending_setup(run).unwrap();
}

fn execute_ready(run: &Run) -> bool {
    let world = run.app.world();
    let execute = world
        .resource::<crate::spell_info::SpellStore>()
        .get(280735)
        .unwrap();
    !world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .on_cooldown(execute, crate::spell_cast::now(world))
}

#[test]
#[ignore = "requires exact Fury72 native player inputs"]
fn fury_execute_consumes_one_sudden_death_stack() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    // Isolate consumption from a fresh foreground RPPM grant on this Execute.
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 29725,
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 52437, Some(actor), 1);
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 52437)
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(stacks(&run), 1, "native Sudden Death applied");
    cast_native(&mut run, 280735, target);
    assert_eq!(
        stacks(&run),
        0,
        "ordinary Execute consumes one Sudden Death stack"
    );
}

#[test]
#[ignore = "requires exact Fury72 native player inputs"]
fn fury_sudden_death_foreground_special_grants_buff_and_resets_execute() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    // Isolate the native29725 producer from Slayer's independent SD reward.
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 444767 | 444769),
    );
    super::super::engage(&mut run);
    super::super::literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        super::super::literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    for _ in 0..40 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 190411, Some(actor), 1);
        for _ in 0..40 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 52437),
            "autos and background specials do not roll Sudden Death"
        );
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(
                |event| event.kind == shared::protocol::CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
            ),
        "the auto-attack negative control must include real white impacts"
    );
    let maximum = run
        .app
        .world()
        .get::<shared::components::Health>(target)
        .unwrap()
        .max;
    for _ in 0..160 {
        run.app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap()
            .current = maximum * 0.1;
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::Rage,
            1300.0,
        )
        .unwrap();
        let (spell, recipient) = if execute_ready(&run) {
            (280735, target)
        } else {
            (190411, actor)
        };
        cast_native(&mut run, spell, recipient);
        if crate::auras::has_aura(run.app.world(), actor, 52437) {
            assert!(
                execute_ready(&run),
                "Sudden Death resets actual Execute cooldown"
            );
            return;
        }
    }
    panic!(
        "foreground special executions must produce native Sudden Death in a seeded 160-cast run"
    );
}
