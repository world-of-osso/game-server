//! Claw Rampage is earned by Berserk melee impacts, not ordinary builders.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_claw_rampage_requires_berserk() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 441835),
        "Claw Rampage selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    for _ in 0..20 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 5221, Some(target), false)
            .unwrap();
    }
    assert!(
        !crate::auras::has_aura(app.world(), actor, 441585),
        "ordinary Shreds cannot grant Claw Rampage"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 106951, Some(actor), 1);
    assert!(crate::auras::has_aura(app.world(), actor, 106951));
    for _ in 0..30 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 5221, Some(target), false)
            .unwrap();
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |a| a.spell_id == 441585,
        );
        for _ in 0..3 {
            app.update();
        }
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let count = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::AuraApplied
                && event.spell_id == Some(441585)
                && event.source == Some(actor.to_bits())
        })
        .count();
    assert!(
        count > 0 && count < 20,
        "Berserk Shreds sometimes grant Ravage, not every time: {count}"
    );
}
