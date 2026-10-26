//! SimC76f8118 sc_rogue.cpp8668–8701: earned Ancient Arts lets a
//! successful finisher collect stored points after its ordinary payment.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_ancient_arts_finisher_collects_after_payment_and_consumes_window() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    for spell in [196911, 1269163] {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert!(crate::auras::set_self_cast_stacks(
        run.app.world_mut(),
        run.actor,
        196911,
        5
    ));
    let (_, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        pools.get_mut(PowerType::ComboPoints).unwrap().current = maximum;
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(196819, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let (points, _) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    assert_eq!(
        points, 5.0,
        "earned Ancient Arts refills stored points after paying the finisher"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 196911),
        "all five stored points were collected"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1269163),
        "successful collection consumes the earned window"
    );
}
