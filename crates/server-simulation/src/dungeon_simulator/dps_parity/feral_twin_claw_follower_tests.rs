//! Twin Claw follows actual selected Feral melee ability impacts.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_melee_abilities_earn_twin_claw() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 1271635),
        "Twin Claw selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    for _ in 0..30 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 5221, Some(target), false)
            .unwrap();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let count = |spell| {
        events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(spell)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    let shreds = count(5221);
    let claws = count(1271636);
    assert!(shreds >= 20, "real landed Shreds: {shreds}");
    assert!(
        claws > 0 && claws < shreds / 2,
        "Twin Claw is earned but not guaranteed: {claws}/{shreds}"
    );
}
