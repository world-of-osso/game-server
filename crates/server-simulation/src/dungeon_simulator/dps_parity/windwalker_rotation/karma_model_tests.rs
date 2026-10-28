//! Player-literal Karma models SimC's explicitly assumed absorbed damage.
use super::super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "300s actual player literal fixture with modeled Karma damage"]
fn windwalker_player_karma_uses_modeled_absorption_and_100s_interval() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(269, 1).unwrap();
    let health = run
        .app
        .world()
        .get::<shared::components::Health>(run.actor)
        .unwrap()
        .max;
    let cap =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 122470, 2)
            / 100.0;
    let redirect =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 122470, 3)
            / 100.0;
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(122470)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    let ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(124280)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(
        casts, 3,
        "the native admitted action uses SimC's 100s default interval"
    );
    assert_eq!(
        ticks.len(),
        18,
        "three modeled redirects each deal six native ticks"
    );
    let first_total: i32 = ticks.iter().take(6).map(|event| event.amount).sum();
    let expected = health * 0.5_f32.min(cap) * redirect;
    assert!(
        first_total as f32 >= expected && (first_total as f32) < expected * 1.25,
        "the first actual redirect {first_total} must use native max health and versatility: {expected}"
    );
}
