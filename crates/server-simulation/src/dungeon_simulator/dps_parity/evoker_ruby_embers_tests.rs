//! Living Flame applies its periodic effect only with selected Ruby Embers.
use super::*;

const LIVING_FLAME: u32 = 361469;
const LIVING_FLAME_DAMAGE: u32 = 361500;
const RUBY_EMBERS: u32 = 365937;

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_ruby_embers_gates_actual_living_flame_periodic_damage() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, RUBY_EMBERS),
            "the 1467 profile does not select Ruby Embers"
        );
        if selected {
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                RUBY_EMBERS,
                Some(actor),
                1
            ));
        }
        let before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            actor,
            LIVING_FLAME,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..400 {
            run.app.update();
        }
        let ticks = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..]
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(primary.to_bits())
                    && event.spell_id == Some(LIVING_FLAME_DAMAGE)
                    && event.periodic
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count();
        // TC a352b1fa spell_evoker.cpp620–647 and SimC76f8118 sc_evoker.cpp5137–5138:
        // without Ruby Embers the periodic effect1 selects no target.
        assert_eq!(
            ticks > 0,
            selected,
            "selected={selected}: Living Flame periodic ticks {ticks}"
        );
    }
}
