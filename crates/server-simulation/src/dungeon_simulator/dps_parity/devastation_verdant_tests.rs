//! Verdant Embrace must deliver its own native heal on the selected recipient.
use super::*;

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_verdant_embrace_delivers_actual_native_heal() {
    let mut run = prepare_literal(1467, 1).unwrap();
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 360995, Some(actor), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && event.spell_id == Some(361195)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.amount > 0),
        "actual Verdant Embrace delivers native361195 to its selected ally"
    );
}
