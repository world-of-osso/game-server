//! Find an Opening increases the rate of earned Opportunity on a real follower.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

fn opportunity_applications(selected: bool) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 260, 70);
    app.world_mut().entity_mut(actor).remove::<crate::followers::AplControlled>()
        .insert(server::test_support::LiteralParityFixture);
    for _ in 0..5 { app.update(); }
    assert!(crate::auras::has_aura(app.world(), actor, 1259457));
    if !selected {
        crate::auras::remove_auras(app.world_mut(), actor, crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1259457);
    }
    let start = app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len();
    for _ in 0..300 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 193315, Some(target), 0);
    }
    app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0[start..]
        .iter().filter(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(195627)
            && matches!(event.kind, CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed)).count()
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_find_an_opening_increases_opportunity_procs() {
    let baseline = opportunity_applications(false);
    let selected = opportunity_applications(true);
    assert!(baseline > 30, "control must earn actual Opportunity: {baseline}");
    assert!(selected > baseline + 10, "control {baseline}, Find an Opening {selected}");
}
