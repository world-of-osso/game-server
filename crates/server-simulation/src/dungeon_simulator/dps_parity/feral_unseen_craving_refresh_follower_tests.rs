//! Actual Unseen impacts extend existing Craving instead of resetting it.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn proc_count(app: &bevy::prelude::App, actor: Entity) -> usize {
    app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
            ) && event.spell_id == Some(1263939)
                && event.source == Some(actor.to_bits())
        })
        .count()
}

fn earn_craving(app: &mut bevy::prelude::App, actor: Entity, target: Entity) -> f64 {
    let previous = proc_count(app, actor);
    for _ in 0..20 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::ComboPoints,
            5.0,
        );
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        let spell =
            crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568).unwrap();
        crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        for _ in 0..12 {
            app.update();
            if proc_count(app, actor) > previous {
                return app
                    .world()
                    .get::<crate::auras::ActiveAuras>(actor)
                    .unwrap()
                    .iter()
                    .find(|a| a.spell_id == 1263939 && a.caster == actor)
                    .unwrap()
                    .expires_at
                    .unwrap();
            }
        }
    }
    panic!("Ravage never earned Unseen Craving");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_unseen_craving_refresh_extends_remaining_duration() {
    let _seed = crate::random::seed_scope(1031263657);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    let first = earn_craving(&mut app, actor, target);
    let second = earn_craving(&mut app, actor, target);
    assert!(
        (second - first - 5.0).abs() < 0.06,
        "two five-point impacts add five seconds to the existing expiry: {first} -> {second}"
    );
}
