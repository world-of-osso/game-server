//! Arcane Surge damage depends on a real follower's mana at cast completion.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::Mana;
use shared::protocol::CombatLogKind;

fn surge_damage(mana_fraction: f32) -> i32 {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 62, 90);
    app.world_mut().entity_mut(actor).remove::<crate::followers::AplControlled>()
        .insert(server::test_support::LiteralParityFixture);
    for _ in 0..5 { app.update(); }
    {
        let mut mana = app.world_mut().get_mut::<Mana>(actor).unwrap();
        mana.current = mana.max * mana_fraction;
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, 365350, Some(target), false).unwrap();
    for _ in 0..120 { app.update(); }
    app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(365350)
            && event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits()))
        .expect("the real Arcane Surge cast hits the dummy").amount
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_arcane_surge_deals_more_damage_with_more_mana() {
    let high = surge_damage(1.0);
    let lower = surge_damage(0.8);
    assert!(high as f32 > lower as f32 * 1.05, "full mana {high}, 80% mana {lower}");
}
