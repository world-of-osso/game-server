//! Native Reap's fragment consumption and delayed damage boundary.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_reap_consumes_active_lesser_fragments_and_hits() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    for _ in 0..2 {
        soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false)
            .expect("native lesser fragment");
    }
    for _ in 0..40 {
        if soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0 {
            break;
        }
        run.app.update();
    }
    let before = soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser);
    assert!(
        before > 0,
        "real lesser fragments must be active before Reap"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1226019,
        Some(target),
    )
    .expect("real native Reap admission");
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1226019)
                && event.kind == CombatLogKind::CastSuccess),
        "Reap must actually execute before checking consumption"
    );
    // SimC76f8118 sc_demon_hunter.cpp5981–6018,6068–6107: consume active
    // lesser fragments on execute and deliver native1225823 after 220ms.
    let after = soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser);
    assert!(
        after < before,
        "Reap must consume actual lesser fragments: {before} -> {after}"
    );
    for _ in 0..40 {
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(1225823)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1225823)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "Reap must deliver its native delayed damage"
    );
}
