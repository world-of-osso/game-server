//! Actual player dispatch must accept the same native granted action as selection.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_selected_thistle_tea_grant_casts_for_player() {
    let mut run = prepare_literal(259, 1).unwrap();
    let spell =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 381623)
            .unwrap();
    assert_eq!(spell, 381623, "selected native Thistle Tea action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(381623)
        }),
        "the selected talent's granted spell must complete an ordinary Player cast"
    );
    let rejected = crate::spell_cast::cast_for_player_with_targets(
        run.app.world_mut(),
        run.actor,
        116,
        crate::spell_cast::CastTargets {
            unit: run.targets[0],
            primary: Some(run.targets[0]),
            destination: None,
        },
    );
    assert!(
        matches!(rejected, Err(crate::spell_cast::CastFailure::NotKnown)),
        "the same Rogue must reject the unlearned Mage action as NotKnown: {rejected:?}"
    );
}
