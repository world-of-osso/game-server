use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_earned_opportunity_pays_native_fan_shots_and_consumes_each_use() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        {
            let mut powers = run.app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(run.actor).unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(run.app.world_mut(), run.actor,
            literal_rotation::execution::Execution::Spell(193315, run.targets[0], Some(run.targets[0]))).unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        if crate::auras::has_aura(run.app.world(), run.actor, 195627) { break; }
    }
    let stacks = run.app.world().get::<crate::auras::ActiveAuras>(run.actor).unwrap()
        .iter().find(|aura| aura.spell_id == 195627).expect("real Sinister Strike must earn Opportunity").stacks;
    let fan = crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 381846, 0) as u8;
    assert!(stacks > 1 && fan > 0, "selected native Fan the Hammer must earn extra uses");
    for _ in 0..10 { run.app.update(); }
    {
        let mut powers = run.app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(run.actor).unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
    }
    run.app.world_mut().resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.clear();
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(185763, run.targets[0], Some(run.targets[0]))).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..12 { run.app.update(); }
    let hits = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter()
        .filter(|event| event.source == Some(run.actor.to_bits()) && event.target == Some(run.targets[0].to_bits())
            && event.kind == CombatLogKind::Damage && event.spell_id == Some(185763) && event.amount > 0).count();
    let expected = 1 + usize::from((stacks - 1).min(fan));
    // SimC76f8118 sc_rogue.cpp5035–5069: one root plus up to the native
    // Fan count, launched100ms apart; each shot spends one earned stack.
    assert_eq!(hits, expected, "native Pistol Shot must pay its earned Fan shots (stacks={stacks},fan={fan})");
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 195627),
        "root and Fan shots must consume all the earned uses");
}
