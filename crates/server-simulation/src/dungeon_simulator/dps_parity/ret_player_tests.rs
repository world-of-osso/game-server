//! The measured Ret player uses the pinned literal rotation.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires frozen native 70 player profile"]
fn ret_player_uses_literal_rotation_for_three_targets() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(70, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    assert!(run.spec_driver.is_some());
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    assert!(run.player_rotation.is_none());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(53385)
                && event.kind == CombatLogKind::CastSuccess),
        "the pinned multitarget APL executes native Divine Storm"
    );
}
