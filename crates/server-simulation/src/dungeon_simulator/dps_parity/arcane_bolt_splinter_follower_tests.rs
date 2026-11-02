use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

fn cast_and_count_splinters(spell: u32) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 62, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>()
        .insert(server::test_support::LiteralParityFixture);
    let known = app.world().get::<crate::class_progression::KnownSpells>(actor).unwrap();
    assert!(known.contains(470021), "Signature Spell is selected");
    assert!(!known.contains(444675), "Shifting Shards is not selected");
    for _ in 0..5 { app.update(); }
    if spell == 1295924 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 1295942, Some(actor), 1);
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false).unwrap();
    for _ in 0..120 { app.update(); }
    let events = &app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.source == Some(actor.to_bits())
        && event.spell_id == Some(spell) && event.kind == CombatLogKind::Damage), "source spell hit the target");
    events.iter().filter(|event| event.source == Some(actor.to_bits())
        && event.spell_id == Some(443763) && event.kind == CombatLogKind::Damage).count()
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_bolt_does_not_conjure_signature_splinters() {
    assert_eq!(cast_and_count_splinters(1295924), 0);
    assert!(cast_and_count_splinters(30451) > 0, "Arcane Blast still conjures splinters");
}
