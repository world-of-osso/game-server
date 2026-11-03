use super::*;
use shared::components::PowerType;

fn cast(run: &mut Run, spell: u32, target: Entity) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn observe_expiry(charged: bool) -> (bool, bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    if charged {
        let actor = run.actor;
        cast(&mut run, 13750, actor);
        assert!(crate::auras::has_aura(run.app.world(), actor, 470398));
    }
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 7.0;
    }
    let target = run.targets[0];
    let now = crate::spell_cast::now(run.app.world());
    cast(&mut run, 315341, target);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 315341));
    while crate::spell_cast::now(run.app.world()) < now + 17.0 {
        run.app.update();
    }
    let active_after_17s = crate::auras::has_aura(run.app.world(), run.actor, 315341);
    while crate::spell_cast::now(run.app.world()) < now + 23.0 {
        run.app.update();
    }
    let active_after_23s = crate::auras::has_aura(run.app.world(), run.actor, 315341);
    (active_after_17s, active_after_23s)
}

#[test]
#[ignore = "requires native Outlaw profile and world database"]
fn supercharged_between_the_eyes_keeps_its_full_retail_stack_duration() {
    assert_eq!(observe_expiry(false), (false, false));
    assert_eq!(observe_expiry(true), (true, false));
}
