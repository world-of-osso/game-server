//! Earned Unseen Predator's Craving uses the selected talent's rank-scaled value.
use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::spell_mods::SpellModOp;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_unseen_craving_increases_damage_by_selected_rank() {
    let _seed = crate::random::seed_scope(1031263657);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    let percent = crate::spell_cast::scripts::script_effect_value(app.world(), actor, 1263658, 0);
    assert_eq!(percent, 15.0, "selected rank2 native trait curve");
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
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
    }
    for _ in 0..15 {
        app.update();
    }
    assert!(
        crate::auras::has_aura(app.world(), actor, 1263939),
        "Unseen impacts earn Craving"
    );
    let store = app.world().resource::<crate::spell_info::SpellStore>();
    let direct = store.get(22568).unwrap().clone();
    let rip = store.get(1079).unwrap().clone();
    let read = |app: &bevy::prelude::App| {
        let world = app.world();
        (
            crate::auras::spell_mods::modify(
                world,
                actor,
                &direct,
                SpellModOp::HealingAndDamage,
                100.0,
            ),
            crate::auras::spell_mods::modify(
                world,
                actor,
                &rip,
                SpellModOp::PeriodicHealingAndDamage,
                100.0,
            ),
            world
                .get::<crate::auras::AuraStatMods>(actor)
                .unwrap()
                .autoattack_damage_mult,
        )
    };
    let with = read(&app);
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |a| a.spell_id == 1263939,
    );
    let without = read(&app);
    for (label, actual, baseline) in [
        ("Bite", with.0, without.0),
        ("Rip", with.1, without.1),
        ("white", with.2, without.2),
    ] {
        assert!(
            (actual / baseline - 1.15).abs() < 0.001,
            "{label} must gain rank2 15%: {actual}/{baseline}"
        );
    }
}
