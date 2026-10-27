//! Actual Boomstick ticks earn selected Mongoose Rounds once, not per target.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_boomstick_ticks_earn_mongoose_rounds_once_per_tick() {
    // SimC76f8118 sc_hunter.cpp6333–6341: each real channel tick emits
    // Boomstick damage, then selected Mongoose Rounds grants native stacks once.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 3).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1253945));
    cast(&mut run, 186270, false);
    let before = owned_stacks(&run);
    assert!(before > 0, "actual Raptor must earn Mongoose Fury before the channel");
    let start = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len();
    cast(&mut run, 1261193, false);
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0[start..];
    let ticks = events.iter().filter(|event| event.source == Some(run.actor.to_bits())
        && event.target == Some(run.targets[0].to_bits()) && event.spell_id == Some(1261215)
        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))).count();
    let stacks_per_tick = crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1253945, 0) as usize;
    assert!(ticks > 0 && stacks_per_tick > 0, "real native channel ticks and selected stacks are required");
    assert_eq!(usize::from(owned_stacks(&run)), usize::from(before) + ticks * stacks_per_tick,
        "real3-target Boomstick must earn Mongoose stacks once per native tick");
}

fn owned_stacks(run: &Run) -> u8 {
    run.app.world().get::<crate::auras::ActiveAuras>(run.actor).unwrap().iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 259388)
        .map_or(0, |aura| aura.stacks)
}
