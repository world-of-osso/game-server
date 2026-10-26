//! Earned Sudden Death resets the actor's actual native Execute endpoint.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use shared::components::{Health, PowerType};

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sudden_death_resets_the_active_execute_variant() {
    for massacre in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(71, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        {
            let mut health = run.app.world_mut().get_mut::<Health>(primary).unwrap();
            health.current = health.max * 0.1;
        }
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::Rage)
            .unwrap()
            .current = 400.0;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 29725 || (!massacre && aura.spell_id == 281001),
        );
        let resolved =
            crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 163201)
                .unwrap();
        assert_eq!(resolved, if massacre { 281000 } else { 163201 });
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(163201, primary, Some(primary)),
        )
        .unwrap();
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(resolved)
            .unwrap()
            .clone();
        let remaining = |run: &Run| {
            let world = run.app.world();
            world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap()
                .remaining_cooldown_ms(&spell, crate::spell_cast::history_now(world))
        };
        assert!(
            remaining(&run) > 0,
            "ordinary Execute starts its native cooldown"
        );
        // Restore the selected passive only after admission so unrelated
        // foreground RPPM cannot reset the cooldown under test.
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 29725, Some(actor), 1);
        for _ in 0..3 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 445579, Some(primary), 1);
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 52437));
        assert_eq!(
            remaining(&run),
            0,
            "Sudden Death must reset the actual native Execute endpoint {resolved}"
        );
    }
}
