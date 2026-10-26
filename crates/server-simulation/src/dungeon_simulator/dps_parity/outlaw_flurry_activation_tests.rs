use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_blade_flurry_activation_hits_each_native_enemy_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 3).unwrap();
    engage(&mut run);
    run.app.world_mut().resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.clear();
    literal_rotation::execution::execute(
        run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(13877, run.actor, None),
    ).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..10 { run.app.update(); }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    for target in &run.targets {
        let hits: Vec<_> = events.iter().filter(|event|
            event.kind == CombatLogKind::Damage && event.spell_id == Some(429951)
            && event.source == Some(run.actor.to_bits()) && event.target == Some(target.to_bits())
            && event.amount > 0).collect();
        // SimC76f8118 sc_rogue.cpp3993–3999: activating Blade Flurry
        // executes one independent native area attack, including the primary.
        assert_eq!(hits.len(), 1, "native Blade Flurry activation must hit {target:?} once");
    }
}
