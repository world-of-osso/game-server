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
    // The instant finisher has resolved; waiting for its GCD also permits real
    // white hits to earn new storage before the consumption assertion.
    let (points, _) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints)
            .unwrap();
    assert_eq!(
        points, 5.0,
        "earned Ancient Arts refills stored points after paying the finisher"
    );
    let remaining: Vec<_> = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == 196911)
        .map(|aura| {
            (
                aura.caster.to_bits(),
                aura.stacks,
                aura.instance_id,
                aura.source.clone(),
            )
        })
        .collect();
    let events: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && (event.spell_id == Some(196911) || event.spell_id.is_none())
        })
        .cloned()
        .collect();
    assert!(
        remaining.is_empty(),
        "all five stored points were collected: {remaining:?}, events: {events:?}"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1269163),
        "successful collection consumes the earned window"
    );
}
