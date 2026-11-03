//! Frantic Momentum requires paid Feral finishers, not every damaging event.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_sudden_ambush_rolls_per_landed_five_point_finisher() {
    let _seed = crate::random::seed_scope(103384667);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 70);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 384667, Some(actor), 1);
    assert!(crate::auras::has_aura(app.world(), actor, 384667));
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    let mut grants = 0;
    for _ in 0..100 {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1244483,
        );
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::ComboPoints,
            5.0,
        );
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 22568, Some(target), false)
            .unwrap();
        for _ in 0..3 {
            app.update();
        }
        grants += usize::from(crate::auras::has_aura(app.world(), actor, 1244483));
    }
    assert!(
        (17..=43).contains(&grants),
        "expected about 30 grants from 100 landed finishers, got {grants}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_frantic_momentum_rolls_only_on_finishers() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 70);
    assert!(
        crate::auras::has_aura(app.world(), actor, 391875),
        "Frantic Momentum selected"
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
        !crate::auras::has_aura(app.world(), actor, 391876),
        "builders cannot grant Frantic Momentum"
    );
    for _ in 0..30 {
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::ComboPoints,
            5.0,
        );
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 22568, Some(target), false)
            .unwrap();
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
            matches!(
                event.kind,
                CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
            ) && event.spell_id == Some(391876)
                && event.source == Some(actor.to_bits())
        })
        .count();
    assert!(
        count > 0 && count < 15,
        "five-point finishers roll, rather than guarantee, a proc: {count}"
    );
}
