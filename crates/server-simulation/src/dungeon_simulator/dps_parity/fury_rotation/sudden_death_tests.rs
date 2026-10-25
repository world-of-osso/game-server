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
fn fury_sudden_death_foreground_special_grants_buff_and_resets_execute() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    super::super::engage(&mut run);
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
