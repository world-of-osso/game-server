use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use shared::components::{Health, PowerType};

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_massacre_consumes_one_sudden_death_use_and_grants_imminent_demise() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 163201),
        Ok(281000),
        "the real selected Massacre override must be active"
    );
    {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * 0.1;
    }
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::Rage)
        .unwrap()
        .current = 400.0;
    for _ in 0..2 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 52437, Some(actor), 1);
    }
    let stacks = |run: &Run, spell: u32| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell && aura.caster == actor)
            .map_or(0, |aura| aura.stacks)
    };
    // At native max2, an incidental foreground RPPM grant cannot change the
    // before-consumption count. This also catches consuming the damage child.
    assert_eq!(stacks(&run, 52437), 2);
    assert_eq!(stacks(&run, 445606), 0);
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(163201, target, Some(target)),
    )
    .unwrap();
    assert_eq!(
        stacks(&run, 52437),
        1,
        "one actual Massacre execution consumes exactly one native Sudden Death use"
    );
    assert_eq!(
        stacks(&run, 445606),
        1,
        "the existing native Imminent Demise consumer includes the active Massacre action"
    );
}
