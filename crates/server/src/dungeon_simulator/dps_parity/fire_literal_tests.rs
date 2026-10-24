//! Native Player boundary, not a source-shape or driver-tag assertion.
use super::{advance_combat, prepare};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Player;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "actual native90 Fire profile/privateDB; pinned literal precombat before300s combat"]
fn fire_literal_native90_orders_real_precombat_casts() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(63, 1).expect("native90 Fire profile admission");
    assert!(run.app.world().get::<Player>(run.actor).is_some());
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let (seconds, _, _) = advance_combat(&mut run)
        .expect("Fire Fixture must execute literal rows, not reject an SBA predictive rule");
    assert!((seconds - 300.0).abs() < 0.001);
    let casts: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    let intellect = casts
        .iter()
        .position(|id| *id == 1459)
        .expect("native Arcane Intellect");
    let images = casts
        .iter()
        .position(|id| *id == 55342)
        .expect("native Mirror Image");
    let pyroblast = casts
        .iter()
        .position(|id| *id == 11366)
        .expect("ordinary precombat Pyroblast");
    assert!(
        intellect < images && images < pyroblast,
        "source-ordered native precombat: {casts:?}"
    );
}
