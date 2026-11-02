//! Tiger's Fury must persist on a follower's applied Rip after the buff ends.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

fn rip_snapshot(empower: bool) -> (f32, f32) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    if empower {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 5217, Some(actor), 1);
        assert!(crate::auras::has_aura(app.world(), actor, 5217));
    }
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 5.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1079, Some(target), false).unwrap();
    let read = |app: &bevy::prelude::App| {
        let aura = app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1079 && a.caster == actor)
            .expect("owned Rip");
        let effect = aura.effects.iter().find(|e| e.index == 0).unwrap();
        (effect.amount, effect.persistent_multiplier.unwrap())
    };
    let snapshot = read(&app);
    if empower {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Expired,
            |a| a.spell_id == 5217,
        );
        assert_eq!(
            read(&app),
            snapshot,
            "existing Rip retains its application snapshot"
        );
    }
    snapshot
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_rip_snapshots_tigers_fury() {
    let _seed = crate::random::seed_scope(70448);
    let ordinary = rip_snapshot(false);
    let empowered = rip_snapshot(true);
    assert!(
        (empowered.1 / ordinary.1 - 1.25).abs() < 0.001,
        "Tiger's Fury snapshot factor: {empowered:?} vs {ordinary:?}"
    );
    assert!(
        (empowered.0 / ordinary.0 - 1.25).abs() < 0.001,
        "Tiger's Fury must increase Rip tick amount: {empowered:?} vs {ordinary:?}"
    );
}
